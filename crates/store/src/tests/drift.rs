//! A live scope-drift finding survives past the turn that saw it, which is
//! `crate::drift`'s whole capability.

use core_model::RepoPath;

use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{at, job_id, open, TempDir};

#[test]
fn a_fresh_path_is_kept_and_read_back_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01DRIFT");
    store
        .record_scope_drift(
            &job_id("01DRIFT"),
            &step_id(),
            &[RepoPath::new("src/log.rs")],
            &at("2026-09-04T18:08:45.000Z"),
        )
        .expect("the first path is kept");
    store
        .record_scope_drift(
            &job_id("01DRIFT"),
            &step_id(),
            &[RepoPath::new("src/parse.rs")],
            &at("2026-09-04T18:09:00.000Z"),
        )
        .expect("the second path is kept");

    let seen = store.scope_drift(&job_id("01DRIFT")).expect("reads back");
    let paths: Vec<&str> = seen.iter().map(|one| one.path.as_str()).collect();
    assert_eq!(paths, vec!["src/log.rs", "src/parse.rs"]);
}

// A path seen twice keeps its first instant — the whole idempotence claim.
#[test]
fn a_path_seen_twice_keeps_its_first_instant() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01TWICE");
    store
        .record_scope_drift(
            &job_id("01TWICE"),
            &step_id(),
            &[RepoPath::new("src/log.rs")],
            &at("2026-09-04T18:08:45.000Z"),
        )
        .unwrap();
    store
        .record_scope_drift(
            &job_id("01TWICE"),
            &step_id(),
            &[RepoPath::new("src/log.rs")],
            &at("2026-09-04T21:00:00.000Z"),
        )
        .unwrap();

    let seen = store.scope_drift(&job_id("01TWICE")).unwrap();
    let [row] = seen.as_slice() else {
        panic!("one row, not two: {seen:?}");
    };
    assert_eq!(row.first_seen_at, at("2026-09-04T18:08:45.000Z"));
}

#[test]
fn a_job_with_no_drift_reads_back_empty() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01CLEAN");
    assert!(store.scope_drift(&job_id("01CLEAN")).unwrap().is_empty());
}

/// Forgetting a Job takes its drift findings with it, counted by name.
#[test]
fn forgetting_a_job_forgets_its_drift() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01GONE");
    store
        .record_scope_drift(
            &job_id("01GONE"),
            &step_id(),
            &[RepoPath::new("src/log.rs")],
            &at("2026-09-04T18:08:45.000Z"),
        )
        .unwrap();

    let removed = store.forget_job(&job_id("01GONE")).expect("forgotten");

    assert_eq!(removed.scope_drift, 1);
    assert_eq!(removed.other, 0, "every table this build knows is named");
}
