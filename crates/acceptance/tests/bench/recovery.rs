//! The apparatus Recovery's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `board.rs` and `focus.rs` are: M1's
//! bench answers "did this Job pass its gates", and Recovery asks what a Job
//! that stopped short of a gate says to the person now holding it.
//!
//! **It builds no classification.** [`core_model::Stuck::of`] is what decides
//! which acts a stopped Job admits, and it is called with a [`Standing`] the
//! test writes out field by field — which is that type's own discipline, so
//! nothing here hands one over with a field defaulted. What this file adds is
//! the two moves a person makes that the shared bench has no verb for, and the
//! round trip [`bench::board`] already established.
//!
//! [`bench::board`]: super::board

use core_model::{Actor, Job, Standing, StepId, StepTarget, Stuck, TransitionReason};
use ipc::{JobDetail, StepFacts};

use super::{Bench, Run};

/// Move one step of the frozen workflow **as the actor named**.
///
/// [`Bench::step_moved`] is always Fleet, which is right for every move the
/// two machines make on their own. This one is not one of those: ending a
/// Drone is something a person did, and `fleet::resume`'s `stopped_by_hand`
/// records `Actor::Human` for exactly that reason — a row saying Fleet took the
/// process away would claim a decision it did not make. Reaching [`Bench`]'s
/// private log rather than copying it is `focus.rs`'s arrangement and for its
/// reason.
///
/// **The step machine does not read the actor**, so this is the bench being
/// truthful about the act rather than the bench being narrowed. What the
/// machine reads is `step_machine::taken_from_a_person`, and its narrowness is
/// asserted in `recovery.rs` rather than described here.
pub fn step_moved_by(bench: &Bench, run: &mut Run, step: &StepId, to: StepTarget, by: Actor) {
    let moved = run
        .job
        .transition_step(step, to, by, super::focus::now(bench))
        .expect("a legal step move");
    bench.step_moves.borrow_mut().push(moved.event);
    run.job = moved.job;
}

/// Who each step move was recorded as, in order.
///
/// The Job's own log has [`Bench::actors`]; the inner machine's had no reader
/// because no earlier milestone asked who moved a step. Recovery does: which
/// of a person and Fleet stopped the step is the difference between a Job that
/// failed and a Job somebody took in hand.
pub fn step_actors(bench: &Bench) -> Vec<Actor> {
    bench
        .step_moves
        .borrow()
        .iter()
        .map(|e| e.actor())
        .collect()
}

/// One Job in full, as `get_job` would answer it, **against the standing the
/// caller names**.
///
/// `bench::board`'s `detail` fixes the standing at "worktree there, Checks
/// passed", which is the ordinary shape at an escalation and is all a Board
/// needed. Recovery's whole subject is that the acts change when those facts
/// do, so the standing is this one's argument. Everything else it passes and
/// omits is `bench::board::detail`'s and carries the same limits.
pub fn opened(
    job: &Job,
    reason: Option<&TransitionReason>,
    standing: Standing,
    steps: &[StepFacts],
) -> JobDetail {
    let stuck = Stuck::of(job, reason, standing);
    JobDetail::of(
        job,
        reason,
        None,
        None,
        steps,
        None,
        None,
        None,
        stuck.as_ref(),
        None,
        None,
        None,
    )
}

/// The acts a Board is offered, in the order they arrived.
///
/// Spelled back out of the wire value rather than read off the domain enum, so
/// what is compared is the string a client dispatches on.
pub fn acts(opened: &JobDetail) -> Vec<&'static str> {
    opened
        .stuck
        .as_ref()
        .map(|stuck| stuck.recourse.iter().map(|act| act.as_wire()).collect())
        .unwrap_or_default()
}
