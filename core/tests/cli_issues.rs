mod support;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{
    fs,
    time::{Duration, SystemTime},
};
use support::*;
fn search(
    cli: &Cli,
    folder: &std::path::Path,
    query: &str,
    flags: &[&str],
    spec: Option<&Value>,
) -> Run {
    let run = cli.run(folder, query, "xlsx", spec, flags);
    run.accepted("completed");
    assert!(run.issues.is_empty());
    run
}
#[test]
fn visible_text_order_timestamp_and_index_upgrade() {
    let cli = Cli::new("issues");
    docs_search_test_support::specialized::issues(&cli.root).unwrap();
    let folder = cli.root.join("excel");
    let path = folder.join("layout.xlsx");
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(
            fs::FileTimes::new()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1700000000)),
        )
        .unwrap();
    let mut prior = None;
    for flags in [vec![], vec!["--use-index"], vec!["--use-index"]] {
        let run = search(&cli, &folder, "ORDER", &flags, None);
        let ranking = run.rankings()[0];
        let locations: Vec<_> = ranking["resultIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| {
                let hit = run.hits.iter().find(|h| &h["resultId"] == id).unwrap();
                json!([hit["location"]["sheetName"], hit["location"]["cellAddress"]])
            })
            .collect();
        assert_eq!(
            json!(locations),
            json!([
                ["Zeta", "A2"],
                ["Zeta", "Z2"],
                ["Zeta", "AA2"],
                ["Zeta", "A10"],
                ["Alpha", "A1"]
            ])
        );
        let mut stable = run.hits.clone();
        for hit in &mut stable {
            assert_eq!(hit["modifiedAt"].as_f64(), Some(1700000000000.0));
            assert!(!hit["previewText"].as_str().unwrap().contains("PHONETIC"));
            hit.as_object_mut().unwrap().remove("resultId");
        }
        stable.sort_by_key(|h| {
            h["documentOrder"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap())
                .collect::<Vec<_>>()
        });
        if let Some(previous) = &prior {
            assert_eq!(&stable, previous);
        }
        prior = Some(stable);
        for query in [
            "PHONETIC_ONLY",
            "INLINE_PHONETIC_ONLY",
            "RICH_PHONETIC_ONLY",
        ] {
            for fuzzy in [false, true] {
                let mut flags = flags.clone();
                if fuzzy {
                    flags.push("--fuzzy-search");
                }
                assert!(search(&cli, &folder, query, &flags, None).hits.is_empty());
                assert!(search(
                    &cli,
                    &folder,
                    "",
                    &flags,
                    Some(&condition("unit", &[query], &[], &[]))
                )
                .hits
                .is_empty());
            }
        }
        let visible = search(&cli, &folder, "顧客", &flags, None);
        assert_eq!(visible.hits.len(), 3);
        for hit in visible.hits {
            let ranges = hit["matchRanges"][0].as_array().unwrap();
            let text: Vec<_> = hit["previewText"].as_str().unwrap().chars().collect();
            assert_eq!(
                text[ranges[0].as_u64().unwrap() as usize..ranges[1].as_u64().unwrap() as usize]
                    .iter()
                    .collect::<String>(),
                "顧客"
            );
        }
    }
    let other = cli.base.path().join("other-root");
    docs_search_test_support::specialized::issue_workbook(&other.join("archived.xlsx")).unwrap();
    search(&cli, &other, "ORDER", &["--use-index"], None);
    {
        let db = Connection::open(cli.db()).unwrap();
        db.execute_batch("UPDATE files SET extraction_version=3; UPDATE units SET text=text || ' PHONETIC_ONLY', loose=loose || ' phonetic_only';").unwrap();
    }
    assert!(
        search(&cli, &folder, "PHONETIC_ONLY", &["--use-index"], None)
            .hits
            .is_empty()
    );
    {
        let db = Connection::open(cli.db()).unwrap();
        let versions: Vec<i64> = db
            .prepare("SELECT DISTINCT extraction_version FROM files")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(versions, vec![4]);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM units WHERE text LIKE '%PHONETIC_ONLY%'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM files WHERE path=?",
                [other.join("archived.xlsx").to_string_lossy().as_ref()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
    println!(
        "PASS visible text, workbook/numeric order, timestamp, v3 to v4 extraction cache rebuild"
    );
}
