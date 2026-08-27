//! Forgetting a Job, and what still cannot be spelled after it.
//!
//! Two claims. A Job that is forgotten takes its whole history with it, so
//! nothing is left for the fold to rebuild a half-Job from. And the
//! append-only rule that made forgetting impossible before V4 still holds
//! everywhere it mattered: a transition cannot be deleted out from under a Job
//! that exists.

use core_model::{Actor, Target};

use crate::tests::{at, job_id, open, top_level, TempDir};
use crate::{LoadJobError, Store};

/// A Job in the store with one transition recorded against it.
fn a_job_with_a_history(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
    let moved = job
        .transition(Target::Queued, Actor::Human, at("2026-08-26T10:00:00.000Z"))
        .expect("approval is a legal move");
    store
        .record_transition(&moved)
        .expect("the transition is recorded");
}

#[test]
fn a_forgotten_job_takes_its_whole_history_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01FORGOTTEN");

    let gone = store
        .forget_job(&job_id("01FORGOTTEN"))
        .expect("the job is forgotten");

    assert!(gone.existed);
    assert_eq!(gone.events, 1);
    assert_eq!(gone.steps, 2, "the two seeded steps go with it");
    assert_eq!(gone.write_targets, 2);
    assert_eq!(gone.manifests, 2);
    assert!(matches!(
        store.load_job(&job_id("01FORGOTTEN")),
        Err(LoadJobError::NoSuchJob { .. })
    ));
}

/// The neighbour is the whole point: `clean` forgets one Manifest's Jobs and
/// another Manifest's history is not its business.
#[test]
fn forgetting_one_job_leaves_every_other_job_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01FORGOTTEN");
    a_job_with_a_history(&mut store, "01KEPT");

    store
        .forget_job(&job_id("01FORGOTTEN"))
        .expect("the job is forgotten");

    let kept = store.load_job(&job_id("01KEPT")).expect("the other job");
    assert_eq!(kept.id(), &job_id("01KEPT"));
    assert_eq!(kept.status(), core_model::JobStatus::Queued);
}

/// Nothing to forget is the state the caller asked for, not a failure.
#[test]
fn an_id_naming_no_job_is_answered_rather_than_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    let gone = store
        .forget_job(&job_id("01NEVEREXISTED"))
        .expect("an absent job is not an error");

    assert!(!gone.existed);
    assert_eq!(gone.events, 0);
}

/// V4 narrowed the trigger; it did not lift it. A `DELETE` aimed at one
/// transition of a live Job is still refused by the database itself.
#[test]
fn a_transition_still_cannot_be_removed_from_a_job_that_exists() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01STANDING");

    let refused = store
        .conn
        .execute("DELETE FROM job_events WHERE job_id = ?1", ("01STANDING",))
        .expect_err("the append-only trigger refuses this");

    assert!(
        refused.to_string().contains("append-only"),
        "the database says why, and it said: {refused}"
    );
}
