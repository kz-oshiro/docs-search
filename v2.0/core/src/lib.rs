mod extract;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

pub use extract::Unit;

const MAX_TEXT_BYTES: u64 = 32 * 1024 * 1024;
pub const SUPPORTED_EXTENSIONS: [&str; 10] = [
    "xlsx", "xlsm", "pptx", "docx", "txt", "jsp", "xhtml", "html", "js", "java",
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
    pub query: String,
    pub recursive: bool,
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
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
    pub result_id: usize,
    pub file_path: String,
    pub file_type: String,
    pub source_kind: String,
    pub location: Value,
    pub preview_text: String,
    pub preview_truncated: bool,
    pub match_ranges: Vec<[usize; 2]>,
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
    Finished {
        reason: &'static str,
        counts: Counts,
    },
}

pub fn validate(request: &SearchRequest) -> Result<PathBuf, InputError> {
    if request.query.trim().is_empty() {
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
    let path = Path::new(&request.root_directory);
    if !path.is_dir() {
        return Err(InputError {
            field: "rootDirectory",
            message: "存在するフォルダーを指定してください。",
        });
    }
    fs::canonicalize(path).map_err(|_| InputError {
        field: "rootDirectory",
        message: "フォルダーを読み取れません。",
    })
}

pub fn normalize_fold(text: &str) -> String {
    text.nfc().case_fold().collect()
}

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

fn preview(value: &str, needle: &str) -> (String, bool) {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 240 {
        return (value.to_owned(), false);
    }
    let width = 180.max(needle.chars().count() + 80);
    let mut start = 0;
    while start < chars.len() {
        let end = (start + width).min(chars.len());
        let segment: String = chars[start..end].iter().collect();
        if normalize_fold(&segment).contains(needle) {
            return (
                format!(
                    "{}{}{}",
                    if start > 0 { "… " } else { "" },
                    segment,
                    if end < chars.len() { " …" } else { "" }
                ),
                true,
            );
        }
        start += (width / 2).max(1);
    }
    (value.to_owned(), false)
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

struct Emitter<F: FnMut(SearchEvent)> {
    id: String,
    sequence: usize,
    counts: Counts,
    emit: F,
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
    fn hit(&mut self, path: &Path, file_type: &str, unit: Unit, needle: &str) {
        self.counts.result_count += 1;
        let (preview_text, preview_truncated) = preview(&unit.text, needle);
        let match_ranges = match_ranges(&preview_text, needle);
        self.send(EventKind::Result {
            hit: SearchHit {
                result_id: self.counts.result_count,
                file_path: display_path(path),
                file_type: file_type.to_owned(),
                source_kind: unit.source_kind.to_owned(),
                location: unit.location,
                preview_text,
                preview_truncated,
                match_ranges,
            },
        });
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
    let root = validate(&request)?;
    let needle = normalize_fold(request.query.trim());
    let selected: HashSet<String> = request.extensions.iter().cloned().collect();
    let mut sink = Emitter {
        id: search_id,
        sequence: 0,
        counts: Counts::default(),
        emit,
    };
    sink.send(EventKind::Started { request });
    sink.progress("discovery");
    let mut files = Vec::<(PathBuf, String)>::new();
    let mut pending = vec![root];
    while let Some(dir) = pending.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
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
    files.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, file_type) in files {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        sink.counts.processed_files += 1;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if normalize_fold(&name).contains(&needle) {
            sink.hit(
                &path,
                &file_type,
                Unit {
                    source_kind: "fileName",
                    location: json!({}),
                    text: name.into_owned(),
                },
                &needle,
            );
        }
        let result = if ["txt", "jsp", "xhtml", "html", "js", "java"].contains(&file_type.as_str())
        {
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
                    extract::text_units(&bytes)
                })
        } else {
            extract::office_units(&path, &file_type)
        };
        match result {
            Ok(units) => {
                for unit in units {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    if normalize_fold(&unit.text).contains(&needle) {
                        sink.hit(&path, &file_type, unit, &needle);
                    }
                }
            }
            Err(error) => sink.issue(issue_for(Some(&path), "read", &error)),
        }
        sink.progress("search");
    }
    let reason = if cancel.load(Ordering::Relaxed) {
        "cancelled"
    } else {
        "completed"
    };
    sink.send(EventKind::Finished {
        reason,
        counts: sink.counts.clone(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{match_ranges, normalize_fold};

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
