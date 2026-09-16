//! Naming every test a run's own summary said failed — for comparing two
//! attempts of one step, which [`crate::one_test_ran`] cannot do: it answers
//! whether one already-named test passed, not what a whole suite's own
//! output names.
//!
//! **The same two runners [`crate::matched`] reads, for the same reason**:
//! nothing here is structured, so a name is read off the runner's own words.
//! A Check whose command is neither answers no names, which is read as
//! "nothing to compare" rather than as a refusal.

use crate::run::Output;

/// Every test this run's own summary named as failing, in the order printed,
/// without duplicates. Empty where the summary named none at all.
pub fn failing_tests(output: &Output) -> Vec<String> {
    failing_tests_in(&format!("{}\n{}", output.stdout, output.stderr))
}

/// The same, over text already in hand rather than a fresh [`Output`] — a
/// file `fleet` read back from an earlier attempt. Both halves of a
/// comparison read through this one function, so a wording change moves both.
pub fn failing_tests_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for name in nextest_failing(text).chain(vitest_failing(text)) {
        if !found.contains(&name) {
            found.push(name);
        }
    }
    found
}

/// nextest's summary block prints once, after the line `Summary [` opens —
/// scoping to after it is what keeps a coincidental `FAIL` inside a panic
/// message from being read as a name.
fn nextest_failing(text: &str) -> impl Iterator<Item = String> + '_ {
    let after = text.rsplit_once("Summary [").map_or("", |(_, after)| after);
    after.lines().filter_map(nextest_fail_line)
}

/// One line: `FAIL [   0.018s] (1/4) nt tests::fails_one`. The name is the
/// last word — confirmed against the installed binary, `-E
/// 'test(=tests::nested::fails_two)'` runs with no binary id in the filter.
fn nextest_fail_line(line: &str) -> Option<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.first().copied() != Some("FAIL") {
        return None;
    }
    // The `(n/m)` counter marks where the binary id starts; the test's own
    // name is the word after it, and Rust identifiers hold no whitespace.
    let counter = words
        .iter()
        .position(|word| word.starts_with('(') && word.ends_with(')'))?;
    words.get(counter + 2).map(|name| name.to_string())
}

/// Every ` FAIL  <file> > ... > <test>` line vitest's "Failed Tests" block
/// prints. A workspace with more than one vitest project prefixes the file
/// with `|<project>| `, which changes nothing: the leaf is still the name.
fn vitest_failing(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines().filter_map(vitest_fail_line)
}

/// The leaf after the last `>` — confirmed against `armada.yml`'s `vitest
/// run -t {}`, which matches only that title and never the file or the
/// `describe` blocks around it. `None` where the line names no test at all:
/// a file that failed to load prints the same `FAIL` word with no `>`.
fn vitest_fail_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let after = trimmed.strip_prefix("FAIL")?;
    if !after.starts_with(char::is_whitespace) {
        return None;
    }
    let at = after.rfind('>')?;
    let leaf = after[at + 1..].trim();
    (!leaf.is_empty()).then(|| leaf.to_string())
}
