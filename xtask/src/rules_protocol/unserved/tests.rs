//! The rule's own negative tests.
//!
//! #697 found ten queries carrying `agent_access = "Yes"` and no route at all,
//! invisible because every rule reading this seam ran from the routes outward.
//! These prove the reverse direction stays proven, and that the allowance
//! cannot quietly become a list of names with no argument in it.

use super::*;
use crate::Finding;

fn findings(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

fn failures(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .filter_map(|f| match f {
            Finding::Fail(what) => Some(what.clone()),
            Finding::Warn(_) => None,
        })
        .collect()
}

const TABLE_SOURCE: &str = r#"
pub const SERVED: &[Route] = &[
    Route {
        operation: "list_jobs",
        method: "GET",
        path: "/jobs",
    },
];
"#;

/// The exact shape #697 found: a name in the inventory, reachable by an agent,
/// and served nowhere.
#[test]
fn an_operation_with_no_route_and_no_reason_fails() {
    let inventory = r#"
[operations.list_jobs]
kind = "query"

[operations.get_health]
kind = "query"
"#;
    let mut report = Report::new("test");
    check(inventory, TABLE_SOURCE, &mut report);
    let failed = failures(&report);
    assert!(
        failed.iter().any(|said| said.contains("get_health")),
        "the unrouted operation must be named: {failed:?}"
    );
    assert!(
        !failed.iter().any(|said| said.contains("list_jobs")),
        "a routed operation must not fail: {failed:?}"
    );
}

/// The allowance is what makes the rule usable, and it is per operation.
#[test]
fn an_operation_named_in_the_allowance_does_not_fail() {
    let inventory = r#"
[operations.list_jobs]
kind = "query"

[operations.pause_job]
kind = "command"
"#;
    let mut report = Report::new("test");
    check(inventory, TABLE_SOURCE, &mut report);
    assert!(failures(&report).is_empty(), "{:?}", failures(&report));
}

/// **The allowance shrinks on its own.** An exemption for something that is
/// built is an argument nobody will reread.
#[test]
fn an_allowance_for_something_that_is_served_fails() {
    let inventory = r#"
[operations.pause_job]
kind = "command"
"#;
    let table = r#"
pub const SERVED: &[Route] = &[
    Route {
        operation: "pause_job",
        method: "POST",
        path: "/jobs/:job_id/pause",
    },
];
"#;
    let mut report = Report::new("test");
    check(inventory, table, &mut report);
    assert!(
        failures(&report)
            .iter()
            .any(|said| said.contains("pause_job") && said.contains("NOT_BUILT")),
        "{:?}",
        failures(&report)
    );
}

/// A rule that compares nothing passes on every repository, including a broken
/// one.
#[test]
fn an_empty_inventory_or_an_empty_table_is_refused_rather_than_passed() {
    let mut report = Report::new("test");
    check("", TABLE_SOURCE, &mut report);
    assert!(!failures(&report).is_empty());

    let mut report = Report::new("test");
    check(
        "[operations.list_jobs]\nkind = \"query\"\n",
        "",
        &mut report,
    );
    assert!(!failures(&report).is_empty());
}

/// **A bare exemption list is the same silent default `#696` existed to
/// remove.** The gate prints the reason, so it must be there to print.
#[test]
fn every_allowance_carries_a_reason_the_gate_prints() {
    for (operation, because) in NOT_BUILT {
        assert!(
            because.len() > 60,
            "`{operation}` is exempt with {} characters of reason, which is a name and not an \
             argument",
            because.len()
        );
    }
    let mut report = Report::new("test");
    check(
        "[operations.pause_job]\nkind = \"command\"\n",
        TABLE_SOURCE,
        &mut report,
    );
    assert!(
        findings(&report)
            .iter()
            .any(|said| said.contains("Retired 2026-09-03")),
        "the reason is printed beside the rule, not only held in the source"
    );
}
