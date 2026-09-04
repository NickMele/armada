//! **A step redone by a loop is counted apart from a step retried after a
//! failure**, and both counts come off the same log.
//!
//! `job-fields.toml` types `retry_count` and `iteration_count` as `job_steps`
//! columns. Neither is one. `job_events` is append-only in the database itself
//! and already records every entry into `running` and the state each came from,
//! so a column beside it would be a second record of one fact and a pair that
//! can disagree — which is the argument `store::attempt` already made for the
//! first counter, holding for the second without change.
//!
//! Every run and every return here is reached by transitioning. Nothing writes
//! a state into a row, so the counts are facts about the log rather than
//! numbers this file chose.

use core_model::{Actor, Job, StepTarget};

use crate::tests::attempt::{on_its_first_run, run_it_again, step_id};
use crate::tests::{at, job_id, open, TempDir};
use crate::Store;

/// Advance the step and let a later verdict route back to it — the two moves
/// that make a second pass, and the only shape `STEP_EDGES` admits for one.
fn loop_back_to_it(store: &mut Store, job: &Job, advanced_at: &str, returned_at: &str) -> Job {
    let job = moved(store, job, StepTarget::Advanced, advanced_at);
    moved(store, &job, StepTarget::Returned, returned_at)
}

fn moved(store: &mut Store, job: &Job, to: StepTarget, when: &str) -> Job {
    let moved = job
        .transition_step(&step_id(), to, Actor::Fleet, at(when))
        .unwrap_or_else(|cause| panic!("moving the step: {cause}"));
    store
        .record_step_transition(&moved)
        .expect("the step move is recorded");
    moved.job
}

/// The pass is the log's answer, the way the attempt is. A step nothing has
/// routed back to is on its first pass — which is every step of every linear
/// workflow, and why the answer is never absent.
#[test]
fn the_pass_is_counted_off_the_log_and_nowhere_else() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01LOOPED";
    let job = job_id(id);

    let running = on_its_first_run(&mut store, id);
    assert_eq!(
        store
            .step_iteration(&job, &step_id())
            .expect("counted")
            .number(),
        1,
        "a step in its first run has been returned to no times"
    );

    let running = loop_back_to_it(
        &mut store,
        &running,
        "2026-08-26T10:06:00.000Z",
        "2026-08-26T10:07:00.000Z",
    );
    assert_eq!(
        store
            .step_iteration(&job, &step_id())
            .expect("counted")
            .number(),
        2,
        "one return is the second pass"
    );

    loop_back_to_it(
        &mut store,
        &running,
        "2026-08-26T10:08:00.000Z",
        "2026-08-26T10:09:00.000Z",
    );
    assert_eq!(
        store
            .step_iteration(&job, &step_id())
            .expect("counted")
            .number(),
        3
    );
    assert_eq!(
        store
            .step_iteration(&job, &core_model::StepId::new("reproduce"))
            .expect("counted")
            .number(),
        1,
        "and the count is over that step alone, so two loops cannot share one"
    );
}

/// The distinction the whole issue is about. A stop and a restart is a retry; an
/// advance and a return is an iteration. Both enter `running`, and only the
/// state each came from tells them apart.
#[test]
fn a_retry_and_a_loop_return_are_two_counts_over_one_log() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01BOTHKINDS";
    let job = job_id(id);

    let running = on_its_first_run(&mut store, id);
    let running = run_it_again(
        &mut store,
        &running,
        "2026-08-26T10:03:00.000Z",
        "2026-08-26T10:04:00.000Z",
    );
    assert_eq!(
        store
            .step_iteration(&job, &step_id())
            .expect("counted")
            .number(),
        1,
        "a retry is the same pass going round again and never moves this one"
    );
    assert_eq!(
        store
            .step_attempt(&job, &step_id())
            .expect("counted")
            .number(),
        2
    );

    let running = loop_back_to_it(
        &mut store,
        &running,
        "2026-08-26T10:05:00.000Z",
        "2026-08-26T10:06:00.000Z",
    );
    assert_eq!(
        store
            .step_iteration(&job, &step_id())
            .expect("counted")
            .number(),
        2
    );
    assert_eq!(
        store
            .step_attempt(&job, &step_id())
            .expect("counted")
            .number(),
        3,
        "and a return *is* another run, which is why its evidence is filed apart"
    );

    run_it_again(
        &mut store,
        &running,
        "2026-08-26T10:07:00.000Z",
        "2026-08-26T10:08:00.000Z",
    );
    assert_eq!(
        store
            .step_iteration(&job, &step_id())
            .expect("counted")
            .number(),
        2,
        "a failure inside the second pass is still the second pass"
    );
    assert_eq!(
        store
            .step_attempt(&job, &step_id())
            .expect("counted")
            .number(),
        4
    );
}

/// The fold's half. A loop return carries no trigger, so nothing but the state
/// it left says it was one — and a Job that had looped would be unreadable off
/// its own log if the fold read only the destination.
#[test]
fn a_job_that_looped_reads_back_off_its_own_log() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01REPLAYLOOP";
    let job = job_id(id);

    let running = on_its_first_run(&mut store, id);
    let looped = loop_back_to_it(
        &mut store,
        &running,
        "2026-08-26T10:06:00.000Z",
        "2026-08-26T10:07:00.000Z",
    );

    let read = store.load_job(&job).expect("the job reads back");
    assert_eq!(read, looped, "the fold rebuilds what the moves left");
    assert_eq!(
        read.current_step_id(),
        Some(&step_id()),
        "and the cursor came back where the return put it"
    );
}
