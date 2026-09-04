//! The rule's own negative tests.
//!
//! Three rows were wrong on one night and the gate said nothing, because
//! nothing read `transitions_in` or `transitions_out` at all. Each shape found
//! then has a test here, both directions of the comparison have one, and so do
//! both ways this rule can end up comparing nothing — which is the failure
//! `crate::rules_protocol` was fixed for in #124 and the one a rule like this
//! fails at silently.

use super::*;
use crate::Finding;

/// Every finding a report holds, as text.
fn findings(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

/// The edges as [`super::check`] takes them, with a line for each.
fn edges(pairs: &[(&str, &str)]) -> Vec<(String, String, usize)> {
    pairs
        .iter()
        .enumerate()
        .map(|(n, (from, to))| (from.to_string(), to.to_string(), n + 1))
        .collect()
}

const WIRED: &[(&str, &str)] = &[
    ("queued", "running"),
    ("running", "queued"),
    ("running", "escalated"),
    ("escalated", "queued"),
    ("escalated", "killed"),
    ("running", "killed"),
    ("queued", "killed"),
];

/// The three statuses the wired set moves between, plus the terminal one that
/// declares no `transitions_out` at all.
const AGREEING: &str = r#"
[statuses.escalated]
terminal = false
transitions_in = ["running -> escalated"]
transitions_out = ["escalated -> queued", "escalated -> killed"]

[statuses.killed]
terminal = true
transitions_in = ["escalated -> killed", "running -> killed", "queued -> killed"]

[statuses.queued]
terminal = false
transitions_in = ["escalated -> queued", "running -> queued"]
transitions_out = ["queued -> running", "queued -> killed"]

[statuses.running]
terminal = false
transitions_in = ["queued -> running"]
transitions_out = ["running -> queued", "running -> escalated", "running -> killed"]
"#;

fn run(statuses: &str, wired: &[(&str, &str)]) -> Report {
    let mut report = Report::new("test");
    check(statuses, &edges(wired), &mut report);
    report
}

fn names(report: &Report, wanted: &[&str]) {
    let findings = findings(report);
    assert!(
        findings
            .iter()
            .any(|f| wanted.iter().all(|want| f.contains(want))),
        "expected a finding naming {wanted:?}: {findings:?}"
    );
}

#[test]
fn a_registry_that_agrees_with_the_table_is_clean() {
    let report = run(AGREEING, WIRED);
    assert!(!report.failed(), "{:?}", findings(&report));
}

/// **The exact shape #394 was filed for.** `escalated -> queued` was wired by
/// #50 and the row that leaves it never said so.
#[test]
fn an_edge_the_table_carries_and_no_row_names_fails() {
    let missing = AGREEING.replace(
        r#"transitions_out = ["escalated -> queued", "escalated -> killed"]"#,
        r#"transitions_out = ["escalated -> killed"]"#,
    );
    let report = run(&missing, WIRED);
    assert!(report.failed());
    names(
        &report,
        &["escalated.transitions_out", "escalated -> queued", EDGES],
    );
}

/// The same slip on the arriving side, which is how `queued.transitions_in`
/// came to be missing two edges at once.
#[test]
fn an_edge_arriving_at_a_row_that_does_not_name_it_fails() {
    let missing = AGREEING.replace(
        r#"transitions_in = ["escalated -> queued", "running -> queued"]"#,
        r#"transitions_in = ["escalated -> queued"]"#,
    );
    let report = run(&missing, WIRED);
    assert!(report.failed());
    names(
        &report,
        &["queued.transitions_in", "running -> queued", STATUSES],
    );
}

/// The other direction: a promise nothing keeps.
#[test]
fn a_row_naming_an_edge_the_table_does_not_carry_fails() {
    let invented = AGREEING.replace(
        r#"transitions_out = ["queued -> running", "queued -> killed"]"#,
        r#"transitions_out = ["queued -> running", "queued -> killed", "queued -> escalated"]"#,
    );
    let report = run(&invented, WIRED);
    assert!(report.failed());
    names(
        &report,
        &["queued.transitions_out", "queued -> escalated", EDGES],
    );
}

/// A field dropped whole is one finding naming the set it owes, not one per
/// edge pointing at a line that no longer says anything about them.
#[test]
fn a_row_that_drops_the_field_is_named_once_with_the_set_it_owes() {
    let dropped = AGREEING.replace(
        r#"transitions_out = ["running -> queued", "running -> escalated", "running -> killed"]"#,
        "",
    );
    let report = run(&dropped, WIRED);
    assert!(report.failed());
    let said: Vec<String> = findings(&report)
        .into_iter()
        .filter(|f| f.contains("`running` declares no `transitions_out`"))
        .collect();
    assert_eq!(said.len(), 1, "one finding for one absent field: {said:?}");
    for edge in [
        "running -> queued",
        "running -> escalated",
        "running -> killed",
    ] {
        assert!(said[0].contains(edge), "the set it owes: {}", said[0]);
    }
}

/// An entry at another status's end is read by nobody, and is reported as
/// that rather than as an edge the table does not carry.
#[test]
fn an_entry_at_the_wrong_end_of_its_own_row_fails() {
    let wrong = AGREEING.replace(
        r#"transitions_in = ["queued -> running"]"#,
        r#"transitions_in = ["queued -> running", "running -> killed"]"#,
    );
    let report = run(&wrong, WIRED);
    assert!(report.failed());
    names(&report, &["running.transitions_in", "arrives at", "killed"]);
}

/// An entry the rule cannot read is one it does not compare, and silence about
/// it is the same hole one entry wide.
#[test]
fn an_entry_that_is_not_an_edge_is_reported_rather_than_skipped() {
    let junk = AGREEING.replace(
        r#"transitions_in = ["queued -> running"]"#,
        r#"transitions_in = ["queued -> running", "queued"]"#,
    );
    let report = run(&junk, WIRED);
    assert!(report.failed());
    names(&report, &["is not a `from -> to` edge"]);
}

/// An edge whose end has no row is invisible to a walk over rows.
#[test]
fn an_edge_naming_a_status_with_no_row_fails() {
    let mut wired = WIRED.to_vec();
    wired.push(("running", "piloted"));
    let report = run(AGREEING, &wired);
    assert!(report.failed());
    names(&report, &["[statuses.piloted]", "running -> piloted"]);
}

/// **The gate's own failure mode, both ways it can happen here.** A rule that
/// compares nothing must not read as agreement: the status rows unreadable and
/// the two fields renamed each leave the comparison with nothing to say, and
/// each would tolerate exactly what #394 found.
#[test]
fn comparing_nothing_fails_rather_than_passing() {
    let no_rows = "# every row moved to another file\n";
    let no_fields = AGREEING
        .replace("transitions_in", "edges_in")
        .replace("transitions_out", "edges_out");
    for (statuses, expected) in [
        (no_rows, "no `[statuses.<key>]` row found at all"),
        (
            no_fields.as_str(),
            "not one row declares `transitions_in` or `transitions_out`",
        ),
    ] {
        let report = run(statuses, WIRED);
        assert!(report.failed(), "empty read must fail: {expected}");
        names(&report, &[expected]);
    }
}

/// The third way, on the other file: the edge table read as empty. Every row
/// agrees with an empty table, so a green run here would mean the rule had
/// been switched off by a reformatting.
#[test]
fn an_empty_edge_table_fails_rather_than_agreeing_with_every_row() {
    let report = run(AGREEING, &[]);
    assert!(report.failed());
    names(&report, &[EDGES, "no `[[transitions]]` row"]);
}

/// The repository itself, both ways — and the reason this rule is worth
/// having in `nextest` as well as in the gate: the three rows #394 names were
/// found by an agent reading, and this is what reads.
#[test]
fn the_repository_agrees_with_itself() {
    let report = every_status_row_names_the_edges_it_carries(&crate::repo_root());
    let found = findings(&report);
    assert!(found.is_empty(), "{found:#?}");
}

/// Spacing inside an entry is not a disagreement. A registry written by hand
/// gets a double space in it eventually.
#[test]
fn the_arrow_is_read_whatever_the_spacing_around_it() {
    assert_eq!(split("a -> b"), Some(("a", "b")));
    assert_eq!(split("a->b"), Some(("a", "b")));
    assert_eq!(split("a  ->  b"), Some(("a", "b")));
    assert_eq!(split("a"), None);
    assert_eq!(split("a -> "), None);
    assert_eq!(split("a -> b -> c"), None);
}
