//! What the Manifest read at Job creation, kept whole — on a fresh store,
//! where the migration itself is `tests::migrate`'s.

use crate::tests::{created_at, job_id, open, top_level};
use crate::{LoadJobError, WriteError};

#[test]
fn a_fresh_job_carries_no_snapshot_until_one_is_set() {
    let mut store = open(&crate::tests::TempDir::new());
    store
        .insert_job(&top_level("01FRESH"), &created_at())
        .expect("the job is stored");

    assert_eq!(
        store
            .manifest_snapshot(&job_id("01FRESH"))
            .expect("the column reads"),
        None,
        "nothing has set one yet"
    );
}

#[test]
fn a_snapshot_that_is_set_is_read_back_whole() {
    let mut store = open(&crate::tests::TempDir::new());
    store
        .insert_job(&top_level("01SNAPPED"), &created_at())
        .expect("the job is stored");

    let text = "version: 1\nid: 01M\ncommands:\n  build:\n    run: cargo build\n";
    store
        .set_manifest_snapshot(&job_id("01SNAPPED"), text)
        .expect("the write succeeds");

    assert_eq!(
        store
            .manifest_snapshot(&job_id("01SNAPPED"))
            .expect("the column reads"),
        Some(text.to_string()),
        "the literal text comes back, not a re-derived one"
    );
}

/// A second call **overwrites** — the shape a re-snapshot needs, whichever
/// caller makes it. This module does not choose who may call it twice.
#[test]
fn setting_a_snapshot_twice_replaces_the_first() {
    let mut store = open(&crate::tests::TempDir::new());
    store
        .insert_job(&top_level("01REVISED"), &created_at())
        .expect("the job is stored");
    store
        .set_manifest_snapshot(&job_id("01REVISED"), "version: 1\nid: 01M\n")
        .expect("the first write");
    store
        .set_manifest_snapshot(&job_id("01REVISED"), "version: 1\nid: 01M\nbase: main\n")
        .expect("the second write");

    assert_eq!(
        store
            .manifest_snapshot(&job_id("01REVISED"))
            .expect("the column reads"),
        Some("version: 1\nid: 01M\nbase: main\n".to_string())
    );
}

#[test]
fn reading_a_snapshot_for_a_job_nobody_stored_is_named_rather_than_empty() {
    let store = open(&crate::tests::TempDir::new());
    match store.manifest_snapshot(&job_id("01ABSENT")) {
        Err(LoadJobError::NoSuchJob { job_id }) => assert_eq!(job_id.as_str(), "01ABSENT"),
        other => panic!("expected NoSuchJob, found {other:?}"),
    }
}

#[test]
fn setting_a_snapshot_for_a_job_nobody_stored_is_refused() {
    let mut store = open(&crate::tests::TempDir::new());
    match store.set_manifest_snapshot(&job_id("01ABSENT"), "version: 1\n") {
        Err(WriteError::NoSuchJob { job_id }) => assert_eq!(job_id.as_str(), "01ABSENT"),
        other => panic!("expected NoSuchJob, found {other:?}"),
    }
}
