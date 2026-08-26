//! Every field the Job row holds, written and read back.
//!
//! The fixtures fill every `Option` and leave no array empty, because a
//! round-trip over an empty record exercises almost nothing and passes anyway.

use core_model::{Actor, Job, JobStatus, RepoPath, Target, WriteTargets};

use crate::tests::{created_at, job_id, open, sub_dispatched, top_level, TempDir};
use crate::{LoadAllError, WriteError};

#[test]
fn a_top_level_job_survives_with_every_field_intact() {
    let dir = TempDir::new();
    let stored = top_level("01FULL");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    assert_eq!(reopened.load_job(&job_id("01FULL")).expect("loads"), stored);
}

#[test]
fn a_sub_dispatched_job_keeps_the_step_that_dispatched_it() {
    let dir = TempDir::new();
    let stored = sub_dispatched("01SUB");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01SUB")).expect("loads");
    assert_eq!(loaded, stored);
    assert_eq!(loaded.status(), JobStatus::Queued, "its entry status");
    assert_eq!(
        loaded.dispatched_by().map(|by| by.step_id.as_str()),
        Some("plan")
    );
}

/// Null is not empty. Zero rows in `job_write_targets` cannot say which of the
/// two a Job means, so the Job row carries the discriminator — and this is what
/// would fail if it stopped doing so.
#[test]
fn undetermined_scope_and_determined_to_write_nothing_stay_apart() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    let undetermined = with_targets("01NULLSCOPE", None);
    let nothing = with_targets("01NOTHING", Some(WriteTargets::nothing()));
    let something = with_targets(
        "01SOMETHING",
        Some(WriteTargets::of(vec![RepoPath::new(
            "crates/store/src/lib.rs",
        )])),
    );
    for job in [&undetermined, &nothing, &something] {
        store.insert_job(job, &created_at()).expect("stored");
    }
    drop(store);

    let reopened = open(&dir);
    assert!(reopened
        .load_job(&job_id("01NULLSCOPE"))
        .expect("loads")
        .write_targets()
        .is_none());
    assert_eq!(
        reopened
            .load_job(&job_id("01NOTHING"))
            .expect("loads")
            .write_targets()
            .map(|targets| targets.paths().len()),
        Some(0)
    );
    assert_eq!(
        reopened
            .load_job(&job_id("01SOMETHING"))
            .expect("loads")
            .write_targets()
            .map(|targets| targets.paths().len()),
        Some(1)
    );
}

fn with_targets(id: &str, targets: Option<WriteTargets>) -> Job {
    let mut new = crate::tests::full_new_job(id);
    new.write_targets = targets;
    Job::create_top_level(new, core_model::TopLevelOrigin::Manual, created_at())
}

#[test]
fn creation_is_not_an_update() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01TWICE");
    store.insert_job(&job, &created_at()).expect("stored");
    match store.insert_job(&job, &created_at()) {
        Err(WriteError::JobAlreadyExists { job_id }) => assert_eq!(job_id.as_str(), "01TWICE"),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_transition_against_a_job_that_was_never_stored_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let moved = top_level("01GHOST")
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    match store.record_transition(&moved) {
        Err(WriteError::NoSuchJob { job_id }) => assert_eq!(job_id.as_str(), "01GHOST"),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// Never edited and never removed — enforced by the database, not by this
/// crate's discipline. There is no method here that would try either; these go
/// straight at the table to show the trigger is what stops them.
#[test]
fn the_log_refuses_to_be_edited_or_removed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01APPENDONLY");
    store.insert_job(&job, &created_at()).expect("stored");
    let moved = job
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&moved).expect("recorded");

    let edit = store
        .conn
        .execute("UPDATE job_events SET status_to = 'killed'", []);
    assert!(edit.is_err(), "a recorded transition is never edited");

    let remove = store.conn.execute("DELETE FROM job_events", []);
    assert!(remove.is_err(), "a recorded transition is never removed");

    assert_eq!(
        store
            .events_for(&job_id("01APPENDONLY"))
            .expect("still there")
            .len(),
        1
    );
}

#[test]
fn the_boot_read_returns_every_job() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for id in ["01ONE", "01TWO", "01THREE"] {
        store.insert_job(&top_level(id), &created_at()).expect("ok");
    }
    drop(store);

    let mut reopened = open(&dir);
    let loaded = reopened.load_all_jobs().expect("all three rebuild");
    assert_eq!(loaded.jobs.len(), 3);
    assert!(
        loaded.repaired.is_empty(),
        "nothing to repair on a clean file"
    );
}

/// The signature that makes the v1 bug unwritable: a caller cannot end up with
/// a shorter list and no error.
#[test]
fn one_unreadable_job_does_not_hide_and_does_not_take_the_others_down() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01GOOD"), &created_at())
        .expect("ok");
    store
        .insert_job(&top_level("01BAD"), &created_at())
        .expect("ok");
    store
        .conn
        .execute(
            "UPDATE jobs SET status = 'a status nobody has' WHERE job_id = '01BAD'",
            [],
        )
        .expect("scribbled on");

    match store.load_all_jobs() {
        Err(LoadAllError::SomeJobsUnreadable { loaded, failed }) => {
            assert_eq!(loaded.jobs.len(), 1, "the good one still came back");
            assert_eq!(failed.len(), 1, "and the bad one is named, not dropped");
        }
        other => panic!("expected both halves, found {other:?}"),
    }
}
