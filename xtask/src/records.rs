use docs_search_test_support::digest;
use serde::Serialize;
use serde_json::{json, Value};
use std::{fs, path::Path};

pub const BACKEND: &[&str] = &[
    "core-api-properties",
    "common-cli",
    "context-cli",
    "conditions-cli",
    "office-cli",
    "issues-cli",
    "index-cli",
    "worker-policy",
];
pub const FRONTEND: &[&str] = &["ui-stage", "ui-node", "ui-preflight", "playwright"];
pub const INFRASTRUCTURE: &[&str] = &["fixture-corpus", "generators", "record-contracts"];

#[derive(Default, Serialize, Debug, PartialEq)]
struct Counts {
    suites: u64,
    passed: u64,
    failed: u64,
    ignored: u64,
    measured: u64,
    filtered_out: u64,
}

// libtest's result lines are the evidence. PASS messages inside CLI tests are
// assertions, not independent test cases, and must not be added to these counts.
fn rust_counts(log: &str) -> Option<Counts> {
    let mut counts = Counts::default();
    for line in log
        .lines()
        .filter_map(|line| line.trim().strip_prefix("test result: "))
    {
        let tail = line
            .strip_prefix("ok. ")
            .or_else(|| line.strip_prefix("FAILED. "))?;
        let mut fields = tail.split(';');
        for (label, total) in [
            ("passed", &mut counts.passed),
            ("failed", &mut counts.failed),
            ("ignored", &mut counts.ignored),
            ("measured", &mut counts.measured),
            ("filtered out", &mut counts.filtered_out),
        ] {
            let field = fields.next()?.trim().strip_suffix(label)?.trim();
            *total += field.parse::<u64>().ok()?;
        }
        counts.suites += 1;
    }
    (counts.suites > 0).then_some(counts)
}

fn node_counts(log: &str) -> Option<Value> {
    let keys = ["tests", "pass", "fail", "cancelled", "skipped", "todo"];
    let mut counts = serde_json::Map::new();
    for line in log.lines().filter_map(|line| line.strip_prefix("# ")) {
        if let Some((key, number)) = line.split_once(' ') {
            if keys.contains(&key) {
                counts.insert(key.to_owned(), json!(number.parse::<u64>().ok()?));
            }
        }
    }
    keys.iter()
        .all(|key| counts.contains_key(*key))
        .then_some(Value::Object(counts))
}

fn group(phases: &[Value], required: &[&str]) -> Value {
    let found: Vec<_> = phases
        .iter()
        .filter(|phase| required.contains(&phase["name"].as_str().unwrap_or("")))
        .collect();
    let status = if found.is_empty() {
        "not-run"
    } else if found.iter().any(|phase| phase["status"] == "failed") {
        "failed"
    } else if required
        .iter()
        .any(|name| !found.iter().any(|phase| phase["name"] == *name))
        || found.iter().any(|phase| phase["status"] != "passed")
    {
        "incomplete"
    } else {
        "passed"
    };
    json!({"status":status,"phases":found.iter().map(|phase| &phase["name"]).collect::<Vec<_>>()})
}

fn playwright_stats(results: &Value) -> Option<Value> {
    let stats = &results["stats"];
    if ["expected", "skipped", "unexpected", "flaky"]
        .iter()
        .any(|key| stats[key].as_u64().is_none())
    {
        return None;
    }
    let errors = results["errors"].as_array()?.len();
    Some(json!({"stats":stats,"errors":errors}))
}

pub fn staging_matches(stages: &[Value]) -> bool {
    !stages.is_empty()
        && stages.iter().all(|stage| {
            stage["files"].as_array().is_some_and(|files| {
                files.len() == crate::ASSETS.len()
                    && crate::ASSETS.iter().all(|name| {
                        let first = stages[0]["files"]
                            .as_array()
                            .and_then(|files| files.iter().find(|file| file["name"] == *name));
                        files.iter().any(|file| {
                            file["name"] == *name
                                && file["sha256"].is_string()
                                && file["sha256"] == file["sourceSha256"]
                                && first.is_some_and(|first| file["sha256"] == first["sha256"])
                        })
                    })
            })
        })
}

pub fn complete(root: &Path, dir: &Path, report: &mut Value) {
    let phases = report["phases"].as_array().cloned().unwrap_or_default();
    let mut backend = group(&phases, BACKEND);
    let mut frontend = group(&phases, FRONTEND);
    let mut infrastructure = group(&phases, INFRASTRUCTURE);
    for (summary, names) in [
        (&mut backend, BACKEND),
        (&mut infrastructure, INFRASTRUCTURE),
    ] {
        let mut total = Counts::default();
        let mut suites = vec![];
        for phase in phases
            .iter()
            .filter(|phase| names.contains(&phase["name"].as_str().unwrap_or("")))
        {
            if phase["command"][0] != "cargo" {
                continue;
            }
            let parsed = phase["stdout"]
                .as_str()
                .and_then(|path| fs::read_to_string(root.join(path)).ok())
                .and_then(|log| rust_counts(&log));
            match parsed {
                Some(counts) => {
                    total.suites += counts.suites;
                    total.passed += counts.passed;
                    total.failed += counts.failed;
                    total.ignored += counts.ignored;
                    total.measured += counts.measured;
                    total.filtered_out += counts.filtered_out;
                    suites.push(json!({"phase":phase["name"],"counts":counts}));
                }
                None if phase["status"] == "passed" => {
                    if summary["status"] != "failed" {
                        summary["status"] = json!("incomplete");
                    }
                    summary["evidenceError"] =
                        json!("successful Cargo phase has no readable libtest result");
                }
                None => {}
            }
        }
        summary["rustTestFunctions"] = json!(total);
        summary["rustSuites"] = json!(suites);
        if (total.failed > 0 || total.ignored > 0 || total.filtered_out > 0)
            && summary["status"] != "failed"
        {
            summary["status"] = json!("incomplete");
        }
    }
    if let Some(phase) = phases.iter().find(|phase| {
        phase["name"] == "worker-policy"
            && (phase["status"] == "passed" || phase["status"] == "failed")
    }) {
        let counts = phase["stdout"]
            .as_str()
            .and_then(|path| fs::read_to_string(root.join(path)).ok())
            .and_then(|log| node_counts(&log));
        match counts {
            Some(counts) => {
                if counts["tests"] == 0
                    || counts["tests"] != counts["pass"]
                    || ["fail", "cancelled", "skipped", "todo"]
                        .iter()
                        .any(|key| counts[key] != 0)
                {
                    if backend["status"] != "failed" {
                        backend["status"] = json!("incomplete");
                    }
                }
                backend["nodeTestCases"] = counts;
            }
            None => {
                if backend["status"] != "failed" {
                    backend["status"] = json!("incomplete");
                }
                backend["evidenceError"] = json!("Node TAP summary is missing or invalid");
            }
        }
    }
    if phases.iter().any(|phase| {
        phase["name"] == "playwright"
            && (phase["status"] == "passed" || phase["status"] == "failed")
    }) {
        let stats = fs::read(dir.join("ui/results.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|results| playwright_stats(&results));
        match stats {
            Some(stats) => {
                if stats["errors"] != 0
                    || stats["stats"]["expected"] == 0
                    || ["skipped", "unexpected", "flaky"]
                        .iter()
                        .any(|key| stats["stats"][key] != 0)
                {
                    if frontend["status"] != "failed" {
                        frontend["status"] = json!("incomplete");
                    }
                }
                frontend["playwright"] = stats;
            }
            None => {
                if frontend["status"] != "failed" {
                    frontend["status"] = json!("incomplete");
                }
                frontend["evidenceError"] =
                    json!("Playwright JSON/stats/errors are missing or invalid");
            }
        }
    }
    report["summary"] = json!({
        "backend":backend,"frontend":frontend,
        "applicationIntegration":{"status":"not-implemented","phases":[]},
        "infrastructure":infrastructure,
    });
    report["durationMs"] = json!(report["finishedAtUnixMs"]
        .as_u64()
        .unwrap_or(0)
        .saturating_sub(report["metadata"]["startedAtUnixMs"].as_u64().unwrap_or(0)));
    let stages: Vec<_> = report["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|artifact| artifact["kind"] == "frontend-staging")
        .cloned()
        .collect();
    let hashes_match = staging_matches(&stages)
        && stages.iter().all(|stage| {
            stage["files"].as_array().unwrap().iter().all(|file| {
                let name = file["name"].as_str().unwrap();
                let directory = stage["path"].as_str().unwrap_or("");
                let expected = file["sha256"].as_str().unwrap();
                [
                    root.join(directory).join(name),
                    root.join("frontend").join(name),
                ]
                .iter()
                .all(|path| digest(path).is_ok_and(|actual| actual == expected))
            })
        });
    report["frontendStaging"] = json!({"status":if stages.is_empty() {"not-run"} else if hashes_match {"passed"} else {"incomplete"},"directories":stages.len()});
    let expected_stages = match report["metadata"]["command"].as_str() {
        Some("ci") => 2,
        Some("test" | "ui" | "build") => 1,
        _ => 0,
    };
    if expected_stages > 0 && (stages.len() != expected_stages || !hashes_match) {
        report["success"] = json!(false);
    }
    let required: &[&str] = match report["metadata"]["command"].as_str() {
        Some("test" | "ci") => &["backend", "frontend", "infrastructure"],
        Some("ui") => &["frontend"],
        _ => &[],
    };
    if required
        .iter()
        .any(|key| report["summary"][key]["status"] != "passed")
    {
        report["success"] = json!(false);
    }
}

pub fn text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| {
            if value.is_null() {
                "—".into()
            } else {
                value.to_string()
            }
        })
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
}

pub fn summary_table(report: &Value) -> String {
    let summary = &report["summary"];
    let rust = &summary["backend"]["rustTestFunctions"];
    let ui = &summary["frontend"]["playwright"]["stats"];
    let node = &summary["backend"]["nodeTestCases"];
    let mut md = "| 区分 | 結果 | 実測件数 |\n| --- | --- | --- |\n".to_owned();
    md += &format!("| バックエンド試験 | {} | Rustテスト関数: 成功 {} / 失敗 {} / ignored {} / filtered {}。Node: 成功 {} / 失敗 {} / skipped {} |\n",
        text(&summary["backend"]["status"]), text(&rust["passed"]), text(&rust["failed"]), text(&rust["ignored"]), text(&rust["filtered_out"]), text(&node["pass"]), text(&node["fail"]), text(&node["skipped"]));
    md += &format!("| フロントエンド試験 | {} | Playwright: 成功 {} / skipped {} / unexpected {} / flaky {} |\n",
        text(&summary["frontend"]["status"]), text(&ui["expected"]), text(&ui["skipped"]), text(&ui["unexpected"]), text(&ui["flaky"]));
    md += &format!(
        "| アプリケーション結合試験 | {} | — |\n",
        text(&summary["applicationIntegration"]["status"])
    );
    md += &format!(
        "\n共通の検証基盤（生成器・記録処理）: {}。件数は試験3区分に加算しません。\n",
        text(&summary["infrastructure"]["status"])
    );
    for key in ["backend", "frontend", "infrastructure"] {
        if let Some(error) = summary[key]["evidenceError"].as_str() {
            md += &format!("\n{key}: {}\n", text(&json!(error)));
        }
    }
    md
}

pub fn overview(report: &Value) -> String {
    let metadata = &report["metadata"];
    let mut md = format!("# Cargo {} report\n\nResult: **{}**\n\n実行ID: `{}`。対象コミット: `{}`。設定版: `{}`。実行時間: {} ms。\n\n引数: `{}`。proptest入力: `{}`。\n\n",
        text(&metadata["command"]), if report["success"] == true { "passed" } else { "failed" },
        text(&metadata["runId"]), text(&metadata["commit"]["stdout"]), text(&metadata["productVersion"]), text(&report["durationMs"]),
        text(&metadata["invocation"]), text(&metadata["proptest"]));
    md += &summary_table(report);
    md += "\n| 環境 | 実測版 |\n| --- | --- |\n";
    for key in ["rustc", "cargo", "node", "npm", "playwright"] {
        let result = &metadata[key];
        let version = if result["exitCode"] == 0 {
            &result["stdout"]
        } else {
            result.get("unavailable").unwrap_or(&result["stderr"])
        };
        md += &format!("| {key} | {} |\n", text(version));
    }
    md += &format!(
        "\n開始/終了（Unix ms）: {} / {}。作業ツリー（開始/終了）: `{}` / `{}`。\n",
        text(&metadata["startedAtUnixMs"]),
        text(&report["finishedAtUnixMs"]),
        text(&metadata["workingTree"]["stdout"]),
        text(&report["sourceAtFinish"]["workingTree"]["stdout"])
    );
    md += &format!(
        "\n資材の最終照合: {}（{}配置）。ソースと全stagingのハッシュを比較。\n",
        text(&report["frontendStaging"]["status"]),
        text(&report["frontendStaging"]["directories"])
    );
    if let Some(artifacts) = report["artifacts"].as_array() {
        for artifact in artifacts {
            match artifact["kind"].as_str() {
                Some("windows-exe") => {
                    md += &format!(
                        "\nEXE: `{}` / {} bytes / SHA256 `{}`。\n",
                        text(&artifact["path"]),
                        text(&artifact["bytes"]),
                        text(&artifact["sha256"])
                    )
                }
                Some("frontend-staging") => {
                    md += &format!(
                        "\n資材: `{}` の{}ファイルのSHA256を記録。\n",
                        text(&artifact["path"]),
                        artifact["files"].as_array().map(Vec::len).unwrap_or(0)
                    )
                }
                _ => {}
            }
        }
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_counts_distinguish_suites_and_inner_assertions() {
        let counts = rust_counts("PASS inner-case\ntest result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1s\n\ntest result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1s").unwrap();
        assert_eq!(counts.passed, 64);
        assert_eq!(counts.suites, 2);
        assert_eq!(counts.failed, 0);
    }
    #[test]
    fn rust_counts_preserve_failures_ignored_and_filtered() {
        let counts = rust_counts("test result: FAILED. 2 passed; 1 failed; 3 ignored; 0 measured; 4 filtered out; finished in 1s").unwrap();
        assert_eq!(
            (
                counts.passed,
                counts.failed,
                counts.ignored,
                counts.filtered_out
            ),
            (2, 1, 3, 4)
        );
        assert!(rust_counts("PASS something").is_none());
        assert!(rust_counts("test result: ok. bad passed; 0 failed;").is_none());
    }
    #[test]
    fn node_tap_summary_keeps_skipped_and_cancelled_counts() {
        let counts = node_counts("TAP version 13\n# tests 8\n# pass 6\n# fail 0\n# cancelled 1\n# skipped 1\n# todo 0\n# duration_ms 10").unwrap();
        assert_eq!(counts["tests"], 8);
        assert_eq!(counts["cancelled"], 1);
        assert_eq!(counts["skipped"], 1);
        assert!(node_counts("# tests 8\n# pass 8").is_none());
    }
    #[test]
    fn missing_or_skipped_phases_cannot_pass_a_group() {
        assert_eq!(group(&[], &["a"])["status"], "not-run");
        assert_eq!(
            group(&[json!({"name":"a","status":"passed"})], &["a", "b"])["status"],
            "incomplete"
        );
        assert_eq!(
            group(&[json!({"name":"a","status":"skipped"})], &["a"])["status"],
            "incomplete"
        );
        assert_eq!(
            group(&[json!({"name":"a","status":"failed"})], &["a"])["status"],
            "failed"
        );
    }
    #[test]
    fn playwright_counts_require_stats_and_global_errors() {
        let results =
            json!({"stats":{"expected":54,"skipped":0,"unexpected":0,"flaky":0},"errors":[]});
        assert_eq!(playwright_stats(&results).unwrap()["stats"]["expected"], 54);
        assert!(playwright_stats(&json!({"stats":{"expected":54}})).is_none());
        let mut broken = results;
        broken["stats"]["skipped"] = json!(-1);
        assert!(playwright_stats(&broken).is_none());
    }
    #[test]
    fn successful_processes_without_result_files_fail_the_report() {
        let phases: Vec<_> = BACKEND.iter().chain(FRONTEND).chain(INFRASTRUCTURE)
            .map(|name| json!({"name":name,"status":"passed","command":["cargo"],"stdout":"missing.log"})).collect();
        let mut report = json!({"metadata":{"command":"test","startedAtUnixMs":100},"finishedAtUnixMs":200,"success":true,"phases":phases});
        let missing = Path::new("__docs_search_missing_evidence__");
        complete(missing, missing, &mut report);
        assert_eq!(report["success"], false);
        assert_eq!(report["summary"]["frontend"]["status"], "incomplete");
        assert_eq!(report["summary"]["backend"]["status"], "incomplete");
        assert_eq!(
            report["summary"]["applicationIntegration"]["status"],
            "not-implemented"
        );
        assert_eq!(report["durationMs"], 100);
    }
    #[test]
    fn staging_requires_all_assets_and_identical_ui_distribution_hashes() {
        let files: Vec<_> = crate::ASSETS
            .iter()
            .map(|name| json!({"name":name,"sha256":"abc","sourceSha256":"abc"}))
            .collect();
        let valid = json!({"files":files});
        assert!(staging_matches(&[valid.clone(), valid.clone()]));
        let mut changed = valid.clone();
        changed["files"][0]["sha256"] = json!("other");
        changed["files"][0]["sourceSha256"] = json!("other");
        assert!(!staging_matches(&[valid.clone(), changed]));
        let mut missing = valid.clone();
        missing["files"].as_array_mut().unwrap().pop();
        assert!(!staging_matches(&[valid, missing]));
        assert!(!staging_matches(&[]));
    }
}
