//! Forgetting a Job, and what still cannot be spelled after it.
//!
//! Three claims now. A Job that is forgotten takes its whole history with it,
//! so nothing is left for the fold to rebuild a half-Job from. The append-only
//! rule that made forgetting impossible before V4 still holds everywhere it
//! mattered: a transition cannot be deleted out from under a Job that exists.
//!
//! And **"its whole history" means every table, including the ones added after
//! the delete was written.** For most of this crate's life it did not:
//! `job_step_judgments`, `job_step_gaming_flags` and `job_step_evidence` each
//! arrived carrying a foreign key to `jobs` and none of them was ever deleted,
//! so the `jobs` row would not go and the whole transaction rolled back with
//! `FOREIGN KEY constraint failed`. It cost nothing while no shipped workflow
//! declared a `judge_check` and the tables stayed empty. The day nine of them
//! went live, `armada clean` stopped being able to forget any Job that reached
//! a gate. The two tests at the foot of this file are the two halves of not
//! having that again: one forgets a Job with rows in all of them, and one asks
//! the schema whether anything is unaccounted for.

use core_model::{Actor, Target};

use crate::schema::tables_pointing_at_a_job;
use crate::tests::attempt::{on_its_first_run, record_a_whole_run, run_it_again};
use crate::tests::{at, job_id, open, top_level, TempDir};
use crate::{Forgotten, LoadJobError, Store};

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
    assert_eq!(
        gone.attachments, 1,
        "the fixture's one attached file goes with it"
    );
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

/// The defect, in the shape a person met it: a Job the Judge ruled on, cleaned.
///
/// Two runs of the step and not one, because #144 keyed all four per-step
/// tables by attempt as well — a delete that named the run rather than the Job
/// would pass this with one run and leave the first run's rows behind with two.
#[test]
fn a_job_the_judge_ruled_on_can_still_be_forgotten() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01JUDGED";
    let job = on_its_first_run(&mut store, id);
    record_a_whole_run(&mut store, id, "the first note", "2026-08-26T10:05:00.000Z");
    run_it_again(
        &mut store,
        &job,
        "2026-08-26T10:06:00.000Z",
        "2026-08-26T10:07:00.000Z",
    );
    record_a_whole_run(
        &mut store,
        id,
        "the same note again",
        "2026-08-26T10:09:00.000Z",
    );

    let gone = store
        .forget_job(&job_id(id))
        .expect("a judged job is forgotten rather than refused by a foreign key");

    assert!(gone.existed);
    assert_eq!(
        gone.step_judgments, 2,
        "one verdict from each of the two runs"
    );
    assert_eq!(
        gone.step_gaming_flags, 2,
        "one flag from each of the two runs"
    );
    assert_eq!(
        gone.step_evidence, 2,
        "one submission from each of the two runs"
    );
    assert_eq!(
        gone.step_checks, 2,
        "one Check result from each of the two runs"
    );
    assert_eq!(
        gone.other, 0,
        "every table those rows are in has a count of its own"
    );
    assert!(matches!(
        store.load_job(&job_id(id)),
        Err(LoadJobError::NoSuchJob { .. })
    ));
}

/// The half the test above cannot cover: a table that exists and that no
/// fixture writes a row into.
///
/// `forget_job` empties it either way, the set being read out of the catalog —
/// so what is left to go wrong is the count, and this is what fails when a new
/// table's rows would be removed without being reported. The fix it asks for is
/// a field on `Forgotten` and an arm in `count_of`.
#[test]
fn every_table_that_points_at_a_job_is_counted_by_name() {
    let dir = TempDir::new();
    let store = open(&dir);

    let tables = tables_pointing_at_a_job(&store.conn).expect("the catalog answers");

    for missed in [
        "job_step_judgments",
        "job_step_gaming_flags",
        "job_step_evidence",
    ] {
        assert!(
            tables.iter().any(|table| table == missed),
            "the derived set holds {missed}, which the hand-kept list did not, \
             and it found: {tables:?}"
        );
    }
    let unnamed: Vec<&String> = tables
        .iter()
        .filter(|table| Forgotten::default().count_of(table).is_none())
        .collect();
    assert!(
        unnamed.is_empty(),
        "`Forgotten` has no field for {unnamed:?}: forgetting a Job empties them \
         and then reports fewer rows than it removed"
    );
}
