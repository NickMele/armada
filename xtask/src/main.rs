//! Armada's repository gate.
//!
//! `cargo xtask verify-foundations` is **written before any crate it checks**,
//! and there is no pending state: a rule whose subject does not exist yet
//! fails, and says what is missing. So red is a legitimate reading, and green
//! says only that every subject named so far has landed. What this design
//! refuses is a green signal with nothing behind it, never red itself.
//!
//! **Neither colour is the signal — the delta is.** A `missing:` line a change
//! introduced is a regression whatever the totals did.
//!
//! No dependencies. The gate must run on a checkout with nothing built.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod capabilities;
mod docs;
mod roadmap;
mod rules;
mod rules_actions;
mod rules_design;
mod rules_docs;
mod rules_enums;
mod rules_icons;
mod rules_privacy;
mod rules_protocol;
mod rules_screens;
mod rules_stories;
mod rules_stylesheets;
mod rules_tokens;
mod rules_toolbelt;
mod rules_unsafe;
mod tokens;
mod tokens_emit;

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
        Report {
            rule,
            findings: Vec::new(),
        }
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
        Some("verify-tokens") => {
            let write = std::env::args().any(|a| a == "--write");
            verify_tokens(write)
        }
        Some("verify-roadmap") => verify_roadmap(),
        Some("verify-docs") => {
            let write = std::env::args().any(|a| a == "--write");
            regenerate("verify-docs", docs::outputs, write)
        }
        Some(other) => {
            eprintln!("xtask: unknown task `{other}`");
            eprintln!("tasks: verify-foundations, verify-tokens [--write], verify-docs [--write], verify-roadmap");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("xtask: no task given");
            eprintln!("tasks: verify-foundations, verify-tokens [--write], verify-docs [--write], verify-roadmap");
            ExitCode::FAILURE
        }
    }
}

fn verify_foundations() -> ExitCode {
    let root = repo_root();
    let reports = vec![
        rules::acceptance_test_exists_and_passes(&root),
        rules::every_failure_mode_has_a_fixture(&root),
        rules::no_file_too_long(&root),
        rules::no_comment_block_too_long(&root),
        rules::no_untyped_json_outside_store_and_ipc(&root),
        rules::no_vendor_literal_outside_adapters(&root),
        rules::no_bloated_claude_md(&root),
        rules::the_v1_harvest_has_an_index(&root),
        rules_privacy::nothing_names_a_person_or_a_machine(&root),
        rules_unsafe::unsafe_is_spoken_only_where_named(&root),
        rules::nothing_writes_its_own_log_format(&root),
        rules_tokens::the_tokens_generate_what_is_checked_in(&root),
        rules_tokens::no_media_query_resolves_through_a_custom_property(&root),
        rules_design::no_off_contract_design_value(&root),
        rules_docs::every_open_question_is_collected(&root),
        rules_docs::every_document_is_indexed(&root),
        rules_docs::nothing_links_to_the_design_workspace(&root),
        rules_docs::every_path_a_document_names_exists(&root),
        capabilities::every_capability_is_bound_and_indexed(&root),
        rules_icons::every_glyph_in_use_is_registered(&root),
        rules_actions::every_action_carries_three_columns(&root),
        rules_stories::every_story_names_its_own_path(&root),
        rules_screens::every_render_has_a_screen(&root),
        rules_stylesheets::every_stylesheet_reaches_the_sheet_the_app_loads(&root),
        rules_protocol::the_router_serves_what_the_inventory_names(&root),
        rules_protocol::version::the_version_and_its_generated_constant_agree(&root),
        rules_enums::every_registry_key_is_a_variant(&root),
        rules_enums::edges::the_registry_and_the_edge_table_hold_the_same_edges(&root),
        rules_enums::reachability::every_status_declares_the_step_states_it_holds(&root),
        rules_toolbelt::the_roster_and_the_allowlist_hold_the_same_set(&root),
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
        println!(
            "Red is a legitimate state. Each line above names its subject — read those, \
             and compare them against the baseline on main rather than to zero."
        );
        ExitCode::FAILURE
    } else {
        println!("\nverify-foundations: green — {warns} warning");
        ExitCode::SUCCESS
    }
}

/// Regenerate a task's checked-in outputs, and either write them or fail on
/// drift.
///
/// The check is a diff, not a lint: a hand-edited output is exactly as much a
/// failure as a stale one, because both mean the checked-in file no longer
/// says what its source says. `--write` is the only way to change one.
fn regenerate(
    task: &str,
    outputs: fn(&Path) -> Result<Vec<(String, String)>, String>,
    write: bool,
) -> ExitCode {
    let root = repo_root();
    let outputs = match outputs(&root) {
        Ok(outputs) => outputs,
        Err(why) => {
            eprintln!("{task}: {why}");
            return ExitCode::FAILURE;
        }
    };

    let mut stale = Vec::new();
    for (rel, want) in &outputs {
        let path = root.join(rel);
        let have = fs::read_to_string(&path).unwrap_or_default();
        if &have == want {
            println!("ok    {rel}");
            continue;
        }
        if write {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::write(&path, want) {
                Ok(()) => println!("wrote {rel}"),
                Err(e) => {
                    eprintln!("{task}: cannot write {rel}: {e}");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            println!("STALE {rel}");
            stale.push(rel.clone());
        }
    }

    if stale.is_empty() {
        println!("\n{task}: green");
        ExitCode::SUCCESS
    } else {
        println!("\n{task}: RED — {} stale", stale.len());
        println!("Run `cargo xtask {task} --write` and commit what it emits.");
        ExitCode::FAILURE
    }
}

fn verify_tokens(write: bool) -> ExitCode {
    regenerate("verify-tokens", tokens::outputs, write)
}

/// The online half of the capability check. Not a gate rule on purpose — see
/// the module comment on `roadmap` for why a check that needs GitHub is a
/// check that must not run in `verify-foundations`.
fn verify_roadmap() -> ExitCode {
    match roadmap::verify() {
        Err(why) => {
            eprintln!("verify-roadmap: {why}");
            ExitCode::FAILURE
        }
        Ok(out) => {
            for (number, title, done, total) in &out.progress {
                println!("{done}/{total}  #{number}  {title}");
            }
            if out.problems.is_empty() {
                if !out.progress.is_empty() {
                    println!();
                }
                println!("verify-roadmap: green");
                return ExitCode::SUCCESS;
            }
            println!();
            for p in &out.problems {
                println!("FAIL  {p}");
            }
            println!("\nverify-roadmap: RED — {}", out.problems.len());
            ExitCode::FAILURE
        }
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

/// Depth-first walk. Skips build output and dot-directories, which are not the
/// gate's business. `out` and `dist` are on the list because the design lint
/// found ninety violations in a compiled stylesheet the moment the app first
/// built — every rule here reads source, and none of them can tell the
/// difference on its own. `storybook-static` joined them the same way and for
/// the same reason, two thousand violations at once, which is what a list like
/// this looks like from the inside: it grows once per build tool, always after
/// the fact, and the alternative is a rule that guesses.
pub fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        const BUILD_OUTPUT: &[&str] =
            &["target", "node_modules", "out", "dist", "storybook-static"];
        if name.starts_with('.') || BUILD_OUTPUT.contains(&name) {
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
