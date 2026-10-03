mod support;
use docs_search_test_support::write;
use rusqlite::Connection;
use std::{
    fs,
    time::{Duration, SystemTime},
};
use support::*;
fn search(cli: &Cli, query: &str, index: bool, fuzzy: bool) -> Run {
    let mut flags = vec![];
    if index {
        flags.push("--use-index");
    }
    if fuzzy {
        flags.push("--fuzzy-search");
    }
    let run = cli.run(&cli.root, query, "txt", None, &flags);
    run.accepted("completed");
    assert!(run.issues.is_empty());
    run
}
fn set_modified(path: &std::path::Path, time: SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(time))
        .unwrap();
}

#[test]
fn indexed_many_candidates_preserve_order_ranges_and_edit_revisions() {
    let cli = Cli::new("index-many-candidates");
    fs::create_dir(&cli.root).unwrap();
    let path = cli.root.join("sample.txt");
    let contents: String = (0..1_501)
        .map(|number| format!("👩‍💻 customer {number}\r\n"))
        .collect();
    let original = format!("\u{feff}{contents}");
    write(&path, &original).unwrap();
    let direct = search(&cli, "customer", false, false);
    assert_eq!(direct.hits.len(), 1_501);
    for (number, hit) in direct.hits.iter().enumerate() {
        assert_eq!(hit["location"]["lineNumber"], serde_json::json!(number + 1));
        assert_eq!(hit["sourceMatchRanges"], serde_json::json!([[4, 12]]));
        assert_eq!(
            hit["editAnchor"]["sourceRevision"],
            docs_search_core::edit::revision(original.as_bytes())
        );
    }
    for fuzzy in [false, true] {
        for _ in 0..2 {
            let indexed = search(&cli, "customer", true, fuzzy);
            assert_eq!(indexed.hits, direct.hits);
        }
    }
    let missing = search(&cli, "notfound", true, false);
    assert!(missing.hits.is_empty());
    assert_eq!(missing.stats(), (0, 1));
    write(&path, "replacement customer\n").unwrap();
    let changed = search(&cli, "customer", true, true);
    assert_eq!(changed.hits.len(), 1);
    assert_eq!(changed.stats(), (1, 0));
    assert_eq!(search(&cli, "customer", true, true).hits, changed.hits);
    fs::remove_file(&path).unwrap();
    assert!(search(&cli, "customer", true, true).hits.is_empty());
    let db = Connection::open(cli.db()).unwrap();
    for table in ["files", "units", "grams", "tokens"] {
        assert_eq!(
            db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn index_freshness_optout_and_recovery() {
    let mut cli = Cli::new("index");
    fs::create_dir(&cli.root).unwrap();
    let path = cli.root.join("sample.txt");
    write(&path, "alpha\n").unwrap();
    let database = cli.db();
    let direct = search(&cli, "alpha", false, false);
    assert_eq!(direct.hits.len(), 1);
    assert_eq!(direct.stats(), (1, 0));
    assert!(!database.parent().unwrap().exists());
    let indexed = search(&cli, "alpha", true, false);
    assert_eq!(indexed.hits, direct.hits);
    assert_eq!(indexed.stats(), (1, 0));
    let reused = search(&cli, "alpha", true, false);
    assert_eq!(reused.hits, direct.hits);
    assert_eq!(reused.stats(), (0, 1));
    let before = fs::read(&database).unwrap();
    let before_time = fs::metadata(&database).unwrap().modified().unwrap();
    let old_time = fs::metadata(&path).unwrap().modified().unwrap();
    write(&path, "bravo\n").unwrap();
    set_modified(&path, old_time + Duration::from_secs(2));
    let current = search(&cli, "bravo", false, false);
    assert_eq!(current.hits.len(), 1);
    assert_eq!(current.stats(), (1, 0));
    assert_eq!(fs::read(&database).unwrap(), before);
    assert_eq!(
        fs::metadata(&database).unwrap().modified().unwrap(),
        before_time
    );
    {
        let db = Connection::open(&database).unwrap();
        assert_eq!(
            db.query_row("SELECT text FROM units", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "alpha"
        );
    }
    let updated = search(&cli, "bravo", true, false);
    assert_eq!(updated.hits, current.hits);
    assert_eq!(updated.stats(), (1, 0));
    assert_eq!(search(&cli, "bravo", true, false).stats(), (0, 1));
    write(&path, "longer alpha\n").unwrap();
    set_modified(&path, old_time + Duration::from_secs(2));
    assert_eq!(search(&cli, "alpha", true, false).stats(), (1, 0));
    fs::remove_file(&path).unwrap();
    let empty = search(&cli, "alpha", true, false);
    assert!(empty.hits.is_empty());
    assert_eq!(empty.stats(), (0, 0));
    {
        let db = Connection::open(&database).unwrap();
        for table in ["files", "units", "grams", "tokens"] {
            assert_eq!(
                db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
    write(&path, "alpha\n").unwrap();
    write(&database, b"not a SQLite database").unwrap();
    let recovered = search(&cli, "alpha", true, false);
    assert_eq!(recovered.hits.len(), 1);
    assert_eq!(recovered.stats(), (1, 0));
    {
        let db = Connection::open(&database).unwrap();
        assert_eq!(
            db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        db.execute_batch("PRAGMA user_version=1").unwrap();
    }
    assert_eq!(search(&cli, "alpha", true, false).stats(), (1, 0));
    {
        let db = Connection::open(&database).unwrap();
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    write(&path, "a_\nalphabet_\n__\n").unwrap();
    for query in ["a!", "alpha!", "__"] {
        let direct = search(&cli, query, false, true);
        assert_eq!(direct.hits.len(), 1, "{query}");
        for _ in 0..2 {
            assert_eq!(search(&cli, query, true, true).hits, direct.hits, "{query}");
        }
    }
    write(&path, "alpha\n").unwrap();
    cli.cache = cli.base.path().join("blocked-cache");
    write(&cli.cache, b"cannot create a directory here").unwrap();
    assert_eq!(search(&cli, "alpha", true, false).hits.len(), 1);
    assert_eq!(search(&cli, "alpha", true, false).stats(), (1, 0));
    println!("PASS index opt-out, initial/reuse, mtime/size, pruning, corruption, schema, fuzzy and unavailable-cache fallback");
}
