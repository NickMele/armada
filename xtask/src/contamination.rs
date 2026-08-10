//! The contamination grep, run from its single source of truth.
//!
//! `ARCHITECTURE.md` §2.4 states the pattern **and is the only place it is
//! stated** — `AGENTS.md` and `README.md` deliberately describe the rule and
//! point at §2.4 rather than repeating it, because a copy in a markdown table
//! is both unrunnable and a second thing to keep in sync. A hardcoded pattern
//! here would be exactly that second copy, so this reads it out of the
//! document instead. If §2.4 changes, this changes with it or fails loudly.
//!
//! Two behaviours worth knowing, both from §2.4's own text: there is **no
//! allowlist** — if it fires, the code changes and not the pattern — and
//! `tests/fixtures/` is exempt, because a fixture config describing a
//! hypothetical repo may legitimately name that repo's directories.

use crate::docs::{blocks, Doc, Finding};
use regex::{Regex, RegexBuilder};
use std::path::Path;

/// Directories the grep covers. Absent ones are not an error: `crates/` does
/// not exist until phase 1, and a check that fails on a repo mid-build would
/// just get disabled.
const ROOTS: &[&str] = &["crates", "tests"];

/// Exempt, per §2.4.
const EXEMPT: &[&str] = &["tests/fixtures"];

pub fn check(root: &Path, corpus: &[Doc]) -> Result<Vec<Finding>, String> {
    let pattern = extract_pattern(corpus)?;
    let re = RegexBuilder::new(&pattern)
        .case_insensitive(true)
        .build()
        .map_err(|e| format!("pattern from ARCHITECTURE.md §2.4 does not compile: {e}"))?;

    let mut findings = Vec::new();
    for dir in ROOTS {
        let base = root.join(dir);
        if !base.exists() {
            continue;
        }
        walk(&base, root, &re, &mut findings)?;
    }
    Ok(findings)
}

/// Pull the alternation out of the `grep -riE "..."` in §2.4's fenced block.
fn extract_pattern(corpus: &[Doc]) -> Result<String, String> {
    let arch = corpus
        .iter()
        .find(|d| d.name == "ARCHITECTURE.md")
        .ok_or("ARCHITECTURE.md not in the corpus")?;

    let grep = Regex::new(r#"grep\s+-[a-zA-Z]*E[a-zA-Z]*\s+"([^"]+)""#).unwrap();
    for b in blocks(&arch.text) {
        if !matches!(b.lang.as_str(), "sh" | "bash" | "shell") {
            continue;
        }
        if let Some(c) = grep.captures(&b.body) {
            return Ok(c[1].to_string());
        }
    }
    Err(
        "no `grep -riE \"...\"` found in a fenced sh block — §2.4 is the only \
         place the pattern may live, so this check cannot run without it"
            .into(),
    )
}

fn walk(dir: &Path, root: &Path, re: &Regex, out: &mut Vec<Finding>) -> Result<(), String> {
    let rel_of = |p: &Path| {
        p.strip_prefix(root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    };

    if EXEMPT.iter().any(|e| rel_of(dir) == *e) {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name == "target" || name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(&path, root, re, out)?;
            continue;
        }

        // Binary files have no business matching a source-contamination grep,
        // and reading them as UTF-8 would just produce noise.
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if let Some(m) = re.find(line) {
                out.push(Finding {
                    file: rel_of(&path),
                    line: n + 1,
                    message: format!(
                        "contamination: `{}` — ARCHITECTURE.md §2.4 has no allowlist, \
                         so the code changes, not the pattern",
                        m.as_str()
                    ),
                });
            }
        }
    }
    Ok(())
}
