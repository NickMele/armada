//! The apparatus Focus's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `mod.rs` is separate from the test
//! files: M1's bench answers "did this Job pass its gates", and Focus asks a
//! different question of the same machinery — which Drone was on the step, and
//! which definition of the step it was measured against. Two subjects, two
//! files, and `mod.rs` stays under the length the gate warns at.
//!
//! It reaches [`Bench`]'s private fields because it is a child module of the
//! one that declares it. That is deliberate: the seams a gate is called with
//! are the bench's own, and a second copy of them here is how the two would
//! come to disagree about the same call.

use adapter_traits::Footprint;
use core_model::{DroneId, FrozenWorkflow, StepEvidence, StepId, Timestamp, Ulid};
use fleet::{rule_on, AtStep, Clock, Keeping, Ruling};
use verification::{Lifted, Request, Submission};

use super::{Bench, Run};

/// A Drone id a test can write down, in the shape [`Ulid`] admits.
///
/// **Minted by the test rather than by the bench**, because which Drone is on a
/// step is the thing being asserted: a bench that handed them out would be
/// answering the question.
pub fn drone(n: u32) -> DroneId {
    DroneId::carried(Ulid::carried(format!("01DRONE{n:019}")))
}

/// The bench's planted clock, which a test needs directly because a Drone
/// arriving and leaving is timed by the caller rather than by a gate.
pub fn now(bench: &Bench) -> Timestamp {
    bench.clock.now()
}

/// Run the step's checks over one submission and decide, **against the
/// workflow definition the caller names.**
///
/// [`Bench::gate`] resolves the step from the bench's own workflow, which is
/// Fleet's current one. This takes the definition as an argument so a test can
/// hand it the Job's snapshot and Fleet's edited copy in turn, and show that
/// the two answer differently — which is the only way to demonstrate that the
/// snapshot is what decides the bar. `fleet::settling` and `fleet::regating`
/// both call `AtStep::named(job.workflow(), ..)`, so the Job's snapshot is what
/// production hands it.
///
/// It records the evidence afterwards for the reason [`Bench::gate`] does: a
/// later step's gaming check reads it as its baseline.
pub async fn gate_against(
    bench: &Bench,
    workflow: &FrozenWorkflow,
    run: &Run,
    step: &StepId,
    submitted: &Submission,
) -> Ruling {
    let at = AtStep::named(workflow, step, &run.worktree).expect("a step of the workflow");
    let recorded = bench.recorded.borrow().clone();
    let entered_with = Footprint::nothing();
    let ruling = rule_on(
        at,
        Request::of(&run.job),
        submitted,
        run.declared.as_ref(),
        &Lifted::of(&run.job),
        Some(&entered_with),
        &recorded,
        &bench.work,
        bench.budget,
        &bench.judging,
        // [`Bench::gate`]'s note applies: inert, because no step here declares
        // a deliverable.
        &Keeping::of(crate::bench::REPO_ROOT, run.job.id()),
    )
    .await;
    let mut held = bench.recorded.borrow_mut();
    held.retain(|(id, _)| id != step);
    held.push((step.clone(), submitted.recorded()));
    ruling
}

/// Every step's latest evidence, the way `store::step_evidence` answers it.
///
/// **The bench holds it because the gate writes it**, one entry per step as
/// each is ruled on. A test that assembled the list itself would be handing the
/// briefing a record no gate had produced, which is the half of the claim that
/// is actually in doubt.
pub fn recorded(bench: &Bench) -> Vec<(StepId, StepEvidence)> {
    bench.recorded.borrow().clone()
}
