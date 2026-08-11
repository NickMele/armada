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
//!
//! **The source repo's own name is not in the committed pattern.** charkit is
//! public, and a grep that names the private repo it was built away from
//! publishes the one string it exists to keep out — while handing every clone
//! an alternative that is nobody else's. So the name is configuration, the same
//! way the clean-room hook's guarded path is (§2.7), and it is *appended* to
//! §2.4's pattern rather than replacing any of it.

use crate::docs::{blocks, Doc, Finding};
use regex::{Regex, RegexBuilder};
use std::path::Path;

/// Extra alternatives, `|`-separated. Exported wins over the file, and
/// exported-empty is a deliberate off switch — the same precedence the
/// clean-room hook uses, so one habit covers both.
const EXTRA_ENV: &str = "CHARKIT_CONTAMINATION_EXTRA";

/// The same alternatives, one per line, `#` comments and blanks skipped.
/// Untracked (see `.gitignore`). Unlike the variable it survives a `cargo
/// xtask` run from a shell that never exported anything, which is every run.
const EXTRA_FILE: &str = ".claude/contamination.local";

/// Directories the grep covers. Absent ones are not an error: `crates/` does
/// not exist until phase 1, and a check that fails on a repo mid-build would
/// just get disabled.
const ROOTS: &[&str] = &["crates", "tests"];

/// Exempt, per §2.4.
const EXEMPT: &[&str] = &["tests/fixtures"];

pub fn check(root: &Path, corpus: &[Doc]) -> Result<Vec<Finding>, String> {
    let pattern = pattern(root, corpus, std::env::var(EXTRA_ENV).ok())?;
    scan(root, &pattern)
}

/// The walk itself, split from `check` so the tests can drive it with a pattern
/// they assembled rather than by mutating the process environment.
fn scan(root: &Path, pattern: &str) -> Result<Vec<Finding>, String> {
    let re = RegexBuilder::new(pattern)
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

/// §2.4's pattern, plus whatever the configured alternatives add.
///
/// `from_env` is threaded in rather than read here so the tests can exercise
/// both sources without mutating the process environment.
fn pattern(root: &Path, corpus: &[Doc], from_env: Option<String>) -> Result<String, String> {
    let base = extract_pattern(corpus)?;
    let extra = extra_alternatives(root, from_env);
    if extra.is_empty() {
        return Ok(base);
    }
    // Escaped, not spliced in raw: a configured value is the *name* of a repo,
    // not a pattern its author debugged. An unescaped `.` or `(` would silently
    // widen the grep, or fail to compile and take §2.4's own alternatives down
    // with it.
    let escaped: Vec<String> = extra.iter().map(|a| regex::escape(a)).collect();
    Ok(format!("{base}|{}", escaped.join("|")))
}

/// The configured alternatives, from the variable if it is exported at all and
/// from the file otherwise.
fn extra_alternatives(root: &Path, from_env: Option<String>) -> Vec<String> {
    let raw = match from_env {
        // Exported-empty yields no alternatives, which is the off switch.
        Some(v) => v.split('|').map(str::to_string).collect(),
        None => match std::fs::read_to_string(root.join(EXTRA_FILE)) {
            Ok(text) => text
                .lines()
                .filter(|l| !l.trim_start().starts_with('#'))
                .map(str::to_string)
                .collect(),
            // Unreadable and absent are the same answer: nothing was named, so
            // §2.4's pattern runs on its own. Failing here would break the gate
            // in every clone that has no private repo to keep out.
            Err(_) => Vec::new(),
        },
    };
    raw.iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect()
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

/// §2.4 requires two assertions of this check, and both are the kind that only
/// fail silently: that the pattern still *matches* — a grep that cannot match
/// is indistinguishable from a clean repository — and that the configured
/// alternatives are really appended, which a machine that has configured none
/// cannot tell apart from a no-op.
///
/// Every banned string here is either split across a `concat!` or invented on
/// the spot, per §2.4: written as a plain literal, the self-test trips the very
/// gate it is testing. `xtask/` is outside the grep's roots today, and this
/// stays honest about it anyway — the roots are one edit away from including
/// everything.
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A stand-in for whatever private repo an operator configures. Invented,
    /// so asserting it gets caught proves the append and publishes nothing.
    const INVENTED: &str = "source-under-glass";

    /// §2.4's own text, reduced to the one thing `extract_pattern` reads.
    fn corpus() -> Vec<Doc> {
        vec![Doc {
            name: "ARCHITECTURE.md",
            rel: "docs/ARCHITECTURE.md",
            text: format!(
                "## 2.4\n\n```sh\ngrep -riE \"tilt|{}|\\\\.claude|backend/|web/\" \\\n     crates/ tests/\n```\n",
                concat!("NEXT_", "PUBLIC")
            ),
        }]
    }

    /// Unique per test and per run — `cargo test` runs these threaded, and a
    /// leftover directory from a panicked run must not be adopted by the next.
    fn scratch(label: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "charkit-contamination-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("crates/core/src")).expect("scratch crates dir");
        std::fs::write(dir.join("crates/core/src/lib.rs"), contents).expect("scratch source");
        dir
    }

    fn config(root: &Path, contents: &str) {
        std::fs::create_dir_all(root.join(".claude")).expect("scratch .claude");
        std::fs::write(root.join(EXTRA_FILE), contents).expect("scratch config");
    }

    fn findings(root: &Path, from_env: Option<&str>) -> Vec<Finding> {
        let pattern = pattern(root, &corpus(), from_env.map(str::to_string)).expect("a pattern");
        scan(root, &pattern).expect("the walk runs")
    }

    #[test]
    fn the_committed_pattern_still_matches_a_known_bad_string() {
        let root = scratch(
            "known-bad",
            &format!("const K: &str = \"{}_API\";\n", concat!("NEXT_", "PUBLIC")),
        );
        assert_eq!(findings(&root, Some("")).len(), 1);
    }

    #[test]
    fn unconfigured_leaves_the_committed_pattern_exactly_as_2_4_states_it() {
        let root = scratch("unconfigured", "fn main() {}\n");
        let base = extract_pattern(&corpus()).expect("a pattern");
        assert_eq!(pattern(&root, &corpus(), None).unwrap(), base);
        assert_eq!(
            pattern(&root, &corpus(), Some(String::new())).unwrap(),
            base
        );
    }

    #[test]
    fn a_configured_alternative_is_appended_and_caught() {
        let root = scratch("appended", &format!("use {INVENTED}::check;\n"));
        assert!(findings(&root, None).is_empty(), "not caught unconfigured");
        assert_eq!(findings(&root, Some(INVENTED)).len(), 1);
    }

    #[test]
    fn the_local_file_arms_the_alternatives_when_nothing_is_exported() {
        let root = scratch("file", &format!("// path: ~/dev/{INVENTED}/check.py\n"));
        config(
            &root,
            &format!("# the repo this one was built away from\n\n{INVENTED}\n"),
        );
        assert_eq!(findings(&root, None).len(), 1);
    }

    #[test]
    fn the_environment_wins_over_the_file() {
        let root = scratch("env-wins", &format!("use {INVENTED}::check;\n"));
        config(&root, "some-other-repo\n");
        assert_eq!(findings(&root, Some(INVENTED)).len(), 1);
        // And the file's own value is not additionally in play.
        let root = scratch("env-wins-other", "use some_other_repo::check;\n");
        config(&root, "some-other-repo\n");
        assert!(findings(&root, Some(INVENTED)).is_empty());
    }

    #[test]
    fn exported_empty_is_an_off_switch_a_file_cannot_re_arm() {
        let root = scratch("off-switch", &format!("use {INVENTED}::check;\n"));
        config(&root, &format!("{INVENTED}\n"));
        assert!(findings(&root, Some("")).is_empty());
        // Unset, though, and the file on disk still arms it.
        assert_eq!(findings(&root, None).len(), 1);
    }

    #[test]
    fn several_alternatives_are_allowed_from_either_source() {
        let root = scratch("several", "use acme::x;\nuse widgets::y;\n");
        assert_eq!(findings(&root, Some("acme|widgets")).len(), 2);
        let root = scratch("several-file", "use acme::x;\nuse widgets::y;\n");
        config(&root, "  acme  \n\n# a comment\nwidgets\n");
        assert_eq!(findings(&root, None).len(), 2);
    }

    #[test]
    fn a_configured_alternative_is_matched_literally() {
        // `a.c` is a name, not a pattern: unescaped it would also catch `abc`,
        // which is a false positive nobody can silence — there is no allowlist.
        let root = scratch("literal", "use abc::x;\n");
        assert!(findings(&root, Some("a.c")).is_empty());
        let root = scratch("literal-hit", "use a.c::x;\n");
        assert_eq!(findings(&root, Some("a.c")).len(), 1);
    }

    #[test]
    fn a_configured_alternative_matches_case_insensitively() {
        let root = scratch("case", &format!("// {}\n", INVENTED.to_uppercase()));
        assert_eq!(findings(&root, Some(INVENTED)).len(), 1);
    }
}
