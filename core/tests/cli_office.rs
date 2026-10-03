mod support;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};
use support::*;
use unicode_casefold::UnicodeCaseFold;
fn run(
    cli: &Cli,
    folder: &Path,
    query: &str,
    spec: Option<&Value>,
    flags: &[&str],
    extensions: &str,
) -> Run {
    let run = cli.run(folder, query, extensions, spec, flags);
    run.accepted(if flags.contains(&"--cancel-on-start") {
        "cancelled"
    } else {
        "completed"
    });
    run
}
#[test]
fn office_batch_ranking_and_index_equivalence() {
    let cli = Cli::new("office");
    docs_search_test_support::specialized::office(&cli.root).unwrap();
    let search = cli.root.join("search");
    let ext = "xlsx,pptx,docx";
    for (query, kind, flag) in [
        ("FORMULA_SIGNAL", "formula", "--include-formulas"),
        ("SHARED_SIGNAL", "formula", "--include-formulas"),
        ("FORMULA_WITHOUT_CACHE", "formula", "--include-formulas"),
        ("NOTE_SIGNAL", "note", "--include-notes"),
        ("ROOT_SIGNAL", "excelComment", "--include-notes"),
        ("REPLY_SIGNAL", "excelComment", "--include-notes"),
        ("HEADER_SIGNAL", "wordHeader", "--include-notes"),
        ("HEADER_TABLE_SIGNAL", "wordHeader", "--include-notes"),
        ("FOOTER_SIGNAL", "wordFooter", "--include-notes"),
        ("WORD_COMMENT_SIGNAL", "wordComment", "--include-notes"),
    ] {
        let off = run(&cli, &search, query, None, &[], ext);
        assert!(
            off.hits.is_empty() && off.issues.is_empty(),
            "{query} defaults"
        );
        let mut expected = None;
        for flags in [
            vec![flag],
            vec![flag, "--use-index"],
            vec![flag, "--use-index"],
        ] {
            let on = run(&cli, &search, query, None, &flags, ext);
            assert_eq!(on.hits.len(), 1, "{query}");
            assert!(on.issues.is_empty());
            assert_eq!(on.hits[0]["sourceKind"], kind);
            let signature = signature(&on.hits);
            if let Some(previous) = &expected {
                assert_eq!(&signature, previous, "{query}");
            }
            expected = Some(signature);
        }
        println!("PASS {query} opt-in sources and direct/index initial/index reused");
    }
    let cached = run(&cli, &search, "42", None, &["--include-formulas"], ext);
    assert_eq!(cached.hits.len(), 2);
    assert_eq!(
        cached
            .hits
            .iter()
            .map(|h| h["sourceKind"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        ["cell", "formula"].into_iter().collect()
    );
    assert!(cached
        .hits
        .iter()
        .all(|h| h["location"]["cellAddress"] == "C1"));
    for query in ["IGNORED_SIGNAL", "ORPHAN_SIGNAL"] {
        let r = run(&cli, &search, query, None, &["--include-notes"], ext);
        assert!(r.hits.is_empty() && r.issues.is_empty());
    }
    for (flags, kind) in [(vec![], "shape"), (vec!["--include-notes"], "excelComment")] {
        let r = run(&cli, &search, "LEGACY_SIGNAL", None, &flags, ext);
        assert_eq!(r.hits.len(), 1);
        assert_eq!(r.hits[0]["sourceKind"], kind);
        assert!(r.issues.is_empty());
    }
    let vml = run(&cli, &search, "VML_SIGNAL", None, &["--include-notes"], ext);
    assert_eq!(vml.hits.len(), 1);
    assert_eq!(vml.hits[0]["sourceKind"], "shape");
    assert_eq!(
        run(
            &cli,
            &cli.root.join("macros"),
            "SHARED_SIGNAL",
            None,
            &["--include-formulas"],
            "xlsm"
        )
        .hits
        .len(),
        1
    );
    let r = run(
        &cli,
        &search,
        "",
        Some(&condition(
            "excelRow",
            &["TAB_01", "FORMULA_SIGNAL"],
            &[],
            &[],
        )),
        &["--include-formulas"],
        ext,
    );
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0]["sourceKind"], "excelRow");
    assert!(r.issues.is_empty());
    for fault in [
        "malformed",
        "missing",
        "external",
        "invalid-utf8",
        "oversized",
        "word-error",
    ] {
        let folder = cli.root.join(fault);
        let r = run(
            &cli,
            &folder,
            "BODY_SIGNAL",
            None,
            &["--include-notes", "--use-index"],
            ext,
        );
        assert_eq!(r.hits.len(), 1, "{fault}");
        assert_eq!(r.issues.len(), 1, "{fault}");
        let r = run(
            &cli,
            &folder,
            "",
            Some(&condition("file", &["BODY_SIGNAL"], &[], &["unknown"])),
            &["--include-notes"],
            ext,
        );
        assert!(r.hits.is_empty());
        assert_eq!(r.issues.len(), 1);
        let db = Connection::open(cli.db()).unwrap();
        let paths: Vec<String> = db
            .prepare("SELECT path FROM files")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(
            !paths
                .iter()
                .any(|p| p.contains(folder.to_string_lossy().as_ref())),
            "{fault}: partial cache"
        );
    }
    for notes in [false, true] {
        for formulas in [false, true] {
            let mut flags = vec![];
            if notes {
                flags.push("--include-notes");
            }
            if formulas {
                flags.push("--include-formulas");
            }
            let direct = run(&cli, &search, "SIGNAL", None, &flags, ext);
            flags.push("--use-index");
            let indexed = run(&cli, &search, "SIGNAL", None, &flags, ext);
            assert_eq!(signature(&direct.hits), signature(&indexed.hits));
        }
    }
    let batch = json!({"mode":"batch","terms":["","TAB_01","tab_01","TAB_010","MISSING_ID"],"matchMode":"identifier"});
    let mut prior = None;
    for (attempt, flags) in [vec![], vec!["--use-index"], vec!["--use-index"]]
        .into_iter()
        .enumerate()
    {
        let r = run(&cli, &search, "", Some(&batch), &flags, ext);
        let terms = r.summary("batchSummary")["terms"].as_array().unwrap();
        assert_eq!(
            terms
                .iter()
                .map(|v| v["term"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["TAB_01", "TAB_010", "MISSING_ID"]
        );
        for field in ["hitCount", "fileCount"] {
            assert_eq!(
                terms
                    .iter()
                    .map(|v| v[field].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                [2, 2, 0]
            );
        }
        assert_eq!(
            terms
                .iter()
                .map(|v| v["hitCount"].as_u64().unwrap())
                .sum::<u64>(),
            r.hits.len() as u64
        );
        assert!(r
            .hits
            .iter()
            .all(|h| terms.iter().any(|t| t["termId"] == h["termId"])));
        assert_eq!(terms.last().unwrap()["status"], "指定範囲内で該当なし");
        assert!(r.issues.is_empty());
        if !flags.is_empty() {
            assert_eq!(r.stats(), if attempt == 1 { (3, 0) } else { (0, 3) });
        }
        let sig = signature(&r.hits);
        if let Some(previous) = &prior {
            assert_eq!(&sig, previous);
        }
        prior = Some(sig);
    }
    for terms in [
        vec!["MISSING_ID".into()],
        (0..256).map(|i| format!("ID_{i}")).collect::<Vec<String>>(),
    ] {
        let r = run(
            &cli,
            &search,
            "",
            Some(&json!({"mode":"batch","terms":terms})),
            &[],
            ext,
        );
        assert_eq!(r.stats(), (3, 0));
    }
    for terms in [
        json!((0..257).map(|i| format!("ID_{i}")).collect::<Vec<_>>()),
        json!(["顧客"]),
    ] {
        cli.run(
            &search,
            "",
            ext,
            Some(&json!({"mode":"batch","terms":terms})),
            &[],
        )
        .rejected("querySpec");
    }
    for (folder, flags, status) in [
        (search.clone(), vec!["--cancel-on-start"], "未確定（中断）"),
        (
            cli.root.join("malformed"),
            vec!["--include-notes"],
            "未確定（一部エラーあり）",
        ),
    ] {
        let r = run(
            &cli,
            &folder,
            "",
            Some(&json!({"mode":"batch","terms":["MISSING_ID"]})),
            &flags,
            ext,
        );
        assert_eq!(r.summary("batchSummary")["terms"][0]["status"], status);
    }
    let r = run(
        &cli,
        &search,
        "",
        Some(&json!({"mode":"batch","terms":["BODY_SIGNAL","TAB_01"],"matchMode":"text"})),
        &[],
        ext,
    );
    assert_eq!(
        r.hits
            .iter()
            .map(|h| h["termId"].to_string())
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    let cases: Value =
        serde_json::from_slice(&std::fs::read(cli.root.join("ranking-cases.json")).unwrap())
            .unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 20);
    let mut report = vec![];
    for case in cases.as_array().unwrap() {
        let folder = cli.root.join(case["folder"].as_str().unwrap());
        let mut prior = None;
        let mut last = None;
        for cached in [false, true, true] {
            let mut flags: Vec<_> = case["flags"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            if cached {
                flags.push("--use-index");
            }
            let r = run(
                &cli,
                &folder,
                case["query"].as_str().unwrap(),
                case.get("spec"),
                &flags,
                case["extensions"].as_str().unwrap(),
            );
            assert!(r.issues.is_empty());
            let order: Vec<_> = r.summary("rankingSummary")["files"]
                .as_array()
                .unwrap()
                .iter()
                .map(filename)
                .collect();
            let prefer = case["prefer"].as_str().unwrap();
            let over = case["over"].as_str().unwrap();
            assert!(
                order.iter().position(|v| v == prefer).unwrap()
                    < order.iter().position(|v| v == over).unwrap(),
                "{}: {order:?}",
                case["name"]
            );
            if let Some(previous) = &prior {
                assert_eq!(&order, previous);
            }
            prior = Some(order);
            if let Some(address) = case.get("firstAddress") {
                let rankings = r.rankings();
                let ranking = rankings
                    .iter()
                    .find(|v| filename(&v["filePath"]) == prefer)
                    .unwrap();
                let hit = r
                    .hits
                    .iter()
                    .find(|h| h["resultId"] == ranking["resultIds"][0])
                    .unwrap();
                assert_eq!(&hit["location"]["cellAddress"], address);
            }
            last = Some(r);
        }
        let r = last.unwrap();
        let rankings = r.rankings();
        let mut old: Vec<_> = rankings
            .iter()
            .map(|v| v["filePath"].as_str().unwrap())
            .collect();
        old.sort_by_key(|path| {
            let hits: Vec<_> = r.hits.iter().filter(|h| h["filePath"] == *path).collect();
            (
                std::cmp::Reverse(
                    hits.iter()
                        .map(|h| h["score"].as_u64().unwrap())
                        .max()
                        .unwrap(),
                ),
                std::cmp::Reverse(hits.iter().any(|h| h["sourceKind"] == "fileName")),
                path.case_fold().collect::<String>(),
            )
        });
        let old_top: Vec<_> = old
            .iter()
            .take(5)
            .map(|s| {
                Path::new(s)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let new_top: Vec<_> = prior.unwrap().into_iter().take(5).collect();
        println!("PASS {} old={old_top:?} new={new_top:?}", case["name"]);
        report.push(json!({"case":case["name"],"query":case["query"],"oldTop5":old_top,"newTop5":new_top,"evidence":rankings.iter().map(|r|r["evidence"].clone()).collect::<Vec<_>>()}));
    }
    if let Some(path) = std::env::var_os("DOCS_SEARCH_RANKING_REPORT") {
        let path = Path::new(&path);
        assert!(!path.exists(), "ranking report must be new");
        docs_search_test_support::write_json(path, &json!(report)).unwrap();
    }
    println!("PASS Office opt-in, partial-error handling, batch completeness, 20 ranking pairs");
}
