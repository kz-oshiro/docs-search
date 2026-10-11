// Exercise the desktop's position contract without Excel or an OS process.
#[path = "../../src-tauri/src/excel_target.rs"]
mod excel_target;

use docs_search_core::{SearchEvidence, SearchHit};
use serde_json::json;

fn hit() -> SearchHit {
    SearchHit {
        result_id: 7,
        file_path: "C:\\日本語 空白\\book.xlsx".into(),
        file_type: "xlsx".into(),
        source_kind: "cell".into(),
        location: json!({"sheetName": "集計'表", "cellAddress": "XFD1048576"}),
        source_match_ranges: vec![],
        document_order: vec![],
        modified_at: None,
        edit_anchor: None,
        term_id: None,
        term: None,
        unit_key: "cell:1".into(),
        part_key: "sheet:1".into(),
        group_key: "row:1".into(),
        row: Some(1_048_576),
        column: Some(16_384),
        content_class: "body".into(),
        anchor: None,
        preview_text: "beacon".into(),
        preview_truncated: false,
        match_ranges: vec![],
        match_type: "exact",
        match_category: "standard",
        score: 100,
        evidence: vec![],
    }
}

fn evidence(sheet: &str, address: &str) -> SearchEvidence {
    SearchEvidence {
        term_id: "all:1".into(),
        term: "beacon".into(),
        source_match_ranges: vec![],
        document_order: vec![],
        edit_anchor: None,
        unit_key: "cell:2".into(),
        part_key: "sheet:2".into(),
        source_kind: "formula".into(),
        content_class: "formula".into(),
        location: json!({"sheetName": sheet, "cellAddress": address}),
        row: Some(2),
        column: Some(2),
        preview_text: "beacon".into(),
        preview_truncated: false,
        match_ranges: vec![],
        match_type: "exact",
        match_category: "standard",
        score: 100,
    }
}

#[test]
fn uses_structured_position_and_preserves_path_and_sheet() {
    let source = hit();
    let target = excel_target::resolve(&source, None).unwrap();
    assert_eq!(target.file_path, source.file_path);
    assert_eq!(target.sheet_name, "集計'表");
    assert_eq!(target.cell_address, "XFD1048576");
    assert!(excel_target::resolve(&source, Some(0)).is_err());
}

#[test]
fn refuses_non_cell_addresses_and_out_of_bounds_positions() {
    let mut source = hit();
    for address in [
        "XFE1", "A1048577", "A0", "A01", "A1:B2", "$A$1", "a1", "A1;exit", "😀1", "AAAA1", "",
    ] {
        source.location["cellAddress"] = json!(address);
        assert!(excel_target::resolve(&source, None).is_err(), "{address}");
    }
    source.location = json!({"sheetName": "", "cellAddress": "A1"});
    assert!(excel_target::resolve(&source, None).is_err());
}

#[test]
fn requires_explicit_evidence_for_rows_and_file_conditions() {
    let mut source = hit();
    source.evidence = vec![evidence("first", "A1"), evidence("other", "D900")];
    for kind in ["excelRow", "fileMatch"] {
        source.source_kind = kind.into();
        assert!(excel_target::resolve(&source, None).is_err());
        assert!(excel_target::resolve(&source, Some(2)).is_err());
        let target = excel_target::resolve(&source, Some(1)).unwrap();
        assert_eq!(target.sheet_name, "other");
        assert_eq!(target.cell_address, "D900");
    }
}

#[test]
fn handles_formulas_comments_and_shape_anchors_without_inventing_positions() {
    let mut source = hit();
    source.file_type = "xlsm".into();
    for kind in ["formula", "excelComment"] {
        source.source_kind = kind.into();
        assert!(excel_target::resolve(&source, None).is_ok());
    }
    source.source_kind = "shape".into();
    assert!(excel_target::resolve(&source, None).is_err());
    source.location["anchor"] = json!("C8");
    assert_eq!(
        excel_target::resolve(&source, None).unwrap().cell_address,
        "C8"
    );
    for kind in ["fileName", "excelRow", "textLine"] {
        source.source_kind = kind.into();
        assert!(excel_target::resolve(&source, None).is_err());
    }
    source.file_type = "txt".into();
    source.source_kind = "cell".into();
    assert!(excel_target::resolve(&source, None).is_err());
}
