//! The six rules. Each returns what is missing, by name.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::{crate_dirs, files_with_ext, walk, Report};

/// Source extensions the line-length and literal rules read.
const SOURCE_EXTS: &[&str] = &["rs", "ts", "tsx"];

/// Where source lives. `docs/` is prose and is not gated.
const SOURCE_ROOTS: &[&str] = &["crates", "apps", "packages", "xtask"];

// ---------------------------------------------------------------- rule one

/// The acceptance test exists and passes.
///
/// **A milestone that can fake itself green proves nothing.** That is why this
/// rule has always been here; what changed at the end of M0 is the direction the
/// falsehood would run. Through M0 the test was written before the code it
/// tested, so a green run meant something had been stubbed, weakened or made to
/// pass, and the rule was satisfied by a non-zero exit. The code the test names
/// now exists, so a red run means the milestone's claim is not carried, and the
/// rule is satisfied by a green one.
///
/// It reports *which kind* of failure, because they are not the same signal.
/// "Does not compile" means the test names an API that moved out from under it,
/// and reconciling it is a change to what the milestone claims. "Failed an
/// assertion" means it builds and the behaviour is gone.
///
/// **A run of nothing is the same falsehood pointing the other way.** An empty
/// test binary exits zero, so the rule counts the tests that ran rather than
/// reading the exit status alone.
///
/// The invocation is `cargo test -p acceptance`. What an acceptance test is
/// here, and what one costs, is `docs/practices/acceptance-tests.md`.
pub fn acceptance_test_exists_and_passes(root: &Path) -> Report {
    let mut report = Report::new("the acceptance test exists and passes");

    if !root.join("crates/acceptance/Cargo.toml").is_file() {
        report.fail("crates/acceptance — the package holding the acceptance test");
        return report;
    }
    let tests = files_with_ext(root, &root.join("crates/acceptance/tests"), &["rs"]);
    if tests.is_empty() {
        report.fail("crates/acceptance/tests/ — no test in the acceptance package");
        return report;
    }

    let run = Command::new("cargo")
        .args(["test", "--package", "acceptance", "--quiet"])
        .current_dir(root)
        .output();
    let Ok(run) = run else {
        report.fail("the run — `cargo test -p acceptance` could not be started");
        return report;
    };

    if !run.status.success() {
        let stderr = String::from_utf8_lossy(&run.stderr);
        let kind = if stderr.contains("error[E") || stderr.contains("could not compile") {
            "does not compile — it names an API that is no longer there. \
             Reconciling it changes what the milestone claims, and is named as such"
        } else {
            "failed an assertion — it builds, and the claim it makes is not carried"
        };
        report.fail(format!("a passing acceptance test — it {kind}"));
        return report;
    }

    if tests_that_ran(&String::from_utf8_lossy(&run.stdout)) == 0 {
        report.fail(
            "a passing acceptance test — it ran none. An empty test binary exits \
             zero, and green over nothing claims nothing",
        );
    }
    report
}

/// How many tests the run actually executed, read out of libtest's own summary
/// lines. Every harness in the package reports one, including the ones with
/// nothing in them, so the counts are summed rather than taken from the first.
fn tests_that_ran(stdout: &str) -> usize {
    let mut total = 0;
    for line in stdout.lines() {
        let Some(rest) = line.trim().strip_prefix("test result: ") else {
            continue;
        };
        let Some((_, counts)) = rest.split_once(". ") else {
            continue;
        };
        if let Some((passed, _)) = counts.split_once(" passed") {
            total += passed.trim().parse::<usize>().unwrap_or(0);
        }
    }
    total
}

// ---------------------------------------------------------------- rule two

/// Every fixture the spec's assertions need exists.
///
/// Two groups, and the second was missing until 24 Aug 2026.
///
/// **The five failure modes** are Workflow's own table, in its order, and the
/// file names are the Fixture Specs' slugs.
///
/// **The three that make those five testable.** Three of the five assertions
/// describe something one stream cannot express, and the gate did not notice
/// because it only counted the failure modes:
///
/// - `thrashing` asserts the Judge is **not** invoked for a well-behaved Drone.
///   That needs a well-behaved stream to assert against.
/// - `silence` asserts a Drone answering a poke resumes and does **not**
///   escalate. Without it, a detector that escalates every quiet Drone passes.
/// - `claims-done-no-evidence` asserts a clarification round capped at two or
///   three, and escalation on exhausting it. A single terminated stream has no
///   cap in it.
const FAILURE_MODES: &[(&str, &str)] = &[
    ("silence", "Silence / Stalled"),
    (
        "claims-done-no-evidence",
        "Claims done, no evidence artifact",
    ),
    (
        "plain-text-bypass",
        "Claims done in plain text, bypasses structured report",
    ),
    ("thrashing", "Thrashing / off-rails"),
    ("evidence-gaming", "Evidence gaming"),
];

/// The control and the negative cases, each named with the assertion it serves.
const SUPPORTING: &[(&str, &str)] = &[
    (
        "happy-path",
        "the control — thrashing's negative assertion needs it",
    ),
    (
        "silence-poke-answered",
        "a poke answered, so quiet alone cannot escalate",
    ),
    (
        "clarification-exhausted",
        "the capped clarification loop, exhausted",
    ),
];

pub fn every_failure_mode_has_a_fixture(root: &Path) -> Report {
    let mut report = Report::new("every fixture the assertions need exists");
    let dir = root.join("crates/testkit/fixtures/ndjson");
    for (slug, why) in FAILURE_MODES.iter().chain(SUPPORTING) {
        if !dir.join(format!("{slug}.ndjson")).is_file() {
            report.fail(format!(
                "crates/testkit/fixtures/ndjson/{slug}.ndjson — {why}"
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
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
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
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
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
    "anthropic",
    "claude",
    "openai",
    "gpt-4",
    "gemini",
    "copilot",
    "github",
    "gitlab",
    "bitbucket",
    "docker",
    // Libraries, not vendors, and on the list for the same reason: naming one
    // outside the adapter layer means the boundary has already leaked. The
    // adapters contract has said "no `git2` or `gh` literal outside adapters"
    // since it was written, and nothing was checking it — found when a step
    // built a worktree behind the VCS trait and went looking for the rule that
    // was supposed to have stopped it going anywhere else.
    "git2",
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
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
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
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
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

// ------------------------------------------------------------- rule eight

/// The v1 harvest has an index, and the index knows every note.
///
/// v1 exists only as a tag now, so the harvest is the last time anything about
/// it gets written down while somebody still remembers why it mattered. An
/// unindexed note is one nobody finds, which makes it the same as one nobody
/// wrote.
///
/// The check runs both ways, like the source manifest: a note missing from the
/// index fails, and so does an index entry pointing at a note that is not there.
pub fn the_v1_harvest_has_an_index(root: &Path) -> Report {
    let mut report = Report::new("the v1 harvest has an index, and it knows every note");
    let dir = root.join("docs/v1-learnings");
    let index = dir.join("INDEX.md");
    let Ok(index_text) = fs::read_to_string(&index) else {
        report.fail("docs/v1-learnings/INDEX.md — the harvest index (M0 step 10 writes it)");
        return report;
    };

    for note in files_with_ext(root, &dir, &["md"]) {
        let name = note.rsplit('/').next().unwrap_or(&note);
        if name == "INDEX.md" {
            continue;
        }
        if !index_text.contains(name) {
            report.fail(format!(
                "{note} is a harvest note the index does not mention"
            ));
        }
    }

    for line in index_text.lines() {
        for token in
            line.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
        {
            if token.ends_with(".md") && token != "INDEX.md" && !dir.join(token).is_file() {
                report.fail(format!(
                    "the index names {token}, which is not in docs/v1-learnings/"
                ));
            }
        }
    }
    report
}

// -------------------------------------------------------------- rule nine

/// Nothing committed here names a person or a machine.
///
/// This repository is public, and **documentation is the larger leak surface,
/// not code**. v1 learned that the expensive way: its privacy gate was written
/// after every reference that had to be scrubbed turned out to live in `docs/`,
/// a README and an agent file, none of which the code-only grep covered.
///
/// The rule is a convention rather than a guess at what a real username looks
/// like: **the only user in a committed path is `user`.** A capture from a real
/// machine gets its home path rewritten to that before it lands, which makes an
/// unrewritten one visible instead of plausible.
///
/// No allowlist. A gate with exemptions is a gate whose exemptions grow.
pub fn nothing_names_a_person_or_a_machine(root: &Path) -> Report {
    let mut report = Report::new("nothing committed names a person or a machine");

    // Credential shapes. Prefixes only — a gate that tries to recognise a secret
    // by entropy fails on both sides.
    const SECRETS: &[(&str, &str)] = &[
        ("sk-ant-", "an API key"),
        ("gho_", "a token"),
        ("ghp_", "a token"),
        ("github_pat_", "a token"),
        ("xoxb-", "a token"),
        ("AKIA", "an access key"),
        ("BEGIN RSA PRIVATE KEY", "a private key"),
        ("BEGIN OPENSSH PRIVATE KEY", "a private key"),
    ];

    walk(root, &mut |path| {
        let Ok(text) = fs::read_to_string(path) else {
            return;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        // The gate names the shapes it forbids, so it always matches itself.
        if rel.starts_with("xtask/") {
            return;
        }
        for (n, line) in text.lines().enumerate() {
            let at = n + 1;
            for prefix in ["/Users/", "/home/"] {
                if let Some(rest) = line.split(prefix).nth(1) {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                        .collect();
                    if !name.is_empty() && name != "user" {
                        report.fail(format!(
                            "{rel}:{at} — a home directory naming `{name}`. Committed paths use `user`"
                        ));
                    }
                }
            }
            for (shape, what) in SECRETS {
                if line.contains(shape) {
                    report.fail(format!("{rel}:{at} — {what}, by its prefix `{shape}`"));
                }
            }
        }
    });
    report
}

// --------------------------------------------------------------- rule ten

/// Everything logs through the envelope. No crate writes its own format.
///
/// The envelope only guarantees a join if every line has one. A `println!` in
/// the middle of `fleet` produces a line that no query reaches, no sink
/// redacts, and nothing can correlate to the Job that caused it — and it is the
/// easiest thing in the world to add while debugging and forget.
///
/// `armada` is exempt: it is the composition root and the CLI, and every one of
/// its verbs writes to stdout on purpose — a command answering a question is
/// not a component logging.
pub fn nothing_writes_its_own_log_format(root: &Path) -> Report {
    let mut report = Report::new("everything logs through the envelope");
    const MACROS: &[&str] = &["println!", "eprintln!", "print!", "eprint!"];

    for dir in crate_dirs(root) {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "armada" {
            continue;
        }
        for path in files_with_ext(root, &dir.join("src"), &["rs"]) {
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            for (n, line) in text.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for macro_name in MACROS {
                    if code.contains(macro_name) {
                        report.fail(format!(
                            "{path}:{} — `{macro_name}` writes a line the envelope does not carry",
                            n + 1
                        ));
                    }
                }
            }
        }
    }
    report
}

/// Rule eleven: the checked-in token outputs say what the CSS says.
///
/// `verify-tokens` is the task that regenerates them, and this rule is what
/// makes the gate notice. A step that ships work no rule watches has quietly
/// narrowed the gate's coverage — the gate only knows what somebody wrote a
/// rule for, and going green means every subject a rule names has landed.
pub fn the_tokens_generate_what_is_checked_in(root: &Path) -> Report {
    let mut report = Report::new("the design tokens generate what is checked in");
    match crate::tokens::outputs(root) {
        Err(why) => report.fail(why),
        Ok(outputs) => {
            for (rel, want) in outputs {
                match fs::read_to_string(root.join(&rel)) {
                    Ok(have) if have == want => {}
                    Ok(_) => report.fail(format!(
                        "{rel} — stale or hand-edited. \
                         Run `cargo xtask verify-tokens --write` and commit what it emits"
                    )),
                    Err(_) => report.fail(format!("{rel} — not generated yet")),
                }
            }
        }
    }
    report
}

// -------------------------------------------------------- rule twenty-three

/// A comment block runs to twenty-five lines and no further.
///
/// Measured before it was written: about three lines in ten of this workspace
/// are comments, and single blocks reached seventy — several module headers
/// longer than the code beneath them.
///
/// **The cap is not a claim that long reasoning is unwanted.** It is a claim
/// about where reasoning goes. A block this long is one of three things: a
/// comment written at three times the length it needs, which reduction fixes
/// and nothing is lost; a decision with its alternatives, which belongs with
/// the decision; or a module explaining how a whole area works, which is a
/// practice doc with the module pointing at it.
///
/// **Reduce before filing.** Assuming a long comment is a document moves the
/// verbosity somewhere new and calls it filing.
pub fn no_comment_block_too_long(root: &Path) -> Report {
    let mut report = Report::new("no comment block over 25 lines");
    const CAP: usize = 25;

    for source_root in SOURCE_ROOTS {
        for path in files_with_ext(root, &root.join(source_root), &["rs"]) {
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            let (mut run, mut started) = (0usize, 0usize);
            let mut worst = (0usize, 0usize);
            for (n, line) in text.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") || t.starts_with('*') || t.starts_with("/*") {
                    if run == 0 {
                        started = n + 1;
                    }
                    run += 1;
                    if run > worst.0 {
                        worst = (run, started);
                    }
                } else {
                    run = 0;
                }
            }
            if worst.0 > CAP {
                report.fail(format!(
                    "{path}:{} — {} lines of comment in one block, over {CAP}. \
                     Reduce it first; file it only if it cannot shorten without \
                     losing a fact. `.claude/skills/comments/SKILL.md`",
                    worst.1, worst.0
                ));
            }
        }
    }
    report
}
