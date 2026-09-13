//! A kept review reads back whole, and a run's next submission replaces it. #903.

use core_model::{
    Area, Bucket, ChangedTest, Confidence, Finding, Proves, ReviewRecord, TestChange,
    TestsInChange, Untested, ViewStep,
};

use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{at, job_id, open, TempDir};

fn removed_without_a_reason() -> ChangedTest {
    ChangedTest {
        name: "a_loaded_machine_holds_a_job_back_too".to_string(),
        change: TestChange::Removed {
            replaced_by: Some("a_machine_whose_cpu_is_saturated_still_admits".to_string()),
        },
        why: None,
    }
}

fn a_review(confidence: Confidence, reason: &str) -> ReviewRecord {
    ReviewRecord {
        confidence,
        reasons: vec![reason.to_string()],
        areas: vec![
            Area::of(
                "Admission",
                "CPU no longer holds a Job",
                &[
                    "crates/fleet/src/headroom.rs",
                    "crates/fleet/src/admitting.rs",
                ],
            )
            .viewed(vec![
                ViewStep {
                    file: "crates/fleet/src/headroom.rs".to_string(),
                    hunk: "@@ -40,6 +40,4 @@".to_string(),
                    summary: "CPU is no longer a way to be short".to_string(),
                    tie_to_next: Some("admitting.rs turned that shortage into a hold".to_string()),
                },
                ViewStep {
                    file: "crates/fleet/src/admitting.rs".to_string(),
                    hunk: "@@ -212,7 +212,6 @@".to_string(),
                    summary: "The hold goes".to_string(),
                    tie_to_next: None,
                },
            ]),
            Area::of(
                "Tests",
                "The hold's test asserts the opposite",
                &["crates/fleet/src/tests/headroom.rs"],
            ),
        ],
        tests: TestsInChange {
            proves: vec![Proves::of(
                "Admission",
                "A saturated CPU holds nothing back",
                2,
            )],
            changed: vec![removed_without_a_reason()],
            untested: vec![Untested::of(
                "The status bar entry",
                "No test opens the sheet from it",
            )],
        },
        findings: vec![
            Finding::of(
                Bucket::NeedsYou,
                "`a_loaded_machine_holds_a_job_back_too` was removed with no reason given",
                "A test taken out or weakened is the reviewer's to explain",
            ),
            Finding::of(
                Bucket::ForContext,
                "The lock order when saving",
                "The author flagged it",
            )
            .viewed(vec![ViewStep {
                file: "crates/fleet/src/admitting.rs".to_string(),
                hunk: "@@ -212,7 +212,6 @@".to_string(),
                summary: "The save takes the lock".to_string(),
                tie_to_next: None,
            }]),
        ],
        unexplained_tests: vec![removed_without_a_reason()],
    }
}

#[test]
fn a_kept_review_reads_back_whole() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01REVIEWED");
    let kept = a_review(Confidence::Confident, "Every Check passed.");
    store
        .record_confidence(
            &job_id("01REVIEWED"),
            &step_id(),
            &kept,
            &at("2026-09-13T10:00:00.000Z"),
        )
        .expect("the review is kept");

    let read = store
        .confidence_record(&job_id("01REVIEWED"))
        .expect("the review reads back")
        .expect("a review was kept");
    assert_eq!(read, kept);
}

#[test]
fn a_second_submission_in_the_same_run_replaces_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01REVIEWED");
    let first = a_review(Confidence::NotConfident, "The first reading.");
    let second = a_review(Confidence::Confident, "The second reading.");
    store
        .record_confidence(
            &job_id("01REVIEWED"),
            &step_id(),
            &first,
            &at("2026-09-13T10:00:00.000Z"),
        )
        .expect("the first is kept");
    store
        .record_confidence(
            &job_id("01REVIEWED"),
            &step_id(),
            &second,
            &at("2026-09-13T10:01:00.000Z"),
        )
        .expect("the second replaces it");

    let read = store
        .confidence_record(&job_id("01REVIEWED"))
        .expect("the review reads back")
        .expect("a review was kept");
    assert_eq!(read, second, "one run holds one review, the latest");
}

#[test]
fn a_job_nobody_reviewed_reads_as_no_review() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01UNREVIEWED");
    assert_eq!(
        store
            .confidence_record(&job_id("01UNREVIEWED"))
            .expect("the read answers"),
        None
    );
}

/// A dismissal is kept with its reason, and dismissing the same finding again replaces it. #907.
#[test]
fn a_dismissal_reads_back_and_a_second_replaces_its_reason() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01DISMISSED");
    let job = job_id("01DISMISSED");
    let first = core_model::Dismissal {
        finding: "The lock order when saving".to_string(),
        reason: "The first reading.".to_string(),
    };
    let second = core_model::Dismissal {
        reason: "Saving takes one lock, so there is no order.".to_string(),
        ..first.clone()
    };
    store
        .record_dismissal(&job, &first, &at("2026-09-13T10:00:00.000Z"))
        .expect("the dismissal is kept");
    store
        .record_dismissal(&job, &second, &at("2026-09-13T10:01:00.000Z"))
        .expect("the second replaces it");

    assert_eq!(
        store.dismissals(&job).expect("the dismissals read"),
        vec![second]
    );
    assert_eq!(
        store
            .dismissals(&job_id("01REVIEWED"))
            .expect("the read answers"),
        vec![],
        "a Job nobody dismissed anything on has none"
    );
}
