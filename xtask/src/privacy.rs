//! The privacy gate: nothing committed here names the source repo or anyone's
//! machine.
//!
//! `ARCHITECTURE.md` §2.4's contamination grep covers `crates/` and `tests/`,
//! because the leak it was designed for is *transcription into code* during
//! phase 3. But this repository is public and its documentation is the larger
//! surface: every reference to the source repo that had to be scrubbed lived in
//! `docs/`, `AGENTS.md` and `README.md`, and not one of them was in the grep's
//! roots. A gate that guards the smaller half of the surface is a gate you stop
//! reading, so this covers the other half.
//!
//! **Two rules, both mechanical, neither with an allowlist.**
//!
//! 1. *The configured private names.* Read from the same two sources §2.4's
//!    pattern extends from — one name configured once arms both checks. Nothing
//!    configured means this rule finds nothing, exactly as the grep runs its
//!    five public alternatives on their own.
//! 2. *This machine's home directory.* `$HOME` is read at run time and its
//!    literal value may not appear in any tracked file. It needs no
//!    configuration, it is different on every machine and on CI, and it is
//!    precisely the string that makes a path a *local* path — `~/.char/char.db`
//!    is documentation, `/Users/someone/Development/...` is a leak.
//!
//! **Why `$HOME` rather than the shape of a home path.** `/Users/<name>/` and
//! `/home/<name>/` are ordinary things for a unit test to construct —
//! `crates/adapters` builds `/home/agent/.char` to assert a join, and that is
//! the test doing its job. Banning the shape means an allowlist of blessed
//! pretend usernames that grows every time someone writes a test, which §2.4
//! rejects for the grep and is no better here. Matching the running machine's
//! own home has no false positives to allowlist: a path that is not yours
//! cannot identify you.
//!
//! **What it therefore does not catch:** a *collaborator's* home path, and the
//! source repo's name in a checkout that has configured none. Both are the same
//! accepted trade as §2.4's — a public repo cannot state the strings it exists
//! to keep out, so the operator who has them states them locally.

use crate::contamination::{extra_alternatives, EXTRA_ENV};
use crate::docs::Finding;
use regex::{Regex, RegexBuilder};
use std::path::Path;
use std::process::Command;

/// Exempt from rule 1 only, per `ARCHITECTURE.md` §2.4: the harvest doc's whole
/// job is to describe the source repo, so a ban would forbid recording the
/// assumptions the implementer is meant to strip. Rule 2 still applies to it —
/// no document has a reason to carry the path of the machine that wrote it.
const EXEMPT: &[&str] = &["docs/harvest.md"];

/// A home directory this short is a container's or a misconfiguration's, not a
/// person's, and greping every tracked file for `/` would report the repository.
const MIN_HOME: usize = 2;

pub fn check(root: &Path) -> Result<Vec<Finding>, String> {
    let names = extra_alternatives(root, std::env::var(EXTRA_ENV).ok());
    let home = std::env::var("HOME").ok();
    scan(root, &tracked(root)?, &names, home.as_deref())
}

/// Every file git would publish, which is the definition this check wants.
///
/// A walk of the working tree would have to reimplement `.gitignore` to avoid
/// reporting `target/` and the untracked `.claude/*.local` files — whose entire
/// purpose is to hold the strings this check hunts for. Asking git removes that
/// class of bug: what is not tracked is not published, so it is not a leak.
fn tracked(root: &Path) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| format!("could not run `git ls-files`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`git ls-files` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

/// The scan itself, split from `check` so the tests can supply a file list, a
/// set of names and a home directory rather than mutating the process
/// environment — `cargo test` runs threaded, where that is unsound.
fn scan(
    root: &Path,
    files: &[String],
    names: &[String],
    home: Option<&str>,
) -> Result<Vec<Finding>, String> {
    let named = build_names(names)?;
    let local = home
        .map(|h| h.trim_end_matches('/'))
        .filter(|h| h.len() >= MIN_HOME);

    let mut findings = Vec::new();
    for rel in files {
        let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if let Some(home) = local {
                if let Some(at) = line.find(home) {
                    findings.push(Finding {
                        file: rel.clone(),
                        line: n + 1,
                        message: format!(
                            "local-machine path: `{}` is this machine's home directory — \
                             write the path relative to the repo, or as `~/`",
                            &line[at..at + home.len()]
                        ),
                    });
                }
            }
            if EXEMPT.contains(&rel.as_str()) {
                continue;
            }
            if let Some(re) = &named {
                if let Some(m) = re.find(line) {
                    findings.push(Finding {
                        file: rel.clone(),
                        line: n + 1,
                        message: format!(
                            "names a configured private repo: `{}` — this repository is \
                             public, so the name is configuration (ARCHITECTURE.md §2.4)",
                            m.as_str()
                        ),
                    });
                }
            }
        }
    }
    Ok(findings)
}

/// The configured names as one case-insensitive alternation, escaped for the
/// same reason §2.4 escapes them: a configured value is the *name* of a repo,
/// not a pattern its author debugged.
fn build_names(names: &[String]) -> Result<Option<Regex>, String> {
    if names.is_empty() {
        return Ok(None);
    }
    let alts: Vec<String> = names.iter().map(|n| regex::escape(n)).collect();
    RegexBuilder::new(&alts.join("|"))
        .case_insensitive(true)
        .build()
        .map(Some)
        .map_err(|e| format!("the configured private names do not compile: {e}"))
}

/// §2.4's self-test discipline applies here too, and for the same reason: a
/// check that cannot match is indistinguishable from a clean repository, and a
/// machine that has configured no private name cannot tell a working rule from
/// a no-op. Every string these tests hunt for is invented on the spot and
/// written to a temporary directory, never committed as a source literal.
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A stand-in for whatever private repo an operator configures.
    const INVENTED: &str = "source-under-glass";

    /// Unique per test and per run — `cargo test` runs these threaded, and a
    /// leftover directory from a panicked run must not be adopted by the next.
    fn scratch(label: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<String>) {
        let dir =
            std::env::temp_dir().join(format!("charkit-privacy-{label}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        for (rel, body) in files {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("scratch dir");
            std::fs::write(&path, body).expect("scratch file");
        }
        (dir, files.iter().map(|(rel, _)| rel.to_string()).collect())
    }

    /// A home directory belonging to nobody, built at run time.
    fn invented_home(label: &str) -> String {
        format!("/Users/{label}-{}", std::process::id())
    }

    fn findings(root: &Path, files: &[String], names: &[&str], home: Option<&str>) -> Vec<Finding> {
        let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        scan(root, files, &names, home).expect("the scan runs")
    }

    #[test]
    fn this_machines_home_directory_is_caught_anywhere_in_the_tree() {
        let home = invented_home("home");
        let (root, files) = scratch(
            "home",
            &[("docs/PLAN.md", &format!("Run it from {home}/dev/thing.\n"))],
        );
        assert_eq!(findings(&root, &files, &[], Some(&home)).len(), 1);
    }

    #[test]
    fn a_home_directory_that_is_not_this_machines_is_left_alone() {
        // What `crates/adapters` does today: construct a pretend home to assert
        // a path join. Banning the shape would make that a finding.
        let (root, files) = scratch(
            "other-home",
            &[("crates/adapters/src/machine.rs", "join(\"/home/agent\")\n")],
        );
        let home = invented_home("other-home");
        assert!(findings(&root, &files, &[], Some(&home)).is_empty());
    }

    #[test]
    fn a_trailing_slash_on_home_does_not_change_the_answer() {
        let home = invented_home("slash");
        let (root, files) = scratch("slash", &[("README.md", &format!("cd {home}/repo\n"))]);
        assert_eq!(
            findings(&root, &files, &[], Some(&format!("{home}/"))).len(),
            1
        );
    }

    #[test]
    fn an_unset_or_degenerate_home_disarms_the_path_rule_rather_than_matching_everything() {
        let (root, files) = scratch("degenerate", &[("README.md", "/anything at all\n")]);
        assert!(findings(&root, &files, &[], None).is_empty());
        assert!(findings(&root, &files, &[], Some("/")).is_empty());
        assert!(findings(&root, &files, &[], Some("")).is_empty());
    }

    #[test]
    fn a_configured_name_is_caught_in_the_documents_the_grep_never_reaches() {
        let (root, files) = scratch(
            "docs",
            &[("docs/PHASES.md", &format!("Ported out of {INVENTED}.\n"))],
        );
        assert!(
            findings(&root, &files, &[], None).is_empty(),
            "not caught unconfigured"
        );
        assert_eq!(findings(&root, &files, &[INVENTED], None).len(), 1);
    }

    #[test]
    fn the_harvest_doc_is_exempt_from_the_name_rule_and_not_from_the_path_rule() {
        let home = invented_home("harvest");
        let (root, files) = scratch(
            "harvest",
            &[(
                "docs/harvest.md",
                &format!("{INVENTED} assumes `uv run`, checked at {home}/x.\n"),
            )],
        );
        let found = findings(&root, &files, &[INVENTED], Some(&home));
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("local-machine path"));
    }

    #[test]
    fn the_names_come_from_contaminations_own_two_sources() {
        let (root, files) = scratch(
            "sources",
            &[("AGENTS.md", &format!("adopted by {INVENTED}\n"))],
        );
        std::fs::create_dir_all(root.join(".claude")).expect("scratch .claude");
        std::fs::write(
            root.join(".claude/contamination.local"),
            format!("# the repo this one was built away from\n{INVENTED}\n"),
        )
        .expect("scratch config");

        let from_file = extra_alternatives(&root, None);
        assert_eq!(scan(&root, &files, &from_file, None).unwrap().len(), 1);

        // Exported wins, including exported-empty as the off switch — the same
        // precedence contamination and the clean-room hook use.
        let from_env = extra_alternatives(&root, Some(String::new()));
        assert!(scan(&root, &files, &from_env, None).unwrap().is_empty());
    }

    #[test]
    fn a_configured_name_is_matched_literally_and_case_insensitively() {
        let (root, files) = scratch("literal", &[("docs/PLAN.md", "use abc::x\n")]);
        assert!(findings(&root, &files, &["a.c"], None).is_empty());

        let (root, files) = scratch(
            "case",
            &[("docs/PLAN.md", &format!("{}\n", INVENTED.to_uppercase()))],
        );
        assert_eq!(findings(&root, &files, &[INVENTED], None).len(), 1);
    }

    #[test]
    fn a_file_git_lists_but_cannot_be_read_as_text_is_skipped_rather_than_failing() {
        let (root, mut files) = scratch("binary", &[("docs/PLAN.md", "clean\n")]);
        std::fs::write(root.join("logo.png"), [0xff, 0xfe, 0x00, 0x9f]).expect("scratch binary");
        files.push("logo.png".into());
        files.push("gone.md".into());
        assert!(findings(&root, &files, &[INVENTED], None).is_empty());
    }
}
