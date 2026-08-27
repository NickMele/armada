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
//! # The inner machine's writer is here now, and it is the second mutator
//!
//! [`transition_step`](Job::transition_step) moves a step and the cursor, and
//! it obeys the same rule as [`transition`](Job::transition): `&self` in, a new
//! [`Job`] out. What it may do is narrowed by [`StepTarget`], which names three
//! destinations of the six states — so the two M1 cannot reach are not refused
//! at runtime, they cannot be asked for.
//!
//! # The other mutators, and what each may touch
//!
//! [`on_branch`](Job::on_branch) writes `branch` alone and mints no event,
//! because no event carries a worktree.
//! [`drone_spawned`](Job::drone_spawned) and
//! [`drone_exited`](Job::drone_exited) write `assigned_drone` alone and do mint
//! one, because presence has to fold. **No method here takes `&mut self`.**

use alloc::vec::Vec;

use crate::envelope::{Actor, Timestamp};
use crate::job::drone::{DroneAssigned, DroneMoved, DronePresence, IllegalDroneMove};
use crate::job::event::{JobEvent, StepEvent};
use crate::job::fields::{
    AcceptanceCriterion, Attachment, Branch, DependencyEdge, DispatchOrigin, Facts, GateManifest,
    Origin, ScopeRevision, Subject, TopLevelOrigin, Urgency, WriteTargets,
};
use crate::job::ids::{DroneId, JobId, ManifestId, ModelName, StepId, Title, WorkflowId};
use crate::job::status::JobStatus;
use crate::job::step::{rows_at_creation, JobStep, StepSeed};
use crate::job::step_machine::{admits_step, IllegalStepTransition, StepTarget};
use crate::job::transition::{admits, IllegalTransition, Target};
use crate::job::workflow::FrozenWorkflow;

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
    /// The name a person reads in a list row. A [`Title`] and not a `String`,
    /// so there is no way to hand this constructor a Job nobody could pick out
    /// of a list — the refusal happens where the text is typed, not here.
    pub title: Title,
    /// The WorkflowDef this Job follows, **resolved once and kept**. Editing
    /// the file it came from changes the next Job, not this one.
    pub workflow: FrozenWorkflow,
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
    /// Files handed to the Job at proposal time, promoted from wherever they
    /// were staged into Fleet's own keeping. Frozen at creation like the rest
    /// of this constructor's fields — a Job does not grow attachments after it
    /// exists, it only ever gets copies of the ones it started with.
    pub attachments: Vec<Attachment>,
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

/// A Job, and the step move that produced it.
///
/// The pair travels together for the reason [`Transitioned`] does: the row and
/// its log entry are written in one transaction, and a signature handing back
/// only the Job would make forgetting the entry the easy path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTransitioned {
    pub job: Job,
    pub event: StepEvent,
}

/// The record of work to be accomplished. **Data, not an actor.**
///
/// Fleet is the only thing that drives a transition on it; a Drone's
/// self-report is an input signal and never authoritative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    id: JobId,
    title: Title,
    /// The instant the constructor was called. Not a field creation *decides*
    /// — it is the argument every constructor already takes — and kept because
    /// no event describes creation, so nothing else can answer how long a Job
    /// has been open.
    created_at: Timestamp,
    status: JobStatus,
    workflow: FrozenWorkflow,
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
    /// Written when the worktree is made, not at creation. `None` is a Job that
    /// has never been dispatched, and is absent rather than a name it does not
    /// yet have.
    branch: Option<Branch>,
    subject: Option<Subject>,
    facts: Facts,
    scope_revisions: Vec<ScopeRevision>,
    attachments: Vec<Attachment>,
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
            title: new.title,
            created_at: at,
            status: entry,
            workflow: new.workflow,
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
            // No worktree exists yet. `on_branch` is what fills this in.
            branch: None,
            subject: new.subject,
            facts: new.facts,
            scope_revisions: new.scope_revisions,
            attachments: new.attachments,
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

    /// Record the branch the Job's worktree was made on.
    ///
    /// **The third mutator, and the only one the log does not describe.** No
    /// event carries a worktree, so this column is the authority for its own
    /// field the way every non-status column is, and the rebuild reads it back
    /// rather than folding it. It takes `&self` and returns a new `Job` like
    /// the other two, so the no-setter property is unchanged.
    ///
    /// It overwrites: a branch can be renamed, and a record that could not say
    /// so would be the formula this field replaced.
    pub fn on_branch(&self, branch: Branch) -> Job {
        let mut job = self.clone();
        job.branch = Some(branch);
        job
    }

    /// Record that a Drone is now working this Job.
    ///
    /// Refuses where one already is: `assigned_drone` is a single pointer, and
    /// a second spawn over a live one would lose the first Drone's id — the
    /// only thing naming its transcript.
    pub fn drone_spawned(
        &self,
        drone: DroneId,
        by: Actor,
        at: Timestamp,
    ) -> Result<DroneAssigned, IllegalDroneMove> {
        if let Some(held) = &self.assigned_drone {
            return Err(IllegalDroneMove::AlreadyAssigned {
                held: held.clone(),
                offered: drone,
            });
        }
        Ok(self.drone_moved(drone, DronePresence::Spawned, by, at))
    }

    /// Record that the Job's Drone is gone, however it went.
    ///
    /// Refuses where none is assigned. A Job that never had a Drone cannot lose
    /// one, and an exit recorded twice would read as two Drones having run.
    pub fn drone_exited(
        &self,
        by: Actor,
        at: Timestamp,
    ) -> Result<DroneAssigned, IllegalDroneMove> {
        let held = self
            .assigned_drone
            .clone()
            .ok_or(IllegalDroneMove::NoneAssigned)?;
        Ok(self.drone_moved(held, DronePresence::Exited, by, at))
    }

    fn drone_moved(
        &self,
        drone: DroneId,
        presence: DronePresence,
        by: Actor,
        at: Timestamp,
    ) -> DroneAssigned {
        let event = DroneMoved::recorded(
            self.id.clone(),
            drone.clone(),
            presence,
            self.status,
            by,
            at,
        );
        let mut job = self.clone();
        job.assigned_drone = match presence {
            DronePresence::Spawned => Some(drone),
            DronePresence::Exited => None,
        };
        DroneAssigned { job, event }
    }

    /// Move one step of the frozen WorkflowDef, or say why it cannot move.
    ///
    /// The second and last mutator. Three things can refuse it: the Job has no
    /// such step, the Job is not in a status the inner machine advances
    /// beneath, or no edge joins the two states. A fourth refusal is absent
    /// because it is unrepresentable — [`StepTarget`] names no state a step
    /// may not be moved to.
    ///
    /// **Entering `running` is what moves `current_step_id`, and nothing
    /// clears it.** `job-fields.toml` says the nested machine is "frozen
    /// otherwise, never cleared, still rendered", so a Job that advanced or
    /// stopped its last step still points at it — which is what a rail renders,
    /// and what lets an escalated Job say which step stopped.
    ///
    /// `at` is a parameter and never a clock reading, for the reason
    /// [`transition`](Job::transition) gives.
    pub fn transition_step(
        &self,
        step_id: &StepId,
        to: StepTarget,
        by: Actor,
        at: Timestamp,
    ) -> Result<StepTransitioned, IllegalStepTransition> {
        let index = self
            .steps
            .iter()
            .position(|row| row.step_id() == step_id)
            .ok_or_else(|| IllegalStepTransition::NoSuchStep {
                step_id: step_id.clone(),
            })?;
        let from = self.steps[index].state();
        admits_step(self.status, step_id, from, &to)?;

        let event = StepEvent::recorded(
            self.id.clone(),
            step_id.clone(),
            from,
            &to,
            self.status,
            by,
            at.clone(),
        );
        let mut job = self.clone();
        job.steps[index] = self.steps[index].moved_to(&to, at);
        if matches!(to, StepTarget::Running) {
            job.current_step_id = Some(step_id.clone());
        }
        Ok(StepTransitioned { job, event })
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }
    /// When the Job was created. The clock the whole-Job elapsed is measured
    /// from, and the one instant on the record that no transition carries.
    pub fn created_at(&self) -> &Timestamp {
        &self.created_at
    }
    /// The name a person reads. Never frozen — a name can be corrected — but
    /// there is no setter here either: correcting one is a write nothing owns
    /// yet, and a `&mut self` added for it would be the first crack in the
    /// no-setter rule the rest of this file holds.
    pub fn title(&self) -> &Title {
        &self.title
    }
    pub fn status(&self) -> JobStatus {
        self.status
    }
    /// The WorkflowDef the Job froze. **What Fleet reads**, in place of the
    /// file, so a step declares at dispatch what it declared at approval.
    pub fn workflow(&self) -> &FrozenWorkflow {
        &self.workflow
    }
    /// Read off the frozen workflow rather than stored beside it: two
    /// statements of one fact can disagree, and this one is a join key.
    pub fn workflow_id(&self) -> &WorkflowId {
        self.workflow.id()
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
    /// The branch the Job's worktree is on. `None` until one is made.
    pub fn branch(&self) -> Option<&Branch> {
        self.branch.as_ref()
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
    /// Files handed to the Job at proposal time. Frozen at creation — nothing
    /// on this record adds to the list afterwards.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
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
