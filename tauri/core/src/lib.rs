pub mod batch;
pub mod context;
pub mod edit;
mod extract;
mod fuzzy;
mod index;
mod query;
pub mod ranking;
pub mod report;

#[cfg(test)]
mod property_tests;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;
#[cfg(test)]
use unicode_segmentation::UnicodeSegmentation;

pub use extract::Unit;

pub fn clear_search_index() -> Result<(), String> {
    index::clear()
}

const MAX_TEXT_BYTES: u64 = 32 * 1024 * 1024;
const OFFICE_EXTENSIONS: [&str; 4] = ["xlsx", "xlsm", "pptx", "docx"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Office documents
    "xlsx",
    "xlsm",
    "pptx",
    "docx",
    // Text, documentation, and tabular data
    "txt",
    "md",
    "markdown",
    "mdx",
    "rst",
    "adoc",
    "log",
    "csv",
    "tsv",
    // Web and markup
    "jsp",
    "xhtml",
    "html",
    "htm",
    "vue",
    "svelte",
    "astro",
    "css",
    "scss",
    "sass",
    "less",
    "svg",
    // JavaScript and TypeScript
    "js",
    "mjs",
    "cjs",
    "jsx",
    "ts",
    "mts",
    "cts",
    "tsx",
    // Programming languages and scripts
    "java",
    "kt",
    "kts",
    "groovy",
    "gradle",
    "py",
    "rb",
    "php",
    "go",
    "rs",
    "c",
    "h",
    "cc",
    "cpp",
    "hpp",
    "cs",
    "swift",
    "scala",
    "sh",
    "bash",
    "zsh",
    "ps1",
    "bat",
    "cmd",
    "sql",
    "dart",
    "lua",
    "r",
    "jl",
    "pl",
    "ex",
    "exs",
    "clj",
    "cljs",
    "fs",
    "fsx",
    "vb",
    "vbs",
    // Structured data and configuration
    "json",
    "jsonc",
    "jsonl",
    "ndjson",
    "yaml",
    "yml",
    "toml",
    "xml",
    "xsd",
    "xsl",
    "xslt",
    "ini",
    "cfg",
    "conf",
    "properties",
    "tf",
    "hcl",
    "tfvars",
    // Schemas and notebooks
    "graphql",
    "gql",
    "proto",
    "ipynb",
];
pub const DEFAULT_EXTENSIONS: [&str; 5] = ["xlsx", "xlsm", "pptx", "docx", "txt"];

pub fn default_extensions() -> Vec<String> {
    DEFAULT_EXTENSIONS
        .iter()
        .map(|ext| (*ext).to_owned())
        .collect()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub root_directory: String,
    #[serde(default)]
    pub additional_directories: Vec<String>,
    #[serde(default)]
    pub excluded_directories: Vec<String>,
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_spec: Option<Value>,
    pub recursive: bool,
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub use_index: bool,
    #[serde(default)]
    pub fuzzy_search: bool,
    #[serde(default)]
    pub include_notes: bool,
    #[serde(default)]
    pub include_formulas: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputError {
    pub field: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub discovered_files: usize,
    pub processed_files: usize,
    pub result_count: usize,
    pub issue_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub source_match_ranges: Vec<[usize; 2]>,
    pub document_order: Vec<u64>,
    pub modified_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_anchor: Option<edit::EditAnchor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term: Option<String>,
    pub result_id: usize,
    pub file_path: String,
    pub file_type: String,
    pub source_kind: String,
    pub unit_key: String,
    pub part_key: String,
    pub group_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    pub content_class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    pub location: Value,
    pub preview_text: String,
    pub preview_truncated: bool,
    pub match_ranges: Vec<[usize; 2]>,
    pub match_type: &'static str,
    pub match_category: &'static str,
    pub score: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<SearchEvidence>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEvidence {
    pub source_match_ranges: Vec<[usize; 2]>,
    pub document_order: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_anchor: Option<edit::EditAnchor>,
    pub term_id: String,
    pub term: String,
    pub unit_key: String,
    pub part_key: String,
    pub source_kind: String,
    pub content_class: String,
    pub location: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    pub preview_text: String,
    pub preview_truncated: bool,
    pub match_ranges: Vec<[usize; 2]>,
    pub match_type: &'static str,
    pub match_category: &'static str,
    pub score: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIssue {
    pub stage: &'static str,
    pub path: Option<String>,
    pub code: &'static str,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEvent {
    pub search_id: String,
    pub sequence: usize,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EventKind {
    Started {
        request: SearchRequest,
    },
    Progress {
        phase: &'static str,
        counts: Counts,
    },
    Result {
        hit: SearchHit,
    },
    Issue {
        issue: SearchIssue,
    },
    FileRanked {
        ranking: ranking::FileRanking,
    },
    RankingSummary {
        files: Vec<String>,
    },
    BatchSummary {
        terms: Vec<batch::TermSummary>,
    },
    ExecutionSummary {
        #[serde(rename = "extractedFiles")]
        extracted_files: usize,
        #[serde(rename = "reusedFiles")]
        reused_files: usize,
    },
    Finished {
        reason: &'static str,
        counts: Counts,
    },
}

struct SearchPaths {
    roots: Vec<PathBuf>,
    excluded: Vec<PathBuf>,
}

fn canonical_directory(raw: &str, field: &'static str) -> Result<PathBuf, InputError> {
    let path = Path::new(raw);
    if raw.trim().is_empty() || !path.is_dir() {
        return Err(InputError {
            field,
            message: "存在するフォルダーを指定してください。",
        });
    }
    fs::canonicalize(path).map_err(|_| InputError {
        field,
        message: "フォルダーを読み取れません。",
    })
}

fn path_key(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

fn remove_nested_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then_with(|| a.cmp(b))
    });
    let mut kept: Vec<(PathBuf, PathBuf)> = Vec::new();
    for path in paths {
        let key = path_key(&path);
        if !kept.iter().any(|(_, parent)| key.starts_with(parent)) {
            kept.push((path, key));
        }
    }
    kept.into_iter().map(|(path, _)| path).collect()
}

fn validate_paths(request: &SearchRequest) -> Result<SearchPaths, InputError> {
    if let Some(spec) = &request.query_spec {
        if !request.query.trim().is_empty() {
            return Err(InputError {
                field: "querySpec",
                message: "通常検索語と高度な検索条件は同時に指定できません。",
            });
        }
        if spec["mode"] == "batch" {
            batch::parse(spec)?;
        } else {
            query::parse(spec)?;
        }
    } else if request.query.trim().is_empty() {
        return Err(InputError {
            field: "query",
            message: "検索語を入力してください。",
        });
    }
    if !request.recursive {
        return Err(InputError {
            field: "rootDirectory",
            message: "この版はサブフォルダーを含む検索のみ対応します。",
        });
    }
    if request.extensions.is_empty() {
        return Err(InputError {
            field: "extensions",
            message: "検索する拡張子を1つ以上選んでください。",
        });
    }
    if request
        .extensions
        .iter()
        .any(|ext| !SUPPORTED_EXTENSIONS.contains(&ext.as_str()))
    {
        return Err(InputError {
            field: "extensions",
            message: "対応していない拡張子が含まれています。",
        });
    }
    let mut roots = vec![canonical_directory(
        &request.root_directory,
        "rootDirectory",
    )?];
    for directory in &request.additional_directories {
        roots.push(canonical_directory(directory, "additionalDirectories")?);
    }
    let mut excluded = Vec::with_capacity(request.excluded_directories.len());
    for directory in &request.excluded_directories {
        excluded.push(canonical_directory(directory, "excludedDirectories")?);
    }
    Ok(SearchPaths {
        roots: remove_nested_paths(roots),
        excluded: remove_nested_paths(excluded),
    })
}

pub fn validate(request: &SearchRequest) -> Result<(), InputError> {
    validate_paths(request).map(|_| ())
}

pub fn normalize_fold(text: &str) -> String {
    text.nfc().case_fold().collect()
}

#[cfg(test)]
fn match_ranges(text: &str, needle: &str) -> Vec<[usize; 2]> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut folded = String::new();
    let mut source_ranges = Vec::new();
    let mut source_index = 0;
    for grapheme in text.graphemes(true) {
        let end = source_index + grapheme.chars().count();
        let part = normalize_fold(grapheme);
        source_ranges.extend(std::iter::repeat_n([source_index, end], part.len()));
        folded.push_str(&part);
        source_index = end;
    }
    debug_assert_eq!(folded, normalize_fold(text));

    let mut ranges: Vec<[usize; 2]> = Vec::new();
    for (offset, _) in folded.match_indices(needle) {
        let start = source_ranges[offset][0];
        let end = source_ranges[offset + needle.len() - 1][1];
        if let Some(last) = ranges.last_mut() {
            if start <= last[1] {
                last[1] = end;
                continue;
            }
        }
        ranges.push([start, end]);
    }
    ranges
}

fn preview(value: &str, range: [usize; 2]) -> (String, bool, [usize; 2]) {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 240 {
        return (value.to_owned(), false, range);
    }
    let start = range[0].saturating_sub(60).min(chars.len());
    let end = (start + 180.max(range[1].saturating_sub(start) + 60)).min(chars.len());
    let before = if start > 0 { "… " } else { "" };
    let after = if end < chars.len() { " …" } else { "" };
    (
        format!(
            "{before}{}{after}",
            chars[start..end].iter().collect::<String>()
        ),
        true,
        [
            range[0] - start + before.chars().count(),
            range[1] - start + before.chars().count(),
        ],
    )
}

fn issue_for(
    path: Option<&Path>,
    stage: &'static str,
    error: &extract::ExtractError,
) -> SearchIssue {
    SearchIssue {
        stage,
        path: path.map(display_path),
        code: error.code,
        reason: error.message.clone(),
    }
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return rest.to_owned();
        }
    }
    value.into_owned()
}

fn supported(path: &Path, selected: &HashSet<String>) -> Option<String> {
    let name = path.file_name()?.to_string_lossy();
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if name.starts_with("~$") && ["xlsx", "xlsm", "pptx", "docx"].contains(&ext.as_str()) {
        return None;
    }
    if selected.contains(&ext) {
        Some(ext)
    } else {
        None
    }
}

fn file_name_unit(name: String) -> Unit {
    let mut unit = Unit::new("fileName", json!({}), name);
    unit.meta.unit_key = "0".into();
    unit.meta.group_key = "fileName".into();
    unit.meta.content_class = "fileName".into();
    unit
}

struct Emitter<F: FnMut(SearchEvent)> {
    id: String,
    sequence: usize,
    counts: Counts,
    emit: F,
    file_hits: Vec<SearchHit>,
    rankings: Vec<ranking::FileRanking>,
    batch_counts: std::collections::HashMap<String, (usize, usize)>,
    extracted_files: usize,
    reused_files: usize,
    same_row_terms: usize,
    modified_at: Option<f64>,
    edit_revision: Option<String>,
}

fn document_order(unit: &Unit) -> Vec<u64> {
    let sequence = unit.meta.unit_key.parse().unwrap_or(0);
    if unit.source_kind == "fileName" {
        return vec![0];
    }
    if let Some(sheet) = unit.location["sheetIndex"].as_u64() {
        let address = unit.location["cellAddress"]
            .as_str()
            .or(unit.meta.anchor.as_deref());
        let position = address.and_then(extract::parse_cell_address);
        let row = unit
            .meta
            .row
            .or(position.map(|p| p.0))
            .map(u64::from)
            .unwrap_or(u64::MAX);
        let column = unit
            .meta
            .column
            .or(position.map(|p| p.1))
            .map(u64::from)
            .unwrap_or(u64::MAX);
        let kind = match unit.source_kind {
            "cell" => 0,
            "formula" => 1,
            "excelComment" => 2,
            _ => 3,
        };
        return vec![1, sheet, row, column, kind, sequence];
    }
    vec![
        1,
        unit.location["lineNumber"]
            .as_u64()
            .or(unit.location["slideNumber"].as_u64())
            .unwrap_or(sequence),
        sequence,
    ]
}

impl<F: FnMut(SearchEvent)> Emitter<F> {
    fn send(&mut self, kind: EventKind) {
        self.sequence += 1;
        (self.emit)(SearchEvent {
            search_id: self.id.clone(),
            sequence: self.sequence,
            kind,
        });
    }
    fn progress(&mut self, phase: &'static str) {
        self.send(EventKind::Progress {
            phase,
            counts: self.counts.clone(),
        });
    }
    fn issue(&mut self, issue: SearchIssue) {
        self.counts.issue_count += 1;
        self.send(EventKind::Issue { issue });
    }
    fn hit(&mut self, path: &Path, file_type: &str, unit: Unit, matched: fuzzy::Match) {
        self.term_hit(path, file_type, unit, matched, None);
    }
    fn publish(&mut self, hit: SearchHit) {
        if let Some(id) = &hit.term_id {
            self.batch_counts.entry(id.clone()).or_default().1 += 1;
        }
        self.file_hits.push(hit.clone());
        self.send(EventKind::Result { hit });
    }
    fn finish_file(&mut self) {
        let mut ids = HashSet::new();
        for hit in &self.file_hits {
            if let Some(id) = &hit.term_id {
                ids.insert(id.clone());
            }
        }
        for id in ids {
            self.batch_counts.entry(id).or_default().0 += 1;
        }
        if let Some(ranking) = ranking::summarize(&self.file_hits, self.same_row_terms) {
            self.rankings.push(ranking.clone());
            self.send(EventKind::FileRanked { ranking });
        }
        self.file_hits.clear();
        self.same_row_terms = 0;
    }
    fn term_hit(
        &mut self,
        path: &Path,
        file_type: &str,
        unit: Unit,
        matched: fuzzy::Match,
        term: Option<&batch::Term>,
    ) {
        self.counts.result_count += 1;
        let (preview_text, preview_truncated, range) = preview(&unit.text, matched.range);
        self.publish(SearchHit {
            source_match_ranges: vec![matched.range],
            document_order: document_order(&unit),
            modified_at: self.modified_at,
            edit_anchor: edit::anchor(&unit, matched.range, self.edit_revision.as_deref()),
            term_id: term.map(|term| term.id.clone()),
            term: term.map(|term| term.text.clone()),
            result_id: self.counts.result_count,
            file_path: display_path(path),
            file_type: file_type.to_owned(),
            source_kind: unit.source_kind.to_owned(),
            unit_key: unit.meta.unit_key,
            part_key: unit.meta.part_key,
            group_key: unit.meta.group_key,
            row: unit.meta.row,
            column: unit.meta.column,
            content_class: unit.meta.content_class,
            anchor: unit.meta.anchor,
            location: unit.location,
            preview_text,
            preview_truncated,
            match_ranges: vec![range],
            match_type: matched.kind,
            match_category: matched.category,
            score: matched.score,
            evidence: vec![],
        });
    }
    fn condition_hit(&mut self, path: &Path, file_type: &str, group: query::GroupMatch<'_>) {
        let query::GroupMatch {
            source_kind,
            location,
            unit_key,
            part_key,
            group_key,
            row,
            column,
            content_class,
            anchor,
            match_category,
            evidence: candidates,
        } = group;
        let evidence: Vec<SearchEvidence> = candidates
            .into_iter()
            .map(|candidate| {
                let (preview_text, preview_truncated, range) =
                    preview(&candidate.unit.text, candidate.matched.range);
                SearchEvidence {
                    source_match_ranges: vec![candidate.matched.range],
                    document_order: document_order(candidate.unit),
                    edit_anchor: edit::anchor(
                        candidate.unit,
                        candidate.matched.range,
                        self.edit_revision.as_deref(),
                    ),
                    term_id: candidate.term_id,
                    term: candidate.term,
                    unit_key: candidate.unit.meta.unit_key.clone(),
                    part_key: candidate.unit.meta.part_key.clone(),
                    source_kind: candidate.unit.source_kind.into(),
                    content_class: candidate.unit.meta.content_class.clone(),
                    location: candidate.unit.location.clone(),
                    row: candidate.unit.meta.row,
                    column: candidate.unit.meta.column,
                    preview_text,
                    preview_truncated,
                    match_ranges: vec![range],
                    match_type: candidate.matched.kind,
                    match_category: candidate.matched.category,
                    score: candidate.matched.score,
                }
            })
            .collect();
        let primary = &evidence[0];
        self.counts.result_count += 1;
        self.publish(SearchHit {
            source_match_ranges: primary.source_match_ranges.clone(),
            document_order: evidence
                .iter()
                .map(|item| item.document_order.clone())
                .min()
                .unwrap_or_default(),
            modified_at: self.modified_at,
            edit_anchor: None,
            term_id: None,
            term: None,
            result_id: self.counts.result_count,
            file_path: display_path(path),
            file_type: file_type.into(),
            source_kind: source_kind.into(),
            unit_key,
            part_key,
            group_key,
            row,
            column,
            content_class,
            anchor,
            location,
            preview_text: primary.preview_text.clone(),
            preview_truncated: primary.preview_truncated,
            match_ranges: primary.match_ranges.clone(),
            match_type: primary.match_type,
            match_category,
            score: primary.score,
            evidence,
        });
    }
    fn condition_hits(
        &mut self,
        path: &Path,
        file_type: &str,
        units: &[&Unit],
        spec: &query::Conditions,
        fuzzy_search: bool,
        cancel: &AtomicBool,
    ) {
        let groups = query::evaluate(units, file_type, spec, fuzzy_search, cancel);
        if spec.scope == query::Scope::File
            && !groups.is_empty()
            && matches!(file_type, "xlsx" | "xlsm")
        {
            self.same_row_terms = query::same_row_terms(units, spec, fuzzy_search, cancel);
        }
        for group in groups {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            self.condition_hit(path, file_type, group);
        }
    }
}

/// Runs an accepted request and emits exactly one terminal event. The caller validates before
/// starting a worker; callbacks can forward events to a GUI or collect them in a test.
pub fn run_search<F: FnMut(SearchEvent)>(
    request: SearchRequest,
    search_id: String,
    cancel: &AtomicBool,
    emit: F,
) -> Result<(), InputError> {
    let paths = validate_paths(&request)?;
    let batch = request
        .query_spec
        .as_ref()
        .filter(|spec| spec["mode"] == "batch")
        .map(batch::parse)
        .transpose()?;
    let conditions = request
        .query_spec
        .as_ref()
        .filter(|spec| spec["mode"] != "batch")
        .map(query::parse)
        .transpose()?;
    let excluded_keys: Vec<PathBuf> = paths.excluded.iter().map(|path| path_key(path)).collect();
    let query = fuzzy::Query::new(&request.query);
    let use_index = request.use_index;
    let fuzzy_search = request.fuzzy_search;
    let extraction_options = extract::ExtractionOptions {
        include_notes: request.include_notes,
        include_formulas: request.include_formulas,
    };
    let extraction_scope = extraction_options.scope();
    let selected: HashSet<String> = request.extensions.iter().cloned().collect();
    let mut sink = Emitter {
        id: search_id,
        sequence: 0,
        counts: Counts::default(),
        emit,
        file_hits: Vec::new(),
        rankings: Vec::new(),
        batch_counts: std::collections::HashMap::new(),
        extracted_files: 0,
        reused_files: 0,
        same_row_terms: 0,
        modified_at: None,
        edit_revision: None,
    };
    sink.send(EventKind::Started { request });
    sink.progress("discovery");
    let mut files = Vec::<(PathBuf, String)>::new();
    let mut pending = paths.roots;
    while let Some(dir) = pending.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let directory_key = path_key(&dir);
        if excluded_keys
            .iter()
            .any(|excluded| directory_key.starts_with(excluded))
        {
            continue;
        }
        match fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            match entry.file_type() {
                                Ok(kind) if kind.is_symlink() => (),
                                Ok(kind) if kind.is_dir() => pending.push(path),
                                Ok(kind) if kind.is_file() => {
                                    if let Some(ext) = supported(&path, &selected) {
                                        sink.counts.discovered_files += 1;
                                        files.push((path, ext));
                                        if sink.counts.discovered_files % 32 == 0 {
                                            sink.progress("discovery");
                                        }
                                    }
                                }
                                Ok(_) => (),
                                Err(error) => sink.issue(SearchIssue {
                                    stage: "discovery",
                                    path: Some(display_path(&path)),
                                    code: "unreadable",
                                    reason: error.to_string(),
                                }),
                            }
                        }
                        Err(error) => sink.issue(SearchIssue {
                            stage: "discovery",
                            path: Some(display_path(&dir)),
                            code: "unreadable",
                            reason: error.to_string(),
                        }),
                    }
                }
            }
            Err(error) => sink.issue(SearchIssue {
                stage: "discovery",
                path: Some(display_path(&dir)),
                code: if error.kind() == std::io::ErrorKind::PermissionDenied {
                    "accessDenied"
                } else {
                    "unreadable"
                },
                reason: error.to_string(),
            }),
        }
    }
    sink.progress("search");
    let mut search_index = if use_index {
        index::Index::open().ok()
    } else {
        None
    };
    if let Some(cache) = search_index.as_mut() {
        let _ = cache.prune_missing();
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, file_type) in files {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        sink.counts.processed_files += 1;
        sink.modified_at = fs::metadata(&path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs_f64() * 1000.0);
        sink.edit_revision = if !OFFICE_EXTENSIONS.contains(&file_type.as_str()) {
            fs::metadata(&path)
                .ok()
                .filter(|metadata| metadata.len() <= MAX_TEXT_BYTES)
                .and_then(|_| fs::read(&path).ok())
                .map(|bytes| edit::revision(&bytes))
        } else {
            None
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let filename_unit = file_name_unit(name.to_string());
        if let Some(spec) = &batch {
            for term in &spec.terms {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Some(matched) = spec.matched(term, &filename_unit.text, fuzzy_search) {
                    sink.term_hit(
                        &path,
                        &file_type,
                        filename_unit.clone(),
                        matched,
                        Some(term),
                    );
                }
            }
        } else if let Some(spec) = &conditions {
            if spec.scope != query::Scope::File {
                sink.condition_hits(
                    &path,
                    &file_type,
                    &[&filename_unit],
                    spec,
                    fuzzy_search,
                    cancel,
                );
            }
        } else if let Some(matched) = fuzzy::evaluate_selected(&name, &query, fuzzy_search) {
            sink.hit(&path, &file_type, filename_unit.clone(), matched);
        }
        let cached_stamp = index::stamp(&path);
        if let Some(cache) = search_index.as_ref() {
            if cache.current(&path, extraction_scope) {
                let cached = if conditions.is_some() || batch.is_some() {
                    cache.all_units(&path)
                } else {
                    cache.candidates(&path, &query, fuzzy_search)
                };
                if let Ok(units) = cached {
                    if cached_stamp.is_some() && cached_stamp == index::stamp(&path) {
                        sink.reused_files += 1;
                        if let Some(spec) = &batch {
                            for unit in &units {
                                for term in &spec.terms {
                                    if cancel.load(Ordering::Relaxed) {
                                        break;
                                    }
                                    if let Some(matched) =
                                        spec.matched(term, &unit.text, fuzzy_search)
                                    {
                                        sink.term_hit(
                                            &path,
                                            &file_type,
                                            unit.clone(),
                                            matched,
                                            Some(term),
                                        );
                                    }
                                }
                                if cancel.load(Ordering::Relaxed) {
                                    break;
                                }
                            }
                        } else if let Some(spec) = &conditions {
                            let mut scoped = if spec.scope == query::Scope::File {
                                vec![&filename_unit]
                            } else {
                                Vec::new()
                            };
                            scoped.extend(units.iter());
                            sink.condition_hits(
                                &path,
                                &file_type,
                                &scoped,
                                spec,
                                fuzzy_search,
                                cancel,
                            );
                        } else {
                            for unit in units {
                                if cancel.load(Ordering::Relaxed) {
                                    break;
                                }
                                if let Some(matched) =
                                    fuzzy::evaluate_selected(&unit.text, &query, fuzzy_search)
                                {
                                    sink.hit(&path, &file_type, unit, matched);
                                }
                            }
                        }
                        sink.finish_file();
                        sink.progress("search");
                        continue;
                    }
                }
            }
        }
        let expected_stamp = index::stamp(&path);
        sink.extracted_files += 1;
        let result = if !OFFICE_EXTENSIONS.contains(&file_type.as_str()) {
            fs::metadata(&path)
                .map_err(extract::ExtractError::from)
                .and_then(|m| {
                    if m.len() > MAX_TEXT_BYTES {
                        Err(extract::ExtractError::limit(
                            "テキストファイルがサイズ上限を超えました。",
                        ))
                    } else {
                        fs::read(&path).map_err(extract::ExtractError::from)
                    }
                })
                .and_then(|bytes| {
                    if bytes.len() as u64 > MAX_TEXT_BYTES {
                        return Err(extract::ExtractError::limit(
                            "テキストファイルがサイズ上限を超えました。",
                        ));
                    }
                    extract::text_units(&bytes).map(|units| extract::ExtractedDocument {
                        units,
                        sheets: vec![],
                        issues: vec![],
                    })
                })
        } else {
            extract::office_units_with_options(&path, &file_type, extraction_options)
        };
        match result {
            Ok(document) => {
                if expected_stamp.is_none() || expected_stamp != index::stamp(&path) {
                    sink.issue(SearchIssue {
                        stage: "read",
                        path: Some(display_path(&path)),
                        code: "changedDuringRead",
                        reason: "検索中にファイルが変更されました。再検索してください。".into(),
                    });
                    sink.finish_file();
                    sink.progress("search");
                    continue;
                }
                for issue in &document.issues {
                    sink.issue(issue_for(Some(&path), "read", issue));
                }
                if let Some(spec) = &batch {
                    for unit in &document.units {
                        for term in &spec.terms {
                            if cancel.load(Ordering::Relaxed) {
                                break;
                            }
                            if let Some(matched) = spec.matched(term, &unit.text, fuzzy_search) {
                                sink.term_hit(&path, &file_type, unit.clone(), matched, Some(term));
                            }
                        }
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                } else if let Some(spec) = &conditions {
                    let mut scoped = if spec.scope == query::Scope::File {
                        vec![&filename_unit]
                    } else {
                        Vec::new()
                    };
                    scoped.extend(document.units.iter());
                    if document.issues.is_empty() || spec.scope != query::Scope::File {
                        sink.condition_hits(&path, &file_type, &scoped, spec, fuzzy_search, cancel);
                    }
                } else {
                    for unit in &document.units {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        if !fuzzy_search || fuzzy::could_match(&unit.text, &query) {
                            if let Some(matched) =
                                fuzzy::evaluate_selected(&unit.text, &query, fuzzy_search)
                            {
                                sink.hit(&path, &file_type, unit.clone(), matched);
                            }
                        }
                    }
                }
                if !cancel.load(Ordering::Relaxed) && document.issues.is_empty() {
                    if let Some(cache) = search_index.as_mut() {
                        match cache.replace(
                            &path,
                            &document,
                            expected_stamp.unwrap(),
                            extraction_scope,
                        ) {
                            Ok(true) => (),
                            Ok(false) => sink.issue(SearchIssue {
                                stage: "read",
                                path: Some(display_path(&path)),
                                code: "changedDuringRead",
                                reason: "検索中にファイルが変更されました。再検索してください。"
                                    .into(),
                            }),
                            Err(_) => search_index = None,
                        }
                    }
                }
            }
            Err(error) => sink.issue(issue_for(Some(&path), "read", &error)),
        }
        sink.finish_file();
        sink.progress("search");
    }
    let reason = if cancel.load(Ordering::Relaxed) {
        "cancelled"
    } else {
        "completed"
    };
    sink.rankings.sort_by(ranking::compare);
    let files = sink
        .rankings
        .iter()
        .map(|ranking| ranking.file_path.clone())
        .collect();
    sink.send(EventKind::RankingSummary { files });
    sink.send(EventKind::ExecutionSummary {
        extracted_files: sink.extracted_files,
        reused_files: sink.reused_files,
    });
    if let Some(batch) = batch {
        let status = if reason == "cancelled" {
            "未確定（中断）"
        } else if sink.counts.issue_count > 0 {
            "未確定（一部エラーあり）"
        } else {
            "確認完了"
        };
        let terms = batch
            .terms
            .into_iter()
            .map(|term| {
                let (file_count, hit_count) =
                    sink.batch_counts.get(&term.id).copied().unwrap_or_default();
                batch::TermSummary {
                    term_id: term.id,
                    term: term.text,
                    file_count,
                    hit_count,
                    status: if hit_count == 0 && status == "確認完了" {
                        "指定範囲内で該当なし".into()
                    } else {
                        status.into()
                    },
                }
            })
            .collect();
        sink.send(EventKind::BatchSummary { terms });
    }
    sink.send(EventKind::Finished {
        reason,
        counts: sink.counts.clone(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{match_ranges, normalize_fold, validate, SearchRequest};
    use serde_json::json;

    #[test]
    fn reserved_query_spec_is_rejected_before_search() {
        let request = SearchRequest {
            root_directory: String::new(),
            additional_directories: vec![],
            excluded_directories: vec![],
            query: "顧客".into(),
            query_spec: Some(json!({"mode": "conditions"})),
            recursive: true,
            extensions: vec!["txt".into()],
            use_index: false,
            fuzzy_search: false,
            include_notes: false,
            include_formulas: false,
        };
        assert_eq!(validate(&request).unwrap_err().field, "querySpec");
    }

    #[test]
    fn match_ranges_follow_search_normalization_and_unicode_positions() {
        assert_eq!(
            match_ranges("予告 BEACON beacon", &normalize_fold("beacon")),
            vec![[3, 9], [10, 16]]
        );
        assert_eq!(
            match_ranges("😀 cafe\u{301}", &normalize_fold("CAFÉ")),
            vec![[2, 7]]
        );
        assert_eq!(
            match_ranges("Straße", &normalize_fold("STRASSE")),
            vec![[0, 6]]
        );
        assert_eq!(match_ranges("ß", &normalize_fold("s")), vec![[0, 1]]);
    }
}
