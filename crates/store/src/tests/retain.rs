//! Retaining a Job's record while giving its resources back.
//!
//! `retain_job` is the other half of [`forget_job`](crate::Store::forget_job):
//! where that clears every table beneath a Job, this clears exactly one —
//! `job_drone_process`, the pid a worktree's removal leaves stale — and
//! touches nothing else. The two claims worth breaking are the two directions:
//! leaving that one table's row behind, or clearing more than it names.

use core_model::{Actor, DroneId, JobId, StepId, Target, Timestamp};

use crate::migrations::tables_pointing_at_a_job;
use crate::tests::attempt::{on_its_first_run, record_a_whole_run, run_it_again};
use crate::tests::{at, job_id, open, top_level, ulid, TempDir};
use crate::{DroneProcess, Store};

/// A Job in the store with one transition recorded against it — the same
/// fixture `tests::forget` uses, so the two files are asking the same question
/// of the two verbs that answer it differently.
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

fn a_process(job: &str, drone: &str, pid: u32) -> DroneProcess {
    DroneProcess {
        job_id: JobId::carried(ulid(job)),
        step_id: StepId::new("fix"),
        drone_id: DroneId::carried(ulid(drone)),
        pid,
        started_at: String::from("Wed  3 Sep 01:14:07 2026"),
        spawned_at: Timestamp::from_rfc3339("2026-09-03T01:14:07.000Z"),
    }
}

#[test]
fn a_retained_jobs_row_survives_with_its_status() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01RETAINED");

    let kept = store
        .retain_job(&job_id("01RETAINED"), &at("2026-09-09T00:00:00.000Z"))
        .expect("the job is retained");

    assert!(kept.existed);
    assert!(!kept.drone_process, "nothing was spawned for this job");
    let job = store
        .load_job(&job_id("01RETAINED"))
        .expect("the row is still there to read");
    assert_eq!(job.status(), core_model::JobStatus::Queued);
}

/// The mark a Board reads to tell a cleared Job from a finished one. Without
/// it, a clear from the board looks like it did nothing.
#[test]
fn retaining_a_job_stamps_when_it_was_reclaimed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01RETAINED");

    store
        .retain_job(&job_id("01RETAINED"), &at("2026-09-09T00:00:00.000Z"))
        .expect("the job is retained");

    let job = store
        .load_job(&job_id("01RETAINED"))
        .expect("the row is still there to read");
    assert_eq!(
        job.reclaimed_at(),
        Some(&at("2026-09-09T00:00:00.000Z")),
        "the instant the caller supplied is what the row now says"
    );
}

/// The neighbour is the whole point, exactly as it is in `tests::forget`:
/// retaining one Job's record must not touch another's.
#[test]
fn retaining_one_job_leaves_every_other_job_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01RETAINED");
    a_job_with_a_history(&mut store, "01UNTOUCHED");

    store
        .retain_job(&job_id("01RETAINED"), &at("2026-09-09T00:00:00.000Z"))
        .expect("the job is retained");

    let untouched = store
        .load_job(&job_id("01UNTOUCHED"))
        .expect("the other job");
    assert_eq!(untouched.status(), core_model::JobStatus::Queued);
}

/// Nothing to retain is the state the caller asked for, not a failure — the
/// same reading `forget_job` gives an id naming no Job.
#[test]
fn an_id_naming_no_job_is_answered_rather_than_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    let kept = store
        .retain_job(&job_id("01NEVEREXISTED"), &at("2026-09-09T00:00:00.000Z"))
        .expect("an absent job is not an error");

    assert!(!kept.existed);
    assert!(!kept.drone_process);
}

/// A live pid is the one resource `retain_job` exists to reclaim.
#[test]
fn retaining_a_job_clears_its_drone_process() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01WITHDRONE");
    store
        .record_drone_process(&a_process("01WITHDRONE", "01DRONE0000000000000001", 4096))
        .expect("the process is recorded");

    let kept = store
        .retain_job(&job_id("01WITHDRONE"), &at("2026-09-09T00:00:00.000Z"))
        .expect("the job is retained");

    assert!(kept.drone_process, "the live pid was cleared");
    assert_eq!(
        store
            .drone_process(&job_id("01WITHDRONE"))
            .expect("the read succeeds"),
        None,
        "a worktree's removal makes the pid stale, and retain clears it"
    );
    assert!(
        store.load_job(&job_id("01WITHDRONE")).is_ok(),
        "the row the pid pointed at is untouched"
    );
}

/// The ordinary shape by the time a sweep reaches a terminal Job: its Drone
/// has already left, and there is nothing here for `retain_job` to clear.
#[test]
fn retaining_a_job_with_no_drone_process_reports_none_cleared() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01NODRONE");

    let kept = store
        .retain_job(&job_id("01NODRONE"), &at("2026-09-09T00:00:00.000Z"))
        .expect("the job is retained");

    assert!(kept.existed);
    assert!(!kept.drone_process);
}

/// The `DELETE` is scoped by `job_id`. A version missing the `WHERE` clause
/// would pass every test above and still wipe every Drone's process on the
/// machine.
#[test]
fn retaining_a_job_leaves_a_neighbours_drone_process_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01RETAINED");
    a_job_with_a_history(&mut store, "01STILLRUNNING");
    store
        .record_drone_process(&a_process(
            "01STILLRUNNING",
            "01DRONE0000000000000002",
            8192,
        ))
        .expect("the process is recorded");

    store
        .retain_job(&job_id("01RETAINED"), &at("2026-09-09T00:00:00.000Z"))
        .expect("the job is retained");

    assert_eq!(
        store
            .drone_process(&job_id("01STILLRUNNING"))
            .expect("the read succeeds")
            .map(|held| held.pid),
        Some(8192),
        "a neighbour's live Drone is not this retain's business"
    );
}

/// **The defect this file exists to catch.** Every table `forget_job` walks
/// except `job_drone_process` is a record, not a resource, and `retain_job`
/// must leave every one of them exactly as it found them — the Job the Judge
/// ruled on twice, with both runs still on it.
#[test]
fn retaining_a_job_the_judge_ruled_on_keeps_every_table_beneath_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01JUDGEDKEPT";
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

    let tables: Vec<String> = tables_pointing_at_a_job(&store.conn)
        .expect("the catalog answers")
        .into_iter()
        .filter(|table| table != "job_drone_process")
        .collect();
    let before: Vec<(String, i64)> = tables
        .iter()
        .map(|table| (table.clone(), count(&store, table, id)))
        .collect();
    assert!(
        before
            .iter()
            .any(|(table, count)| table == "job_events" && *count > 0),
        "the fixture wrote a history to keep: {before:?}"
    );
    assert!(
        before
            .iter()
            .any(|(table, count)| table == "job_step_judgments" && *count > 0),
        "the fixture wrote at least one verdict to keep: {before:?}"
    );

    let kept = store
        .retain_job(&job_id(id), &at("2026-09-09T00:00:00.000Z"))
        .expect("a judged job is retained rather than refused by a foreign key");
    assert!(kept.existed);

    for (table, before) in before {
        assert_eq!(
            count(&store, &table, id),
            before,
            "{table} is a record, not a resource, and retain_job must not touch it"
        );
    }
    assert!(
        store.load_job(&job_id(id)).is_ok(),
        "the row every one of those tables points at is still there"
    );
}

fn count(store: &Store, table: &str, id: &str) -> i64 {
    store
        .conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE job_id = ?1"),
            (id,),
            |row| row.get(0),
        )
        .unwrap_or_else(|why| panic!("counting {table}: {why}"))
}
