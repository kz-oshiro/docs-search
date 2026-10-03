use docs_search_test_support::{common, digest, icons, specialized, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
mod records;
mod release;
const ASSETS: &[&str] = &[
    "index.html",
    "style.css",
    "theme.css",
    "theme.js",
    "boot.js",
    "app.js",
    "view.js",
    "tauri.js",
];
#[derive(Serialize)]
struct Phase {
    name: String,
    command: Option<Vec<String>>,
    working_directory: Option<String>,
    environment: Option<Value>,
    status: String,
    exit_code: Option<i32>,
    started_at_unix_ms: u128,
    duration_ms: u128,
    stdout: Option<String>,
    stderr: Option<String>,
    reason: Option<String>,
}
struct Run {
    root: PathBuf,
    dir: PathBuf,
    target: PathBuf,
    log_prefix: &'static str,
    phases: Vec<Phase>,
    metadata: Value,
    artifacts: Vec<Value>,
}
fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn panic_reason(error: Box<dyn std::any::Any + Send>) -> String {
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap_or("unknown panic");
    format!("test runner panicked: {message}")
}
fn capture(root: &Path, program: &str, args: &[&str]) -> Value {
    match Command::new(program).args(args).current_dir(root).output() {
        Ok(output) => {
            json!({"exitCode":output.status.code(),"stdout":String::from_utf8_lossy(&output.stdout).trim(),"stderr":String::from_utf8_lossy(&output.stderr).trim()})
        }
        Err(error) => json!({"unavailable":error.to_string()}),
    }
}
fn npm() -> &'static str {
    if cfg!(windows) {
        "npm.cmd"
    } else {
        "npm"
    }
}
impl Run {
    fn new(command: &str) -> Result<Self> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let dir = root.join(format!(
            "outputs/runs/{stamp}-{}-{command}",
            std::process::id()
        ));
        fs::create_dir_all(dir.parent().unwrap())?;
        fs::create_dir(&dir)?;
        let mut metadata = json!({"schemaVersion":2,"command":command,"invocation":std::env::args().skip(1).collect::<Vec<_>>(),"runId":dir.file_name().unwrap().to_string_lossy(),"startedAtUnixMs":stamp,"commit":capture(&root,"git",&["rev-parse","HEAD"]),
            "workingTree":capture(&root,"git",&["status","--porcelain"]),"rustc":capture(&root,"rustc",&["--version"]),"cargo":capture(&root,"cargo",&["--version"]),
            "node":capture(&root,"node",&["--version"]),"npm":capture(&root,npm(),&["--version"]),"playwright":capture(&root,"node",&["-p","require('./tests/ui/node_modules/@playwright/test/package.json').version"]),"playwrightRequired":"1.63.0","platform":std::env::consts::OS,"architecture":std::env::consts::ARCH,
            "proptest":{"casesEnv":std::env::var("PROPTEST_CASES").ok(),"rngSeedEnv":std::env::var("PROPTEST_RNG_SEED").ok()}});
        let output = Command::new("cargo")
            .args(["metadata", "--no-deps", "--locked", "--format-version", "1"])
            .current_dir(&root)
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
        }
        let value: Value = serde_json::from_slice(&output.stdout)?;
        let packages = value["packages"]
            .as_array()
            .ok_or("missing workspace packages")?;
        metadata["productVersion"] = packages
            .iter()
            .find(|package| package["name"] == "docs-search-desktop")
            .ok_or("missing desktop package")?["version"]
            .clone();
        metadata["packageVersions"] = json!(packages
            .iter()
            .map(|package| json!({"name":package["name"],"version":package["version"]}))
            .collect::<Vec<_>>());
        let tauri: Value =
            serde_json::from_slice(&fs::read(root.join("src-tauri/tauri.conf.json"))?)?;
        metadata["tauriVersion"] = tauri["version"].clone();
        let target = PathBuf::from(
            value["target_directory"]
                .as_str()
                .ok_or("missing target directory")?,
        );
        Ok(Self {
            root,
            dir,
            target,
            log_prefix: "",
            phases: vec![],
            metadata,
            artifacts: vec![],
        })
    }
    fn branch(&self, log_prefix: &'static str) -> Self {
        Self {
            root: self.root.clone(),
            dir: self.dir.clone(),
            target: self.target.clone(),
            log_prefix,
            phases: vec![],
            metadata: self.metadata.clone(),
            artifacts: vec![],
        }
    }
    fn absorb(&mut self, branch: Self) {
        self.phases.extend(branch.phases);
        self.artifacts.extend(branch.artifacts);
    }
    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
    fn skipped(&mut self, name: &str, reason: &str) {
        println!("SKIP {name}: {reason}");
        self.phases.push(Phase {
            name: name.into(),
            command: None,
            working_directory: None,
            environment: None,
            status: "skipped".into(),
            exit_code: None,
            started_at_unix_ms: unix_ms(),
            duration_ms: 0,
            stdout: None,
            stderr: None,
            reason: Some(reason.into()),
        });
    }
    fn phase(
        &mut self,
        name: &str,
        program: &str,
        args: &[&str],
        cwd: &Path,
        env: &[(&str, &Path)],
    ) -> bool {
        println!("START {name}");
        let started_at_unix_ms = unix_ms();
        let start = Instant::now();
        let index = self.phases.len() + 1;
        let out = self
            .dir
            .join(format!("{}{index:02}-{name}.stdout.log", self.log_prefix));
        let err = self
            .dir
            .join(format!("{}{index:02}-{name}.stderr.log", self.log_prefix));
        let result = (|| -> Result<_> {
            let mut cmd = Command::new(program);
            cmd.args(args)
                .current_dir(cwd)
                .stdout(Stdio::from(fs::File::create(&out)?))
                .stderr(Stdio::from(fs::File::create(&err)?));
            for (key, value) in env {
                cmd.env(key, value);
            }
            Ok(cmd.status()?)
        })();
        let (passed, code, reason) = match result {
            Ok(status) => (status.success(), status.code(), None),
            Err(error) => {
                let _ = fs::write(&err, error.to_string());
                (false, None, Some(error.to_string()))
            }
        };
        let duration = start.elapsed().as_millis();
        println!(
            "{} {name} ({duration} ms) — {}",
            if passed { "PASS" } else { "FAIL" },
            self.relative(&out)
        );
        self.phases.push(Phase {
            name: name.into(),
            command: Some(
                std::iter::once(program)
                    .chain(args.iter().copied())
                    .map(str::to_owned)
                    .collect(),
            ),
            working_directory: Some(self.relative(cwd)),
            environment: Some(Value::Object(
                env.iter()
                    .map(|(key, value)| ((*key).to_owned(), json!(self.relative(value))))
                    .collect(),
            )),
            status: if passed { "passed" } else { "failed" }.into(),
            exit_code: code,
            started_at_unix_ms,
            duration_ms: duration,
            stdout: Some(self.relative(&out)),
            stderr: Some(self.relative(&err)),
            reason,
        });
        passed
    }
    fn local<F>(&mut self, name: &str, action: F) -> bool
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        println!("START {name}");
        let started_at_unix_ms = unix_ms();
        let start = Instant::now();
        let result = action(self);
        let success = result.is_ok();
        let reason = result.err().map(|e| e.to_string());
        let duration = start.elapsed().as_millis();
        println!(
            "{} {name} ({duration} ms){}",
            if success { "PASS" } else { "FAIL" },
            reason
                .as_ref()
                .map(|v| format!(": {v}"))
                .unwrap_or_default()
        );
        self.phases.push(Phase {
            name: name.into(),
            command: None,
            working_directory: None,
            environment: None,
            status: if success { "passed" } else { "failed" }.into(),
            exit_code: Some(if success { 0 } else { 1 }),
            started_at_unix_ms,
            duration_ms: duration,
            stdout: None,
            stderr: None,
            reason,
        });
        success
    }
    fn stage(&mut self, folder: &Path) -> Result<()> {
        for path in [folder.parent().unwrap(), folder] {
            if let Ok(metadata) = fs::symlink_metadata(path) {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    metadata.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = metadata.file_type().is_symlink();
                if linked {
                    return Err("frontend staging must use a regular directory".into());
                }
            }
        }
        fs::create_dir_all(folder)?;
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            let name = entry.file_name();
            if !ASSETS.contains(&name.to_string_lossy().as_ref())
                || entry.file_type()?.is_symlink()
                || !entry.file_type()?.is_file()
            {
                return Err(format!(
                    "unexpected frontend staging entry: {}",
                    entry.path().display()
                )
                .into());
            }
        }
        for asset in ASSETS {
            let bytes = fs::read(self.root.join("frontend").join(asset))?;
            let destination = folder.join(asset);
            if fs::read(&destination).ok().as_deref() != Some(bytes.as_slice()) {
                fs::write(destination, bytes)?;
            }
        }
        let files = ASSETS
            .iter()
            .map(|asset| -> Result<Value> {
                let source = digest(&self.root.join("frontend").join(asset))?;
                let staged = digest(&folder.join(asset))?;
                if source != staged {
                    return Err(format!("frontend staging hash differs: {asset}").into());
                }
                Ok(json!({"name":asset,"sourceSha256":source,"sha256":staged}))
            })
            .collect::<Result<Vec<_>>>()?;
        self.artifacts
            .push(json!({"kind":"frontend-staging","path":self.relative(folder),"files":files}));
        Ok(())
    }
    fn ui(&mut self, case: Option<&str>) -> bool {
        let folder = self.root.join("tests/ui");
        let site = self.dir.join("site");
        let artifacts = self.dir.join("ui");
        let staged = self.local("ui-stage", |run| {
            run.stage(&site)?;
            fs::create_dir(&artifacts)?;
            Ok(())
        });
        let node = self.phase("ui-node", "node", &["check-node.mjs"], &folder, &[]);
        let prepared = if node {
            self.phase(
                "ui-preflight",
                "node",
                &["preflight.mjs"],
                &folder,
                &[("DOCS_SEARCH_UI_ARTIFACTS", &artifacts)],
            )
        } else {
            self.skipped(
                "ui-preflight",
                "Node.js 20 or newer is required; run cargo xtask setup after installing Node.js",
            );
            false
        };
        if !staged || !prepared {
            self.skipped(
                "playwright",
                "frontend staging or explicit setup prerequisite failed; run cargo xtask setup",
            );
            return false;
        }
        self.artifacts.push(json!({
            "kind": "ui-workers",
            "path": self.relative(&artifacts.join("workers.json")),
        }));
        let mut args = vec!["node_modules/@playwright/test/cli.js", "test"];
        if let Some(case) = case {
            args.extend(["--grep", case]);
        }
        self.phase(
            "playwright",
            "node",
            &args,
            &folder,
            &[
                ("DOCS_SEARCH_UI_SITE", &site),
                ("DOCS_SEARCH_UI_ARTIFACTS", &artifacts),
            ],
        )
    }
    fn tests(&mut self) -> bool {
        self.metadata["automaticTestExecution"] = json!({
            "backendFrontendParallel": true,
            "cargoPhases": "sequential",
        });
        let mut backend = self.branch("backend-");
        let mut frontend = self.branch("frontend-");
        // Only the independent Node/browser group overlaps the Cargo group.
        // Each branch owns its reports and logs; no Run state is shared mutably.
        let (joined, frontend_passed) = std::thread::scope(|scope| {
            let background = scope.spawn(move || {
                let passed = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    backend.backend_tests()
                })) {
                    Ok(passed) => passed,
                    Err(error) => {
                        backend.local("backend-runner", |_| Err(panic_reason(error).into()))
                    }
                };
                (backend, passed)
            });
            let passed = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                frontend.ui(None)
            })) {
                Ok(passed) => passed,
                Err(error) => {
                    frontend.local("frontend-runner", |_| Err(panic_reason(error).into()))
                }
            };
            (background.join(), passed)
        });
        let backend_passed = match joined {
            Ok((backend, passed)) => {
                self.absorb(backend);
                passed
            }
            Err(error) => self.local("backend-runner", |_| Err(panic_reason(error).into())),
        };
        self.absorb(frontend);
        self.phases.sort_by_key(|phase| phase.started_at_unix_ms);
        // Validate result files before CI starts the build. This snapshot covers
        // tests only; the final CI report also covers the distribution EXE.
        let mut metadata = self.metadata.clone();
        metadata["command"] = json!("test");
        let mut evidence = json!({"metadata":metadata,"finishedAtUnixMs":unix_ms(),"success":backend_passed && frontend_passed,"phases":self.phases,"artifacts":self.artifacts});
        records::complete(&self.root, &self.dir, &mut evidence);
        if evidence["success"] != true {
            println!(
                "FAIL automatic-test-evidence; inspect the report summaries and failed phase logs"
            );
        }
        evidence["success"] == true
    }
    fn backend_tests(&mut self) -> bool {
        let root = self.root.clone();
        let corpus = self.dir.join("corpus");
        let shared=self.local("fixture-corpus",|run|{let manifest=common::generate(&corpus,"load")?;run.artifacts.push(json!({"kind":"fixture-manifest","path":run.relative(&corpus.join("manifest.json")),"files":manifest["files"].as_array().unwrap().len(),"seed":manifest["seed"]}));Ok(())});
        let env: Vec<_> = if shared {
            vec![("DOCS_SEARCH_TEST_CORPUS", corpus.as_path())]
        } else {
            vec![]
        };
        let mut success = shared;
        success &= self.phase(
            "core-api-properties",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "docs-search-core",
                "--lib",
                "--test",
                "requirements_v3",
                "--",
                "--nocapture",
            ],
            &root,
            &[],
        );
        success &= self.phase(
            "generators",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "docs-search-test-support",
                "--",
                "--nocapture",
            ],
            &root,
            &env,
        );
        success &= self.phase(
            "record-contracts",
            "cargo",
            &["test", "--locked", "-p", "xtask", "--", "--nocapture"],
            &root,
            &[],
        );
        for (name, test) in [
            ("common-cli", "cli_backend"),
            ("context-cli", "cli_context"),
            ("conditions-cli", "cli_conditions"),
            ("office-cli", "cli_office"),
            ("issues-cli", "cli_issues"),
            ("index-cli", "cli_index"),
        ] {
            let report = self.dir.join("ranking-top5.json");
            let mut child_env = env.clone();
            if test == "cli_office" {
                child_env.push(("DOCS_SEARCH_RANKING_REPORT", report.as_path()));
            }
            success &= self.phase(
                name,
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "docs-search-core",
                    "--test",
                    test,
                    "--",
                    "--nocapture",
                ],
                &root,
                &child_env,
            );
        }
        let folder = self.root.join("tests/ui");
        success &= self.phase(
            "worker-policy",
            "node",
            &["--test", "--test-reporter=tap", "workers.test.mjs"],
            &folder,
            &[],
        );
        success
    }
    fn build(&mut self) -> bool {
        if !cfg!(windows) {
            self.skipped(
                "desktop-build",
                "Windows desktop EXE build requires a Windows host",
            );
            return false;
        }
        let folder = self.root.join("target/frontend-dist");
        if !self.local("desktop-stage", |run| run.stage(&folder)) {
            self.skipped("desktop-build", "frontend staging failed");
            return false;
        }
        let root = self.root.clone();
        if !self.phase(
            "desktop-build",
            "cargo",
            &[
                "build",
                "--release",
                "--locked",
                "-p",
                "docs-search-desktop",
            ],
            &root,
            &[],
        ) {
            return false;
        }
        self.local("desktop-artifact",|run|{let source=run.target.join("release/docs-search-desktop.exe");let folder=run.dir.join("artifacts");fs::create_dir(&folder)?;let destination=folder.join("docs-search-desktop.exe");fs::copy(&source,&destination)?;let sha256=digest(&destination)?;run.artifacts.push(json!({"kind":"windows-exe","path":run.relative(&destination),"sha256":sha256,"bytes":fs::metadata(&destination)?.len()}));Ok(())})
    }
    fn setup(&mut self) -> bool {
        let folder = self.root.join("tests/ui");
        let node = self.phase("setup-node", "node", &["check-node.mjs"], &folder, &[]);
        if !node {
            self.skipped("setup-npm", "Node.js 20 or newer is required");
            self.skipped("setup-chromium", "Node prerequisite failed");
            return false;
        }
        if !self.phase(
            "setup-npm",
            npm(),
            &["ci", "--no-audit", "--no-fund"],
            &folder,
            &[],
        ) {
            self.skipped("setup-chromium", "npm ci failed");
            return false;
        }
        let installed = self.phase(
            "setup-chromium",
            "node",
            &[
                "node_modules/@playwright/test/cli.js",
                "install",
                "chromium",
            ],
            &folder,
            &[],
        );
        let checked = self.phase("setup-preflight", "node", &["preflight.mjs"], &folder, &[]);
        installed && checked
    }
    fn report(&self, success: bool) -> Result<bool> {
        let mut document = json!({"metadata":self.metadata,"finishedAtUnixMs":unix_ms(),"sourceAtFinish":{"commit":capture(&self.root,"git",&["rev-parse","HEAD"]),"workingTree":capture(&self.root,"git",&["status","--porcelain"])},"success":success,"phases":self.phases,"artifacts":self.artifacts});
        records::complete(&self.root, &self.dir, &mut document);
        fs::write(
            self.dir.join("report.json"),
            serde_json::to_string_pretty(&document)? + "\n",
        )?;
        let mut md = records::overview(&document);
        md += "\n| Phase | Status | Exit | Start offset (ms) | Duration (ms) | Logs / reason |\n|---|---|---:|---:|---:|---|\n";
        for phase in &self.phases {
            let detail =
                phase
                    .reason
                    .clone()
                    .unwrap_or_else(|| match (&phase.stdout, &phase.stderr) {
                        (Some(out), Some(err)) => format!(
                            "[stdout]({}) · [stderr]({})",
                            Path::new(out).file_name().unwrap().to_string_lossy(),
                            Path::new(err).file_name().unwrap().to_string_lossy()
                        ),
                        _ => "—".into(),
                    });
            md += &format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                phase.name,
                phase.status,
                phase
                    .exit_code
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into()),
                phase.started_at_unix_ms.saturating_sub(
                    self.metadata["startedAtUnixMs"].as_u64().unwrap_or(0) as u128,
                ),
                phase.duration_ms,
                detail.replace('|', "\\|").replace('\n', " ")
            );
        }
        if self.metadata["automaticTestExecution"]["backendFrontendParallel"] == true {
            md += "\nBackend and frontend phases overlap. Phase durations must not be summed as wall time.\n";
        }
        if let Some(workers) = self
            .artifacts
            .iter()
            .find(|artifact| artifact["kind"] == "ui-workers")
        {
            let path = workers["path"].as_str().unwrap();
            let prefix = format!("{}/", self.relative(&self.dir));
            let relative = path.strip_prefix(prefix.as_str()).unwrap_or(path);
            md += &format!(
                "\nUI CPU measurement and selected workers: [workers.json]({relative}).\n"
            );
        }
        md += "\nMetadata and artifact SHA256: [report.json](report.json).\n";
        fs::write(self.dir.join("report.md"), md)?;
        println!("Report: {}", self.dir.join("report.md").display());
        Ok(document["success"] == true)
    }
}
fn execute() -> Result<bool> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        println!("cargo xtask <test|build|ci|ui|setup|fixtures|icons|compare-fixtures|release-record>\n  ui [--case REGEX]\n  fixtures [--profile acceptance|load] [--kind common|context|conditions|office|issues] [--output NEW_DIRECTORY]\n  compare-fixtures --old OLD_DIRECTORY --new NEW_DIRECTORY\n  release-record --run RUN_DIRECTORY --tag vX.Y.Z --notes CHANGES.md [--repo OWNER/REPO] [--published]");
        return Ok(true);
    };
    if command == "release-record" {
        return release::execute(&args[1..]);
    }
    let mut profile = "acceptance";
    let mut kind = "common";
    let mut output = None;
    let mut case = None;
    let mut open_folder = false;
    let mut old = None;
    let mut new = None;
    let mut options = args[1..].iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--profile" | "-Profile" if command == "fixtures" => {
                profile = options.next().ok_or("profile value required")?
            }
            "--output" if command == "fixtures" => {
                output = Some(PathBuf::from(
                    options.next().ok_or("output value required")?,
                ))
            }
            "--kind" if command == "fixtures" => {
                kind = options.next().ok_or("fixture kind required")?
            }
            "--old" if command == "compare-fixtures" => {
                old = Some(PathBuf::from(options.next().ok_or("old corpus required")?))
            }
            "--new" if command == "compare-fixtures" => {
                new = Some(PathBuf::from(options.next().ok_or("new corpus required")?))
            }
            "--case" if command == "ui" => {
                case = Some(options.next().ok_or("case regex required")?.as_str())
            }
            "--open" if command == "fixtures" => open_folder = true,
            "-NoOpen" if command == "fixtures" => open_folder = false,
            _ => return Err(format!("unknown option for {command}: {option}").into()),
        }
    }
    if ![
        "test",
        "build",
        "ci",
        "ui",
        "setup",
        "fixtures",
        "icons",
        "compare-fixtures",
    ]
    .contains(&command)
    {
        return Err(format!("unknown command: {command}").into());
    }
    let mut run = Run::new(command)?;
    let success=match command {
        "test"=>run.tests(),"build"=>run.build(),"ui"=>run.ui(case),"setup"=>run.setup(),
        "ci"=>{let passed=run.tests();if passed {run.build()} else {run.skipped("desktop-build","required automatic tests did not all pass");false}},
        "fixtures"=>run.local("fixtures",|run|{
            let output=output.unwrap_or_else(||run.root.join("outputs/test-data").join(format!("{kind}-{profile}-{}",run.dir.file_name().unwrap().to_string_lossy())));
            if output.exists() && fs::read_dir(&output)?.next().is_some() {return Err("fixture output directory must be new or empty".into());}
            match kind {"common"=>{common::generate(&output,profile)?;},"context"=>specialized::context(&output.join("row-window.xlsx"))?,"conditions"=>specialized::conditions(&output)?,"office"=>specialized::office(&output)?,"issues"=>specialized::issues(&output)?,_=>return Err("fixture kind must be common, context, conditions, office or issues".into())}
            let documents=if kind=="common" {let manifest:Value=serde_json::from_slice(&fs::read(output.join("manifest.json"))?)?;json!(manifest["files"].as_array().unwrap().len())} else {Value::Null};
            run.artifacts.push(json!({"kind":"fixtures","corpus":kind,"profile":profile,"path":run.relative(&output),"seed":docs_search_test_support::SEED,"documentCount":documents,"inventoryFiles":docs_search_test_support::files(&output)?.len()}));println!("Fixtures: {}",output.display());
            if open_folder {let program=if cfg!(windows) {"explorer.exe"} else if cfg!(target_os="macos") {"open"} else {"xdg-open"};let absolute=if output.is_absolute() {output} else {std::env::current_dir()?.join(output)};Command::new(program).arg(absolute).spawn()?;}Ok(())}),
        "icons"=>run.local("icons",|run|icons::generate(&run.root.join("src-tauri/icons"))),
        "compare-fixtures"=>run.local("fixture-comparison",|run|{
            let comparison=docs_search_test_support::compare::fixtures(&old.ok_or("--old is required")?,&new.ok_or("--new is required")?)?;
            let path=run.dir.join("fixture-comparison.json");fs::write(&path,serde_json::to_string_pretty(&comparison)?+"\n")?;run.artifacts.push(json!({"kind":"fixture-comparison","path":run.relative(&path)}));
            if comparison["sameContract"]==true {Ok(())} else {Err("fixture contracts differ; inspect fixture-comparison.json".into())}
        }),
        _=>unreachable!()
    };
    run.report(success)
}
fn main() {
    match execute() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
