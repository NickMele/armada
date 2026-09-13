//! A Job's plan: read for the wire, and changed by the Drone working it.
//!
//! **Which step may make which change is one predicate**, [`permitted`], and
//! [`plan_grants`] is the same two questions asked where a toolbelt is built —
//! `crate::spawning::dispatches`' shape, so an allowlist and a refusal cannot
//! disagree. **Nothing here gates a submission**: a task's state is a claim.

use std::fmt;

use adapter_traits::{AgentHarness, Delivery, Grant, Vcs, WorkProduct};
use core_model::{
    Actor, EvidenceType, FrozenWorkflow, JobId, PlanChange, PlanRefused, ResolvedStep, StepEvidence,
    StepId, WorkPlan,
};
use ipc::mcp::NotRecorded;
use store::{PlanHand, PlanNotKept};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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

/// What `<plan_step_id>.evidence` reads: the plan as it stands when the gate
/// runs, replacing whatever a Drone submitted about it. `reference_docs` and
/// `baseline_ref` both reach this through `AtStep::baseline`, so one write
/// here reaches both. `#895`.
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
    recorded.retain(|(id, _)| id != step.id());
    recorded.push((
        step.id().clone(),
        StepEvidence {
            evidence_type: EvidenceType::Plan,
            claimed: plan.rendered(),
            shown_by: String::from("Fleet's own record of the plan"),
            not_claimed: String::new(),
        },
    ));
    recorded
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
}
