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
//! 1. *The configured private names*, in a file's contents **and in its path**.
//!    Read from the same two sources §2.4's pattern extends from — one name
//!    configured once arms both checks. Nothing configured means this rule
//!    finds nothing, exactly as the grep runs its five public alternatives on
//!    their own.
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
//!
//! [`history`] applies the same two rules to what git **publishes** rather than
//! to what is checked out. See its own documentation for why that is a separate
//! command and not part of the gate.

use crate::docs::Finding;
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Extra alternatives, `|`-separated. Exported wins over the file, and
/// exported-empty is a deliberate off switch.
///
/// Moved here from the contamination grep when that check retired: this is now
/// the only guard that reads it, and a constant living in a deleted module is
/// how a config surface quietly loses its owner.
pub const EXTRA_ENV: &str = "CHARKIT_CONTAMINATION_EXTRA";

/// The same alternatives, one per line, `#` comments and blanks skipped.
/// Untracked (see `.gitignore`). Unlike the variable it survives a `cargo
/// xtask` run from a shell that never exported anything, which is every run.
///
/// **The filename keeps the old spelling on purpose.** It is gitignored and
/// already sitting on every machine that has one; renaming it would disarm the
/// gate on exactly those machines, silently, which is the failure
/// [`unconfigured_finding`] exists to make impossible.
const EXTRA_FILE: &str = ".claude/contamination.local";

/// The configured private names, from the variable if it is exported at all
/// and from the file otherwise.
///
/// `from_env` is threaded in rather than read here so the tests can exercise
/// both sources without mutating the process environment.
pub fn extra_alternatives(root: &Path, from_env: Option<String>) -> Vec<String> {
    let raw = match from_env {
        // Exported-empty yields no alternatives, which is the off switch.
        Some(v) => v.split('|').map(str::to_string).collect(),
        None => match std::fs::read_to_string(root.join(EXTRA_FILE)) {
            Ok(text) => text
                .lines()
                .filter(|l| !l.trim_start().starts_with('#'))
                .map(str::to_string)
                .collect(),
            // Unreadable and absent are the same answer: nothing was named. The
            // run is then reported as unconfigured and fails loudly rather than
            // passing quietly — see `unconfigured_finding`.
            Err(_) => Vec::new(),
        },
    };
    raw.iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect()
}

/// Exempt from rule 1 only, per `ARCHITECTURE.md` §2.4: the harvest doc's whole
/// job is to describe the source repo, so a ban would forbid recording the
/// assumptions the implementer is meant to strip. Rule 2 still applies to it —
/// no document has a reason to carry the path of the machine that wrote it.
const EXEMPT: &[&str] = &["docs/harvest.md"];

/// A home directory this short is a container's or a misconfiguration's, not a
/// person's, and greping every tracked file for `/` would report the repository.
const MIN_HOME: usize = 2;

/// Whether rule 1 has anything at all to look for.
///
/// §2.4's discipline — *a check that cannot match is indistinguishable from a
/// clean repository* — applies to what the gate **says** as much as to what it
/// does. On a checkout that has configured no name, half of this gate is a
/// no-op, and a bare `clean — …, privacy` reads as though both rules passed.
/// The caller labels the run instead, so the disarmed state is visible in the
/// one line anybody actually looks at.
pub fn name_rule_armed(root: &Path) -> bool {
    !extra_alternatives(root, std::env::var(EXTRA_ENV).ok()).is_empty()
}

/// Opts a checkout out of [`unconfigured_finding`], for someone who genuinely
/// cannot arm the rule.
///
/// It has to be an explicit act. The whole failure this guards against is a
/// disarmed gate that looks armed, and an opt-out that can be reached by
/// accident recreates it.
pub const UNCONFIGURED_OK_ENV: &str = "CHARKIT_PRIVACY_UNCONFIGURED_OK";

/// A finding when the name rule cannot run, unless it was opted out of.
///
/// **The label was not enough.** Suffixing the summary with `(name rule
/// unconfigured)` made the disarmed state *visible*, but it still exited `0`,
/// and an exit code is what a script, a merge gate and an agent in a hurry
/// actually read. This repository is public permanently now, so the gate is a
/// standing check rather than a transitional one, and a standing check that
/// silently passes when it is switched off is worse than no check at all — it
/// converts "nobody verified this" into "verified clean".
///
/// Non-zero by default; `CHARKIT_PRIVACY_UNCONFIGURED_OK=1` for an outside
/// contributor who has neither the local file nor the repository secret and is
/// not in a position to get either.
pub fn unconfigured_finding(root: &Path) -> Option<Finding> {
    unconfigured(
        name_rule_armed(root),
        std::env::var(UNCONFIGURED_OK_ENV).is_ok(),
    )
}

/// The decision, with the environment already read — so it is testable without
/// mutating process-global state from a parallel test run.
fn unconfigured(armed: bool, acknowledged: bool) -> Option<Finding> {
    if armed || acknowledged {
        return None;
    }
    Some(Finding {
        file: ".claude/contamination.local".into(),
        line: 0,
        message: format!(
            "the private-name rule is UNCONFIGURED, so this gate checked nothing for it — \
             that is the guard being off, not passing. Arm it with a private repo name in \
             `.claude/contamination.local`, or the `{EXTRA_ENV}` secret on CI. If you cannot \
             have either, set `{UNCONFIGURED_OK_ENV}=1` to acknowledge it deliberately \
             (ARCHITECTURE.md §2.4)"
        ),
    })
}

pub fn check(root: &Path) -> Result<Vec<Finding>, String> {
    let names = extra_alternatives(root, std::env::var(EXTRA_ENV).ok());
    let home = std::env::var("HOME").ok();
    let mut findings = scan(root, &tracked(root)?, &names, home.as_deref())?;
    findings.extend(unconfigured_finding(root));
    Ok(findings)
}

/// This machine's home, normalised, or nothing if it is too short to identify
/// anybody — see [`MIN_HOME`].
fn local_home(home: Option<&str>) -> Option<&str> {
    home.map(|h| h.trim_end_matches('/'))
        .filter(|h| h.len() >= MIN_HOME)
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
    let local = local_home(home);

    let mut findings = Vec::new();
    for rel in files {
        // The path before the contents. A document named after the source repo
        // leaks it in the file listing, in every `git log --stat` and in the
        // GitHub tree, whether or not a single line inside it says the word —
        // and a file whose *name* is the leak is one nobody thinks to grep.
        // Line 0 because there is no line: the finding is the path itself.
        if !EXEMPT.contains(&rel.as_str()) {
            if let Some(re) = &named {
                if let Some(m) = re.find(rel) {
                    findings.push(Finding {
                        file: rel.clone(),
                        line: 0,
                        message: format!(
                            "the file's own path names a configured private repo: `{}` — \
                             rename the file (ARCHITECTURE.md §2.4)",
                            m.as_str()
                        ),
                    });
                }
            }
        }

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

/// How a set of banned strings is named in a report that must not print them.
const HISTORY: &str = "(reachable commits)";

/// The same two rules, over what git **publishes** rather than what is checked
/// out: every ref, and every commit reachable from one.
///
/// Scrubbing the working tree changes nothing anybody can see. `origin/main`,
/// every other pushed branch and every tag still serve the old files, and every
/// commit that ever carried the string is still reachable from them and still
/// rendered by GitHub. This turns that remaining exposure into a count and a
/// list of refs, so the decision to rewrite is made against a measurement.
///
/// **Why this is not in `doclint`.** The only fixes are a history rewrite and a
/// force-push: they are the operator's call, they are destructive, and they
/// cannot be performed by a merge gate. A gate that fails for a condition the
/// contributor cannot act on is a gate that gets switched off — and every
/// commit already merged would fail it forever, including the ones that did the
/// scrubbing. So it reports, on request, and the gate keeps guarding the only
/// thing a PR can still change: the tree.
/// `history` needs the loud failure more than `check` does, not less: its
/// `$HOME` rule finds nothing in a repository whose commits were all written
/// elsewhere, so an unconfigured run is *expected* to be silent — and silence
/// on exactly the surface the operator asked about reads as an all-clear.
pub fn history(root: &Path) -> Result<Vec<Finding>, String> {
    let mut findings = scan_history(root, &needles(root))?;
    findings.extend(unconfigured_finding(root));
    Ok(findings)
}

/// Every banned literal as one list. The two rules differ in the working tree —
/// `docs/harvest.md` is exempt from one of them — but a ref that publishes
/// either is equally published, so history draws no distinction.
fn needles(root: &Path) -> Vec<String> {
    let mut needles = extra_alternatives(root, std::env::var(EXTRA_ENV).ok());
    let home = std::env::var("HOME").ok();
    if let Some(home) = local_home(home.as_deref()) {
        needles.push(home.to_string());
    }
    needles
}

/// Split from [`history`] so the tests can supply their own banned strings and
/// their own throwaway repository, for the same reason [`scan`] is split.
fn scan_history(root: &Path, needles: &[String]) -> Result<Vec<Finding>, String> {
    let refs = refs(root)?;
    if needles.is_empty() || refs.is_empty() {
        return Ok(Vec::new());
    }

    let mut findings = Vec::new();
    for name in &refs {
        let files = tip_hits(root, name, needles)?;
        if !files.is_empty() {
            findings.push(Finding {
                file: name.clone(),
                line: 0,
                message: format!(
                    "this ref publishes a banned string in {} tracked file(s) ({}) — a clean \
                     working tree does not clean a ref (ARCHITECTURE.md §2.4)",
                    files.len(),
                    sample(&files)
                ),
            });
        }
    }

    let touching = commits_touching(root, needles)?;
    if !touching.is_empty() {
        findings.push(Finding {
            file: HISTORY.to_string(),
            line: 0,
            message: format!(
                "{} commit(s) add or remove a banned string in their diff ({}) — every one is \
                 still served by GitHub, and only a rewrite removes them (ARCHITECTURE.md §2.4)",
                touching.len(),
                sample(&shorten(&touching))
            ),
        });
    }

    let naming = commits_naming(root, needles)?;
    if !naming.is_empty() {
        findings.push(Finding {
            file: HISTORY.to_string(),
            line: 0,
            message: format!(
                "{} commit message(s) name a banned string ({}) — messages survive a rewrite \
                 that only rewrites trees (ARCHITECTURE.md §2.4)",
                naming.len(),
                sample(&shorten(&naming))
            ),
        });
    }

    Ok(findings)
}

/// Every ref a clone would receive, which is the definition of published here —
/// branches, remote-tracking branches and tags alike. A branch nobody pushed is
/// still listed, because the leak it carries is one `git push` away and the
/// operator is the one who decides whether it matters.
fn refs(root: &Path) -> Result<Vec<String>, String> {
    Ok(git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
    )?
    .lines()
    // `refs/remotes/<remote>/HEAD` is a symbolic ref to a branch already in
    // this list, so counting it doubles one ref and informs nobody.
    .filter(|r| !r.ends_with("/HEAD"))
    .map(str::to_string)
    .collect())
}

/// The files at a ref's tip that carry a banned string, by contents or by path
/// — the same pair of surfaces [`scan`] covers in the working tree.
fn tip_hits(root: &Path, name: &str, needles: &[String]) -> Result<Vec<String>, String> {
    let mut args = vec!["grep", "--no-color", "-I", "-l", "-i", "-F"];
    for needle in needles {
        args.push("-e");
        args.push(needle);
    }
    args.push(name);

    let prefix = format!("{name}:");
    let mut hits: Vec<String> = git(root, &args)?
        .lines()
        .filter_map(|l| l.strip_prefix(&prefix))
        .map(str::to_string)
        .collect();

    let lowered: Vec<String> = needles.iter().map(|n| n.to_lowercase()).collect();
    for path in git(root, &["ls-tree", "-r", "--name-only", name])?.lines() {
        let lower = path.to_lowercase();
        if lowered.iter().any(|n| lower.contains(n.as_str())) {
            hits.push(path.to_string());
        }
    }

    hits.sort();
    hits.dedup();
    Ok(hits)
}

/// Commits whose diff adds or removes a banned string anywhere in the reachable
/// graph — the ones a tip-only scan cannot see, because the string was removed
/// by a later commit that left the leak fully intact one click away.
fn commits_touching(root: &Path, needles: &[String]) -> Result<Vec<String>, String> {
    let mut shas = Vec::new();
    for needle in needles {
        let pickaxe = format!("-S{}", ere_literal(needle));
        shas.extend(
            git(
                root,
                &[
                    "log",
                    "--all",
                    "--format=%H",
                    "-i",
                    "--pickaxe-regex",
                    &pickaxe,
                ],
            )?
            .lines()
            .map(str::to_string),
        );
    }
    Ok(unique(shas))
}

/// Commits whose *message* names one. Worth its own rule because the obvious
/// rewrite — replay the trees with the string substituted — does not touch a
/// message, so this is the half that survives a fix that looked complete.
fn commits_naming(root: &Path, needles: &[String]) -> Result<Vec<String>, String> {
    let mut shas = Vec::new();
    for needle in needles {
        let grep = format!("--grep={needle}");
        shas.extend(
            git(root, &["log", "--all", "--format=%H", "-i", "-F", &grep])?
                .lines()
                .map(str::to_string),
        );
    }
    Ok(unique(shas))
}

/// `git`, with "found nothing" treated as an answer rather than a failure.
///
/// `git grep` and `git log --grep` exit 1 when they match nothing, which is the
/// result this wants most of the time; anything above that is a real error and
/// is reported as one rather than read as a clean repository.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("could not run `git {}`: {e}", args.join(" ")))?;
    match out.status.code() {
        Some(0 | 1) => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => Err(format!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )),
    }
}

/// A literal, spelled so that POSIX ERE can read none of it as syntax.
///
/// `--pickaxe-regex` is what makes `-S` case-insensitive, and case matters: a
/// name written three ways across a history is three leaks. But a configured
/// value is the *name of a repo*, not a pattern its author debugged, so a `.`
/// in it has to match a dot and nothing else. A bracket expression is the one
/// ERE construct in which an otherwise-special character stands for itself; the
/// three that cannot go inside one take a backslash instead.
fn ere_literal(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            c if c.is_ascii_alphanumeric() || c == '_' => c.to_string(),
            ']' | '^' | '\\' => format!("\\{c}"),
            c => format!("[{c}]"),
        })
        .collect()
}

/// Deduplicated, in the order git reported them — which is newest first, so the
/// sample in a finding is the most recent commits rather than an arbitrary set.
fn unique(shas: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for sha in shas {
        if seen.insert(sha.clone()) {
            out.push(sha);
        }
    }
    out
}

fn shorten(shas: &[String]) -> Vec<String> {
    shas.iter()
        .map(|s| s[..s.len().min(8)].to_string())
        .collect()
}

/// Enough of a list to recognise it, and never the whole thing: these reports
/// run in terminals and, once the name is a CI secret, in logs.
fn sample(items: &[String]) -> String {
    const SHOWN: usize = 3;
    let head = items
        .iter()
        .take(SHOWN)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if items.len() > SHOWN {
        format!("{head}, … and {} more", items.len() - SHOWN)
    } else {
        head
    }
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

    /// The label said "unconfigured" and exited 0. An exit code is what a merge
    /// gate and an agent in a hurry actually read, so the label alone let a
    /// switched-off guard read as a pass.
    #[test]
    fn an_unarmed_gate_is_a_finding_rather_than_a_silent_pass() {
        assert!(
            unconfigured(false, false).is_some(),
            "an unarmed name rule must fail loudly, not pass quietly"
        );
    }

    #[test]
    fn an_armed_gate_says_nothing() {
        assert!(unconfigured(true, false).is_none());
    }

    /// The opt-out exists for an outside contributor with neither the local file
    /// nor the repository secret. It has to be deliberate: an opt-out reachable
    /// by accident recreates the exact failure this finding exists to catch.
    #[test]
    fn the_opt_out_silences_it_but_only_when_asked() {
        assert!(unconfigured(false, true).is_none());
    }

    /// The message has to say what to do. A gate that fails without naming the
    /// fix gets bypassed rather than satisfied.
    #[test]
    fn the_message_names_both_ways_to_arm_it_and_the_opt_out() {
        let f = unconfigured(false, false).expect("unarmed");
        for needle in [
            EXTRA_ENV,
            UNCONFIGURED_OK_ENV,
            ".claude/contamination.local",
        ] {
            assert!(f.message.contains(needle), "message omits `{needle}`");
        }
    }

    #[test]
    fn a_file_named_after_the_source_repo_is_caught_even_when_its_contents_are_clean() {
        let rel = format!("docs/{INVENTED}-port.md");
        let (root, files) = scratch("filename", &[(&rel, "Nothing in here says it.\n")]);
        assert!(
            findings(&root, &files, &[], None).is_empty(),
            "not caught unconfigured"
        );

        let found = findings(&root, &files, &[INVENTED], None);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("the file's own path"));
        // No line to point at, and claiming line 1 would send a reader looking
        // for a word that is not on it.
        assert_eq!(found[0].line, 0);
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

    /// A throwaway repository, built commit by commit, because the whole point
    /// of these rules is what git kept rather than what is on disk — a file
    /// list cannot stand in for a ref graph.
    ///
    /// Committing a real string into a real repository is safe here for the
    /// same reason the scans above are: it is invented at run time and lives in
    /// a temporary directory that is not this one.
    fn repo(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("charkit-history-{label}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("scratch repo");
        git_ok(&dir, &["init", "-q", "-b", "main"]);
        git_ok(&dir, &["config", "user.email", "gate@example.invalid"]);
        git_ok(&dir, &["config", "user.name", "gate"]);
        // Whoever runs the suite may sign their commits; this repository is
        // deleted in a moment and has no key.
        git_ok(&dir, &["config", "commit.gpgsign", "false"]);
        dir
    }

    fn git_ok(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn commit(dir: &Path, rel: &str, body: &str, message: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("scratch dir");
        std::fs::write(&path, body).expect("scratch file");
        git_ok(dir, &["add", "-A"]);
        git_ok(dir, &["commit", "-q", "-m", message]);
    }

    fn published(root: &Path, names: &[&str]) -> Vec<Finding> {
        let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        scan_history(root, &names).expect("the history scan runs")
    }

    fn refs_reported(found: &[Finding]) -> Vec<&str> {
        found
            .iter()
            .filter(|f| f.file.starts_with("refs/"))
            .map(|f| f.file.as_str())
            .collect()
    }

    #[test]
    fn a_ref_whose_tip_publishes_the_name_is_reported() {
        let root = repo("tip");
        commit(
            &root,
            "README.md",
            &format!("ported out of {INVENTED}\n"),
            "docs",
        );

        let found = published(&root, &[INVENTED]);
        assert_eq!(refs_reported(&found), ["refs/heads/main"]);
        assert!(published(&root, &[]).is_empty(), "not caught unconfigured");
    }

    #[test]
    fn a_name_scrubbed_from_the_tip_is_still_reported_in_the_commit_that_added_it() {
        // Exactly this repository's own situation: the tree is clean and every
        // commit that made it clean is still reachable from the branch.
        let root = repo("scrubbed");
        commit(&root, "README.md", &format!("out of {INVENTED}\n"), "one");
        commit(&root, "README.md", "out of the source repo\n", "two");

        let found = published(&root, &[INVENTED]);
        assert!(refs_reported(&found).is_empty(), "the tip is clean");
        let history: Vec<&Finding> = found.iter().filter(|f| f.file == HISTORY).collect();
        assert_eq!(history.len(), 1);
        assert!(history[0].message.contains("2 commit(s)"));
    }

    #[test]
    fn a_tag_left_pointing_at_the_old_commit_is_reported_when_the_branch_is_clean() {
        let root = repo("tag");
        commit(&root, "README.md", &format!("out of {INVENTED}\n"), "one");
        git_ok(&root, &["tag", "phase-0"]);
        commit(&root, "README.md", "out of the source repo\n", "two");

        assert_eq!(
            refs_reported(&published(&root, &[INVENTED])),
            ["refs/tags/phase-0"]
        );
    }

    #[test]
    fn a_file_named_after_the_repo_is_reported_even_when_every_line_of_it_is_clean() {
        let root = repo("path");
        commit(
            &root,
            &format!("docs/{INVENTED}-port.md"),
            "Nothing in here says it.\n",
            "notes",
        );

        let found = published(&root, &[INVENTED]);
        assert_eq!(refs_reported(&found), ["refs/heads/main"]);
        assert!(found[0].message.contains("1 tracked file(s)"));
    }

    #[test]
    fn a_commit_message_that_names_it_is_reported_when_no_file_ever_did() {
        let root = repo("message");
        commit(
            &root,
            "README.md",
            "clean\n",
            &format!("port the {INVENTED} check"),
        );

        let found = published(&root, &[INVENTED]);
        assert!(refs_reported(&found).is_empty(), "no file ever carried it");
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("commit message(s)"));
    }

    #[test]
    fn a_configured_name_is_matched_literally_by_the_commit_scan_too() {
        // `a.c` as a pattern matches `abc`, and a name is not a pattern — the
        // pickaxe is the one scan that has to be told so, because making it
        // case-insensitive means handing git a regex.
        let root = repo("literal");
        commit(&root, "README.md", "use abc::x\n", "code");
        assert!(published(&root, &["a.c"]).is_empty());
        assert!(!published(&root, &["a.c", "abc"]).is_empty());
    }

    #[test]
    fn a_repository_with_no_refs_at_all_reports_nothing_rather_than_failing() {
        // `git log --all` is fatal in a repository with no commits, and a fresh
        // `git init` is the state every clean-room instruction starts from.
        let root = repo("empty");
        assert!(published(&root, &[INVENTED]).is_empty());
    }
}
