//! Acceptance tests for docs/rust-requirements-v3.0.0.md through the public API.
use docs_search_core::{
    batch, run_search, validate, EventKind, SearchEvent, SearchHit, SearchRequest,
    SUPPORTED_EXTENSIONS,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "docs-search-v3-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn request(&self, query: &str) -> SearchRequest {
        serde_json::from_value(json!({
            "rootDirectory": self.0, "query": query, "recursive": true, "extensions": ["txt"]
        }))
        .unwrap()
    }
    fn run(&self, request: SearchRequest) -> Vec<SearchEvent> {
        let mut events = Vec::new();
        run_search(
            request,
            "v3-acceptance".into(),
            &AtomicBool::new(false),
            |event| events.push(event),
        )
        .unwrap();
        check_events(&events);
        events
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn hits(events: &[SearchEvent]) -> Vec<&SearchHit> {
    events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::Result { hit } => Some(hit),
            _ => None,
        })
        .collect()
}

fn check_events(events: &[SearchEvent]) {
    assert!(matches!(
        events.first().unwrap().kind,
        EventKind::Started { .. }
    ));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Started { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Finished { .. }))
            .count(),
        1
    );
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.search_id, "v3-acceptance");
        assert_eq!(event.sequence, index + 1);
    }
    let EventKind::Finished { counts, .. } = &events.last().unwrap().kind else {
        panic!("Finished must be last")
    };
    assert_eq!(counts.result_count, hits(events).len());
    assert_eq!(
        counts.issue_count,
        events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Issue { .. }))
            .count()
    );
    assert!(counts.processed_files <= counts.discovered_files);
    let ids: std::collections::HashSet<_> = hits(events).iter().map(|h| h.result_id).collect();
    assert_eq!(ids.len(), counts.result_count);
    for event in events {
        if let EventKind::FileRanked { ranking } = &event.kind {
            assert!(ranking.result_ids.iter().all(|id| ids.contains(id)));
        }
    }
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::RankingSummary { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::ExecutionSummary { .. }))
            .count(),
        1
    );
}

#[test]
fn r3_01_request_defaults_preserve_api_compatibility_and_independent_options() {
    let fixture = Fixture::new();
    let base = json!({"rootDirectory": fixture.0, "query": "needle", "recursive": true});
    let request: SearchRequest = serde_json::from_value(base.clone()).unwrap();
    assert_eq!(request.extensions, ["xlsx", "xlsm", "pptx", "docx", "txt"]);
    assert!(request.additional_directories.is_empty() && request.excluded_directories.is_empty());
    assert!(request.query_spec.is_none());
    assert!(
        !request.use_index
            && !request.fuzzy_search
            && !request.include_notes
            && !request.include_formulas
    );
    for (index, fuzzy) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut value = base.clone();
        value["useIndex"] = json!(index);
        value["fuzzySearch"] = json!(fuzzy);
        let request: SearchRequest = serde_json::from_value(value).unwrap();
        assert_eq!((request.use_index, request.fuzzy_search), (index, fuzzy));
        validate(&request).unwrap();
    }
}

#[test]
fn r3_02_invalid_inputs_are_rejected_before_any_event() {
    let fixture = Fixture::new();
    let base = fixture.request("needle");
    let mut cases = Vec::new();
    let mut request = base.clone();
    request.query = " \t ".into();
    cases.push((request, "query"));
    let mut request = base.clone();
    request.recursive = false;
    cases.push((request, "rootDirectory"));
    let mut request = base.clone();
    request.root_directory = fixture.0.join("missing").to_string_lossy().into();
    cases.push((request, "rootDirectory"));
    fixture.write("file.txt", b"");
    let mut request = base.clone();
    request.additional_directories = vec![fixture.0.join("file.txt").to_string_lossy().into()];
    cases.push((request, "additionalDirectories"));
    let mut request = base.clone();
    request.excluded_directories = vec![fixture.0.join("missing").to_string_lossy().into()];
    cases.push((request, "excludedDirectories"));
    let mut request = base.clone();
    request.extensions.clear();
    cases.push((request, "extensions"));
    for extension in ["pdf", ".txt", "TXT"] {
        let mut request = base.clone();
        request.extensions = vec![extension.into()];
        cases.push((request, "extensions"));
    }
    let mut request = base;
    request.query_spec = Some(json!({"mode": "batch", "terms": ["needle"]}));
    cases.push((request, "querySpec"));
    for (request, field) in cases {
        let mut events = Vec::new();
        let error = run_search(
            request,
            "v3-acceptance".into(),
            &AtomicBool::new(false),
            |event| events.push(event),
        )
        .unwrap_err();
        assert_eq!(error.field, field);
        assert!(
            events.is_empty(),
            "input rejection must not create a search"
        );
    }
}

#[test]
fn r3_03_overlapping_roots_exclusions_and_selected_extensions_count_once() {
    let fixture = Fixture::new();
    fixture.write("a.TXT", "needle");
    fixture.write("child/b.txt", "needle");
    fixture.write("child/excluded/broken.txt", [0x81]);
    fixture.write("ignored.md", "needle");
    fixture.write(".hidden.txt", "needle");
    fixture.write("~$temporary.xlsx", "not a zip");
    let mut request = fixture.request("needle");
    request.extensions = vec!["txt".into(), "txt".into(), "xlsx".into()];
    request.additional_directories = vec![
        fixture.0.to_string_lossy().into(),
        fixture.0.join("child").to_string_lossy().into(),
    ];
    request.excluded_directories = vec![fixture.0.join("child/excluded").to_string_lossy().into()];
    let events = fixture.run(request);
    let EventKind::Finished { counts, reason } = &events.last().unwrap().kind else {
        unreachable!()
    };
    assert_eq!(*reason, "completed");
    assert_eq!(
        (
            counts.discovered_files,
            counts.processed_files,
            counts.result_count,
            counts.issue_count
        ),
        (3, 3, 3, 0)
    );
}

#[test]
fn r3_03_excluding_the_root_is_a_valid_empty_search() {
    let fixture = Fixture::new();
    fixture.write("sample.txt", "needle");
    let mut request = fixture.request("needle");
    request.excluded_directories = vec![request.root_directory.clone()];
    let events = fixture.run(request);
    let EventKind::Finished { counts, reason } = &events.last().unwrap().kind else {
        unreachable!()
    };
    assert_eq!(*reason, "completed");
    assert_eq!(
        (
            counts.discovered_files,
            counts.processed_files,
            counts.result_count,
            counts.issue_count
        ),
        (0, 0, 0, 0)
    );
}

#[test]
fn r3_04_all_89_text_extensions_use_the_same_line_contract() {
    let fixture = Fixture::new();
    assert_eq!(SUPPORTED_EXTENSIONS.len(), 93);
    let text: Vec<_> = SUPPORTED_EXTENSIONS
        .iter()
        .filter(|ext| !["xlsx", "xlsm", "pptx", "docx"].contains(ext))
        .collect();
    assert_eq!(text.len(), 89);
    for (index, ext) in text.iter().enumerate() {
        fixture.write(
            &format!("file_{index}.{ext}"),
            "before\nneedle needle\nafter\n",
        );
    }
    let mut request = fixture.request("needle");
    request.extensions = SUPPORTED_EXTENSIONS.iter().map(|s| (*s).into()).collect();
    let events = fixture.run(request);
    assert_eq!(hits(&events).len(), 89);
    for hit in hits(&events) {
        assert_eq!(hit.source_kind, "textLine");
        assert_eq!(hit.location["lineNumber"], 2);
        assert_eq!(hit.content_class, "body");
        assert!(hit.edit_anchor.is_some());
    }
}

#[test]
fn r3_10_many_batch_terms_keep_unicode_ranges_and_first_input_order() {
    let fixture = Fixture::new();
    let terms: Vec<String> = (0..256).map(|number| format!("TAB_{number:03}")).collect();
    let contents = format!("👩‍💻 {} / TAB_0000 / 顧客TAB_001\r\n", terms.join(" "));
    fixture.write("sample.txt", &contents);
    for match_mode in ["identifier", "text"] {
        for fuzzy in [false, true] {
            let mut request = fixture.request("");
            request.fuzzy_search = fuzzy;
            request.query_spec =
                Some(json!({"mode":"batch", "matchMode":match_mode, "terms":terms}));
            let events = fixture.run(request);
            let results = hits(&events);
            assert_eq!(results.len(), 256);
            for (number, hit) in results.iter().enumerate() {
                let term_id = format!("batch:{}", number + 1);
                assert_eq!(hit.term_id.as_deref(), Some(term_id.as_str()));
                assert_eq!(hit.term.as_deref(), Some(terms[number].as_str()));
                assert_eq!(
                    hit.source_match_ranges,
                    vec![[4 + number * 8, 11 + number * 8]]
                );
                assert!(hit.edit_anchor.is_some());
            }
            let summary = events
                .iter()
                .find_map(|event| match &event.kind {
                    EventKind::BatchSummary { terms } => Some(terms),
                    _ => None,
                })
                .unwrap();
            assert_eq!(summary.len(), 256);
            assert!(summary
                .iter()
                .all(|term| term.file_count == 1 && term.hit_count == 1));
        }
    }
}

#[test]
fn r3_17_search_generated_revision_rejects_external_changes_outside_matched_line() {
    let fixture = Fixture::new();
    let original = "\u{feff}before\r\n👩‍💻 needle\r\nafter\r\n";
    fixture.write("sample.txt", original);
    let events = fixture.run(fixture.request("needle"));
    let results = hits(&events);
    assert_eq!(results.len(), 1);
    let (_, view) = docs_search_core::edit::prepare(results[0], None).unwrap();
    assert_eq!(view.selected_text, "needle");
    assert_eq!(
        fs::read(fixture.0.join("sample.txt")).unwrap(),
        original.as_bytes()
    );
    let changed = original.replace("after", "other");
    fixture.write("sample.txt", &changed);
    assert!(docs_search_core::edit::prepare(results[0], None).is_err());
    assert_eq!(
        fs::read(fixture.0.join("sample.txt")).unwrap(),
        changed.as_bytes()
    );
}

#[test]
fn r3_05_normal_query_is_literal_and_does_not_cross_lines() {
    let fixture = Fixture::new();
    fixture.write(
        "sample.txt",
        "alpha\nbeta\nalpha AND beta\na.*b\nalpha beta\n",
    );
    for (query, line) in [(" alpha AND beta ", 3), ("a.*b", 4)] {
        let events = fixture.run(fixture.request(query));
        assert_eq!(hits(&events).len(), 1);
        assert_eq!(hits(&events)[0].location["lineNumber"], line);
    }
    assert!(hits(&fixture.run(fixture.request("alpha\nbeta"))).is_empty());
}

#[test]
fn r3_05_file_name_and_distinct_lines_are_separate_results() {
    let fixture = Fixture::new();
    fixture.write("needle.txt", "needle needle\nneedle\n");
    let events = fixture.run(fixture.request("needle"));
    assert_eq!(hits(&events).len(), 3);
    assert_eq!(
        hits(&events)
            .iter()
            .filter(|h| h.source_kind == "fileName")
            .count(),
        1
    );
    assert_eq!(
        hits(&events)
            .iter()
            .filter(|h| h.source_kind == "textLine")
            .count(),
        2
    );
}

#[test]
fn r3_06_unicode_original_ranges_survive_folding_and_preview_clipping() {
    let fixture = Fixture::new();
    fixture.write(
        "sample.txt",
        format!("{}😀 cafe\u{301} Straße suffix\n", "あ".repeat(400)),
    );
    for (query, selected) in [("CAFÉ", "cafe\u{301}"), ("STRASSE", "Straße")] {
        let events = fixture.run(fixture.request(query));
        let hit = hits(&events)[0];
        assert!(hit.preview_truncated);
        assert_eq!(hit.match_category, "standard");
        let [start, end] = hit.match_ranges[0];
        assert_eq!(
            hit.preview_text
                .chars()
                .skip(start)
                .take(end - start)
                .collect::<String>(),
            selected
        );
        let [start, end] = hit.source_match_ranges[0];
        let source = fs::read_to_string(fixture.0.join("sample.txt")).unwrap();
        assert_eq!(
            source
                .chars()
                .skip(start)
                .take(end - start)
                .collect::<String>(),
            selected
        );
    }
}

#[test]
fn r3_07_fuzzy_is_opt_in_and_standard_hits_keep_their_category() {
    let fixture = Fixture::new();
    fixture.write("sample.txt", "顧客ＩＤ\n顧客ID\ncustmer\n");
    let events = fixture.run(fixture.request("顧客ID"));
    assert_eq!(hits(&events).len(), 1);
    let mut request = fixture.request("顧客ID");
    request.fuzzy_search = true;
    let events = fixture.run(request);
    assert_eq!(hits(&events).len(), 2);
    assert!(hits(&events)
        .iter()
        .any(|h| h.match_type == "normalized" && h.match_category == "fuzzy" && h.score == 95));
    assert!(hits(&events)
        .iter()
        .any(|h| h.match_category == "standard" && h.score == 100));
    assert!(hits(&fixture.run(fixture.request("customer"))).is_empty());
    let mut request = fixture.request("customer");
    request.fuzzy_search = true;
    let events = fixture.run(request);
    assert_eq!(
        (hits(&events)[0].match_type, hits(&events)[0].score),
        ("editDistance", 50)
    );
}

#[test]
fn r3_09_conditions_validate_group_and_character_boundaries() {
    let fixture = Fixture::new();
    let make = |all: Value| {
        let mut request = fixture.request("");
        request.query_spec = Some(json!({"mode":"conditions", "scope":"unit", "all":all}));
        request
    };
    validate(&make(json!((0..32)
        .map(|i| format!("term{i}"))
        .collect::<Vec<_>>())))
    .unwrap();
    assert_eq!(
        validate(&make(json!((0..33)
            .map(|i| format!("term{i}"))
            .collect::<Vec<_>>())))
        .unwrap_err()
        .field,
        "querySpec"
    );
    validate(&make(json!(["あ".repeat(200)]))).unwrap();
    assert!(validate(&make(json!(["あ".repeat(201)]))).is_err());
    validate(&make(json!(vec!["same"; 128]))).unwrap();
    assert!(validate(&make(json!(vec!["same"; 129]))).is_err());
    for spec in [
        json!({"mode":"conditions","scope":"unknown","all":["a"]}),
        json!({"mode":"conditions","scope":"file","all":["CAFÉ"],"not":["cafe\u{301}"]}),
        json!({"mode":"conditions","scope":"unit","not":["a"]}),
        json!({"mode":"conditions","scope":"unit","all":["a"],"unknown":true}),
    ] {
        let mut request = fixture.request("");
        request.query_spec = Some(spec);
        assert_eq!(validate(&request).unwrap_err().field, "querySpec");
    }
}

#[test]
fn r3_10_batch_validates_limits_after_deduplication() {
    let spec = |terms: Vec<String>| json!({"mode":"batch", "terms":terms});
    assert_eq!(
        batch::parse(&spec((0..256).map(|i| format!("T_{i}")).collect()))
            .unwrap()
            .terms
            .len(),
        256
    );
    assert!(batch::parse(&spec((0..257).map(|i| format!("T_{i}")).collect())).is_err());
    assert_eq!(
        batch::parse(&spec(vec!["same".into(); 4096]))
            .unwrap()
            .terms
            .len(),
        1
    );
    assert!(batch::parse(&spec(vec!["same".into(); 4097])).is_err());
    assert!(batch::parse(&spec(vec!["a".repeat(200)])).is_ok());
    assert!(batch::parse(&spec(vec!["a".repeat(201)])).is_err());
    assert!(batch::parse(&spec(vec![" ".into()])).is_err());
    for term in ["1ID", "顧客", "TAB-01", "two words"] {
        assert!(batch::parse(&spec(vec![term.into()])).is_err());
    }
}

#[test]
fn r3_10_batch_counts_each_term_unit_once_and_zero_is_confirmed_only_when_complete() {
    let fixture = Fixture::new();
    fixture.write("a.txt", "TAB_01 TAB_01 TAB_010\nTAB_01\n");
    fixture.write("b.txt", "TAB_010\n");
    let mut request = fixture.request("");
    request.query_spec =
        Some(json!({"mode":"batch","terms":["TAB_01","tab_01","TAB_010","MISSING"]}));
    request.fuzzy_search = true;
    let events = fixture.run(request.clone());
    let summary = events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::BatchSummary { terms } => Some(terms),
            _ => None,
        })
        .unwrap();
    assert_eq!(summary.len(), 3);
    assert_eq!((summary[0].file_count, summary[0].hit_count), (1, 2));
    assert_eq!((summary[1].file_count, summary[1].hit_count), (2, 2));
    assert_eq!(summary[2].status, "指定範囲内で該当なし");
    assert_eq!(hits(&events).len(), 4);
    assert!(hits(&events)
        .iter()
        .all(|h| h.match_type == "identifier" && h.match_category == "standard"));
    fixture.write("broken.txt", [0x81]);
    let events = fixture.run(request);
    let summary = events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::BatchSummary { terms } => Some(terms),
            _ => None,
        })
        .unwrap();
    assert_eq!(summary[2].hit_count, 0);
    assert_eq!(summary[2].status, "未確定（一部エラーあり）");
}

#[test]
fn r3_11_cancelling_after_a_result_keeps_results_and_final_counts() {
    let fixture = Fixture::new();
    fixture.write("sample.txt", "needle\nneedle\nneedle\n");
    let cancel = AtomicBool::new(false);
    let mut events = Vec::new();
    run_search(
        fixture.request("needle"),
        "v3-acceptance".into(),
        &cancel,
        |event| {
            if matches!(event.kind, EventKind::Result { .. }) {
                cancel.store(true, Ordering::Relaxed);
            }
            events.push(event);
        },
    )
    .unwrap();
    check_events(&events);
    assert_eq!(hits(&events).len(), 1);
    assert!(matches!(
        &events.last().unwrap().kind,
        EventKind::Finished {
            reason: "cancelled",
            ..
        }
    ));
    let ranked = events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::FileRanked { ranking } => Some(ranking),
            _ => None,
        })
        .unwrap();
    assert_eq!(ranked.result_ids, vec![hits(&events)[0].result_id]);
}

#[test]
fn r3_11_cancelled_batch_has_one_terminal_and_uncertain_zeroes() {
    let fixture = Fixture::new();
    fixture.write("sample.txt", "TAB_01");
    let mut request = fixture.request("");
    request.query_spec = Some(json!({"mode":"batch", "terms":["TAB_01"]}));
    let mut events = Vec::new();
    run_search(
        request,
        "v3-acceptance".into(),
        &AtomicBool::new(true),
        |e| events.push(e),
    )
    .unwrap();
    check_events(&events);
    assert!(hits(&events).is_empty());
    let summary = events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::BatchSummary { terms } => Some(terms),
            _ => None,
        })
        .unwrap();
    assert_eq!(summary[0].status, "未確定（中断）");
}

#[test]
fn r3_12_file_errors_do_not_stop_other_files_and_limits_are_reported() {
    let fixture = Fixture::new();
    fixture.write("good.txt", "needle");
    fixture.write("encoding.txt", [0x81]);
    fixture.write("broken.xlsx", "not a zip");
    fs::File::create(fixture.0.join("large.txt"))
        .unwrap()
        .set_len(32 * 1024 * 1024 + 1)
        .unwrap();
    let mut request = fixture.request("needle");
    request.extensions.push("xlsx".into());
    let events = fixture.run(request);
    assert_eq!(hits(&events).len(), 1);
    let codes: std::collections::HashSet<_> = events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Issue { issue } => {
                assert_eq!(issue.stage, "read");
                Some(issue.code)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        codes,
        ["unsupportedEncoding", "unreadable", "resourceLimit"]
            .into_iter()
            .collect()
    );
    let EventKind::Finished { counts, reason } = &events.last().unwrap().kind else {
        unreachable!()
    };
    assert_eq!(*reason, "completed");
    assert_eq!(counts.processed_files, 4);
    assert_eq!(counts.issue_count, 3);
}

#[test]
fn r3_13_document_order_uses_numeric_lines_and_ranking_caps_repetition() {
    let fixture = Fixture::new();
    fixture.write("sample.txt", "needle\n".repeat(12));
    let events = fixture.run(fixture.request("needle"));
    let all = hits(&events);
    let ranking = events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::FileRanked { ranking } => Some(ranking),
            _ => None,
        })
        .unwrap();
    let lines: Vec<_> = ranking
        .result_ids
        .iter()
        .map(|id| {
            all.iter().find(|h| h.result_id == *id).unwrap().location["lineNumber"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(lines, (1..=12).collect::<Vec<_>>());
    assert_eq!(ranking.evidence.body_evidence, 5);
    assert_eq!(ranking.evidence.distinct_evidence, 5);
    assert_eq!(ranking.evidence.matched_terms, 1);
}

#[test]
fn r3_15_search_is_read_only_and_returns_file_modification_time() {
    let fixture = Fixture::new();
    let original = b"needle\r\nunchanged\r\n";
    fixture.write("sample.txt", original);
    let path = fixture.0.join("sample.txt");
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let events = fixture.run(fixture.request("needle"));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    let expected = modified
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * 1000.0;
    assert_eq!(hits(&events)[0].modified_at, Some(expected));
}
