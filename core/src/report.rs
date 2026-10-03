use crate::SearchRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Tsv,
    Json,
    Matrixcsv,
    Matrixtsv,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv | Self::Matrixcsv => "csv",
            Self::Tsv | Self::Matrixtsv => "tsv",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema_version: u8,
    pub generated_at: String,
    pub search_id: String,
    pub request: SearchRequest,
    pub finished_reason: String,
    pub counts: ReportCounts,
    pub scope: String,
    pub filter_text: String,
    pub filter_extension: String,
    #[serde(default)]
    pub filter_include: String,
    #[serde(default)]
    pub filter_exclude: String,
    pub selected_file_count: usize,
    pub rows: Vec<ReportRow>,
    pub issues: Vec<ReportIssue>,
    #[serde(default)]
    pub rankings: Vec<crate::ranking::FileRanking>,
    #[serde(default)]
    pub batch_summary: Vec<crate::batch::TermSummary>,
    #[serde(default)]
    pub execution_summary: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportCounts {
    pub discovered_files: usize,
    pub processed_files: usize,
    pub result_count: usize,
    pub issue_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    #[serde(default)]
    pub modified_at: Option<f64>,
    #[serde(default)]
    pub document_order: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub term_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub term: Option<String>,
    pub result_id: usize,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    pub source_kind: String,
    pub unit_key: String,
    pub part_key: String,
    pub group_key: String,
    pub row: Option<u32>,
    pub column: Option<u32>,
    pub content_class: String,
    pub anchor: Option<String>,
    pub location: String,
    pub location_data: Value,
    pub preview_text: String,
    pub preview_truncated: bool,
    pub match_ranges: Vec<[usize; 2]>,
    pub match_type: String,
    pub match_category: String,
    pub score: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportIssue {
    pub stage: String,
    pub path: Option<String>,
    pub code: String,
    pub reason: String,
}

impl Report {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.search_id.is_empty()
            || !matches!(
                self.finished_reason.as_str(),
                "completed" | "cancelled" | "failed"
            )
            || !matches!(self.scope.as_str(), "all" | "filtered")
            || self.issues.len() != self.counts.issue_count
            || (self.scope == "all" && self.rows.len() != self.counts.result_count)
        {
            return Err("出力する検索記録の形式または件数が不正です。".into());
        }
        Ok(())
    }
}

const HEADERS: [&str; 10] = [
    "ファイル名",
    "絶対パス",
    "形式",
    "検索箇所",
    "内容区分",
    "場所",
    "抜粋",
    "抜粋省略",
    "一致方式",
    "一致分類",
];

fn safe_cell(value: &str) -> String {
    if value
        .trim_start_matches(|ch: char| ch.is_whitespace() || ch.is_control())
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, '=' | '+' | '-' | '@'))
    {
        format!("'{value}")
    } else {
        value.to_owned()
    }
}

fn csv_cell(value: &str) -> String {
    let value = safe_cell(value);
    if value
        .chars()
        .any(|ch| matches!(ch, ',' | '"' | '\r' | '\n'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn tsv_cell(value: &str) -> String {
    safe_cell(value)
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn source_label(kind: &str) -> &str {
    match kind {
        "fileName" => "ファイル名",
        "cell" => "セル",
        "formula" => "Excel数式文字列",
        "note" => "PowerPointノート",
        "excelComment" => "Excelコメント",
        "wordHeader" => "Wordヘッダー",
        "wordFooter" => "Wordフッター",
        "wordComment" => "Wordコメント",
        "excelRow" => "Excelの行",
        "fileMatch" => "ファイル全体",
        "shape" => "図形",
        "slideTableCell" => "スライド表のセル",
        "paragraph" => "本文段落",
        "wordTableParagraph" => "Word表内の段落",
        "textLine" => "テキスト行",
        _ => kind,
    }
}

fn content_label(kind: &str) -> &str {
    match kind {
        "body" => "本文",
        "fileName" => "ファイル名",
        "note" => "注記",
        "formula" => "数式文字列",
        _ => kind,
    }
}

fn match_label(kind: &str) -> &str {
    match kind {
        "exact" => "完全一致",
        "caseFolded" | "normalized" | "separatorVariant" | "kanaVariant" => "表記揺れ",
        "identifier" => "識別子一致",
        "prefix" => "前方一致",
        "substring" => "部分一致",
        "editDistance" => "タイプミス候補",
        _ => kind,
    }
}

fn category_label(kind: &str) -> &str {
    match kind {
        "standard" => "一致検索",
        "fuzzy" => "あいまい検索",
        _ => kind,
    }
}

pub fn table_text(report: &Report, format: Format) -> Result<String, String> {
    report.validate()?;
    let (separator, cell): (&str, fn(&str) -> String) = match format {
        Format::Csv | Format::Matrixcsv => (",", csv_cell),
        Format::Tsv | Format::Matrixtsv => ("\t", tsv_cell),
        Format::Json => return Err("表形式を指定してください。".into()),
    };
    let mut output = String::new();
    if matches!(format, Format::Matrixcsv | Format::Matrixtsv) {
        if report
            .request
            .query_spec
            .as_ref()
            .is_none_or(|spec| spec["mode"] != "batch")
        {
            return Err("語×ファイル出力は一括検索で利用できます。".into());
        }
        let mut remaining: std::collections::BTreeSet<&str> = report
            .rows
            .iter()
            .map(|row| row.file_path.as_str())
            .collect();
        let mut files = Vec::new();
        for ranking in &report.rankings {
            let path = ranking.file_path.as_str();
            if remaining.remove(path) {
                files.push(path);
            }
        }
        files.extend(remaining);
        let mut headers = vec![
            "検索語ID".to_owned(),
            "検索語".into(),
            "確認状態".into(),
            "出力範囲".into(),
        ];
        headers.extend(files.iter().map(|path| (*path).into()));
        output.push_str(
            &headers
                .iter()
                .map(|value| cell(value))
                .collect::<Vec<_>>()
                .join(separator),
        );
        output.push_str("\r\n");
        let mut counts = std::collections::HashMap::new();
        for row in &report.rows {
            if let Some(id) = &row.term_id {
                *counts
                    .entry((id.as_str(), row.file_path.as_str()))
                    .or_insert(0usize) += 1;
            }
        }
        for term in &report.batch_summary {
            let mut values = vec![
                cell(&term.term_id),
                cell(&term.term),
                cell(&term.status),
                cell(&report.scope),
            ];
            values.extend(files.iter().map(|path| {
                counts
                    .get(&(term.term_id.as_str(), *path))
                    .copied()
                    .unwrap_or(0)
                    .to_string()
            }));
            output.push_str(&values.join(separator));
            output.push_str("\r\n");
        }
        return Ok(output);
    }
    let advanced = report
        .request
        .query_spec
        .as_ref()
        .is_some_and(|spec| spec["mode"] == "conditions");
    let batch = report
        .request
        .query_spec
        .as_ref()
        .is_some_and(|spec| spec["mode"] == "batch");
    let mut headers = HEADERS.to_vec();
    if advanced {
        headers.push("検索語と根拠");
    }
    if batch {
        headers.extend(["検索語ID", "検索語"]);
    }
    headers.push("順位理由");
    output.push_str(&headers.join(separator));
    output.push_str("\r\n");
    for row in &report.rows {
        let fields = [
            row.file_name.as_str(),
            row.file_path.as_str(),
            row.file_type.as_str(),
            source_label(&row.source_kind),
            content_label(&row.content_class),
            row.location.as_str(),
            row.preview_text.as_str(),
            if row.preview_truncated {
                "あり"
            } else {
                "なし"
            },
            match_label(&row.match_type),
            category_label(&row.match_category),
        ];
        let mut values = fields.map(cell).to_vec();
        if advanced {
            let evidence = row
                .evidence
                .iter()
                .map(|item| {
                    format!(
                        "{}: {} — {}",
                        item["term"].as_str().unwrap_or(""),
                        item["locationText"].as_str().unwrap_or(""),
                        item["previewText"].as_str().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ");
            values.push(cell(&evidence));
        }
        if batch {
            values.push(cell(row.term_id.as_deref().unwrap_or("")));
            values.push(cell(row.term.as_deref().unwrap_or("")));
        }
        let reasons = report
            .rankings
            .iter()
            .find(|ranking| ranking.file_path == row.file_path)
            .map(|ranking| ranking.reasons.join(" / "))
            .unwrap_or_default();
        values.push(cell(&reasons));
        output.push_str(&values.join(separator));
        output.push_str("\r\n");
    }
    Ok(output)
}

pub fn file_bytes(report: &Report, format: Format) -> Result<Vec<u8>, String> {
    report.validate()?;
    match format {
        Format::Csv | Format::Matrixcsv => {
            let mut bytes = vec![0xef, 0xbb, 0xbf];
            bytes.extend_from_slice(table_text(report, format)?.as_bytes());
            Ok(bytes)
        }
        Format::Tsv | Format::Matrixtsv => Ok(table_text(report, format)?.into_bytes()),
        Format::Json => {
            serde_json::to_vec_pretty(report).map_err(|_| "調査記録を作成できませんでした。".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    fn sample(preview: &str) -> Report {
        Report {
            schema_version: 1,
            generated_at: "2026-09-30T00:00:00Z".into(),
            search_id: "search-1".into(),
            request: SearchRequest {
                root_directory: "C:\\docs".into(),
                additional_directories: vec![],
                excluded_directories: vec![],
                query: "顧客".into(),
                query_spec: None,
                recursive: true,
                extensions: vec!["xlsx".into()],
                use_index: false,
                fuzzy_search: false,
                include_notes: false,
                include_formulas: false,
            },
            finished_reason: "completed".into(),
            counts: ReportCounts {
                discovered_files: 1,
                processed_files: 1,
                result_count: 1,
                issue_count: 0,
            },
            scope: "all".into(),
            filter_text: String::new(),
            filter_extension: String::new(),
            filter_include: String::new(),
            filter_exclude: String::new(),
            selected_file_count: 1,
            rows: vec![ReportRow {
                modified_at: None,
                document_order: vec![],
                term_id: None,
                term: None,
                result_id: 1,
                file_name: "設計.xlsx".into(),
                file_path: "C:\\docs\\設計.xlsx".into(),
                file_type: "xlsx".into(),
                source_kind: "cell".into(),
                unit_key: "1".into(),
                part_key: "xl/worksheets/sheet1.xml".into(),
                group_key: "xl/worksheets/sheet1.xml#row:1".into(),
                row: Some(1),
                column: Some(1),
                content_class: "body".into(),
                anchor: None,
                location: "Sheet1!A1".into(),
                location_data: json!({"sheetName": "Sheet1", "cellAddress": "A1"}),
                preview_text: preview.into(),
                preview_truncated: false,
                match_ranges: vec![[0, 2]],
                match_type: "exact".into(),
                match_category: "standard".into(),
                score: 100,
                evidence: vec![],
            }],
            issues: vec![],
            rankings: vec![],
            batch_summary: vec![],
            execution_summary: None,
        }
    }

    #[test]
    fn csv_quotes_fields_and_adds_bom() {
        let report = sample("顧客,\"必須\"\n次行");
        let bytes = file_bytes(&report, Format::Csv).unwrap();
        assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        let csv = std::str::from_utf8(&bytes[3..]).unwrap();
        assert!(csv.contains("\"顧客,\"\"必須\"\"\n次行\""));
    }

    #[test]
    fn tsv_escapes_controls_and_formula_prefix() {
        let report = sample(" \t=1\\x\n次行");
        let tsv = table_text(&report, Format::Tsv).unwrap();
        assert!(tsv.contains("' \\t=1\\\\x\\n次行"));
        assert_eq!(tsv.lines().count(), 2);
        assert!(!file_bytes(&report, Format::Tsv)
            .unwrap()
            .starts_with(&[0xef, 0xbb, 0xbf]));
        let json = String::from_utf8(file_bytes(&report, Format::Json).unwrap()).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&json).unwrap()["rows"][0]["previewText"],
            " \t=1\\x\n次行"
        );
    }

    #[test]
    fn rejects_mismatched_unfiltered_count() {
        let mut report = sample("顧客");
        report.counts.result_count = 2;
        assert!(table_text(&report, Format::Csv).is_err());
    }

    #[test]
    fn advanced_report_keeps_terms_and_evidence() {
        let mut report = sample("顧客番号");
        report.request.query.clear();
        report.request.query_spec = Some(json!({
            "mode": "conditions", "scope": "excelRow",
            "all": ["顧客", "必須"], "any": [], "not": []
        }));
        report.rows[0].source_kind = "excelRow".into();
        report.rows[0].location = "Sheet1 / 行 1".into();
        report.rows[0].evidence = vec![
            json!({"termId": "all:1", "term": "顧客", "locationText": "Sheet1!A1", "previewText": "顧客番号"}),
            json!({"termId": "all:2", "term": "必須", "locationText": "Sheet1!D1", "previewText": "必須"}),
        ];
        let csv = table_text(&report, Format::Csv).unwrap();
        assert!(csv
            .lines()
            .next()
            .unwrap()
            .ends_with("検索語と根拠,順位理由"));
        assert!(csv.contains("顧客: Sheet1!A1 — 顧客番号 | 必須: Sheet1!D1 — 必須"));
        let saved: Value =
            serde_json::from_slice(&file_bytes(&report, Format::Json).unwrap()).unwrap();
        assert_eq!(saved["request"]["querySpec"]["scope"], "excelRow");
        assert_eq!(saved["rows"][0]["evidence"][1]["termId"], "all:2");
    }

    #[test]
    fn batch_matrix_preserves_zero_terms_uncertainty_and_safe_cells() {
        let mut report = sample("顧客");
        report.request.query.clear();
        report.request.query_spec = Some(json!({
            "mode": "batch", "terms": ["=TAB,01", "MISSING"], "matchMode": "text"
        }));
        report.rows[0].term_id = Some("batch:1".into());
        report.rows[0].term = Some("=TAB,01".into());
        let mut second = sample("顧客").rows.remove(0);
        second.result_id = 2;
        second.file_path = "C:\\docs\\a.txt".into();
        second.file_name = "a.txt".into();
        second.file_type = "txt".into();
        second.term_id = Some("batch:1".into());
        second.term = Some("=TAB,01".into());
        report.rows.push(second);
        report.counts.discovered_files = 3;
        report.counts.processed_files = 3;
        report.counts.result_count = 2;
        report.counts.issue_count = 1;
        report.issues = vec![ReportIssue {
            stage: "read".into(),
            path: Some("C:\\docs\\broken.pptx".into()),
            code: "unreadable".into(),
            reason: "注記部品を読めません。".into(),
        }];
        report.selected_file_count = 2;
        report.rankings = report
            .rows
            .iter()
            .map(|row| crate::ranking::FileRanking {
                file_path: row.file_path.clone(),
                evidence: Default::default(),
                reasons: vec![],
                result_ids: vec![row.result_id],
            })
            .collect();
        report.scope = "filtered".into();
        report.batch_summary = vec![
            crate::batch::TermSummary {
                term_id: "batch:1".into(),
                term: "=TAB,01".into(),
                file_count: 2,
                hit_count: 2,
                status: "未確定（一部エラーあり）".into(),
            },
            crate::batch::TermSummary {
                term_id: "batch:2".into(),
                term: "MISSING".into(),
                file_count: 0,
                hit_count: 0,
                status: "未確定（一部エラーあり）".into(),
            },
        ];
        let csv = table_text(&report, Format::Matrixcsv).unwrap();
        assert!(csv
            .lines()
            .next()
            .unwrap()
            .ends_with("C:\\docs\\設計.xlsx,C:\\docs\\a.txt"));
        assert!(csv.contains("batch:1,\"'=TAB,01\",未確定（一部エラーあり）,filtered,1,1\r\n"));
        assert!(csv.contains("batch:2,MISSING,未確定（一部エラーあり）,filtered,0,0\r\n"));
        assert!(file_bytes(&report, Format::Matrixcsv)
            .unwrap()
            .starts_with(&[0xef, 0xbb, 0xbf]));
        report.rows.clear();
        report.selected_file_count = 0;
        let tsv = table_text(&report, Format::Matrixtsv).unwrap();
        assert_eq!(tsv.lines().count(), 3);
        assert!(tsv.contains("batch:2\tMISSING\t未確定（一部エラーあり）\tfiltered\r\n"));
        let saved: Value =
            serde_json::from_slice(&file_bytes(&report, Format::Json).unwrap()).unwrap();
        assert_eq!(saved["batchSummary"][0]["term"], "=TAB,01");
        assert!(table_text(&sample("顧客"), Format::Matrixcsv).is_err());
    }

    proptest! {
        #[test]
        fn r3_18_json_preserves_arbitrary_unicode_and_controls(chars in prop::collection::vec(any::<char>(), 0..400)) {
            let payload: String = chars.into_iter().collect();
            let mut report = sample(&payload);
            report.rows[0].match_ranges.clear();
            let encoded = file_bytes(&report, Format::Json).unwrap();
            let decoded: Report = serde_json::from_slice(&encoded).unwrap();
            prop_assert_eq!(decoded.rows[0].preview_text.as_str(), payload.as_str());
            prop_assert_eq!(serde_json::to_value(&decoded).unwrap(), serde_json::to_value(&report).unwrap());
        }

        #[test]
        fn r3_18_tsv_controls_cannot_create_extra_rows_or_columns(payload in "[A-Za-z0-9=+@ \\t,\"\\r\\n\\\\]{0,200}") {
            let mut report = sample(&payload);
            report.rows[0].match_ranges.clear();
            let text = table_text(&report, Format::Tsv).unwrap();
            let lines: Vec<_> = text.split("\r\n").collect();
            prop_assert_eq!(lines.len(), 3);
            prop_assert_eq!(lines[0].split('\t').count(), lines[1].split('\t').count());
            let preview = lines[1].split('\t').nth(6).unwrap();
            prop_assert!(!preview.contains(['\r', '\n', '\t']));
            if payload.trim_start().starts_with(['=', '+', '-', '@']) {
                prop_assert!(preview.starts_with('\''));
            }
        }
    }
}
