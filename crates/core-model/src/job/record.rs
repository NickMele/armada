//! The `jobs` row, and the only thing that moves it.
//!
//! # No setter, and no `&mut self` at all
//!
//! Every field is private, nothing here takes `&mut self`, and the only method
//! that produces a [`Job`] with a different status is
//! [`transition`](Job::transition). A caller holding a `Job` cannot write its
//! status because there is nothing to call, which is the difference between a
//! rule and a check.
//!
//! # What is deliberately absent
//!
//! **Evidence.** The registry gives it its own table, three open questions
//! about how it serialises, and a rule that captured Check output goes to disk
//! with a pointer in the row. It is loaded and appended by `store`, not carried
//! inline on the record — a blob on the Job row would rewrite whole on every
//! append.
//!
//! **A writer for the inner machine.** Nothing here advances a step or moves
//! `current_step_id`. The step machine has no edge table in the registry, and
//! inventing one under cover of "the Job record" is how a second machine gets
//! built by accident. A later step owns it, and it must arrive as a named
//! transition rather than as a setter.

use alloc::vec::Vec;

use crate::envelope::{Actor, Timestamp};
use crate::job::event::JobEvent;
use crate::job::fields::{
    AcceptanceCriterion, DependencyEdge, DispatchOrigin, Facts, GateManifest, Origin,
    ScopeRevision, Subject, TopLevelOrigin, Urgency, WriteTargets,
};
use crate::job::ids::{DroneId, JobId, ManifestId, ModelName, StepId, WorkflowId};
use crate::job::status::JobStatus;
use crate::job::step::{rows_at_creation, JobStep, StepSeed};
use crate::job::transition::{admits, IllegalTransition, Target};

/// Everything creation decides, and nothing it does not.
///
/// A plain struct with public fields rather than a builder: there is no
/// `Default`, so a caller writes every field out and cannot forget one, and
/// there is no `status` field, so a Job cannot be created into a state. That is
/// the same refusal `DroneSpawnConfig` makes about a raw argv builder.
///
/// `origin` is absent for the same reason: it is written from `dispatched_by`,
/// and which constructor was called is what decides it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewJob {
    pub id: JobId,
    /// Which WorkflowDef this Job follows. Frozen at creation.
    pub workflow_id: WorkflowId,
    /// Which project this Job belongs to, and the Job Board's scoping key.
    /// Exactly one, always present — work with no Manifest is not a Job.
    pub owner_manifest_id: ManifestId,
    pub urgency: Urgency,
    /// Whether the write targets must land as one unit.
    pub atomic: bool,
    pub model: ModelName,
    /// Frozen at creation; appendable only at an approved widening. You may
    /// raise the bar, not lower it.
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    /// One per step of the frozen WorkflowDef, in order.
    pub steps: Vec<StepSeed>,
    pub dependencies: Vec<DependencyEdge>,
    pub gate_manifests: Vec<GateManifest>,
    /// `None` is scope not yet determined; `Some` with no paths is determined
    /// to write nothing.
    pub write_targets: Option<WriteTargets>,
    pub subject: Option<Subject>,
    pub redispatched_from: Option<JobId>,
    pub facts: Facts,
    /// Entry zero is the initial scope.
    pub scope_revisions: Vec<ScopeRevision>,
}

/// A Job, and the event that produced it.
///
/// Returned together because they are written together: the registry requires
/// the transition and its event to land in one SQLite transaction, and a
/// signature that hands back only the Job would make forgetting the event the
/// easy path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transitioned {
    pub job: Job,
    pub event: JobEvent,
}

/// The record of work to be accomplished. **Data, not an actor.**
///
/// Fleet is the only thing that drives a transition on it; a Drone's
/// self-report is an input signal and never authoritative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    id: JobId,
    status: JobStatus,
    workflow_id: WorkflowId,
    owner_manifest_id: ManifestId,
    origin: Origin,
    urgency: Urgency,
    atomic: bool,
    model: ModelName,
    acceptance_criteria: Vec<AcceptanceCriterion>,
    current_step_id: Option<StepId>,
    assigned_drone: Option<DroneId>,
    dependencies: Vec<DependencyEdge>,
    dispatched_by: Option<DispatchOrigin>,
    redispatched_from: Option<JobId>,
    subject: Option<Subject>,
    facts: Facts,
    scope_revisions: Vec<ScopeRevision>,
    write_targets: Option<WriteTargets>,
    gate_manifests: Vec<GateManifest>,
    steps: Vec<JobStep>,
}

impl Job {
    /// A top-level Job. Enters at `awaiting_approval`: a person must approve
    /// the dispatch before anything runs.
    ///
    /// [`TopLevelOrigin`] cannot name `sub_dispatched`, so the registry's
    /// "where they disagree, `dispatched_by` wins" holds by construction rather
    /// than by a check on write.
    pub fn create_top_level(new: NewJob, origin: TopLevelOrigin, at: Timestamp) -> Job {
        Job::create(new, origin.into(), None, JobStatus::AwaitingApproval, at)
    }

    /// A Job spawned by a step of another Job. Enters at `queued`, already
    /// approved as part of its parent.
    pub fn create_sub_dispatched(new: NewJob, by: DispatchOrigin, at: Timestamp) -> Job {
        Job::create(new, Origin::SubDispatched, Some(by), JobStatus::Queued, at)
    }

    /// The one place a `Job` is built, and the one place other than
    /// [`transition`](Job::transition) that names a status.
    ///
    /// Private, and takes the entry status from its two callers rather than
    /// from anybody outside this file.
    fn create(
        new: NewJob,
        origin: Origin,
        dispatched_by: Option<DispatchOrigin>,
        entry: JobStatus,
        at: Timestamp,
    ) -> Job {
        let steps = rows_at_creation(&new.id, new.steps, &at);
        Job {
            id: new.id,
            status: entry,
            workflow_id: new.workflow_id,
            owner_manifest_id: new.owner_manifest_id,
            origin,
            urgency: new.urgency,
            atomic: new.atomic,
            model: new.model,
            acceptance_criteria: new.acceptance_criteria,
            // A Job at the approval gate, and a `queued` Job, have no current
            // step. What sets this is the inner machine, which is not here.
            current_step_id: None,
            assigned_drone: None,
            dependencies: new.dependencies,
            dispatched_by,
            redispatched_from: new.redispatched_from,
            subject: new.subject,
            facts: new.facts,
            scope_revisions: new.scope_revisions,
            write_targets: new.write_targets,
            gate_manifests: new.gate_manifests,
            steps,
        }
    }

    /// Move the Job, or say why it cannot move.
    ///
    /// Takes `&self` and returns a new `Job` rather than consuming: on the
    /// error path a consumed Job is gone, and the caller that most needs to
    /// report the refusal would have nothing left to report it about. The
    /// no-setter property does not depend on consuming — it comes from every
    /// field being private and no method taking `&mut self`.
    ///
    /// `at` is a parameter and never a clock reading. A model that reads the
    /// clock cannot be tested and cannot be replayed.
    pub fn transition(
        &self,
        to: Target,
        by: Actor,
        at: Timestamp,
    ) -> Result<Transitioned, IllegalTransition> {
        admits(self.status, &to)?;
        let arriving = to.status();
        let event = JobEvent::recorded(self.id.clone(), self.status, arriving, to.reason(), by, at);
        let mut job = self.clone();
        job.status = arriving;
        Ok(Transitioned { job, event })
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }
    pub fn status(&self) -> JobStatus {
        self.status
    }
    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }
    pub fn owner_manifest_id(&self) -> &ManifestId {
        &self.owner_manifest_id
    }
    pub fn origin(&self) -> Origin {
        self.origin
    }
    pub fn urgency(&self) -> Urgency {
        self.urgency
    }
    pub fn atomic(&self) -> bool {
        self.atomic
    }
    pub fn model(&self) -> &ModelName {
        &self.model
    }
    pub fn acceptance_criteria(&self) -> &[AcceptanceCriterion] {
        &self.acceptance_criteria
    }
    pub fn current_step_id(&self) -> Option<&StepId> {
        self.current_step_id.as_ref()
    }
    /// Presence, not state. Null also suspends the liveness clock.
    pub fn assigned_drone(&self) -> Option<&DroneId> {
        self.assigned_drone.as_ref()
    }
    pub fn dependencies(&self) -> &[DependencyEdge] {
        &self.dependencies
    }
    pub fn dispatched_by(&self) -> Option<&DispatchOrigin> {
        self.dispatched_by.as_ref()
    }
    pub fn redispatched_from(&self) -> Option<&JobId> {
        self.redispatched_from.as_ref()
    }
    pub fn subject(&self) -> Option<&Subject> {
        self.subject.as_ref()
    }
    pub fn facts(&self) -> &Facts {
        &self.facts
    }
    pub fn scope_revisions(&self) -> &[ScopeRevision] {
        &self.scope_revisions
    }
    pub fn write_targets(&self) -> Option<&WriteTargets> {
        self.write_targets.as_ref()
    }
    pub fn gate_manifests(&self) -> &[GateManifest] {
        &self.gate_manifests
    }
    /// Every `job_steps` row, in the order they were written.
    pub fn steps(&self) -> &[JobStep] {
        &self.steps
    }
    /// One row by step id.
    pub fn step(&self, step_id: &StepId) -> Option<&JobStep> {
        self.steps.iter().find(|row| row.step_id() == step_id)
    }
    /// The `workflow_status` projection: the row `current_step_id` names.
    /// Derived, never stored — storing it would risk the two disagreeing.
    pub fn current_step(&self) -> Option<&JobStep> {
        self.current_step_id.as_ref().and_then(|id| self.step(id))
    }
}
