//! The inner machine: the four step states M1 reaches, and the edges between.
//!
//! # There is no registry edge table, and this does not invent one
//!
//! `domain/step-states.toml` declares six states and says nothing about which
//! moves are legal. Built here are the edges M1 walks: `not_started -> running`,
//! `running -> advanced`, `running -> stopped`, which is the one a refusal
//! needs — `job-statuses.toml` gives `escalated` the step state `stopped`, and
//! without the edge a refused step stayed `running` with no writer for
//! `last_verdict` at all — and `stopped -> running`, which is how a person
//! resumes it.
//!
//! **`stopped -> running` is one edge for two acts.** A redirect speaks to the
//! Drone that is still there and a restart puts a new one on the worktree, and
//! the difference between them is the process rather than the step: the step
//! was stopped and is being worked again either way. Which act a Job admits is
//! decided by whether it holds a Drone, which is `fleet::resume`'s to ask.
//!
//! `awaiting_human` needs a human advance gate and `retrying` a retry budget,
//! and M1 has neither. Both stay declared on [`StepState`], because a stored
//! row may render any of the six, and **neither is reachable**: [`StepTarget`]
//! has no variant naming one.
//!
//! # The outer machine gates the inner one
//!
//! `job-fields.toml`'s `workflow_status` row: a step "advances only while the
//! Job is `running` or `awaiting_review`; frozen otherwise, never cleared,
//! still rendered." [`ADVANCING_STATUSES`] is that sentence — which is why a
//! step stops *before* the Job escalates, and why a step move and a status move
//! belong in one log in one order.

use core::fmt;

use crate::job::escalation::StepLevelTrigger;
use crate::job::ids::StepId;
use crate::job::status::{JobStatus, StepState};

/// One legal move of a step.
///
/// No trigger and no reason field: what qualifies a stop travels on
/// [`StepTarget::Stopped`], because it is the destination that decides whether
/// there is a reason at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepEdge {
    pub from: StepState,
    pub to: StepState,
}

/// The edges M1 walks. Nothing else in this crate decides what is legal for a
/// step.
pub static STEP_EDGES: &[StepEdge] = &[
    step_edge(StepState::NotStarted, StepState::Running),
    step_edge(StepState::Running, StepState::Advanced),
    step_edge(StepState::Running, StepState::Stopped),
    step_edge(StepState::Stopped, StepState::Running),
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
/// Three variants for the three destinations M1 reaches. `not_started` is
/// written at creation and is not a destination; `awaiting_human` and
/// `retrying` need a human gate or a retry budget, and cannot be passed to
/// [`Job::transition_step`](crate::Job::transition_step) because there is
/// nothing to pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepTarget {
    /// The step is being worked. Entering it is what moves the Job's cursor.
    ///
    /// Reached at dispatch and again on a resume. It carries no reason, and
    /// the row keeps whatever verdict stopped it — so a resumed step still
    /// says what it is being resumed from.
    Running,
    /// The step passed its advance gate.
    ///
    /// It carries no verdict, because the state *is* the verdict: `advanced`
    /// means "the step passed its advance gate" and `last_verdict`'s `passed`
    /// means the same thing said about the ruling. A caller choosing one
    /// independently of the other could make the two disagree.
    Advanced,
    /// The step's retries are spent. It is neither retrying nor waiting on a
    /// person — folding either of those into this one would make a designed
    /// human gate and a dead stop render alike.
    ///
    /// **It carries the trigger and not a verdict**, which is the same rule
    /// [`Advanced`](StepTarget::Advanced) states from the other side: the
    /// destination fixes that the verdict is `failed`, so no caller can stop a
    /// step and call it passed. What the state cannot say is *why*, and that is
    /// exactly the payload. [`StepLevelTrigger`] is why the reason cannot be a
    /// Job-level one, which `last_verdict` does not admit.
    Stopped(StepLevelTrigger),
}

impl StepTarget {
    /// The state this target arrives at.
    pub fn state(&self) -> StepState {
        match self {
            StepTarget::Running => StepState::Running,
            StepTarget::Advanced => StepState::Advanced,
            StepTarget::Stopped(_) => StepState::Stopped,
        }
    }

    /// What qualifies the move, where the destination stores one. `None` on the
    /// two that do not, for [`StepEdge`]'s reason.
    pub fn why(&self) -> Option<StepLevelTrigger> {
        match self {
            StepTarget::Running | StepTarget::Advanced => None,
            StepTarget::Stopped(why) => Some(*why),
        }
    }

    /// The target that arrives at a stored state carrying a stored reason,
    /// where one exists.
    ///
    /// Coming off disk the pair is data and can be anything, so this is where
    /// the narrowing the type system does at every call site is paid — once, on
    /// the way in. A reason on a destination that stores none is refused rather
    /// than dropped, the same way a missing one on `stopped` is.
    pub fn arriving_at(state: StepState, why: Option<StepLevelTrigger>) -> Option<StepTarget> {
        match (state, why) {
            (StepState::Running, None) => Some(StepTarget::Running),
            (StepState::Advanced, None) => Some(StepTarget::Advanced),
            (StepState::Stopped, Some(why)) => Some(StepTarget::Stopped(why)),
            _ => None,
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
