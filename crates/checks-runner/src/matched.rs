//! Telling "the named test ran" apart from "the name matched nothing at
//! all", for `fleet::fixing::run_on_main`'s sake. #1204.
//!
//! **The exit code alone cannot carry this.** Both nextest and vitest exit
//! `0` on a name that matches nothing, the same code a real pass reports, so
//! this reads the summary line each one prints instead.

use crate::run::Output;
use verification::Exit;

/// What running a named test by [`crate::one_test`]'s command came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneTestRan {
    /// It ran, and exited as the Check expects.
    Passed,
    /// It ran, and did not exit as the Check expects.
    Failed,
    /// The name matched nothing the runner could find. **Neither a pass nor
    /// a failure** — a caller that folded this into either would be telling
    /// a Drone that a mistyped or stale name already passes on main.
    NoMatch,
}

/// Read a command's exit and output the way [`crate::one_test`]'s command
/// must be read: the summary line first, and the exit code only where the
/// summary does not say the name matched nothing.
pub fn one_test_ran(exit: &Exit, output: &Output, expect_exit_code: i64) -> OneTestRan {
    if matched_nothing(output) {
        return OneTestRan::NoMatch;
    }
    match exit {
        Exit::Code(code) if i64::from(*code) == expect_exit_code => OneTestRan::Passed,
        _ => OneTestRan::Failed,
    }
}

fn matched_nothing(output: &Output) -> bool {
    nextest_matched_nothing(output) || vitest_matched_nothing(output)
}

/// nextest's own wording for a filter that matched nothing (`error: no tests
/// to run`), and the summary line's own count read the same way — the
/// wording is the more specific signal and the count is what survives it
/// changing.
fn nextest_matched_nothing(output: &Output) -> bool {
    let combined = format!("{}\n{}", output.stdout, output.stderr);
    combined.contains("error: no tests to run") || nextest_ran_count(&combined) == Some(0)
}

/// The count nextest's summary line states ran, out of `Summary [...] N
/// test(s) run: P passed, ...`. `None` where the line is not there at all.
fn nextest_ran_count(text: &str) -> Option<u32> {
    text.lines().find_map(|line| {
        let words: Vec<&str> = line.split_whitespace().collect();
        let at = words.iter().position(|word| *word == "run:")?;
        match words.get(at.checked_sub(1)?) {
            Some(&"test") | Some(&"tests") => words.get(at.checked_sub(2)?)?.parse().ok(),
            _ => None,
        }
    })
}

/// vitest's summary line, `Tests  N passed | M skipped (T)`, read for the one
/// case that means the name matched nothing: every test in the run was
/// skipped, which is what a `-t` filter matching no test case does — vitest
/// still visits every file and skips each one rather than reporting zero.
fn vitest_matched_nothing(output: &Output) -> bool {
    output
        .stdout
        .lines()
        .chain(output.stderr.lines())
        .any(vitest_summary_line_is_all_skipped)
}

fn vitest_summary_line_is_all_skipped(line: &str) -> bool {
    let Some(after) = line.trim().strip_prefix("Tests") else {
        return false;
    };
    let Some(total) = total_in_parens(after) else {
        return false;
    };
    total > 0 && skipped_count(after) == total
}

/// The number in the summary line's trailing `(N)`.
fn total_in_parens(after: &str) -> Option<u32> {
    let open = after.rfind('(')?;
    let close = after.rfind(')')?;
    after.get(open + 1..close)?.trim().parse().ok()
}

/// The number just before the word `skipped`, or zero where the summary line
/// names no skipped tests at all.
fn skipped_count(after: &str) -> u32 {
    let words: Vec<&str> = after.split_whitespace().collect();
    words
        .iter()
        .position(|word| *word == "skipped")
        .and_then(|at| at.checked_sub(1))
        .and_then(|before| words.get(before))
        .and_then(|word| word.parse().ok())
        .unwrap_or(0)
}
