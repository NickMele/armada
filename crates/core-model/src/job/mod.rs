//! The Job record and its transition machine.
//!
//! A Job is **data, not an actor**: which WorkflowDef it follows, where it is,
//! where each of its steps got to, and which Drone if any is on it. Fleet is
//! the only thing that drives a transition; a Drone's self-report is an input
//! signal and never authoritative.
//!
//! # Where the shape comes from
//!
//! Not from this module. `crates/core-model/domain/` is the authority — twelve
//! statuses, thirty-four edges, six step states, thirteen escalation triggers
//! and the field list, each carried here with the registry's own wire values as
//! the variant spellings. A page in the design workspace that disagrees with
//! one of those files is stale, not right, and so is this code.
//!
//! # The two-level machine, and the half that is here
//!
//! [`JobStatus`] is the outer half and [`StepState`] is the inner one, and both
//! are built. The registry gives step states no edge table, so [`step_machine`]
//! transcribes nothing: it declares the three states M1 reaches and the two
//! edges it walks, and makes the other three unreachable by giving
//! [`StepTarget`] no variant that names them. The full edge table stays an
//! honest gap rather than a guess.
//!
//! [`step_machine`]: crate::StepTarget
//!
//! # What "no setter" means concretely
//!
//! [`Job`]'s fields are private, no method on it takes `&mut self`, and
//! [`Job::transition`] and [`Job::transition_step`] are the only things that
//! return a `Job` differing from the one they were given. There is no
//! constructor that accepts a status or a step state either — [`Job::create_top_level`] enters at `awaiting_approval` and
//! [`Job::create_sub_dispatched`] enters at `queued`, and neither takes the
//! choice from a caller. A test that wanted a Job in some other state has to
//! transition into it, which is the point: a test that constructs its way there
//! asserts nothing about the machine it claims to be testing.

mod check;
mod drone;
mod escalation;
mod event;
mod fields;
mod ids;
mod judge;
mod record;
mod scope;
mod status;
mod step;
mod step_machine;
mod transition;
mod workflow;

#[cfg(test)]
mod tests;

pub use check::{CheckOutcome, StepCheck};
pub use drone::{DroneAssigned, DroneMoved, DronePresence, IllegalDroneMove};
pub use escalation::{EscalationTrigger, TriggerKind, TriggerLevel};
pub use event::{JobEvent, StepEvent};
pub use fields::{
    AcceptanceCriterion, BlankBranch, Branch, CriterionSource, DependencyDirection, DependencyEdge,
    DispatchOrigin, Facts, GateManifest, GateOutcome, NotRunDisposition, NotRunReason, Origin,
    ScopeRevision, ScopeRevisionOutcome, Subject, TopLevelOrigin, Urgency, WriteTargets,
};
pub use ids::{
    BlankModel, BlankTitle, CriterionId, DroneId, JobId, ManifestId, ModelName, RepoPath, StepId,
    Title, WorkflowId,
};
pub use judge::{JudgeCheck, JudgeCriterion, JudgeVerdict, Judgment};
pub use record::{Job, NewJob, StepTransitioned, Transitioned};
pub use scope::{under, ContextSource, DeclarePlanAt, DeclaredPaths, EvidenceScope};
pub use status::{JobStatus, StepState};
pub use step::{JobStep, StepSeed, StepVerdict};
pub use step_machine::{
    IllegalStepTransition, StepEdge, StepTarget, ADVANCING_STATUSES, STEP_EDGES,
};
pub use transition::{
    CriteriaOwed, Edge, IllegalTransition, PilotReason, Target, TransitionReason, EDGES,
};
pub use workflow::{
    AdvanceGate, EvidenceType, FrozenWorkflow, ResolvedCheck, ResolvedStep, DIFF_NONEMPTY,
    MANIFEST_CHECK,
};
