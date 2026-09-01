//! The Job record and its transition machine.
//!
//! A Job is **data, not an actor**: which WorkflowDef it follows, where it is,
//! where each of its steps got to, and which Drone if any is on it. Fleet is
//! the only thing that drives a transition; a Drone's self-report is an input
//! signal and never authoritative.
//!
//! **The shape does not come from this module.** `crates/core-model/domain/` is
//! the authority — the statuses, the edges, the step states, the escalation
//! triggers and the field list, each carried here with the registry's own wire
//! values as the variant spellings. A page in the design workspace that
//! disagrees with one of those files is stale, not right, and so is this code.
//!
//! **Two levels.** [`JobStatus`] is the outer half and [`StepState`] the inner,
//! and both are built. The registry gives step states no edge table, so
//! [`step_machine`](crate::StepTarget) declares the edges M1 walks and says on
//! each why it is there, rather than transcribing a file. [`Guard`] is where
//! the halves meet: a status edge may carry a condition on the Job's step rows.
//!
//! **"No setter" concretely.** [`Job`]'s fields are private, no method takes
//! `&mut self`, and [`Job::transition`] and [`Job::transition_step`] are the
//! only things returning a `Job` different from the one handed in. No
//! constructor accepts a status or a step state either, so a test wanting a Job
//! in some other state has to transition into it — a test that constructs its
//! way there asserts nothing about the machine it claims to be testing.

mod attempt;
mod check;
mod collision;
mod covers;
mod drone;
mod escalation;
mod event;
mod fields;
mod gaming;
mod guard;
mod ids;
mod judge;
mod note;
mod record;
mod scope;
mod status;
mod step;
mod step_machine;
mod stuck;
mod transition;
mod workflow;

#[cfg(test)]
mod tests;

pub use attempt::Attempt;
pub use check::{CheckOutcome, StepCheck};
pub use collision::{collisions, Collision, ScopeClaim};
pub use covers::{BadPattern, Covers, PathPattern};
pub use drone::{DroneAssigned, DroneMoved, DronePresence, IllegalDroneMove};
pub use escalation::{EscalationTrigger, StepLevelTrigger, TriggerKind, TriggerLevel};
pub use event::{JobEvent, StepEvent};
pub use fields::{
    AcceptanceCriterion, AdmissionHold, Attachment, BlankBranch, Branch, CriterionSource,
    DependencyDirection, DependencyEdge, DispatchOrigin, Facts, GateManifest, GateOutcome,
    NotRunDisposition, NotRunReason, Origin, QueuedReason, Resumption, ScopeRevision,
    ScopeRevisionOutcome, Subject, TopLevelOrigin, Urgency, WriteTargets,
};
pub use gaming::{CitedAt, DecidedBy, EvidenceRef, GamingCheck, GamingFlag, GamingPattern};
pub use guard::Guard;
pub use ids::{
    BlankModel, BlankTitle, CriterionId, DroneId, JobId, ManifestId, ModelName, RepoPath, StepId,
    Title, WorkflowId,
};
pub use judge::{JudgeCheck, JudgeCriterion, JudgeVerdict, Judgment};
pub use note::{RedirectAlreadyWaiting, RedirectWaiting};
pub use record::{Job, NewJob, StepTransitioned, Transitioned};
pub use scope::{under, ContextSource, DeclarePlanAt, DeclaredPaths, EvidenceScope};
pub use status::{JobStatus, StepState};
pub use step::{JobStep, StepEvidence, StepSeed, StepVerdict};
pub use step_machine::{
    IllegalStepTransition, StepEdge, StepTarget, ADVANCING_STATUSES, STEP_EDGES,
};
pub use stuck::{Recourse, Standing, Stuck};
pub use transition::{
    CriteriaOwed, Edge, IllegalTransition, PilotReason, Target, TransitionReason, EDGES,
};
pub use workflow::{
    AdvanceGate, EvidenceType, FrozenWorkflow, ResolvedCheck, ResolvedStep, ARTIFACT_EXISTS,
    DIFF_NONEMPTY, MANIFEST_CHECK,
};
