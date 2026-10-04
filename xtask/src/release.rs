use crate::records;
use docs_search_test_support::{digest, Result};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn checked(root: &Path, program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "{program} {args:?} failed (exit {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn require_ci(report: &Value, tag: &str) -> Result<()> {
    if report["metadata"]["schemaVersion"] != 2
        || report["metadata"]["command"] != "ci"
        || report["success"] != true
    {
        return Err("release records require a successful schema-2 cargo xtask ci run".into());
    }
    let version = report["metadata"]["productVersion"]
        .as_str()
        .ok_or("missing product version")?;
    if tag != format!("v{version}")
        || report["metadata"]["tauriVersion"] != version
        || !version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
    {
        return Err("tag and Tauri configuration must match the tested desktop version".into());
    }
    let phases = report["phases"].as_array().ok_or("missing phases")?;
    let required = records::BACKEND
        .iter()
        .chain(records::FRONTEND)
        .chain(records::INFRASTRUCTURE)
        .copied()
        .chain(["desktop-stage", "desktop-build", "desktop-artifact"]);
    if required.into_iter().any(|name| {
        !phases
            .iter()
            .any(|p| p["name"] == name && p["status"] == "passed" && p["exit_code"] == 0)
    }) || phases.iter().any(|p| p["status"] != "passed")
    {
        return Err("required CI phases are missing, skipped or failed".into());
    }
    for key in ["backend", "frontend", "infrastructure"] {
        if report["summary"][key]["status"] != "passed" {
            return Err("test evidence is incomplete".into());
        }
    }
    if report["frontendStaging"]["status"] != "passed"
        || report["frontendStaging"]["directories"] != 2
    {
        return Err("UI/distribution staging did not match the tested source".into());
    }
    for key in ["backend", "infrastructure"] {
        let counts = &report["summary"][key]["rustTestFunctions"];
        if counts["passed"].as_u64().unwrap_or(0) == 0
            || ["failed", "ignored", "filtered_out"]
                .iter()
                .any(|key| counts[key] != 0)
        {
            return Err(
                "Rust counts must contain passing tests and no failed/ignored/filtered tests"
                    .into(),
            );
        }
    }
    let ui = &report["summary"]["frontend"]["playwright"];
    let node = &report["summary"]["backend"]["nodeTestCases"];
    if node["tests"].as_u64().unwrap_or(0) == 0
        || node["tests"] != node["pass"]
        || ["fail", "cancelled", "skipped", "todo"]
            .iter()
            .any(|key| node[key] != 0)
    {
        return Err("complete Node TAP evidence is required".into());
    }
    if ui["stats"]["expected"].as_u64().unwrap_or(0) == 0
        || ui["errors"] != 0
        || ["skipped", "unexpected", "flaky"]
            .iter()
            .any(|key| ui["stats"][key] != 0)
    {
        return Err("complete Playwright evidence is required".into());
    }
    let start = &report["metadata"];
    let end = &report["sourceAtFinish"];
    for source in [start, end] {
        if source["commit"]["exitCode"] != 0
            || source["workingTree"]["exitCode"] != 0
            || source["workingTree"]["stdout"] != ""
        {
            return Err(
                "tested source must be committed and clean at both start and finish".into(),
            );
        }
    }
    if start["commit"]["stdout"] != end["commit"]["stdout"] {
        return Err("source commit changed during CI".into());
    }
    Ok(())
}

fn documentation_only(paths: &str) -> bool {
    paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .all(|path| {
            ["README.md", "AGENTS.md"].contains(&path)
                || (path.starts_with("docs/") && (path.ends_with(".md") || path.ends_with(".html")))
        })
}

fn source_changes(root: &Path, tested: &str, target: &str) -> Result<Vec<String>> {
    checked(
        root,
        "git",
        &["merge-base", "--is-ancestor", tested, target],
    )?;
    let paths = checked(
        root,
        "git",
        &[
            "diff",
            "--name-only",
            "--no-renames",
            "-z",
            tested,
            target,
            "--",
        ],
    )?;
    if !documentation_only(&paths) {
        return Err(
            "product/test/build sources differ from the tested commit; run CI for that source"
                .into(),
        );
    }
    Ok(paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

fn run_file(root: &Path, dir: &Path, relative: &str) -> Result<PathBuf> {
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(dir) {
        return Err("evidence file does not belong to the selected run".into());
    }
    Ok(path)
}

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn publication_issues(snapshot: &Value, target: &str, artifact: &Value) -> Vec<String> {
    let mut issues = vec![];
    for (valid, reason) in [
        (
            snapshot["remoteTag"] == target,
            "remote tag differs from the release target",
        ),
        (
            snapshot["tagName"] == snapshot["requestedTag"],
            "GitHub Release tag differs",
        ),
        (
            snapshot["draft"] == false && snapshot["prerelease"] == false,
            "Release is draft/prerelease or unavailable",
        ),
        (
            snapshot["asset"]["state"] == "uploaded",
            "EXE asset is not uploaded",
        ),
        (
            snapshot["asset"]["size"] == artifact["bytes"],
            "uploaded EXE size differs",
        ),
        (
            snapshot["asset"]["digest"]
                .as_str()
                .unwrap_or("")
                .eq_ignore_ascii_case(&format!(
                    "sha256:{}",
                    artifact["sha256"].as_str().unwrap_or("")
                )),
            "uploaded EXE SHA256 differs or is unavailable",
        ),
    ] {
        if !valid {
            issues.push(reason.to_owned());
        }
    }
    issues
}

// Read-only observation. Publishing, pushing, tagging and uploading remain
// separate operations performed only within the user's release request.
fn observe(
    root: &Path,
    repo: &str,
    tag: &str,
    tested: &str,
    target: &str,
    artifact: &Value,
) -> Result<Value> {
    let endpoint = format!("repos/{repo}/releases/tags/{tag}");
    let release: Value = serde_json::from_str(&checked(root, "gh", &["api", &endpoint])?)?;
    let latest: Value = serde_json::from_str(&checked(
        root,
        "gh",
        &["api", &format!("repos/{repo}/releases/latest")],
    )?)?;
    let latest_tag = latest["tag_name"]
        .as_str()
        .ok_or("Latest tag is unavailable")?;
    let reference = format!("refs/tags/{tag}");
    let peeled = format!("{reference}^{{}}");
    let remote = checked(
        root,
        "git",
        &[
            "ls-remote",
            &format!("https://github.com/{repo}.git"),
            "refs/heads/main",
            &reference,
            &peeled,
        ],
    )?;
    let sha = |name: &str| {
        remote.lines().find_map(|line| {
            let (sha, reference) = line.split_once('\t')?;
            (reference == name).then_some(sha.to_owned())
        })
    };
    let main = sha("refs/heads/main").unwrap_or_default();
    let asset = release["assets"].as_array().and_then(|assets| {
        assets
            .iter()
            .find(|asset| asset["name"] == "docs-search-desktop.exe")
    });
    let mut snapshot = json!({"checkedAtUnixMs":crate::unix_ms(),"requestedTag":tag,"url":release["html_url"],"tagName":release["tag_name"],
        "draft":release["draft"],"prerelease":release["prerelease"],"publishedAt":release["published_at"],
        "latestTag":latest_tag,"isLatest":latest_tag == tag,
        "remoteTag":sha(&peeled).or_else(|| sha(&reference)),"remoteMain":main,
        "asset":asset.map(|asset| json!({"id":asset["id"],"name":asset["name"],"state":asset["state"],"size":asset["size"],"digest":asset["digest"],"url":asset["browser_download_url"]}))});
    let mut issues = publication_issues(&snapshot, target, artifact);
    if let Err(error) = source_changes(root, tested, &main) {
        issues.push(format!("remote main: {error}"));
    }
    if let Err(error) = checked(root, "git", &["merge-base", "--is-ancestor", target, &main]) {
        issues.push(format!(
            "release target is not an ancestor of remote main: {error}"
        ));
    }
    snapshot["verified"] = json!(issues.is_empty());
    snapshot["issues"] = json!(issues);
    Ok(snapshot)
}

pub fn execute(args: &[String]) -> Result<bool> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let (mut run, mut tag, mut notes) = (None, None, None);
    let mut repo = "kz-oshiro/docs-search";
    let mut published = false;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--run" => {
                run = Some(PathBuf::from(
                    options.next().ok_or("--run requires a directory")?,
                ))
            }
            "--tag" => tag = Some(options.next().ok_or("--tag requires a tag")?.as_str()),
            "--notes" => {
                notes = Some(PathBuf::from(
                    options.next().ok_or("--notes requires a file")?,
                ))
            }
            "--repo" => repo = options.next().ok_or("--repo requires OWNER/REPO")?,
            "--published" => published = true,
            _ => return Err(format!("unknown release-record option: {option}").into()),
        }
    }
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || [".", ".."].contains(part)
                || !part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
        })
    {
        return Err("--repo must be OWNER/REPO".into());
    }
    let tag = tag.ok_or("--tag is required")?;
    let dir = root.join(run.ok_or("--run is required")?).canonicalize()?;
    if dir.parent() != Some(root.join("outputs/runs").canonicalize()?.as_path()) {
        return Err("--run must select one outputs/runs directory".into());
    }
    let report_path = dir.join("report.json");
    let report = read_json(&report_path)?;
    require_ci(&report, tag)?;
    if report["metadata"]["runId"].as_str() != dir.file_name().and_then(|name| name.to_str()) {
        return Err("report run ID differs from its directory".into());
    }
    if !checked(root, "git", &["status", "--porcelain"])?.is_empty() {
        return Err("commit source/document changes before preparing release records".into());
    }
    let head = checked(root, "git", &["rev-parse", "HEAD"])?;
    let reference = format!("refs/tags/{tag}^{{commit}}");
    let target = checked(root, "git", &["rev-parse", "--verify", &reference]).unwrap_or(head);
    let tested = report["metadata"]["commit"]["stdout"]
        .as_str()
        .ok_or("missing tested commit")?;
    let changes = source_changes(root, tested, &target)?;
    let artifacts = report["artifacts"].as_array().ok_or("missing artifacts")?;
    let artifact = artifacts
        .iter()
        .find(|asset| asset["kind"] == "windows-exe")
        .ok_or("missing EXE")?;
    let exe = run_file(
        root,
        &dir,
        artifact["path"].as_str().ok_or("missing EXE path")?,
    )?;
    if fs::metadata(&exe)?.len() != artifact["bytes"].as_u64().ok_or("missing EXE size")?
        || digest(&exe)? != artifact["sha256"].as_str().ok_or("missing EXE SHA256")?
    {
        return Err("EXE bytes differ from the CI record".into());
    }
    let staging: Vec<_> = artifacts
        .iter()
        .filter(|asset| asset["kind"] == "frontend-staging")
        .cloned()
        .collect();
    if staging.len() != 2 || !records::staging_matches(&staging) {
        return Err("UI/distribution staging hash evidence is incomplete".into());
    }
    let notes_path = root.join(notes.ok_or("--notes is required")?);
    let note_text = fs::read_to_string(&notes_path)?;
    if note_text.trim().is_empty() {
        return Err("write a short change description in --notes".into());
    }
    let mut logs = vec![];
    for phase in report["phases"].as_array().unwrap() {
        for key in ["stdout", "stderr"] {
            if let Some(path) = phase[key].as_str() {
                logs.push(
                    json!({"path":path,"content":fs::read_to_string(run_file(root, &dir, path)?)?}),
                );
            }
        }
    }
    let mut evidence = json!({"logs":logs});
    for (key, path) in [
        ("playwright", "ui/results.json"),
        ("workers", "ui/workers.json"),
        ("rankingTop5", "ranking-top5.json"),
        ("fixtureManifest", "corpus/manifest.json"),
    ] {
        evidence[key] = read_json(&dir.join(path))?;
    }
    let publication = if published {
        match observe(root, repo, tag, tested, &target, artifact) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                json!({"checkedAtUnixMs":crate::unix_ms(),"verified":false,"issues":[error.to_string()]})
            }
        }
    } else {
        json!({"verified":null,"status":"not-checked"})
    };
    let record = json!({"schemaVersion":1,"tag":tag,"repository":repo,"releaseTargetCommit":target,"documentationChangesSinceTest":changes,
        "ciReportSha256":digest(&report_path)?,"changesSha256":digest(&notes_path)?,"changes":note_text,"ci":report,"evidence":evidence,"publication":publication});
    let base = format!("https://github.com/{repo}/releases/download/{tag}");
    // Public notes are written for users. Detailed evidence belongs in validation.md.
    // Add its pinned repository link after committing the published validation record.
    let body = format!("# docs-search {tag}\n\n{}\n", note_text.trim());
    let overview = records::overview(&record["ci"]);
    let mut validation = format!(
        "# {tag} 検証・公開記録\n\n自動生成。Release対象: `{}`。試験後の文書差分: `{}`。\n\n{}",
        records::text(&record["releaseTargetCommit"]),
        records::text(&record["documentationChangesSinceTest"]),
        overview.split_once("\n\n").unwrap().1
    );
    validation += &format!(
        "\n原データ・工程コマンド・ログ・ケース結果: [validation.json]({base}/validation.json)。\n"
    );
    if published {
        validation += &format!("\n公開照合: **{}**。確認時刻（Unix ms）: {}。\n\nRelease: {}。remote main/tag: `{}` / `{}`。Latest: `{}`。\n\n照合理由: `{}`。\n",
            if publication["verified"] == true { "passed" } else { "failed" }, records::text(&publication["checkedAtUnixMs"]), records::text(&publication["url"]),
            records::text(&publication["remoteMain"]), records::text(&publication["remoteTag"]), records::text(&publication["isLatest"]), records::text(&publication["issues"]));
    } else {
        validation += "\n公開情報: 未収集（公開後の --published で照合）。\n";
    }
    let destination = dir.join("release").join(tag);
    fs::create_dir_all(&destination)?;
    fs::write(
        destination.join("validation.json"),
        serde_json::to_string_pretty(&record)? + "\n",
    )?;
    fs::write(destination.join("validation.md"), validation)?;
    fs::write(destination.join("release-notes.md"), body)?;
    println!("Release records: {}", destination.display());
    Ok(!published || publication["verified"] == true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_and_test_changes_require_new_ci() {
        assert!(documentation_only("docs/development.md\0README.md\0"));
        assert!(documentation_only(""));
        for path in [
            "core/src/lib.rs",
            "xtask/src/main.rs",
            "tests/README.md",
            "Cargo.lock",
            "docs/config.json",
        ] {
            assert!(!documentation_only(path), "{path}");
        }
    }
    #[test]
    fn failed_build_only_and_legacy_runs_are_rejected() {
        for report in [
            json!({"metadata":{"schemaVersion":1,"command":"ci"},"success":true}),
            json!({"metadata":{"schemaVersion":2,"command":"ci"},"success":false}),
            json!({"metadata":{"schemaVersion":2,"command":"build"},"success":true}),
        ] {
            assert!(require_ci(&report, "v3.0.2").is_err());
        }
    }
    #[test]
    fn release_requires_complete_counts_a_clean_source_and_matching_version() {
        let phases: Vec<_> = records::BACKEND
            .iter()
            .chain(records::FRONTEND)
            .chain(records::INFRASTRUCTURE)
            .copied()
            .chain(["desktop-stage", "desktop-build", "desktop-artifact"])
            .map(|name| json!({"name":name,"status":"passed","exit_code":0}))
            .collect();
        let source = json!({"commit":{"exitCode":0,"stdout":"tested"},"workingTree":{"exitCode":0,"stdout":""}});
        let counts = json!({"passed":64,"failed":0,"ignored":0,"filtered_out":0});
        let mut valid = json!({"metadata":source,"sourceAtFinish":source,"success":true,"phases":phases,"frontendStaging":{"status":"passed","directories":2},
            "summary":{"backend":{"status":"passed","rustTestFunctions":counts,"nodeTestCases":{"tests":8,"pass":8,"fail":0,"cancelled":0,"skipped":0,"todo":0}},"infrastructure":{"status":"passed","rustTestFunctions":counts},
                "frontend":{"status":"passed","playwright":{"errors":0,"stats":{"expected":54,"skipped":0,"unexpected":0,"flaky":0}}}}});
        valid["metadata"]["schemaVersion"] = json!(2);
        valid["metadata"]["command"] = json!("ci");
        valid["metadata"]["productVersion"] = json!("1.2.3");
        valid["metadata"]["tauriVersion"] = json!("1.2.3");
        assert!(require_ci(&valid, "v1.2.3").is_ok());
        assert!(require_ci(&valid, "v1.2.4").is_err());
        for (pointer, value) in [
            (
                "/sourceAtFinish/workingTree/stdout",
                json!(" M frontend/app.js"),
            ),
            ("/sourceAtFinish/commit/stdout", json!("other")),
            ("/summary/backend/rustTestFunctions/ignored", json!(1)),
            ("/summary/frontend/playwright/stats/skipped", json!(1)),
            ("/phases/0/status", json!("skipped")),
        ] {
            let mut invalid = valid.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            assert!(require_ci(&invalid, "v1.2.3").is_err(), "{pointer}");
        }
    }
    #[test]
    fn publication_requires_uploaded_bytes_and_the_exact_tag() {
        let artifact = json!({"sha256":"abc","bytes":100});
        let valid = json!({"remoteTag":"commit","requestedTag":"v1","tagName":"v1","draft":false,"prerelease":false,
            "asset":{"state":"uploaded","size":100,"digest":"sha256:abc"}});
        assert!(publication_issues(&valid, "commit", &artifact).is_empty());
        for (pointer, value) in [
            ("/remoteTag", json!("old")),
            ("/asset/digest", Value::Null),
            ("/asset/size", json!(99)),
            ("/draft", json!(true)),
        ] {
            let mut invalid = valid.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            assert!(
                !publication_issues(&invalid, "commit", &artifact).is_empty(),
                "{pointer}"
            );
        }
    }
}
