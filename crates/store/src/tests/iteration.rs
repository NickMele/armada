//! **A step redone by a loop is counted apart from a step retried after a
//! failure**, and both counts come off the same log.
//!
//! `job-fields.toml` types `retry_count` and `iteration_count` as `job_steps`
//! columns. Neither is one: `job_events` is append-only in the database itself
//! and already records every entry into `running` and the state each came from,
//! so a column beside it would be a pair that can disagree. That is the
//! argument `store::attempt` made for the first counter, holding for the
//! second without change.
//!
//! # Whose count it is
//!
//! `iteration_count` is the **emitting** step's — the step that answered
//! `request_changes` — and not the step the verdict routes back to.
//! `docs/journeys/triage-queue.md` settles it: the cap and the count it bounds
//! must live on one step or `loop_cap` never fires, and two loops sharing a
//! target step would otherwise sum into one number. The emitter makes no move
//! of its own on a return, so the row the routed-to step writes names it.
//!
//! The shape is Design Plan's, with this crate's two-step fixture standing in:
//! `reproduce` is the step redone and `fix` is the gate that sends it back.
//! Every run and every return is reached by transitioning, so the counts are
//! facts about the log rather than numbers this file chose.
use core_model::{Actor, EscalationTrigger, Job, StepId, StepLevelTrigger, StepTarget};

use crate::tests::attempt::on_its_first_run;
use crate::tests::{at, job_id, open, TempDir};
use crate::Store;

/// The step a verdict routes back to — the draft, in Design Plan's words.
fn drafted() -> StepId {
    StepId::new("reproduce")
}

/// The step that emits the verdict, and therefore the step the pass is charged
/// to. `attempt::on_its_first_run` is what puts a Drone on it.
fn gate() -> StepId {
    StepId::new("fix")
}

/// A Job with a Drone on the gate and the draft being written.
///
/// The two steps are entered out of order against the workflow, and it does not
/// matter here: every count in this file is per step, and the fixture's first
/// step has no run of its own until this puts one there.
fn drafting(store: &mut Store, id: &str) -> Job {
    let job = on_its_first_run(store, id);
    moved(store, &job, &drafted(), StepTarget::Running, "10:03")
}

/// A Job with one pass over both steps behind it: the draft written and
/// cleared, the gate being worked, and a verdict about to send it back.
fn a_pass_over_both_steps(store: &mut Store, id: &str) -> Job {
    let job = drafting(store, id);
    moved(store, &job, &drafted(), StepTarget::Advanced, "10:04")
}

/// The gate sends the work back: the draft is redone, and the pass is the
/// gate's.
fn loop_back(store: &mut Store, job: &Job, when: &str) -> Job {
    moved(store, job, &drafted(), StepTarget::Returned(gate()), when)
}

/// Advance the draft again, so a second verdict has something to send back.
fn cleared_again(store: &mut Store, job: &Job, when: &str) -> Job {
    moved(store, job, &drafted(), StepTarget::Advanced, when)
}

/// A failure inside a pass: the draft stops and is started again. This is the
/// act the return has to be told apart from.
fn run_it_again(store: &mut Store, job: &Job, stopped_at: &str, started_at: &str) -> Job {
    let why = StepLevelTrigger::of(EscalationTrigger::GateFailure)
        .expect("a gate failure is a step-level trigger");
    let job = moved(store, job, &drafted(), StepTarget::Stopped(why), stopped_at);
    moved(store, &job, &drafted(), StepTarget::Running, started_at)
}

fn moved(store: &mut Store, job: &Job, step: &StepId, to: StepTarget, when: &str) -> Job {
    let moved = job
        .transition_step(
            step,
            to,
            Actor::Fleet,
            at(&format!("2026-08-26T{when}:00.000Z")),
        )
        .unwrap_or_else(|cause| panic!("moving {}: {cause}", step.as_str()));
    store
        .record_step_transition(&moved)
        .expect("the step move is recorded");
    moved.job
}

/// The pass is the log's answer, the way the attempt is. A step that has routed
/// nothing back is on its first pass — which is every step of every linear
/// workflow, and why the answer is never absent.
#[test]
fn the_pass_is_counted_off_the_log_and_nowhere_else() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01LOOPED";
    let job = job_id(id);

    let running = a_pass_over_both_steps(&mut store, id);
    assert_eq!(
        store
            .step_iteration(&job, &gate())
            .expect("counted")
            .number(),
        1,
        "a gate that has sent nothing back is on its first pass"
    );

    let running = loop_back(&mut store, &running, "10:05");
    assert_eq!(
        store
            .step_iteration(&job, &gate())
            .expect("counted")
            .number(),
        2,
        "one return is the second pass"
    );

    let running = cleared_again(&mut store, &running, "10:06");
    loop_back(&mut store, &running, "10:07");
    assert_eq!(
        store
            .step_iteration(&job, &gate())
            .expect("counted")
            .number(),
        3
    );
}

/// The decision this issue records, asserted as the difference it makes: the
/// count belongs to the step that emitted the verdict, and the step that was
/// redone twice is still on its first pass.
///
/// **This is the reading that survives two loops sharing a target step.** The
/// routed-to reading would sum them, and a cap on either gate would fire on the
/// other gate's passes.
#[test]
fn the_pass_is_the_emitting_steps_and_not_the_step_it_routes_back_to() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01WHOSECOUNT";
    let job = job_id(id);

    let running = a_pass_over_both_steps(&mut store, id);
    let running = loop_back(&mut store, &running, "10:05");
    let running = cleared_again(&mut store, &running, "10:06");
    loop_back(&mut store, &running, "10:07");

    assert_eq!(
        store
            .step_iteration(&job, &gate())
            .expect("counted")
            .number(),
        3,
        "two returns, and the gate that made both is on its third pass"
    );
    assert_eq!(
        store
            .step_iteration(&job, &drafted())
            .expect("counted")
            .number(),
        1,
        "the step redone twice has emitted nothing, so its own cap is untouched"
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

    let running = a_pass_over_both_steps(&mut store, id);
    let running = loop_back(&mut store, &running, "10:05");
    run_it_again(&mut store, &running, "10:06", "10:07");

    assert_eq!(
        store
            .step_iteration(&job, &gate())
            .expect("counted")
            .number(),
        2,
        "a failure inside the second pass is still the second pass"
    );
    assert_eq!(
        store
            .step_attempt(&job, &drafted())
            .expect("counted")
            .number(),
        3,
        "and every one of them is a run, which is why their evidence is filed apart"
    );
}

/// The defect `#263` closes, asserted as the two numbers it separates. The
/// attempt keys the per-run records and has to climb across a return or a
/// second pass's verdicts overwrite a first pass's; the retry budget resets,
/// because `retry_limit`'s registry row says re-entry as designed is a fresh
/// one.
#[test]
fn the_retry_budget_resets_on_a_return_and_the_attempt_does_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01FRESHBUDGET";
    let job = job_id(id);

    let running = drafting(&mut store, id);
    let running = run_it_again(&mut store, &running, "10:04", "10:05");
    assert_eq!(
        store
            .step_spent(&job, &drafted())
            .expect("counted")
            .number(),
        2,
        "one hand-back spent, and the two readings still agree"
    );
    assert_eq!(
        store
            .step_attempt(&job, &drafted())
            .expect("counted")
            .number(),
        2
    );

    let running = cleared_again(&mut store, &running, "10:06");
    let running = loop_back(&mut store, &running, "10:07");
    assert_eq!(
        store
            .step_spent(&job, &drafted())
            .expect("counted")
            .number(),
        1,
        "the return opens a pass, and nothing has failed inside it"
    );
    assert_eq!(
        store
            .step_attempt(&job, &drafted())
            .expect("counted")
            .number(),
        3,
        "while the coordinate the records are filed under keeps climbing"
    );

    run_it_again(&mut store, &running, "10:08", "10:09");
    assert_eq!(
        store
            .step_spent(&job, &drafted())
            .expect("counted")
            .number(),
        2,
        "and the new pass spends its own budget from the start"
    );
}

/// Every step of every linear workflow, where the two readings are one number.
/// That is why the defect above was invisible: nothing had ever looped.
#[test]
fn a_step_nothing_returns_to_spends_exactly_what_it_attempts() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01NOLOOP";
    let job = job_id(id);

    let running = on_its_first_run(&mut store, id);
    moved(&mut store, &running, &gate(), StepTarget::Advanced, "10:03");

    for step in [drafted(), gate()] {
        assert_eq!(
            store.step_spent(&job, &step).expect("counted").number(),
            store.step_attempt(&job, &step).expect("counted").number(),
            "{} has no return to reset at",
            step.as_str()
        );
    }
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

    let running = a_pass_over_both_steps(&mut store, id);
    let looped = loop_back(&mut store, &running, "10:05");

    let read = store.load_job(&job).expect("the job reads back");
    assert_eq!(read, looped, "the fold rebuilds what the moves left");
    assert_eq!(
        read.current_step_id(),
        Some(&drafted()),
        "and the cursor came back where the return put it"
    );
}
