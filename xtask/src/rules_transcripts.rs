//! Rule: no test reads a transcript path straight off the disk.
//!
//! **A row is written by a task nothing awaits**, so a test that reads the file
//! once reads whatever the scheduler had got round to. That passed on every
//! idle machine and failed on the merge line three times in `fleet`, each time
//! reported as the absence of the row rather than as a race — `#1436`'s panic
//! carried `[]` and nothing else.
//!
//! **Why a rule and not a written reading.** `restarting::until_spoken` already
//! waited, correctly, with its reason in a doc comment, before either of the
//! next two sites was written. The knowledge was in the crate and did not reach
//! their authors. A paragraph in a practice doc is the same mechanism that
//! already failed, so what changes is what is available to reach for: one
//! waiting reader, and this rule refusing the way round it.

use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

#[cfg(test)]
mod tests;

/// The one reader, which is where the bare read belongs and is private inside
/// it. A named file with a stated reason rather than an allowlist — a second
/// entry here is the rule being worked around.
const THE_ONE_READER: &str = "crates/fleet/src/tests/transcript/reading.rs";

/// How a transcript path is built. The rule follows this name and nothing else:
/// a test reading a fixture it wrote itself cannot race, and a rule over
/// un-retried reads in general would hit ninety-nine of those and need an
/// allowlist that rots.
const THE_PATH: &str = "transcript_of(";

/// Calls that take a path and hand back its bytes.
const READS: &[&str] = &["read_to_string(", "fs::read(", "File::open("];

/// No test module reads a `transcript_of` path directly — it goes through the
/// waiting reader, which is the only thing that can tell an empty file from a
/// file the writer has not reached yet.
pub fn no_bare_transcript_read_in_a_test(root: &Path) -> Report {
    let mut report = Report::new("no bare transcript read in a test");
    for source_root in ["crates", "apps", "xtask"] {
        for path in files_with_ext(root, &root.join(source_root), &["rs"]) {
            // The gate spells out the shapes it forbids, here and in its own
            // tests, the same exemption every other rule in this directory
            // carries.
            if path == THE_ONE_READER || path.starts_with("xtask/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            let Some(from) = where_the_tests_start(&path, &text) else {
                continue;
            };
            for line in bare_transcript_reads(&text, from) {
                report.fail(format!(
                    "{path}:{line} — a transcript read once, in a test. \
                     `tests::transcript::reading::Transcript` waits for the row"
                ));
            }
        }
    }
    report
}

/// The line a file's test module starts at, where it has one.
///
/// A file under `src/tests/` is one throughout. Anywhere else it is the
/// `#[cfg(test)] mod tests {` block, taken to the end of the file — the `mod
/// tests;` form names a file this already covers by path.
fn where_the_tests_start(path: &str, text: &str) -> Option<usize> {
    if path.contains("/src/tests/") || path.ends_with("/src/tests.rs") {
        return Some(0);
    }
    let lines: Vec<&str> = text.lines().collect();
    lines.iter().enumerate().find_map(|(n, line)| {
        let opens = line.trim_start().starts_with("mod tests {")
            || line.trim_start().starts_with("mod tests{");
        let gated = n > 0 && lines[n - 1].trim_start().starts_with("#[cfg(test)]");
        (opens && gated).then_some(n)
    })
}

/// Every line from `from` on that reads a path built by [`THE_PATH`], as line
/// numbers.
///
/// Two shapes: the read and the build on one line, and a read of a binding an
/// earlier line put the path in. Nothing here is dataflow — a path handed
/// through a function call is not followed — because the shape that burned
/// three times is a `let` and a read a few lines apart, and a rule that guesses
/// further is a rule that fires on something else.
///
/// **A name is forgotten when it is bound to something else and when the item
/// it was bound in ends.** Without the first, `let path = log_of(..)` in the
/// next function inherits the `path` before it and the rule names a read of the
/// Job's log — which it did, at the first run over this repository.
fn bare_transcript_reads(text: &str, from: usize) -> Vec<usize> {
    let mut holding: Vec<String> = Vec::new();
    let mut awaiting: Option<String> = None;
    let mut found = Vec::new();
    for (n, raw) in text.lines().enumerate().skip(from) {
        let line = raw.trim();
        if line.starts_with("//") {
            continue;
        }
        if raw == "}" {
            holding.clear();
            continue;
        }
        if READS.iter().any(|read| line.contains(read))
            && (line.contains(THE_PATH) || holding.iter().any(|name| names(line, name)))
        {
            found.push(n + 1);
        }
        if line.contains(THE_PATH) {
            if let Some(name) = bound_here(line).or_else(|| awaiting.take()) {
                holding.push(name);
            }
        } else if let Some(name) = bound_here(line) {
            if line.ends_with('=') {
                awaiting = Some(name);
            } else {
                holding.retain(|held| *held != name);
            }
        }
    }
    found
}

/// The name a `let` on this line binds, if it binds one.
fn bound_here(line: &str) -> Option<String> {
    let rest = line.strip_prefix("let ")?;
    let rest = rest.strip_prefix("mut ").unwrap_or(rest);
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Whether the line names this binding, as a word rather than as a substring —
/// `path` must not match `pathological`.
fn names(line: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(name).any(|(at, _)| {
        let before = line[..at].chars().next_back().is_some_and(word);
        let after = line[at + name.len()..].chars().next().is_some_and(word);
        !before && !after
    })
}
