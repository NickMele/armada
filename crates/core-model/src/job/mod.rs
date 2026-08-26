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
//! [`JobStatus`] is the outer half and [`StepState`] is the inner one. **Only
//! the outer machine is built.** The registry gives step states no edge table,
//! so there is nothing to transcribe and nothing here advances a step: rows are
//! written at creation, all `not_started`, and stay there. The inner machine's
//! writer is a later step's, and when it arrives it must be a named transition
//! and not a setter, for the reason this module refuses one.
//!
//! # What "no setter" means concretely
//!
//! [`Job`]'s fields are private, no method on it takes `&mut self`, and
//! [`Job::transition`] is the only thing that returns a `Job` whose status
//! differs from the one it was given. There is no constructor that accepts a
//! status either — [`Job::create_top_level`] enters at `awaiting_approval` and
//! [`Job::create_sub_dispatched`] enters at `queued`, and neither takes the
//! choice from a caller. A test that wanted a Job in some other state has to
//! transition into it, which is the point: a test that constructs its way there
//! asserts nothing about the machine it claims to be testing.

mod escalation;
mod event;
mod fields;
mod ids;
mod record;
mod status;
mod step;
mod transition;

#[cfg(test)]
mod tests;

pub use escalation::{EscalationTrigger, TriggerKind, TriggerLevel};
pub use event::JobEvent;
pub use fields::{
    AcceptanceCriterion, CriterionSource, DependencyDirection, DependencyEdge, DispatchOrigin,
    Facts, GateManifest, GateOutcome, NotRunDisposition, NotRunReason, Origin, ScopeRevision,
    ScopeRevisionOutcome, Subject, TopLevelOrigin, Urgency, WriteTargets,
};
pub use ids::{
    BlankTitle, CriterionId, DroneId, JobId, ManifestId, ModelName, RepoPath, StepId, Title,
    WorkflowId,
};
pub use record::{Job, NewJob, Transitioned};
pub use status::{JobStatus, StepState};
pub use step::{JobStep, StepSeed, StepVerdict};
pub use transition::{
    CriteriaOwed, Edge, IllegalTransition, PilotReason, Target, TransitionReason, EDGES,
};
