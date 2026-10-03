#![allow(dead_code)]
pub use docs_search_test_support::TempDir;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub struct Cli {
    pub base: TempDir,
    pub root: PathBuf,
    pub cache: PathBuf,
}
pub struct Run {
    pub code: i32,
    pub stderr: String,
    pub events: Vec<Value>,
    pub hits: Vec<Value>,
    pub issues: Vec<Value>,
}
impl Cli {
    pub fn new(label: &str) -> Self {
        let base = TempDir::new(label);
        let root = base.path().join("fixture");
        let cache = base.path().join("local-app-data");
        Self { base, root, cache }
    }
    pub fn run(
        &self,
        folder: &Path,
        query: &str,
        extensions: &str,
        spec: Option<&Value>,
        flags: &[&str],
    ) -> Run {
        self.run_args(
            folder,
            query,
            &["--extensions".into(), extensions.into()],
            spec,
            flags,
        )
    }
    pub fn run_args(
        &self,
        folder: &Path,
        query: &str,
        extra: &[String],
        spec: Option<&Value>,
        flags: &[&str],
    ) -> Run {
        let mut command = Command::new(env!("CARGO_BIN_EXE_docs-search-cli"));
        command
            .arg(folder)
            .arg(query)
            .args(extra)
            .args(flags)
            .env("LOCALAPPDATA", &self.cache);
        if let Some(spec) = spec {
            command.args(["--query-spec-json", &spec.to_string()]);
        }
        let output = command.output().expect("CLI child");
        let events: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).expect("JSON line"))
            .collect();
        let hits = events
            .iter()
            .filter(|v| v["type"] == "result")
            .map(|v| v["hit"].clone())
            .collect();
        let issues = events
            .iter()
            .filter(|v| v["type"] == "issue")
            .map(|v| v["issue"].clone())
            .collect();
        Run {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8(output.stderr).unwrap(),
            events,
            hits,
            issues,
        }
    }
    pub fn db(&self) -> PathBuf {
        self.cache.join("docs-search/search-index.sqlite3")
    }
}
impl Run {
    pub fn accepted(&self, reason: &str) {
        assert_eq!(self.code, 0, "{}", self.stderr);
        assert_eq!(self.events.first().unwrap()["type"], "started");
        assert_eq!(self.events.last().unwrap()["type"], "finished");
        assert_eq!(
            self.events
                .iter()
                .filter(|e| e["type"] == "started")
                .count(),
            1
        );
        assert_eq!(
            self.events
                .iter()
                .filter(|e| e["type"] == "finished")
                .count(),
            1
        );
        for (i, event) in self.events.iter().enumerate() {
            assert_eq!(event["sequence"], json!(i + 1));
            assert_eq!(event["searchId"], "cli");
        }
        let last = self.events.last().unwrap();
        assert_eq!(last["reason"], reason);
        assert_eq!(last["counts"]["resultCount"], json!(self.hits.len()));
        assert_eq!(last["counts"]["issueCount"], json!(self.issues.len()));
        assert!(
            last["counts"]["processedFiles"].as_u64().unwrap()
                <= last["counts"]["discoveredFiles"].as_u64().unwrap()
        );
        for ranking in self.rankings() {
            let mut actual: Vec<_> = ranking["resultIds"]
                .as_array()
                .unwrap()
                .iter()
                .map(Value::to_string)
                .collect();
            actual.sort();
            let mut wanted: Vec<_> = self
                .hits
                .iter()
                .filter(|h| h["filePath"] == ranking["filePath"])
                .map(|h| h["resultId"].to_string())
                .collect();
            wanted.sort();
            assert_eq!(actual, wanted);
        }
    }
    pub fn rejected(&self, field: &str) {
        assert_eq!(self.code, 2, "{}", self.stderr);
        assert!(self.events.is_empty());
        assert!(self.stderr.contains(field), "{}", self.stderr);
    }
    pub fn summary(&self, kind: &str) -> &Value {
        let matches: Vec<_> = self.events.iter().filter(|v| v["type"] == kind).collect();
        assert_eq!(matches.len(), 1, "{kind}");
        matches[0]
    }
    pub fn rankings(&self) -> Vec<&Value> {
        self.events
            .iter()
            .filter(|e| e["type"] == "fileRanked")
            .map(|e| &e["ranking"])
            .collect()
    }
    pub fn stats(&self) -> (u64, u64) {
        let v = self.summary("executionSummary");
        (
            v["extractedFiles"].as_u64().unwrap(),
            v["reusedFiles"].as_u64().unwrap(),
        )
    }
}
pub fn filename(value: &Value) -> String {
    Path::new(value.as_str().unwrap())
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}
pub fn canonical(value: &Value) -> String {
    fn sort(v: &Value) -> Value {
        match v {
            Value::Object(o) => {
                let mut items: Vec<_> = o.iter().collect();
                items.sort_by_key(|(k, _)| *k);
                Value::Object(
                    items
                        .into_iter()
                        .map(|(k, v)| (k.clone(), sort(v)))
                        .collect(),
                )
            }
            Value::Array(a) => json!(a.iter().map(sort).collect::<Vec<_>>()),
            _ => v.clone(),
        }
    }
    sort(value).to_string()
}
pub fn signature(hits: &[Value]) -> Vec<String> {
    let mut values: Vec<_> = hits
        .iter()
        .map(|h| {
            canonical(&json!([
                filename(&h["filePath"]),
                h["sourceKind"],
                h["location"],
                h["previewText"],
                h["matchType"],
                h.get("termId").unwrap_or(&json!(""))
            ]))
        })
        .collect();
    values.sort();
    values
}
pub fn condition(scope: &str, all: &[&str], any: &[&str], not: &[&str]) -> Value {
    json!({"mode":"conditions","scope":scope,"all":all,"any":any,"not":not})
}
