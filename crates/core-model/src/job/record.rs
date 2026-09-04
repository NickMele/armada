//! The `jobs` row, and the only thing that moves it.
//! **No setter, and no `&mut self` at all.** Every field is private, and the
//! only method producing a [`Job`] with a different status is
//! [`transition`](Job::transition). A caller holding a `Job` cannot write its
//! status because there is nothing to call — the difference between a rule and
//! a check.
//!
//! **Evidence is deliberately absent.** The registry gives it its own table,
//! three open questions about how it serialises, and a rule that captured Check
//! output goes to disk with a pointer in the row. It is loaded and appended by
//! `store` rather than carried inline: a blob on the Job row would rewrite
//! whole on every append.
//!
//! **The inner machine's writer is the second mutator.**
//! [`transition_step`](Job::transition_step) moves a step and the cursor under
//! the same rule as [`transition`](Job::transition): `&self` in, a new [`Job`]
//! out. What it may do is narrowed by [`StepTarget`], which names only states
//! M1 reaches — so a step state M1 does not reach is not refused at runtime, it
//! cannot be asked for.
//!
//! **The other mutators write one field and mint nothing** —
//! [`on_branch`](Job::on_branch), [`redirect_waits`](Job::redirect_waits),
//! [`redirect_delivered`](Job::redirect_delivered) — because no event carries a
//! worktree or a person's note. [`drone_spawned`](Job::drone_spawned) and
//! [`drone_exited`](Job::drone_exited) do mint one: presence has to fold.

use alloc::vec::Vec;

use crate::envelope::{Actor, Timestamp};
use crate::job::drone::{DroneAssigned, DroneMoved, DronePresence, IllegalDroneMove};
use crate::job::escalation::StepLevelTrigger;
use crate::job::event::{JobEvent, StepEvent};
use crate::job::fields::{
    AcceptanceCriterion, Attachment, Branch, DependencyEdge, DispatchOrigin, Facts, GateManifest,
    Origin, ScopeRevision, Subject, TopLevelOrigin, Urgency, WriteTargets,
};
use crate::job::ids::{DroneId, JobId, ManifestId, ModelName, StepId, Title, WorkflowId};
use crate::job::note::{RedirectAlreadyWaiting, RedirectWaiting};
use crate::job::status::{JobStatus, StepState};
use crate::job::step::{rows_at_creation, JobStep, StepSeed, StepVerdict};
use crate::job::step_machine::{admits_step, IllegalStepTransition, StepTarget};
use crate::job::transition::{admits, IllegalTransition, Target};
use crate::job::workflow::{FrozenWorkflow, ResolvedStep};

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
    dependencies: Vec<DependencyEdge>,
    dispatched_by: Option<DispatchOrigin>,
    redispatched_from: Option<JobId>,
    /// Written when the worktree is made, not at creation. `None` is a Job that
    /// has never been dispatched, and is absent rather than a name it does not
    /// yet have.
    branch: Option<Branch>,
    /// A person's note written where there was no Drone to take it, waiting
    /// for the next one. `None` on every Job that has never been sent back
    /// across a gate, and on every Job whose note has been delivered — see
    /// [`note`](mod@crate::job::note) for why those two are one value.
    redirect_waiting: Option<RedirectWaiting>,
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
            dependencies: new.dependencies,
            dispatched_by,
            redispatched_from: new.redispatched_from,
            // No worktree exists yet. `on_branch` is what fills this in.
            branch: None,
            // Nothing has been said to a Job that does not exist yet.
            redirect_waiting: None,
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
    /// **The steps are read here, and this is the only place the outer machine
    /// looks at the inner one.** An edge carrying a [`Guard`](crate::Guard) is
    /// admitted only where the Job's `job_steps` rows satisfy it, which is what
    /// makes `completed_success` unwritable while a step is still running.
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
        admits(self.status, &to, &self.steps)?;
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

    /// Hold a person's note until the next Drone opens with it.
    ///
    /// **The fourth mutator, and the second the log does not describe.** It
    /// takes `&self` and returns a new `Job` like the other three, so the
    /// no-setter property is unchanged.
    ///
    /// **It does not overwrite, which is the whole of the decision.** A second
    /// note arriving over an undelivered first is
    /// [`RedirectAlreadyWaiting`] — see that type for why the other two
    /// answers were rejected.
    pub fn redirect_waits(&self, note: RedirectWaiting) -> Result<Job, RedirectAlreadyWaiting> {
        if let Some(held) = &self.redirect_waiting {
            return Err(RedirectAlreadyWaiting { held: held.clone() });
        }
        let mut job = self.clone();
        job.redirect_waiting = Some(note);
        Ok(job)
    }

    /// The note went into a Drone's opening brief, so nothing is waiting.
    ///
    /// **Cleared on delivery and not on being acted on**, which is the ruling:
    /// a note that outlived one boundary would reach a Drone working a part it
    /// was never about. Answering the same `Job` where nothing was waiting is
    /// correct rather than a fault — every spawn asks, and almost none of them
    /// is carrying anything.
    pub fn redirect_delivered(&self) -> Job {
        let mut job = self.clone();
        job.redirect_waiting = None;
        job
    }

    /// Record that a Drone is now working one step of this Job.
    ///
    /// **Refuses where one already is on that step, and the refusal narrowed
    /// rather than went away.** `assigned_drone` is one pointer per step, and
    /// `restart_step` is the act that puts a second Drone on the same step, so
    /// a spawn over a live one would still lose the first Drone's id — the only
    /// thing naming its transcript. What it no longer refuses is a Drone on
    /// step two while step one's row still names the Drone that worked it,
    /// which is the whole point of the column being per step.
    ///
    /// Refuses a step the Job does not have. The Job's steps are frozen at
    /// creation, so a name that is not among them is a caller's mistake and
    /// never a Job that has moved on.
    pub fn drone_spawned(
        &self,
        step_id: &StepId,
        drone: DroneId,
        by: Actor,
        at: Timestamp,
    ) -> Result<DroneAssigned, IllegalDroneMove> {
        let index = self.step_index(step_id)?;
        if let Some(held) = self.steps[index].assigned_drone() {
            return Err(IllegalDroneMove::AlreadyAssigned {
                step: step_id.clone(),
                held: held.clone(),
                offered: drone,
            });
        }
        Ok(self.drone_moved(index, drone, DronePresence::Spawned, by, at))
    }

    /// Record that the Drone on a step is gone, however it went.
    ///
    /// Refuses where none is on that step. A step that never had a Drone cannot
    /// lose one, and an exit recorded twice would read as two Drones having run
    /// it.
    pub fn drone_exited(
        &self,
        step_id: &StepId,
        by: Actor,
        at: Timestamp,
    ) -> Result<DroneAssigned, IllegalDroneMove> {
        let index = self.step_index(step_id)?;
        let held = self.steps[index].assigned_drone().cloned().ok_or_else(|| {
            IllegalDroneMove::NoneAssigned {
                step: step_id.clone(),
            }
        })?;
        Ok(self.drone_moved(index, held, DronePresence::Exited, by, at))
    }

    /// Where a step is among the rows, or the refusal that it is not one.
    fn step_index(&self, step_id: &StepId) -> Result<usize, IllegalDroneMove> {
        self.steps
            .iter()
            .position(|row| row.step_id() == step_id)
            .ok_or_else(|| IllegalDroneMove::NoSuchStep {
                step: step_id.clone(),
            })
    }

    fn drone_moved(
        &self,
        index: usize,
        drone: DroneId,
        presence: DronePresence,
        by: Actor,
        at: Timestamp,
    ) -> DroneAssigned {
        let event = DroneMoved::recorded(
            self.id.clone(),
            self.steps[index].step_id().clone(),
            drone.clone(),
            presence,
            self.status,
            by,
            at,
        );
        let mut job = self.clone();
        job.steps[index] = self.steps[index].drone_now(match presence {
            DronePresence::Spawned => Some(drone),
            DronePresence::Exited => None,
        });
        DroneAssigned { job, event }
    }

    /// Move one step of the frozen WorkflowDef, or say why it cannot move.
    ///
    /// The second and last mutator. Four things can refuse it: the Job has no
    /// such step, the Job is not in a status the inner machine advances
    /// beneath, no edge joins the two states, or a stopped step was advanced as
    /// a pass rather than as an override. A refusal for the destination itself
    /// is absent because it is unrepresentable — [`StepTarget`] names no state
    /// a step may not be moved to.
    ///
    /// **Entering `running` is what moves `current_step_id`, and nothing
    /// clears it — including backwards.** A loop return is the one move that
    /// takes the cursor to a *lower* ordinal, and it is the target step's
    /// cursor that moves: `verdict_routing` names where the work goes, and
    /// where the work goes is where the next Drone is put. The step that
    /// emitted the verdict is not moved by this call.
    ///
    /// **Which state that emitting step is left in is not decided here or
    /// anywhere else yet** — `workflows.toml`'s `design_plan` row leaves it
    /// open, and none of the six declared states says "ruled, and waiting for
    /// the loop to come back". `fleet::gate` is where the second move will be
    /// driven from when it is decided.
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
        if to.begins_a_run() {
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
    /// What the Drone on `step` is spawned as: the step's own, or the Job's
    /// where the step named none.
    ///
    /// **The fallback is spelled here and nowhere else.** It used to be spelled
    /// nowhere at all — one process spanned the whole Job, so `job.model()` was
    /// the answer at every step by construction. Now that a step is its own
    /// process the two can differ, and a second call site deciding what absent
    /// means is a second place a step could quietly stop getting the model its
    /// workflow asked for.
    ///
    /// A [`StepId`] this workflow does not declare answers with the Job's. That
    /// is not a silent miss to be worried about: `FrozenWorkflow::step` is the
    /// same lookup every caller already does, and a Drone put on a step the
    /// workflow has never heard of has a larger problem than which model it is.
    pub fn model_at(&self, step: &StepId) -> &ModelName {
        self.workflow
            .step(step)
            .and_then(ResolvedStep::model)
            .unwrap_or(&self.model)
    }
    pub fn acceptance_criteria(&self) -> &[AcceptanceCriterion] {
        &self.acceptance_criteria
    }
    pub fn current_step_id(&self) -> Option<&StepId> {
        self.current_step_id.as_ref()
    }
    /// The Drone on this Job. Presence, not state — and null also suspends the
    /// liveness clock.
    ///
    /// **Derived from the step rows, never stored.** The pointer is a
    /// `job_steps` column, and this is the reading every caller of the old
    /// Job-level field was after: whether a process is on the Job right now. It
    /// goes null at a step boundary and stays null while a person reads at
    /// `awaiting_review`, because there is no Drone then — and each step's row
    /// still names the Drone that worked it, which is what a finished Job's
    /// history is read from.
    ///
    /// **Any step and not the current one.** A Drone is always put on the step
    /// the cursor names, so on every history the two readings agree; taking it
    /// from the cursor would make this answer `None` on a record where a
    /// pointer is genuinely set, which is the one answer it must never give.
    pub fn assigned_drone(&self) -> Option<&DroneId> {
        self.steps_holding_a_drone().next().map(|(_, drone)| drone)
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
    /// The note the next Drone will open with, where a person left one.
    ///
    /// **Presence is "a person spoke and nobody was there to hear it"**, and
    /// it is not a status: the Job stands wherever the act that carried the
    /// note left it.
    pub fn redirect_waiting(&self) -> Option<&RedirectWaiting> {
        self.redirect_waiting.as_ref()
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
    /// Every step that has a Drone on it, in the order the rows were written.
    ///
    /// **The reading a boot reconciliation is made of.** A Drone is spoken to
    /// through a pipe the Fleet that spawned it holds, so a Fleet that has just
    /// started holds none of them and every one of these pointers is a claim
    /// about a process nobody can reach. Ordinarily one, because one Drone
    /// works one step at a time; the iterator is what makes that a reading
    /// rather than an assumption.
    pub fn steps_holding_a_drone(&self) -> impl Iterator<Item = (&StepId, &DroneId)> {
        self.steps
            .iter()
            .filter_map(|step| step.assigned_drone().map(|drone| (step.step_id(), drone)))
    }

    /// The `workflow_status` projection: the row `current_step_id` names.
    /// Derived, never stored — storing it would risk the two disagreeing.
    pub fn current_step(&self) -> Option<&JobStep> {
        self.current_step_id.as_ref().and_then(|id| self.step(id))
    }

    /// The step that stopped, and the trigger that stopped it.
    ///
    /// **The one reading, because four acts and one classification make it.**
    /// A restart lands on this step, an override lifts this trigger, a gate
    /// re-run answers it, and [`Stuck`](crate::Stuck) reports it; each walked
    /// the rows itself, and a fifth spelling of the search is how a screen
    /// comes to name a step no act would move.
    ///
    /// **Both or neither.** A `stopped` row with no `failed(<trigger>)` on it
    /// is a row nothing could say why about — the machine does not admit one,
    /// and returning the step alone would offer acts with nothing to act on. It
    /// asks nothing about the Job's status, which is each act's own guard.
    pub fn stopped_on(&self) -> Option<(&StepId, StepLevelTrigger)> {
        self.steps
            .iter()
            .find(|step| step.state() == StepState::Stopped)
            .and_then(|step| match step.last_verdict() {
                Some(StepVerdict::Failed(trigger)) => Some((step.step_id(), trigger)),
                _ => None,
            })
    }
}
