//! A person running a stopped step's Checks again: the row it writes, and the
//! two counts it must leave alone. #1105.

use core_model::{
    Actor, Attempt, EscalationTrigger, Spent, StepLevelTrigger, StepState, StepTarget, Target,
};

use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{at, job_id, open, TempDir};
use crate::Store;

/// The run a Drone worked and the retry budget it spent, as the log counts them.
fn counts(store: &Store, id: &str) -> (Attempt, Spent) {
    (
        store
            .step_attempt(&job_id(id), &step_id())
            .expect("the runs are counted"),
        store
            .step_spent(&job_id(id), &step_id())
            .expect("the budget is counted"),
    )
}

/// **The shape trigger admits the row, and neither count moves.** A re-run
/// crosses `stopped -> running` as a restart does; the trigger it carries is
/// what keeps it out of the step's runs, so the records it files land under the
/// Drone's run and no retry is spent.
#[test]
fn a_rerun_of_the_checks_is_recorded_and_is_not_a_run() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01RECHECKED";
    let why = StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger");

    let job = on_its_first_run(&mut store, id);
    let stopped = job
        .transition_step(
            &step_id(),
            StepTarget::Stopped(why),
            Actor::Fleet,
            at("2026-08-26T10:03:00.000Z"),
        )
        .expect("the step stops");
    store.record_step_transition(&stopped).expect("recorded");
    let held = stopped
        .job
        .transition(
            Target::AwaitingRepair,
            Actor::Fleet,
            at("2026-08-26T10:04:00.000Z"),
        )
        .expect("the Job is held for repair");
    store.record_transition(&held).expect("recorded");
    let before = counts(&store, id);

    let resumed = held
        .job
        .transition(
            Target::Running,
            Actor::Human,
            at("2026-08-26T10:05:00.000Z"),
        )
        .expect("a person takes it back to running");
    store.record_transition(&resumed).expect("recorded");
    let rechecking = resumed
        .job
        .transition_step(
            &step_id(),
            StepTarget::Rechecking(why),
            Actor::Human,
            at("2026-08-26T10:06:00.000Z"),
        )
        .expect("the step leaves stopped for the re-run");
    store
        .record_step_transition(&rechecking)
        .expect("a trigger on this row is admitted");

    assert_eq!(
        counts(&store, id),
        before,
        "no Drone worked it, so there is no new run and no retry spent"
    );
    assert_eq!(
        store
            .load_job(&job_id(id))
            .expect("the Job reads back")
            .step(&step_id())
            .map(|row| row.state()),
        Some(StepState::Running)
    );
}
