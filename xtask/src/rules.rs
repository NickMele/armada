//! The six rules. Each returns what is missing, by name.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::{crate_dirs, files_with_ext, walk, Report};

/// Source extensions the line-length and literal rules read.
const SOURCE_EXTS: &[&str] = &["rs", "ts", "tsx"];

/// Where source lives. `docs/` is prose and is not gated.
const SOURCE_ROOTS: &[&str] = &["crates", "apps", "packages", "xtask"];

// ---------------------------------------------------------------- rule one

/// The acceptance test exists and fails.
///
/// **The gate cannot yet run it.** M0 step 9 writes the test and is what
/// decides how it is invoked; until it lands there is nothing to invoke, so
/// this rule fails on absence and names the directory it is looking in. When
/// step 9 lands the test it also wires the run here — a rule that only checks
/// for a file would go green on an empty `#[test] fn it_works() {}`, which is
/// the exact shape this milestone exists to refuse.
pub fn acceptance_test_exists_and_fails(root: &Path) -> Report {
    let mut report = Report::new("the acceptance test exists and fails");
    let dir = root.join("tests/acceptance");
    let found = files_with_ext(root, &dir, &["rs"]);
    if found.is_empty() {
        report.fail("tests/acceptance/ — no acceptance test (M0 step 9 writes it)");
    } else {
        report.fail(format!(
            "the run — {} present, and step 9 must wire how this gate invokes them",
            found.len()
        ));
    }
    report
}

// ---------------------------------------------------------------- rule two

/// Every Drone compliance failure mode named on the Workflow concept has a
/// fixture. The five modes are Workflow's own table, in its order; the file
/// names are the testkit Fixture Specs' own slugs.
const FAILURE_MODES: &[(&str, &str)] = &[
    ("silence", "Silence / Stalled"),
    ("claims-done-no-evidence", "Claims done, no evidence artifact"),
    ("plain-text-bypass", "Claims done in plain text, bypasses structured report"),
    ("thrashing", "Thrashing / off-rails"),
    ("evidence-gaming", "Evidence gaming"),
];

pub fn every_failure_mode_has_a_fixture(root: &Path) -> Report {
    let mut report = Report::new("every Drone compliance failure mode has a fixture");
    let dir = root.join("crates/testkit/fixtures/ndjson");
    for (slug, mode) in FAILURE_MODES {
        if !dir.join(format!("{slug}.ndjson")).is_file() {
            report.fail(format!(
                "crates/testkit/fixtures/ndjson/{slug}.ndjson — {mode}"
            ));
        }
    }
    report
}

// -------------------------------------------------------------- rule three

/// No source file over 900 lines. Warn at 500.
///
/// A long file is not wrong on its own; it is where the reasoning stopped
/// fitting in one place. The warn threshold is the one that does the work.
pub fn no_file_too_long(root: &Path) -> Report {
    let mut report = Report::new("no source file over 900 lines, warn at 500");
    for source_root in SOURCE_ROOTS {
        for path in files_with_ext(root, &root.join(source_root), SOURCE_EXTS) {
            let Ok(text) = fs::read_to_string(root.join(&path)) else { continue };
            let lines = text.lines().count();
            if lines > 900 {
                report.fail(format!("{path} is {lines} lines, over 900"));
            } else if lines > 500 {
                report.warn(format!("{path} is {lines} lines, over 500"));
            }
        }
    }
    report
}

// --------------------------------------------------------------- rule four

/// Every file under `crates/*/src/` is in `foundations-manifest.txt`.
///
/// The manifest is the answer to "was this file meant to be here" — one
/// repo-relative path per line, sorted, no globs and no comments, so a file
/// that appears without anybody deciding it should is visible as a diff.
/// The check runs both ways: an unlisted file fails, and so does a listed
/// file that no longer exists.
pub fn every_source_file_is_in_the_manifest(root: &Path) -> Report {
    let mut report = Report::new("every file under crates/*/src/ is in the manifest");
    let manifest_path = root.join("foundations-manifest.txt");
    let Ok(manifest_text) = fs::read_to_string(&manifest_path) else {
        report.fail("foundations-manifest.txt — the manifest itself");
        return report;
    };
    let listed: BTreeSet<String> = manifest_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();

    let mut on_disk = BTreeSet::new();
    for dir in crate_dirs(root) {
        walk(&dir.join("src"), &mut |path| {
            if let Ok(rel) = path.strip_prefix(root) {
                on_disk.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        });
    }

    for path in on_disk.difference(&listed) {
        report.fail(format!("{path} is on disk and not in the manifest"));
    }
    for path in listed.difference(&on_disk) {
        report.fail(format!("{path} is in the manifest and not on disk"));
    }

    let sorted: Vec<&String> = {
        let mut v: Vec<&String> = listed.iter().collect();
        v.sort();
        v
    };
    let as_written: Vec<&str> = manifest_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if as_written.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        != sorted.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    {
        report.fail("foundations-manifest.txt — sorted order");
    }
    report
}

// --------------------------------------------------------------- rule five

/// No `serde_json::from_*` outside `store` and `ipc`.
///
/// Untyped JSON is allowed exactly where bytes enter the process: the store
/// reading its own rows, and ipc reading the wire. Everywhere else a value
/// arrives already typed, and a `from_str` in the middle of the system means
/// something was serialised to get it there.
pub fn no_untyped_json_outside_store_and_ipc(root: &Path) -> Report {
    let mut report = Report::new("no serde_json::from_* outside store and ipc");
    let allowed = ["crates/store/", "crates/ipc/"];
    for source_root in SOURCE_ROOTS {
        for path in files_with_ext(root, &root.join(source_root), &["rs"]) {
            // The gate names the pattern it forbids, so it always matches
            // itself. `guard_write.py` carries the same exemption for the same
            // reason, and the two must not drift.
            if allowed.iter().any(|a| path.starts_with(a)) || path.starts_with("xtask/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else { continue };
            for (n, line) in text.lines().enumerate() {
                if line.contains("serde_json::from_") {
                    report.fail(format!("{path}:{} — serde_json::from_*", n + 1));
                }
            }
        }
    }
    report
}

// ---------------------------------------------------------------- rule six

/// No vendor literal outside `adapters`.
///
/// The adapter boundary is the only place that knows whose API it is talking
/// to. A vendor's name anywhere else — in a type, a string, a comment — is the
/// boundary having leaked, and it leaks in comments first.
///
/// **This list is the gate's own, not the plan's.** M0 step 7 names the rule
/// and not its vocabulary; these are the vendors Armada actually reaches for.
/// Adding one is a one-line change and should be made deliberately.
const VENDOR_LITERALS: &[&str] = &[
    "anthropic", "claude", "openai", "gpt-4", "gemini", "copilot",
    "github", "gitlab", "bitbucket", "docker",
];

pub fn no_vendor_literal_outside_adapters(root: &Path) -> Report {
    let mut report = Report::new("no vendor literal outside adapters");
    for source_root in SOURCE_ROOTS {
        for path in files_with_ext(root, &root.join(source_root), SOURCE_EXTS) {
            // The gate names vendors in order to forbid them, and the adapters
            // are where naming them is the job.
            if path.starts_with("crates/adapters/") || path.starts_with("xtask/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else { continue };
            let lower = text.to_lowercase();
            for vendor in VENDOR_LITERALS {
                if lower.contains(vendor) {
                    let line = lower
                        .lines()
                        .position(|l| l.contains(vendor))
                        .map(|n| n + 1)
                        .unwrap_or(0);
                    report.fail(format!("{path}:{line} — the literal `{vendor}`"));
                }
            }
        }
    }
    report
}

// -------------------------------------------------------------- rule seven

/// Thresholds for a `CLAUDE.md`. Far below the general source ceiling, because
/// these files are read by an agent at the start of every task rather than by a
/// person looking something up.
const CLAUDE_MD_FAIL: usize = 50;
const CLAUDE_MD_WARN: usize = 30;

/// No `CLAUDE.md` over fifty lines.
///
/// **A CLAUDE.md routes; it does not explain.** Anything longer than a pointer
/// belongs in a practice doc, a skill, or Notion — one of which is already the
/// authority on it, so a copy in a CLAUDE.md can only drift downward while
/// being read more often than the original.
///
/// v1's single agent file reached 328 lines by accretion, one reasonable-looking
/// paragraph at a time. No individual addition was wrong, which is exactly why a
/// ceiling is the only thing that stops it.
pub fn no_bloated_claude_md(root: &Path) -> Report {
    let mut report = Report::new("no CLAUDE.md over 50 lines, warn at 30");
    let mut found = Vec::new();
    walk(root, &mut |path| {
        if path.file_name().and_then(|n| n.to_str()) == Some("CLAUDE.md") {
            found.push(path.to_path_buf());
        }
    });
    found.sort();
    for path in found {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let lines = text.lines().count();
        if lines > CLAUDE_MD_FAIL {
            report.fail(format!(
                "{rel} is {lines} lines, over {CLAUDE_MD_FAIL} — move the explanation, keep the pointer"
            ));
        } else if lines > CLAUDE_MD_WARN {
            report.warn(format!("{rel} is {lines} lines, over {CLAUDE_MD_WARN}"));
        }
    }
    report
}
