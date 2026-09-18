//! `verify-foundations` on one turn, read as a delta against `main`'s own
//! run. Every function here takes text a caller already has — no
//! filesystem, no subprocess — matching `scripts/land`'s own `finding_counts`,
//! `not_installed`, `failing_lines`, `a_report` and the pure half of
//! `foundations_delta`.
//!
//! **Only a new failing line is red.** Red already on `main` is legitimate;
//! a warning is not a failure either way; and a line that only moved down the
//! file — a finding renumbered by an unrelated insertion above it — is not a
//! new one. `finding_counts` strips the line number out before counting, and
//! counts rather than dedupes: a second violation of a rule already known to
//! fail is still a new failure, not one folding onto the first.

use std::collections::BTreeMap;

/// How many times each failing line was printed, with its `path:N` subject
/// normalised to `path` first.
///
/// **Counted, not deduplicated into a set.** A second instance of an
/// already-known finding — the same rule, a different line — reads
/// identically to the first once the number is stripped, and folding it onto
/// the known count would make a genuinely new violation disappear.
pub fn finding_counts<S: AsRef<str>>(lines: &[S]) -> BTreeMap<String, usize> {
    let mut counted = BTreeMap::new();
    for line in lines {
        *counted
            .entry(strip_line_numbers(line.as_ref()))
            .or_insert(0) += 1;
    }
    counted
}

/// The rule and finding lines a run printed — what a delta compares.
pub fn failing_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("FAIL") || line.starts_with("missing:"))
        .map(str::to_string)
        .collect()
}

/// Whether a run said anything about the rules.
///
/// **A non-zero exit printing no failing line is a crash, not a red gate.**
/// A branch that breaks the `xtask` build exits with a compiler error and
/// prints no `FAIL`/`missing:` line at all — that must not read as an
/// all-green comparison.
pub fn a_report(text: &str, exit_code: i32) -> bool {
    exit_code == 0 || !failing_lines(text).is_empty()
}

/// The tool a run says is missing, or `None` where it ran and failed on its
/// own account.
///
/// Two fixed wordings, matched by hand rather than through a `regex`
/// dependency this workspace does not otherwise carry (`Cargo.lock` has no
/// `regex` anywhere in the graph): cargo's own for a missing subcommand, and
/// `checks-runner`'s (`crates/armada/src/declared.rs`) for a program that is
/// not on `PATH`. Narrow on purpose — any other non-zero exit is the Check
/// itself failing, which is a red gate, not a missing tool.
pub fn not_installed(text: &str) -> Option<String> {
    missing_cargo_subcommand(text).or_else(|| missing_checks_runner_binary(text))
}

fn missing_cargo_subcommand(text: &str) -> Option<String> {
    let marker = "no such command: ";
    let after = &text[text.find(marker)? + marker.len()..];
    let after = after.strip_prefix('`').unwrap_or(after);
    let end = after
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(after.len());
    let name = &after[..end];
    (!name.is_empty()).then(|| name.to_string())
}

fn missing_checks_runner_binary(text: &str) -> Option<String> {
    let marker = "needs `";
    let after = &text[text.find(marker)? + marker.len()..];
    let end = after.find('`')?;
    let (name, rest) = (&after[..end], &after[end + 1..]);
    rest.starts_with(", which is not on this machine's PATH")
        .then(|| name.to_string())
}

/// The result of comparing one turn's `verify-foundations` output against
/// what `main` already reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FoundationsComparison {
    /// Present here more times than `main`'s own run already knows to fail —
    /// this branch's own doing, and red.
    New(Vec<String>),
    /// [`a_report`] was false: the run crashed rather than naming a rule, so
    /// nothing was gated. The last few lines it printed, for the caller's
    /// message.
    Crashed(Vec<String>),
}

/// `base_output` is `main`'s own already-validated report (a caller that has
/// not yet confirmed that — a fresh cache entry — validates it with
/// [`a_report`] first; a `base` whose own gate cannot run is a different
/// failure entirely, and not this function's business). `run_output` and
/// `run_exit_code` are this turn's.
pub fn foundations_delta(
    base_output: &str,
    run_output: &str,
    run_exit_code: i32,
) -> FoundationsComparison {
    if !a_report(run_output, run_exit_code) {
        return FoundationsComparison::Crashed(last_lines(run_output, 3));
    }
    let known = finding_counts(&failing_lines(base_output));
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut new = Vec::new();
    for line in failing_lines(run_output) {
        let at = strip_line_numbers(&line);
        let count = seen.entry(at.clone()).or_insert(0);
        *count += 1;
        if *count > known.get(&at).copied().unwrap_or(0) {
            new.push(line);
        }
    }
    FoundationsComparison::New(new)
}

/// Of `names`, the ones a per-base Check-verdict cache already recorded as
/// failing on `main` itself — filtering only. Building that cache (running
/// each Check in a base worktree, and keeping the answer per commit) is
/// orchestration this stage does not do; `checks_on_the_base` in
/// `scripts/land` is the reference, and its cache read/write is Stage 3/4's.
pub fn already_red_on_base(names: &[String], known: &BTreeMap<String, bool>) -> Vec<String> {
    names
        .iter()
        .filter(|name| known.get(name.as_str()) == Some(&false))
        .cloned()
        .collect()
}

/// The last `n` non-blank-trimmed lines of `text`, matching Python's
/// `text.strip().splitlines()[-3:]`.
fn last_lines(text: &str, n: usize) -> Vec<String> {
    let lines: Vec<&str> = text.trim().lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].iter().map(|line| line.to_string()).collect()
}

/// Strips every `:\d+` run out of `line` — the number a finding's `path:N`
/// subject carries, which shifts whenever an unrelated line is inserted
/// above it in the same file. No `regex` dependency: this is the one
/// substitution that needs it, so it is hand-rolled instead.
fn strip_line_numbers(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}
