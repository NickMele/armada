//! The inner machine: the three step states M1 reaches, and the two edges
//! between them.
//!
//! # There is no registry edge table, and this does not invent one
//!
//! `domain/step-states.toml` declares six states and their meanings and says
//! nothing about which moves are legal. The Job record refused to build this
//! machine for that reason. What is built here is not a transcription of a
//! table that does not exist — it is **the three states M1 reaches and the two
//! edges it walks**, named as such:
//!
//! - `not_started -> running` — the step starts being worked;
//! - `running -> advanced` — the step passed its advance gate.
//!
//! `awaiting_human` needs a human advance gate and M1's gates are `auto`.
//! `retrying` and `stopped` need a retry budget M1 does not have. All three
//! stay declared on [`StepState`], because the registry declares six and a
//! stored row may render any of them — and **none of them is reachable**, in
//! the only way that lasts: [`StepTarget`] has no variant naming one, so no
//! call moving a step there can be written. Not a check that refuses; a call
//! that does not exist.
//!
//! The remaining edges are an honest gap. `advanced -> running` in particular
//! is implied by `job-fields.toml`'s reason for storing `state` at all — "a
//! loop workflow, where a step can have advanced and then be re-entered" — and
//! is *not* here, because implied by a `why` is not declared, and no workflow
//! loops at M1.
//!
//! # The outer machine gates the inner one, and the registry says so
//!
//! `job-fields.toml`'s `workflow_status` row: "Advances only while the Job is
//! `running` or `awaiting_review`; frozen otherwise, never cleared, still
//! rendered." [`ADVANCING_STATUSES`] is that sentence, and
//! [`Job::transition_step`](crate::Job::transition_step) refuses beneath any
//! other status.
//!
//! That gate is why a step move and a status move belong in **one log in one
//! order**: replaying a step move requires knowing which status the Job stood
//! in when it happened, and two independently keyed logs cannot be interleaved
//! to answer that.

use core::fmt;

use crate::job::ids::StepId;
use crate::job::status::{JobStatus, StepState};

/// One legal move of a step.
///
/// No trigger and no reason field: neither reachable destination stores a
/// qualifying reason, and a field that is always `None` reads as a field that
/// is working.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepEdge {
    pub from: StepState,
    pub to: StepState,
}

/// The two edges M1 walks. Nothing else in this crate decides what is legal
/// for a step.
pub static STEP_EDGES: &[StepEdge] = &[
    step_edge(StepState::NotStarted, StepState::Running),
    step_edge(StepState::Running, StepState::Advanced),
];

const fn step_edge(from: StepState, to: StepState) -> StepEdge {
    StepEdge { from, to }
}

/// The Job statuses beneath which a step moves at all.
///
/// `job-fields.toml`, `workflow_status`: the nested machine "advances only
/// while the Job is `running` or `awaiting_review`; frozen otherwise, never
/// cleared, still rendered." Frozen is what this list produces — a refusal, not
/// a silent no-op, so a caller that drove a step under the wrong status hears
/// about it.
pub const ADVANCING_STATUSES: &[JobStatus] = &[JobStatus::Running, JobStatus::AwaitingReview];

/// Where a step is going.
///
/// Two variants for the two destinations M1 reaches. [`StepState`] has six and
/// this has two on purpose: the four that are not here — `not_started`, which
/// is written at creation and is not a destination, and the three that need a
/// human gate or a retry budget — cannot be passed to
/// [`Job::transition_step`](crate::Job::transition_step) because there is
/// nothing to pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepTarget {
    /// The step is being worked. Entering it is what moves the Job's cursor.
    Running,
    /// The step passed its advance gate.
    ///
    /// It carries no verdict, because the state *is* the verdict: `advanced`
    /// means "the step passed its advance gate" and `last_verdict`'s `passed`
    /// means the same thing said about the ruling. A caller choosing one
    /// independently of the other could make the two disagree.
    Advanced,
}

impl StepTarget {
    /// The state this target arrives at.
    pub fn state(&self) -> StepState {
        match self {
            StepTarget::Running => StepState::Running,
            StepTarget::Advanced => StepState::Advanced,
        }
    }

    /// The target that arrives at a stored state, where one exists.
    ///
    /// `None` for the four states no target names. Coming off disk a state is
    /// data and can be any of the six, so this is where the narrowing the type
    /// system does at every call site is paid — once, on the way in.
    pub fn arriving_at(state: StepState) -> Option<StepTarget> {
        match state {
            StepState::Running => Some(StepTarget::Running),
            StepState::Advanced => Some(StepTarget::Advanced),
            StepState::NotStarted
            | StepState::AwaitingHuman
            | StepState::Retrying
            | StepState::Stopped => None,
        }
    }
}

/// A step move the machine refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IllegalStepTransition {
    /// The Job's frozen WorkflowDef has no step by this name. The rows are
    /// written at creation and never added to, so this is a caller naming a
    /// step of some other Job.
    NoSuchStep { step_id: StepId },
    /// The Job is not in a status the inner machine advances beneath. Frozen,
    /// in the registry's word — and still rendered, which is why this refuses
    /// rather than clearing anything.
    StepsAreFrozen { step_id: StepId, status: JobStatus },
    /// Both states exist and no edge joins them. A move from a state to itself
    /// lands here: no self-edge is declared, and a move that changes nothing
    /// would still write an event.
    NoSuchEdge {
        step_id: StepId,
        from: StepState,
        to: StepState,
    },
}

impl fmt::Display for IllegalStepTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IllegalStepTransition::NoSuchStep { step_id } => {
                write!(f, "no step `{}` on this job", step_id.as_str())
            }
            IllegalStepTransition::StepsAreFrozen { step_id, status } => write!(
                f,
                "step `{}` is frozen: a step advances only beneath {}, and the job is {}",
                step_id.as_str(),
                Advancing,
                status.as_wire()
            ),
            IllegalStepTransition::NoSuchEdge { step_id, from, to } => write!(
                f,
                "step `{}`: no edge {} -> {}",
                step_id.as_str(),
                from.as_wire(),
                to.as_wire()
            ),
        }
    }
}

impl core::error::Error for IllegalStepTransition {}

/// [`ADVANCING_STATUSES`] as one readable phrase, so the refusal names the set
/// rather than a hard-coded pair that could stop matching it.
struct Advancing;

impl fmt::Display for Advancing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, status) in ADVANCING_STATUSES.iter().enumerate() {
            if index > 0 {
                f.write_str(" or ")?;
            }
            f.write_str(status.as_wire())?;
        }
        Ok(())
    }
}

/// Whether the machine admits this move, and why not if it does not.
pub(crate) fn admits_step(
    status: JobStatus,
    step_id: &StepId,
    from: StepState,
    to: &StepTarget,
) -> Result<(), IllegalStepTransition> {
    if !ADVANCING_STATUSES.contains(&status) {
        return Err(IllegalStepTransition::StepsAreFrozen {
            step_id: step_id.clone(),
            status,
        });
    }
    let arriving = to.state();
    if !STEP_EDGES
        .iter()
        .any(|edge| edge.from == from && edge.to == arriving)
    {
        return Err(IllegalStepTransition::NoSuchEdge {
            step_id: step_id.clone(),
            from,
            to: arriving,
        });
    }
    Ok(())
}
