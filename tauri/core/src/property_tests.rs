//! Generated acceptance properties complement the fixed R3 examples.
//! Keep shrunk failures in core/proptest-regressions/ for subsequent runs.
use crate::{
    batch, edit, extract, fuzzy, query, ranking, run_search, EventKind, SearchEvent, SearchHit,
    SearchRequest, Unit,
};
use proptest::prelude::*;
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

// Bias toward normalization expansions, combining marks and multibyte positions,
// while still admitting every Unicode scalar rather than just a fixed alphabet.
fn unicode_text(max: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            4 => prop::sample::select(vec!["a", "A", "é", "e\u{301}", "ß", "İ", "Σ", "ς", "\u{345}", "\u{315}", "あ", "ア", "Ａ", "_", "-", " ", "\t", "👩‍💻"])
                .prop_map(str::to_owned),
            1 => any::<char>().prop_map(|c| c.to_string()),
        ],
        0..=max,
    ).prop_map(|parts| parts.concat())
}

fn line_fragment() -> impl Strategy<Value = String> {
    unicode_text(24).prop_map(|text| text.replace(['\r', '\n'], " "))
}

fn text_and_query() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        1 => (unicode_text(32), unicode_text(8)),
        // Construct positive matches as well; arbitrary independent strings alone
        // would mostly exercise the no-match path.
        2 => (unicode_text(12), unicode_text(8), unicode_text(12))
            .prop_map(|(prefix, needle, suffix)| (format!("{prefix} {needle} {suffix}"), needle)),
    ]
}

fn reference_fold(text: &str) -> String {
    text.nfc().case_fold().collect()
}

fn ranking_input() -> impl Strategy<Value = ranking::FileRanking> {
    (
        0u8..=100,
        0usize..=256,
        0usize..=256,
        0usize..=5,
        0usize..=5,
        "[a-zA-Z0-9_]{1,12}",
    )
        .prop_map(
            |(quality, matched_terms, same_row_terms, body_evidence, distinct_evidence, path)| {
                ranking::FileRanking {
                    file_path: path,
                    evidence: ranking::RankingEvidence {
                        quality,
                        matched_terms,
                        same_row_terms,
                        body_evidence,
                        distinct_evidence,
                    },
                    reasons: vec![],
                    result_ids: vec![],
                }
            },
        )
}

fn ranking_hit(index: usize, score: u8, term: usize, body: bool) -> SearchHit {
    SearchHit {
        source_match_ranges: vec![[0, 1]],
        document_order: vec![index as u64],
        modified_at: None,
        edit_anchor: None,
        term_id: Some(format!("batch:{term}")),
        term: Some(format!("TERM_{term}")),
        result_id: index + 1,
        file_path: "sample.txt".into(),
        file_type: "txt".into(),
        source_kind: "textLine".into(),
        unit_key: index.to_string(),
        part_key: String::new(),
        group_key: index.to_string(),
        row: None,
        column: None,
        content_class: if body { "body" } else { "note" }.into(),
        anchor: None,
        location: json!({"lineNumber": index + 1}),
        preview_text: "x".into(),
        preview_truncated: false,
        match_ranges: vec![[0, 1]],
        match_type: "substring",
        match_category: "standard",
        score,
        evidence: vec![],
    }
}

struct Fixture(PathBuf);
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "docs-search-proptest-{}-{}-{}",
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
        fs::write(self.0.join(path), bytes).unwrap();
    }
    fn request(&self, query: &str) -> SearchRequest {
        serde_json::from_value(json!({"rootDirectory": self.0, "query": query, "recursive": true, "extensions": ["txt"]})).unwrap()
    }
    fn run(&self, request: SearchRequest) -> Vec<SearchEvent> {
        let mut events = Vec::new();
        run_search(
            request,
            "property".into(),
            &AtomicBool::new(false),
            |event| events.push(event),
        )
        .unwrap();
        events
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn results(events: &[SearchEvent]) -> Vec<&SearchHit> {
    events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::Result { hit } => Some(hit),
            _ => None,
        })
        .collect()
}

proptest! {
    // Config::default honors PROPTEST_CASES and PROPTEST_RNG_SEED, and retains
    // SourceParallel failure persistence. The standard default is 256 cases.
    #[test]
    fn r3_06_canonical_equivalence_and_fold_idempotence(text in unicode_text(48)) {
        let folded = crate::normalize_fold(&text);
        let decomposed: String = text.nfd().collect();
        prop_assert_eq!(&folded, &crate::normalize_fold(&decomposed));
        prop_assert_eq!(&folded, &crate::normalize_fold(&folded));
    }

    #[test]
    fn r3_06_standard_matching_agrees_with_normalized_substring_model((text, needle) in text_and_query()) {
        let query = fuzzy::Query::new(&needle);
        let expected = !needle.trim().is_empty() && reference_fold(&text).contains(&reference_fold(needle.trim()));
        let actual = fuzzy::evaluate_selected(&text, &query, false);
        prop_assert_eq!(actual.is_some(), expected, "text={:?}, query={:?}", text, needle);
        if let Some(matched) = actual {
            let [start, end] = matched.range;
            prop_assert!(start < end && end <= text.chars().count());
            let marked: String = text.chars().skip(start).take(end - start).collect();
            prop_assert!(reference_fold(&marked).contains(&reference_fold(needle.trim())));
            prop_assert_eq!(matched.category, "standard");
        }
    }

    #[test]
    fn r3_07_fuzzy_search_preserves_standard_hits_and_valid_source_ranges((text, needle) in text_and_query()) {
        let query = fuzzy::Query::new(&needle);
        let standard = fuzzy::evaluate_selected(&text, &query, false);
        let expanded = fuzzy::evaluate_selected(&text, &query, true);
        if standard.is_some() {
            prop_assert!(expanded.is_some());
            prop_assert_eq!(expanded.as_ref().unwrap().category, "standard");
        }
        if let Some(matched) = expanded {
            prop_assert!(matched.range[0] < matched.range[1]);
            prop_assert!(matched.range[1] <= text.chars().count());
            prop_assert!(matched.score <= 100);
        }
    }

    #[test]
    fn r3_07_candidate_prefilter_never_discards_an_actual_fuzzy_match((text, needle) in text_and_query()) {
        let query = fuzzy::Query::new(&needle);
        if fuzzy::evaluate_selected(&text, &query, true).is_some() {
            prop_assert!(fuzzy::could_match(&text, &query), "text={:?}, query={:?}", text, needle);
        }
    }

    #[test]
    fn r3_08_index_candidates_preserve_all_actual_matches((text, needle) in text_and_query()) {
        let fixture = Fixture::new(); fixture.write("sample.txt", text.as_bytes());
        let path = fixture.0.join("sample.txt");
        let units = extract::text_units(text.as_bytes()).unwrap();
        let document = extract::ExtractedDocument { units, sheets: vec![], issues: vec![] };
        let mut index = crate::index::Index::in_memory().unwrap();
        prop_assert!(index.replace(&path, &document, crate::index::stamp(&path).unwrap(), crate::index::DEFAULT_SCOPE).unwrap());
        let query = fuzzy::Query::new(&needle);
        for fuzzy in [false, true] {
            let expected: HashSet<_> = document.units.iter().filter(|unit| fuzzy::evaluate_selected(&unit.text, &query, fuzzy).is_some()).map(|unit| &unit.meta.unit_key).collect();
            let candidates = index.candidates(&path, &query, fuzzy).unwrap();
            let actual: HashSet<_> = candidates.iter().filter(|unit| fuzzy::evaluate_selected(&unit.text, &query, fuzzy).is_some()).map(|unit| &unit.meta.unit_key).collect();
            prop_assert_eq!(actual, expected, "text={:?}, query={:?}, fuzzy={}", text, needle, fuzzy);
        }
    }

    #[test]
    fn r3_06_preview_keeps_the_selected_unicode_text(prefix in unicode_text(180), selected in "[a-zA-Z0-9]{1,200}", suffix in unicode_text(180)) {
        let source = format!("{prefix}{selected}{suffix}");
        let range = [prefix.chars().count(), prefix.chars().count() + selected.chars().count()];
        let (preview, truncated, [first, last]) = crate::preview(&source, range);
        prop_assert!(first < last && last <= preview.chars().count());
        prop_assert_eq!(preview.chars().skip(first).take(last - first).collect::<String>(), selected);
        prop_assert_eq!(truncated, source.chars().count() > 240);
    }

    #[test]
    fn r3_10_batch_dedup_keeps_first_spelling_and_order(raw in prop::collection::vec(unicode_text(12), 0..=300)) {
        let mut seen = HashSet::new();
        let expected: Vec<_> = raw.iter().map(|term| term.trim()).filter(|term| !term.is_empty())
            .filter(|term| seen.insert(reference_fold(term))).collect();
        let actual = batch::parse(&json!({"mode":"batch", "matchMode":"text", "terms":raw}));
        if expected.is_empty() || expected.len() > 256 {
            prop_assert!(actual.is_err());
        } else {
            let actual = actual.unwrap();
            prop_assert_eq!(actual.terms.len(), expected.len());
            for (index, (term, expected)) in actual.terms.iter().zip(expected).enumerate() {
                prop_assert_eq!(&term.text, expected);
                prop_assert_eq!(&term.id, &format!("batch:{}", index + 1));
            }
        }
    }

    #[test]
    fn r3_10_identifier_boundaries_and_fuzzy_independence(
        identifier in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        left in prop::sample::select(vec!['a', '9', '_', 'é', 'あ', '０', ' ', '-', '😀']),
        right in prop::sample::select(vec!['a', '9', '_', 'é', 'あ', '０', ' ', '-', '😀'])
    ) {
        let spec = batch::parse(&json!({"mode":"batch", "terms":[identifier]})).unwrap();
        let text = format!("{left}{identifier}{right}");
        let expected = !(left.is_alphanumeric() || left == '_' || right.is_alphanumeric() || right == '_');
        for fuzzy in [false, true] {
            let matched = spec.matched(&spec.terms[0], &text, fuzzy);
            prop_assert_eq!(matched.is_some(), expected);
            if let Some(matched) = matched {
                prop_assert_eq!(matched.range, [1, 1 + identifier.chars().count()]);
                prop_assert_eq!((matched.kind, matched.category, matched.score), ("identifier", "standard", 90));
            }
        }
    }

    #[test]
    fn r3_09_condition_scopes_agree_with_boolean_group_model(
        rows in prop::collection::vec((0u8..3, 1u32..6, any::<bool>(), any::<bool>(), any::<bool>()), 0..20)
    ) {
        let units: Vec<_> = rows.iter().enumerate().map(|(index, &(sheet, row, alpha, beta, blocked))| {
            let mut unit = Unit::new("cell", json!({"sheetName":format!("Sheet{sheet}"),"sheetIndex":sheet,"cellAddress":format!("A{row}")}),
                format!("{} {} {}", if alpha {"ALPHA_SIGNAL"} else {""}, if beta {"BETA_SIGNAL"} else {""}, if blocked {"BLOCK_SIGNAL"} else {""}));
            unit.meta.unit_key = index.to_string(); unit.meta.part_key = format!("sheet{sheet}");
            unit.meta.row = Some(row); unit.meta.column = Some(1); unit.meta.group_key = format!("sheet{sheet}#row:{row}");
            unit
        }).collect();
        let refs: Vec<_> = units.iter().collect();
        for scope in ["unit", "excelRow", "file"] {
            let mut groups: BTreeMap<(u8, u32), (bool, bool, bool)> = BTreeMap::new();
            for (index, &(sheet, row, alpha, beta, blocked)) in rows.iter().enumerate() {
                let key = match scope { "unit" => (0, index as u32), "excelRow" => (sheet, row), _ => (0, 0) };
                let entry = groups.entry(key).or_default(); entry.0 |= alpha; entry.1 |= beta; entry.2 |= blocked;
            }
            let expected = groups.values().filter(|&&(alpha, beta, blocked)| alpha && beta && !blocked).count();
            let spec = query::parse(&json!({"mode":"conditions", "scope":scope, "all":["ALPHA_SIGNAL","BETA_SIGNAL"], "not":["BLOCK_SIGNAL"]})).unwrap();
            for fuzzy in [false, true] {
                let actual = query::evaluate(&refs, "xlsx", &spec, fuzzy, &AtomicBool::new(false));
                prop_assert_eq!(actual.len(), expected, "scope={}", scope);
                for group in actual {
                    prop_assert_eq!(group.match_category, "standard");
                    prop_assert_eq!(group.evidence.len(), 2);
                    if scope == "excelRow" {
                        prop_assert!(group.evidence.iter().all(|e| e.unit.meta.row == group.row && e.unit.meta.part_key == group.part_key));
                    }
                }
            }
        }
    }

    #[test]
    fn r3_13_ranking_comparator_is_antisymmetric_and_transitive(a in ranking_input(), b in ranking_input(), c in ranking_input()) {
        let ab = ranking::compare(&a, &b);
        prop_assert_eq!(ab, ranking::compare(&b, &a).reverse());
        prop_assert_eq!(ranking::compare(&a, &a), std::cmp::Ordering::Equal);
        if ab != std::cmp::Ordering::Greater && ranking::compare(&b, &c) != std::cmp::Ordering::Greater {
            prop_assert!(ranking::compare(&a, &c) != std::cmp::Ordering::Greater);
        }
    }

    #[test]
    fn r3_13_ranking_evidence_is_order_independent_and_deduplicated(data in prop::collection::vec((0u8..=100, 0usize..8, any::<bool>()), 1..32)) {
        let hits: Vec<_> = data.iter().enumerate().map(|(index, &(score, term, body))| ranking_hit(index, score, term, body)).collect();
        let baseline = ranking::summarize(&hits, 0).unwrap();
        let mut reverse = hits.clone(); reverse.reverse();
        let reordered = ranking::summarize(&reverse, 0).unwrap();
        prop_assert_eq!(serde_json::to_value(&baseline.evidence).unwrap(), serde_json::to_value(&reordered.evidence).unwrap());
        prop_assert_eq!(&baseline.result_ids, &reordered.result_ids);
        reverse.extend(hits);
        let duplicated = ranking::summarize(&reverse, 0).unwrap();
        prop_assert_eq!(serde_json::to_value(&baseline.evidence).unwrap(), serde_json::to_value(&duplicated.evidence).unwrap());
        prop_assert!(baseline.evidence.body_evidence <= 5 && baseline.evidence.distinct_evidence <= 5);
        prop_assert_eq!(baseline.evidence.matched_terms, data.iter().map(|entry| entry.1).collect::<HashSet<_>>().len());
    }

    #[test]
    fn r3_04_utf8_bom_and_crlf_preserve_unicode_lines(lines in prop::collection::vec(line_fragment(), 0..16), bom in any::<bool>(), crlf in any::<bool>()) {
        let separator = if crlf { "\r\n" } else { "\n" };
        let mut bytes = if bom { vec![0xef, 0xbb, 0xbf] } else { vec![] };
        for line in &lines { bytes.extend_from_slice(line.as_bytes()); bytes.extend_from_slice(separator.as_bytes()); }
        // An initial U+FEFF without an explicit BOM is itself a BOM byte prefix.
        let expected: Vec<_> = lines.iter().enumerate().map(|(index, line)| {
            if !bom && index == 0 { line.strip_prefix('\u{feff}').unwrap_or(line).to_owned() } else { line.clone() }
        }).collect();
        let actual = extract::text_units(&bytes).unwrap();
        prop_assert_eq!(actual.len(), expected.len());
        for (index, (unit, text)) in actual.iter().zip(expected).enumerate() {
            prop_assert_eq!(&unit.text, &text);
            prop_assert_eq!(unit.location["lineNumber"].as_u64(), Some(index as u64 + 1));
        }
        prop_assert_eq!(actual.iter().map(|unit| &unit.meta.unit_key).collect::<HashSet<_>>().len(), actual.len());
    }

    #[test]
    fn r3_17_edit_save_cancel_and_external_changes_preserve_unselected_bytes(
        prefix in line_fragment(), suffix in line_fragment(), replacement in line_fragment(), bom in any::<bool>(), crlf in any::<bool>(), operation in 0u8..3
    ) {
        let fixture = Fixture::new(); let newline = if crlf { "\r\n" } else { "\n" };
        let header = if bom { "\u{feff}" } else { "" };
        // Distinct separators keep adjacent combining marks out of the identifier.
        let original = format!("{header}first{newline}{prefix} NEEDLE_SIGNAL {suffix}{newline}last{newline}");
        fixture.write("sample.txt", original.as_bytes());
        let events = fixture.run(fixture.request("NEEDLE_SIGNAL"));
        let hit = results(&events).into_iter().find(|hit| hit.source_kind == "textLine").unwrap();
        let (draft, view) = edit::prepare(hit, None).unwrap();
        prop_assert_eq!(view.selected_text, "NEEDLE_SIGNAL");
        match operation {
            0 => {
                draft.save(&replacement).unwrap();
                let expected = original.replacen("NEEDLE_SIGNAL", &replacement, 1);
                prop_assert_eq!(fs::read(fixture.0.join("sample.txt")).unwrap(), expected.as_bytes());
            }
            1 => { drop(draft); prop_assert_eq!(fs::read(fixture.0.join("sample.txt")).unwrap(), original.as_bytes()); }
            _ => {
                let external = original.replacen("first", "FIRST", 1); // Same byte length.
                fixture.write("sample.txt", external.as_bytes());
                prop_assert!(draft.save(&replacement).is_err());
                prop_assert!(edit::prepare(hit, None).is_err());
                prop_assert_eq!(fs::read(fixture.0.join("sample.txt")).unwrap(), external.as_bytes());
            }
        }
    }

    #[test]
    fn r3_11_generated_search_and_cancellation_keep_event_counts_and_published_ids(
        files in prop::collection::vec(prop::collection::vec(any::<bool>(), 0..8), 0..5), broken in any::<bool>(), stop_after in prop::option::of(1usize..20)
    ) {
        let fixture = Fixture::new(); let mut expected_hits = 0;
        for (index, lines) in files.iter().enumerate() {
            let contents: String = lines.iter().map(|matched| { if *matched { expected_hits += 1; "needle needle\n" } else { "other\n" } }).collect();
            fixture.write(&format!("sample{index}.txt"), contents);
        }
        if broken { fixture.write("zbroken.txt", [0x81]); }
        let cancel = AtomicBool::new(false); let mut hit_count = 0; let mut events = Vec::new();
        run_search(fixture.request("needle"), "property".into(), &cancel, |event| {
            if matches!(event.kind, EventKind::Result { .. }) { hit_count += 1; if Some(hit_count) == stop_after { cancel.store(true, Ordering::Relaxed); } }
            events.push(event);
        }).unwrap();
        let cancelled = stop_after.is_some_and(|limit| limit <= expected_hits);
        let expected_count = stop_after.map_or(expected_hits, |limit| expected_hits.min(limit));
        prop_assert_eq!(results(&events).len(), expected_count);
        prop_assert_eq!(events.iter().filter(|e| matches!(e.kind, EventKind::Started { .. })).count(), 1);
        prop_assert_eq!(events.iter().filter(|e| matches!(e.kind, EventKind::Finished { .. })).count(), 1);
        let mut published = HashSet::new(); let mut issue_count = 0;
        for (index, event) in events.iter().enumerate() {
            prop_assert_eq!(event.sequence, index + 1); prop_assert_eq!(&event.search_id, "property");
            match &event.kind {
                EventKind::Result { hit } => { prop_assert!(published.insert(hit.result_id)); }
                EventKind::Issue { .. } => { issue_count += 1; }
                EventKind::FileRanked { ranking } => { prop_assert!(ranking.result_ids.iter().all(|id| published.contains(id))); }
                _ => {}
            }
        }
        let EventKind::Finished { reason, counts } = &events.last().unwrap().kind else { panic!("Finished must be last") };
        prop_assert_eq!(*reason, if cancelled { "cancelled" } else { "completed" });
        prop_assert_eq!(counts.result_count, expected_count); prop_assert_eq!(counts.issue_count, issue_count);
        prop_assert_eq!(issue_count, usize::from(broken && !cancelled));
        prop_assert_eq!(counts.discovered_files, files.len() + usize::from(broken));
        prop_assert!(counts.processed_files <= counts.discovered_files);
        if !cancelled { prop_assert_eq!(counts.processed_files, counts.discovered_files); }
    }

    #[test]
    fn r3_10_generated_batch_summaries_match_term_by_file_model(
        files in prop::collection::vec(prop::collection::vec((any::<bool>(), any::<bool>()), 0..8), 0..5)
    ) {
        let fixture = Fixture::new();
        for (index, lines) in files.iter().enumerate() {
            let text: String = lines.iter().map(|&(a,b)| format!("{} {}\n", if a {"TAB_01 TAB_01"} else {""}, if b {"TAB_010"} else {""})).collect();
            fixture.write(&format!("sample{index}.txt"), text);
        }
        let mut request = fixture.request(""); request.query_spec = Some(json!({"mode":"batch", "terms":["TAB_01","tab_01","TAB_010"]}));
        let events = fixture.run(request);
        let summary = events.iter().find_map(|e| match &e.kind { EventKind::BatchSummary { terms } => Some(terms), _ => None }).unwrap();
        prop_assert_eq!(summary.len(), 2);
        for (index, term) in summary.iter().enumerate() {
            let selected = |pair: &(bool, bool)| if index == 0 { pair.0 } else { pair.1 };
            let expected_hits = files.iter().flatten().filter(|pair| selected(pair)).count();
            let expected_files = files.iter().filter(|lines| lines.iter().any(selected)).count();
            prop_assert_eq!((term.hit_count, term.file_count), (expected_hits, expected_files));
            prop_assert_eq!(&term.status, if expected_hits == 0 { "指定範囲内で該当なし" } else { "確認完了" });
        }
        prop_assert_eq!(results(&events).len(), summary.iter().map(|term| term.hit_count).sum::<usize>());
        let execution = events.iter().find_map(|e| match e.kind { EventKind::ExecutionSummary { extracted_files, reused_files } => Some((extracted_files,reused_files)), _ => None }).unwrap();
        prop_assert_eq!(execution, (files.len(), 0));
    }
}
