//! Armada's repository gate.
//!
//! `cargo xtask verify-foundations` is **written before any crate it checks**
//! and is red from that moment until the last subject lands. There is no
//! pending state: a rule whose subject does not exist yet fails, and says what
//! is missing. A gate that goes green before the milestone is finished is the
//! green-signal-with-nothing-behind-it failure it exists to prevent.
//!
//! No dependencies. The gate must run on a checkout with nothing built.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod rules;

/// One rule's outcome. `Warn` never fails the gate; `Fail` always does.
pub enum Finding {
    Fail(String),
    Warn(String),
}

/// What a rule reports back.
pub struct Report {
    pub rule: &'static str,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn new(rule: &'static str) -> Self {
        Report { rule, findings: Vec::new() }
    }
    pub fn fail(&mut self, what: impl Into<String>) {
        self.findings.push(Finding::Fail(what.into()));
    }
    pub fn warn(&mut self, what: impl Into<String>) {
        self.findings.push(Finding::Warn(what.into()));
    }
    pub fn failed(&self) -> bool {
        self.findings.iter().any(|f| matches!(f, Finding::Fail(_)))
    }
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("verify-foundations") => verify_foundations(),
        Some(other) => {
            eprintln!("xtask: unknown task `{other}`");
            eprintln!("tasks: verify-foundations");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("xtask: no task given");
            eprintln!("tasks: verify-foundations");
            ExitCode::FAILURE
        }
    }
}

fn verify_foundations() -> ExitCode {
    let root = repo_root();
    let reports = vec![
        rules::acceptance_test_exists_and_fails(&root),
        rules::every_failure_mode_has_a_fixture(&root),
        rules::no_file_too_long(&root),
        rules::every_source_file_is_in_the_manifest(&root),
        rules::no_untyped_json_outside_store_and_ipc(&root),
        rules::no_vendor_literal_outside_adapters(&root),
        rules::no_bloated_claude_md(&root),
        rules::the_v1_harvest_has_an_index(&root),
    ];

    let mut out = String::new();
    let (mut fails, mut warns) = (0usize, 0usize);
    for report in &reports {
        let mark = if report.failed() { "FAIL" } else { "ok  " };
        let _ = writeln!(out, "{mark}  {}", report.rule);
        for finding in &report.findings {
            match finding {
                Finding::Fail(what) => {
                    fails += 1;
                    let _ = writeln!(out, "        missing: {what}");
                }
                Finding::Warn(what) => {
                    warns += 1;
                    let _ = writeln!(out, "        warn:    {what}");
                }
            }
        }
    }
    print!("{out}");

    if fails > 0 {
        println!("\nverify-foundations: RED — {fails} failing, {warns} warning");
        println!("Red is the expected state until M0 is finished. Each line above names its subject.");
        ExitCode::FAILURE
    } else {
        println!("\nverify-foundations: green — {warns} warning");
        ExitCode::SUCCESS
    }
}

/// The repository root, found by walking up from this crate's manifest.
pub fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

/// Every file under `dir` with one of `exts`, sorted, as repo-relative paths.
/// A missing directory yields nothing rather than an error — the gate reports
/// absence through its own rules, not through a panic.
pub fn files_with_ext(root: &Path, dir: &Path, exts: &[&str]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    walk(dir, &mut |path| {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if exts.contains(&ext) {
            if let Ok(rel) = path.strip_prefix(root) {
                found.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    });
    found
}

/// Depth-first walk. Skips `target`, `node_modules` and dot-directories, which
/// are build output rather than source and are not the gate's business.
pub fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            walk(&path, visit);
        } else {
            visit(&path);
        }
    }
}

/// Every directory directly under `crates/`, sorted. Empty when there are none.
pub fn crate_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join("crates")) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                dirs.push(entry.path());
            }
        }
    }
    dirs.sort();
    dirs
}
