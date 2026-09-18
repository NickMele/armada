//! A Job's plan: read for the wire, and changed by the Drone working it.
//!
//! **Which step may make which change is one predicate**, [`permitted`], and
//! [`plan_grants`] is the same two questions asked where a toolbelt is built —
//! `crate::spawning::dispatches`' shape, so an allowlist and a refusal cannot
//! disagree. **Nothing here gates a submission**: a task's state is a claim.

use std::fmt;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Grant, Vcs, WorkProduct};
use api::Refusal;
use core_model::{
    Actor, DropReason, EvidenceType, FrozenWorkflow, JobId, NewTask, PlanChange, PlanRefused,
    PlanTask, ResolvedStep, StepEvidence, StepId, TaskId, TaskState, TaskUpdate, Timestamp,
    WorkPlan,
};
use ipc::mcp::NotRecorded;
use store::{PlanHand, PlanNotKept};

use crate::adrift::Adrift;
use crate::budget::budgeted_for;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};

/// Why a Drone's change to the plan was not kept. **None of these moves a
/// step**, and each says what to do instead in words the Drone can act on.
#[derive(Debug)]
pub(crate) enum NotPlanned {
    NothingIsWorking,
    /// The Job stands at a step its workflow does not name: a fault in Fleet.
    NoSuchStep {
        step: StepId,
    },
    NotItsToRecord {
        step: StepId,
    },
    NotItsToFollow {
        step: StepId,
    },
    Refused(PlanRefused),
    /// The store would not take a change the plan could. Not the Drone's.
    NotKept(String),
}

impl fmt::Display for NotPlanned {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        const CARRY_ON: &str = "Carry on with the part you were given";
        match self {
            NotPlanned::NothingIsWorking => out.write_str(
                "no Job is being worked, so there is no plan for this call to change. \
                 Stop — the Job this Drone was started for has already ended",
            ),
            NotPlanned::NoSuchStep { step } => write!(
                out,
                "the Job is standing at step `{}`, which its workflow does not name. \
                 This is a fault in Fleet and not in the call",
                step.as_str()
            ),
            NotPlanned::NotItsToRecord { step } => write!(
                out,
                "step `{}` does not record the Job's plan, so `record_plan` is not this \
                 part's to call. {CARRY_ON}",
                step.as_str()
            ),
            NotPlanned::NotItsToFollow { step } => write!(
                out,
                "step `{}` does not work from the Job's plan, so it has no task to add or \
                 update. {CARRY_ON}",
                step.as_str()
            ),
            NotPlanned::Refused(PlanRefused::NoPlan) => write!(
                out,
                "no plan has been recorded for this Job, so there is no task to add or \
                 update. {CARRY_ON}"
            ),
            NotPlanned::Refused(PlanRefused::NoSuchTask { named }) => write!(
                out,
                "the plan holds no task {named}. Name one of the plan's own task ids and \
                 call again"
            ),
            NotPlanned::Refused(PlanRefused::NoSuchPlace { named }) => write!(
                out,
                "the plan holds no task {named} to add after. Name one of its ids, or send \
                 \"\" to add it at the end"
            ),
            NotPlanned::Refused(PlanRefused::StaysDropped { named }) => write!(
                out,
                "task {named} was dropped, and a dropped task stays dropped. If the work \
                 is needed after all, say so in `not_claimed` when you submit; a person \
                 can add it back as a new task. {CARRY_ON}"
            ),
            NotPlanned::NotKept(why) => write!(
                out,
                "the change could not be written down ({why}). It is not yours to fix. \
                 {CARRY_ON}"
            ),
        }
    }
}

impl From<NotPlanned> for NotRecorded {
    fn from(why: NotPlanned) -> NotRecorded {
        NotRecorded {
            because: why.to_string(),
        }
    }
}

/// What a step's part in the plan puts in its toolbelt.
pub(crate) fn plan_grants(step: &ResolvedStep) -> Vec<Grant> {
    let mut grants = Vec::new();
    if step.records_plan() {
        grants.push(Grant::RecordThePlan);
    }
    if step.follows_plan() {
        grants.push(Grant::WorkThePlan);
    }
    grants
}

/// Whether this step may make this change. **A recording is the recording
/// step's alone**, so no later step can replace the plan it is working from.
pub(crate) fn permitted(step: &ResolvedStep, change: &PlanChange) -> Result<(), NotPlanned> {
    match change {
        PlanChange::Recorded { .. } if !step.records_plan() => Err(NotPlanned::NotItsToRecord {
            step: step.id().clone(),
        }),
        PlanChange::Added { .. } | PlanChange::Updated { .. } if !step.follows_plan() => {
            Err(NotPlanned::NotItsToFollow {
                step: step.id().clone(),
            })
        }
        _ => Ok(()),
    }
}

/// What `<recording_step_id>.evidence` reads: the plan as it stands when the
/// gate runs, put beside whatever a Drone submitted about the step's own
/// product. `reference_docs` and `baseline_ref` both reach this through
/// `AtStep::baseline`, so one write here reaches both. `#895`, widened by
/// `#1006`.
///
/// **The plan is never the whole of the record where the step has a product
/// of its own.** A plan-product step's own submission already *is* the plan,
/// so that case still replaces whole, exactly as before `#1006` — there is
/// nothing beside it to lose. A step that declared `records_plan: true`
/// beside another product keeps that submission and gets the plan appended
/// after it: Epic's `dispatch` and `roll_up` still read `plan.evidence` as
/// the split document `plan` produced, with the plan added rather than
/// standing in its place.
pub(crate) fn with_the_plan(
    mut recorded: Vec<(StepId, StepEvidence)>,
    workflow: &FrozenWorkflow,
    plan: Option<&WorkPlan>,
) -> Vec<(StepId, StepEvidence)> {
    let (Some(step), Some(plan)) = (
        workflow.steps().iter().find(|step| step.records_plan()),
        plan,
    ) else {
        return recorded;
    };
    let own = recorded
        .iter()
        .position(|(id, _)| id == step.id())
        .map(|at| recorded.remove(at).1);
    let with_the_plan = match own {
        // The step's own product is not the plan: keep what it submitted and
        // put the plan after it, labelled apart, so a later step's
        // `<step>.evidence` reads both rather than losing one.
        Some(own) if own.evidence_type != EvidenceType::Plan => StepEvidence {
            claimed: format!(
                "{}\n\nThe Job's plan, as Fleet recorded it:\n\n{}",
                own.claimed,
                plan.rendered()
            ),
            ..own
        },
        // The step's own product is the plan, or nothing was recorded for it
        // yet: the plan is the whole of what there is to read.
        Some(_) | None => StepEvidence {
            evidence_type: EvidenceType::Plan,
            claimed: plan.rendered(),
            shown_by: String::from("Fleet's own record of the plan"),
            not_claimed: String::new(),
        },
    };
    recorded.push((step.id().clone(), with_the_plan));
    recorded
}

/// What a person's add or drop is told to a working Drone. **Fleet's own
/// sentence, never a person's words** — `redirect_drone` is where those
/// travel. `docs/contracts/agent-prompt.md` section 4a has the drafted
/// wording. `#897`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanChanged(String);

impl PlanChanged {
    /// A task added, at the id the plan gave it.
    pub fn added(id: TaskId, task: &NewTask) -> PlanChanged {
        PlanChanged(format!(
            "THE PLAN CHANGED\n\nA person added a task to this Job's plan: {id} {}. It \
             is open, for you or a later part to pick up. This is not a question. Carry \
             on with the part you were given.",
            task.title()
        ))
    }

    /// A task dropped, with the reason.
    pub fn dropped(id: TaskId, title: &str, reason: &str) -> PlanChanged {
        PlanChanged(format!(
            "THE PLAN CHANGED\n\nA person dropped a task from this Job's plan: {id} \
             {title}. Reason: {reason}. This is settled, not a question to raise — the \
             task stays dropped unless a person adds it back. Carry on with the part \
             you were given."
        ))
    }

    /// The turn, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// Why the store would not keep a person's change, once the plan itself would
/// have taken it. Not the person's to fix.
fn plan_not_kept(job: &JobId, why: PlanNotKept) -> Adrift {
    match why {
        PlanNotKept::Refused(refused) => Adrift::PlanRefused {
            job: job.clone(),
            why: refused,
        },
        other => Adrift::PlanNotKept {
            job: job.clone(),
            because: other.to_string(),
        },
    }
}

/// The word a kept change is answered with. An added task answers with its id,
/// which is the one thing the Drone needs to name it later.
pub(crate) fn receipt_word(change: &PlanChange, plan: &WorkPlan) -> String {
    match change {
        PlanChange::Recorded { .. } => "recorded".to_string(),
        PlanChange::Updated { .. } => "updated".to_string(),
        PlanChange::Added { .. } => plan
            .tasks()
            .iter()
            .map(|task| task.id())
            .max()
            .map(|id| id.to_string())
            .unwrap_or_default(),
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The plan this Job's history leaves, or `None` where none was recorded.
    pub(crate) async fn plan_of(&self, job: &JobId) -> Result<Option<WorkPlan>, Adrift> {
        self.store()
            .lock()
            .await
            .work_plan(job)
            .map_err(Adrift::Reading)
    }

    /// A row's task counts. **Absent is a Job with no plan**, which a Board
    /// draws as no task field rather than an empty one.
    pub(crate) async fn task_counts(&self, job: &JobId) -> Result<Option<ipc::TaskCounts>, Adrift> {
        Ok(self.plan_of(job).await?.map(|plan| plan.counts().into()))
    }

    /// Keep one change the working Drone made to its Job's plan.
    ///
    /// The Drone names no Job and no step; both are read off **its own** slot,
    /// held for the whole call so the step cannot advance between the check and
    /// the write — `Fleet::declare_scope`'s binding, for its reason.
    pub(crate) async fn change_plan(
        &self,
        caller: &JobId,
        change: &PlanChange,
    ) -> Result<WorkPlan, NotPlanned> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotPlanned::NothingIsWorking);
        };
        let working = slot.lock().await;
        let Some(at_work) = working.as_ref() else {
            return Err(NotPlanned::NothingIsWorking);
        };
        let (job, step, _) = at_work.standing();
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotPlanned::NoSuchStep { step: step.clone() })?;
        let declared = record
            .workflow()
            .step(&step)
            .ok_or_else(|| NotPlanned::NoSuchStep { step: step.clone() })?;
        permitted(declared, change)?;
        let at = self.now();
        let plan = self
            .store()
            .lock()
            .await
            .change_plan(&job, change, PlanHand::Step(&step), &at)
            .map_err(|why| match why {
                PlanNotKept::Refused(refused) => NotPlanned::Refused(refused),
                other => NotPlanned::NotKept(other.to_string()),
            })?;
        drop(working);
        self.publish(ipc::Event::JobPlanChanged(ipc::JobPlanChanged {
            job_id: (&job).into(),
            tasks: plan.counts().into(),
            actor: Actor::Drone.into(),
            at: (&at).into(),
        }));
        Ok(plan)
    }

    /// A person adds a task to the Job's plan, from Bridge. `#897`.
    ///
    /// **Refused by name**: an empty title, or an `after` naming nothing the
    /// plan holds — both `Adrift::Unnameable`, `redirect_drone`'s reuse of it
    /// for a value that cannot work — or `Adrift::PlanRefused` where the Job
    /// has no plan at all.
    pub(crate) async fn add_task_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        add: ipc::AddTask,
    ) -> Result<ipc::WorkPlan, Refusal> {
        let scope: Vec<&str> = add.scope.iter().map(String::as_str).collect();
        let task = NewTask::new(&add.title, &add.note, &scope, &add.expects)
            .ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let after = match add.after.trim() {
            "" => None,
            named => Some(TaskId::read(named).ok_or_else(|| self.refusal(Adrift::Unnameable))?),
        };
        let plan = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let job = job_id.to_domain();
            async move { fleet.added_by_person(&job, task, after).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok((&plan).into())
    }

    /// A person drops a task from the Job's plan, with a reason, from Bridge.
    /// `#897`.
    ///
    /// **Refused by name**: an empty reason, or a task the plan does not
    /// hold — both `Adrift::Unnameable` — a Job with no plan at all
    /// (`Adrift::PlanRefused`), or a task already `done` or already
    /// `dropped` (`Adrift::TaskAlreadySettled`) — a person's drop is not
    /// repeating a decision already made.
    pub(crate) async fn drop_task_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        body: ipc::DropTask,
    ) -> Result<ipc::WorkPlan, Refusal> {
        let task =
            TaskId::read(body.task.trim()).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let reason =
            DropReason::new(&body.reason).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let plan = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let job = job_id.to_domain();
            async move { fleet.dropped_by_person(&job, task, reason).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok((&plan).into())
    }

    async fn added_by_person(
        &self,
        job: &JobId,
        task: NewTask,
        after: Option<TaskId>,
    ) -> Result<WorkPlan, Adrift> {
        let at = self.now();
        let change = PlanChange::Added {
            task: task.clone(),
            after,
        };
        let plan = {
            let mut store = self.store().lock().await;
            store
                .change_plan(job, &change, PlanHand::Person, &at)
                .map_err(|why| plan_not_kept(job, why))?
        };
        let id = plan
            .tasks()
            .iter()
            .map(PlanTask::id)
            .max()
            .expect("the task just added is in the plan it leaves");
        self.told_plan_changed(job, &plan, &at);
        self.deliver_plan_change(job, &PlanChanged::added(id, &task))
            .await;
        Ok(plan)
    }

    async fn dropped_by_person(
        &self,
        job: &JobId,
        task: TaskId,
        reason: DropReason,
    ) -> Result<WorkPlan, Adrift> {
        let at = self.now();
        let change = PlanChange::Updated {
            task,
            to: TaskUpdate::Dropped(reason.clone()),
            // A person's drop says why in `reason`; nothing proved a task
            // that is not being done.
            shown: None,
        };
        let plan = {
            let mut store = self.store().lock().await;
            let settled = store
                .work_plan(job)
                .map_err(Adrift::Reading)?
                .and_then(|plan| plan.task(task).cloned());
            if let Some(existing) = settled {
                if matches!(existing.state(), TaskState::Dropped | TaskState::Done) {
                    return Err(Adrift::TaskAlreadySettled {
                        job: job.clone(),
                        named: task,
                        state: existing.state(),
                    });
                }
            }
            store
                .change_plan(job, &change, PlanHand::Person, &at)
                .map_err(|why| plan_not_kept(job, why))?
        };
        let title = plan
            .task(task)
            .map(PlanTask::title)
            .unwrap_or_default()
            .to_string();
        self.told_plan_changed(job, &plan, &at);
        self.deliver_plan_change(job, &PlanChanged::dropped(task, &title, reason.as_str()))
            .await;
        Ok(plan)
    }

    /// Publish `job.plan_changed`, actor `Human` — a person's act, never
    /// Fleet's own.
    fn told_plan_changed(&self, job: &JobId, plan: &WorkPlan, at: &Timestamp) {
        self.publish(ipc::Event::JobPlanChanged(ipc::JobPlanChanged {
            job_id: job.into(),
            tasks: plan.counts().into(),
            actor: Actor::Human.into(),
            at: at.into(),
        }));
    }

    /// Tell a working Drone what a person's add or drop changed. **Only where
    /// there is a live session on this Job** — at a step boundary, or with no
    /// session, this sends nothing, and the next brief's THE PLAN carries it.
    /// It never respawns to deliver itself, `redirect_drone`'s own rule.
    async fn deliver_plan_change(&self, job: &JobId, note: &PlanChanged) {
        let Some(slot) = self.slot_of(job).await else {
            return;
        };
        let working = slot.lock().await;
        let Some(at_work) = working.as_ref().filter(|at_work| at_work.is(job)) else {
            return;
        };
        at_work.instructed(Occasion::Plan, note.text());
        let _ = at_work.session().plan_changed(note).await;
    }
}
