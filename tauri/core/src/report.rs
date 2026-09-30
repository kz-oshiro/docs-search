use crate::SearchRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Tsv,
    Json,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
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
    pub selected_file_count: usize,
    pub rows: Vec<ReportRow>,
    pub issues: Vec<ReportIssue>,
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
        Format::Csv => (",", csv_cell),
        Format::Tsv => ("\t", tsv_cell),
        Format::Json => return Err("表形式を指定してください。".into()),
    };
    let mut output = String::new();
    output.push_str(&HEADERS.join(separator));
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
        output.push_str(&fields.map(cell).join(separator));
        output.push_str("\r\n");
    }
    Ok(output)
}

pub fn file_bytes(report: &Report, format: Format) -> Result<Vec<u8>, String> {
    report.validate()?;
    match format {
        Format::Csv => {
            let mut bytes = vec![0xef, 0xbb, 0xbf];
            bytes.extend_from_slice(table_text(report, format)?.as_bytes());
            Ok(bytes)
        }
        Format::Tsv => Ok(table_text(report, format)?.into_bytes()),
        Format::Json => {
            serde_json::to_vec_pretty(report).map_err(|_| "調査記録を作成できませんでした。".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            selected_file_count: 1,
            rows: vec![ReportRow {
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
            }],
            issues: vec![],
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
}
