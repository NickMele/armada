//! The rule's own negative tests.
//!
//! #124 found `drone.spawned` and `drone.exited` on the wire and absent from
//! `SERVED`, and nothing had caught it — the check ran the operation
//! inventory against the table but never asked whether every published event
//! kind was in it. This proves the fix stays proven: a variant `Event` adds
//! and `SERVED` does not carry fails here before it ever reaches the gate.

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

const INVENTORY_SOURCE: &str = r##"
[operations.list_jobs]
kind = "query"

[operations."job.created"]
kind = "event"

[operations."drone.spawned"]
kind = "event"
unbuilt = "#1"
"##;

const TABLE_SOURCE: &str = r#"
pub const SERVED: &[Route] = &[
    Route {
        operation: "list_jobs",
        method: "GET",
        path: "/jobs",
    },
    Route {
        operation: "job.created",
        method: "GET",
        path: "/events",
    },
];
"#;

const ROUTER_SOURCE: &str = r#"
Router::new()
    .route("/jobs", get(list_jobs::<D>))
    .route("/events", get(events::<D>))
"#;

/// One enum variant beyond what `TABLE_SOURCE` lists — `drone.spawned`, the
/// exact shape #124 found.
const EVENT_SOURCE_WITH_UNLISTED_KIND: &str = r#"
#[serde(tag = "kind")]
pub enum Event {
    #[serde(rename = "job.created")]
    JobCreated(JobCreated),
    #[serde(rename = "drone.spawned")]
    DroneSpawned(DroneSpawned),
}
"#;

const EVENT_SOURCE_FULLY_LISTED: &str = r#"
#[serde(tag = "kind")]
pub enum Event {
    #[serde(rename = "job.created")]
    JobCreated(JobCreated),
}
"#;

fn table_and_router(table: &str) -> String {
    format!("{table}\n{ROUTER_SOURCE}")
}

#[test]
fn a_variant_the_table_does_not_list_fails() {
    let mut report = Report::new("test");
    check(
        INVENTORY_SOURCE,
        &table_and_router(TABLE_SOURCE),
        EVENT_SOURCE_WITH_UNLISTED_KIND,
        &mut report,
    );
    assert!(report.failed());
    let findings = findings(&report);
    assert!(
        findings
            .iter()
            .any(|f| f.contains("drone.spawned") && f.contains(EVENT_ENUM)),
        "expected a finding naming the unlisted kind: {findings:?}"
    );
}

#[test]
fn every_variant_listed_is_clean() {
    let mut report = Report::new("test");
    check(
        INVENTORY_SOURCE,
        &table_and_router(TABLE_SOURCE),
        EVENT_SOURCE_FULLY_LISTED,
        &mut report,
    );
    assert!(!report.failed(), "{:?}", findings(&report));
}

#[test]
fn parsing_reads_every_rename_between_the_braces_and_nothing_after() {
    let source = format!(
        "{EVENT_SOURCE_WITH_UNLISTED_KIND}\n#[serde(rename = \"after.the.enum\")]\nstruct Other;"
    );
    let kinds = published_event_kinds(&source);
    assert_eq!(kinds, vec!["job.created", "drone.spawned"]);
}

/// An event with no variant and no `unbuilt` — #645's own finding, the shape
/// `alert.raised` and its three siblings sat in for two weeks.
#[test]
fn a_declared_event_with_no_variant_and_no_mark_fails() {
    const INVENTORY_WITH_UNMARKED_EVENT: &str = r#"
[operations."job.created"]
kind = "event"

[operations."drone.spawned"]
kind = "event"
"#;
    let mut report = Report::new("test");
    check(
        INVENTORY_WITH_UNMARKED_EVENT,
        &table_and_router(TABLE_SOURCE),
        EVENT_SOURCE_FULLY_LISTED,
        &mut report,
    );
    let findings = findings(&report);
    assert!(report.failed(), "{findings:?}");
    assert!(
        findings
            .iter()
            .any(|f| f.contains("drone.spawned") && f.contains("no variant")),
        "expected a finding naming the unmarked, unsent event: {findings:?}"
    );
}

/// A mark on an event `Event` already publishes is the same lie in reverse.
#[test]
fn a_mark_on_an_event_that_is_sent_fails() {
    const EVENT_SOURCE_BOTH_LISTED: &str = r#"
#[serde(tag = "kind")]
pub enum Event {
    #[serde(rename = "job.created")]
    JobCreated(JobCreated),
    #[serde(rename = "drone.spawned")]
    DroneSpawned(DroneSpawned),
}
"#;
    let mut report = Report::new("test");
    check(
        INVENTORY_SOURCE,
        &table_and_router(TABLE_SOURCE),
        EVENT_SOURCE_BOTH_LISTED,
        &mut report,
    );
    let findings = findings(&report);
    assert!(report.failed(), "{findings:?}");
    assert!(
        findings
            .iter()
            .any(|f| f.contains("drone.spawned") && f.contains("already publishes")),
        "expected a finding naming the marked, sent event: {findings:?}"
    );
}

/// A marked, unsent event is the deliberately-ahead case: it warns, on
/// `unbuilt`'s own precedent, and does not fail the gate.
#[test]
fn a_marked_unsent_event_warns_and_does_not_fail() {
    let mut report = Report::new("test");
    check(
        INVENTORY_SOURCE,
        &table_and_router(TABLE_SOURCE),
        EVENT_SOURCE_FULLY_LISTED,
        &mut report,
    );
    let findings = findings(&report);
    assert!(
        findings
            .iter()
            .any(|f| f.contains("drone.spawned") && f.contains("not sent yet")),
        "expected a warning naming the marked event: {findings:?}"
    );
}

/// `unbuilt`'s value is an issue reference or nothing, on `actions.toml`'s own
/// rule for the column of the same name.
#[test]
fn an_unbuilt_value_that_is_not_an_issue_reference_fails() {
    const INVENTORY_WITH_BARE_MARK: &str = r#"
[operations."job.created"]
kind = "event"

[operations."drone.spawned"]
kind = "event"
unbuilt = "soon"
"#;
    let mut report = Report::new("test");
    check(
        INVENTORY_WITH_BARE_MARK,
        &table_and_router(TABLE_SOURCE),
        EVENT_SOURCE_FULLY_LISTED,
        &mut report,
    );
    let findings = findings(&report);
    assert!(report.failed(), "{findings:?}");
    assert!(
        findings
            .iter()
            .any(|f| f.contains("\"soon\"") && f.contains("not an issue reference")),
        "expected a finding naming the bare mark: {findings:?}"
    );
}

/// **The gate's own failure mode.** A rule that finds nothing to compare must
/// not read as agreement — that is a hand-kept inventory silently tolerating a
/// missing row, one level up from what #124 was about. Both ways the parser
/// can find nothing — the enum renamed or moved, and the enum present but
/// reformatted past every `#[serde(rename = ...)]` line it matches — fail
/// rather than pass on zero comparisons.
#[test]
fn no_variants_found_fails_rather_than_passing_on_nothing() {
    for source in [
        "// pub enum Event moved to another file",
        "pub enum Event {\n}\n",
    ] {
        let mut report = Report::new("test");
        check(
            INVENTORY_SOURCE,
            &table_and_router(TABLE_SOURCE),
            source,
            &mut report,
        );
        assert!(report.failed(), "empty parse must fail: {source:?}");
        let findings = findings(&report);
        assert!(
            findings
                .iter()
                .any(|f| f.contains(EVENT_ENUM) && f.contains("no")),
            "expected a finding saying the parser found nothing: {findings:?}"
        );
    }
}
