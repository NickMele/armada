//! A pass Fleet opened to clear a pull request's conflicts with its base.
//! `#1131`.
//!
//! **Read off the Job's own record, never remembered**, so a restart mid-pass
//! reads the same answer. The pass is open where the latest loop return on the
//! Job was Fleet's and the gate it named has not held again since. Nothing else
//! of Fleet's walks a return: a verdict routed back is always a person's answer.

use core_model::{Actor, Job, StepId, StepState};
use store::{Moved, RecordedEvent};

/// Where a conflict-clearing pass stands.
pub(crate) struct Clearing<'a> {
    /// The step the work went back to, whose Drone clears the markers.
    pub(crate) redoing: &'a StepId,
}

/// The conflict-clearing pass `step` is on, where it is on one.
pub(crate) fn clearing<'a>(
    events: &'a [RecordedEvent],
    job: &Job,
    step: &StepId,
) -> Option<Clearing<'a>> {
    let (at, event, redoing, gate) =
        events
            .iter()
            .enumerate()
            .rev()
            .find_map(|(at, event)| match event.moved() {
                Moved::Step {
                    step_id,
                    returned_by: Some(gate),
                    ..
                } => Some((at, event, step_id, gate)),
                _ => None,
            })?;
    if event.actor() != Actor::Fleet {
        return None;
    }
    let held_again = events[at + 1..].iter().any(|later| {
        matches!(
            later.moved(),
            Moved::Step { step_id, to, .. }
                if step_id == gate
                    && matches!(
                        to,
                        StepState::AwaitingHuman | StepState::Advanced | StepState::Stopped
                    )
        )
    });
    let on_it = job.step(step)?.ordinal() <= job.step(gate)?.ordinal();
    (!held_again && on_it).then_some(Clearing { redoing })
}
