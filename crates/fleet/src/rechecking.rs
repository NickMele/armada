//! Running a stopped step's Checks again, on a person's press (#1105).
//!
//! The step's gate is asked again on the worktree as it stands and the evidence
//! already submitted, with no Drone and no retry spent. **Decided before
//! anything moves**: a failure, or one with budget to hand back, leaves the Job
//! held; any other reading takes `awaiting_repair -> running` as a person's act.
//! Fleet never presses it on its own, for `crate::regating`'s reason, and the
//! run is a task of its own for `crate::showing_again`'s.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use core_model::{
    Actor, CheckOutcome, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus,
    Level, StepId, StepLevelTrigger, StepTarget, Target, DIFF_NONEMPTY,
};
use verification::{Lifted, Request};

use crate::adrift::Adrift;
use crate::at_step::AtStep;
use crate::daemon::Fleet;
use crate::gate::{rule_on, Began, Ruling};
use crate::keeping::Keeping;
use crate::regating::came_to;

/// Why the Checks cannot run again. **Each is checked before anything runs**,
/// and `core_model::Stuck` offers the act on exactly the reading these refuse.
///
/// A worktree that is gone is not here: it is `Adrift::WorktreeGone`, the
/// sentence `restart_step` already says, naming the redispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unrecheckable {
    /// The Job is not held for repair.
    NotHeldForRepair { status: JobStatus },
    /// No step of the Job stopped.
    NoStepStopped,
    /// The step stopped on something other than a failed Check.
    NotACheck { trigger: EscalationTrigger },
    /// Every Check the stopped step recorded passed.
    ChecksPassed,
    /// A Drone is standing in the Job's slot.
    DroneStanding,
    /// A re-run of this Job's Checks is already out.
    AlreadyRerunning,
}

impl Unrecheckable {
    /// What a person is told, in one sentence and then what to do instead.
    pub fn said(&self) -> String {
        match self {
            Unrecheckable::NotHeldForRepair { status } => format!(
                "this Job is {}, and only a Job a failed Check held for repair has Checks to run \
                 again",
                status.as_wire()
            ),
            Unrecheckable::NoStepStopped => String::from(
                "no step of this Job stopped, so there are no Checks to run again. Redispatch it \
                 to start over",
            ),
            Unrecheckable::NotACheck { trigger } => format!(
                "the step stopped on {}, not on a failed Check. Restart the step to have it \
                 worked again",
                trigger.as_wire()
            ),
            Unrecheckable::ChecksPassed => String::from(
                "every Check on the stopped step passed, so running them again answers nothing. \
                 Restart the step to have it worked again",
            ),
            Unrecheckable::DroneStanding => String::from(
                "a Drone is standing on this Job. Redirect it rather than running its Checks \
                 from outside",
            ),
            Unrecheckable::AlreadyRerunning => String::from(
                "this Job's Checks are already running again. Wait for that run to finish",
            ),
        }
    }
}

/// Which Jobs have a re-run of their Checks out.
///
/// **In memory and never written down**, for `crate::showing_again::Pressing`'s
/// reason: it is true only while the process running it lives.
#[derive(Clone, Debug, Default)]
pub(crate) struct Rechecking(Arc<Mutex<BTreeSet<JobId>>>);

impl Rechecking {
    /// Whether a re-run is out on this Job.
    pub(crate) fn holds(&self, job: &JobId) -> bool {
        self.held().contains(job)
    }

    /// Take this Job for a re-run, or `None` where one is already out.
    fn take(&self, job: &JobId) -> Option<Held> {
        if !self.held().insert(job.clone()) {
            return None;
        }
        Some(Held {
            rechecking: self.clone(),
            job: job.clone(),
        })
    }

    fn held(&self) -> MutexGuard<'_, BTreeSet<JobId>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// One Job held for a re-run, given back however the run ends.
struct Held {
    rechecking: Rechecking,
    job: JobId,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.rechecking.held().remove(&self.job);
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
    /// Run the stopped step's Checks again on the worktree as it stands.
    ///
    /// **Takes Fleet by `Arc`** — see this module's header. The run is spawned
    /// and awaited, so a caller that stops waiting leaves it running to the end.
    pub async fn rerun_checks(self: Arc<Self>, job_id: &JobId) -> Result<Job, Adrift> {
        let refused = |why| Adrift::CannotRerunChecks {
            job: job_id.clone(),
            why,
        };
        // **Held before anything is read**, so a restart or a redispatch pressed
        // while these refusals are asked is refused rather than slipping in
        // between them. A refusal below gives the hold back as it returns.
        let Some(held) = self.rechecking().take(job_id) else {
            return Err(refused(Unrecheckable::AlreadyRerunning));
        };
        let job = self.load(job_id).await?;
        let (step, stopped_by) = self.failed_on_a_check(&job).await?;
        // Before the slot, in `restart_step`'s order: a reclaimed worktree names
        // the redispatch whatever else is true.
        self.surviving_worktree(&job)?;
        if self.a_drone_stands_on(job_id).await {
            return Err(refused(Unrecheckable::DroneStanding));
        }
        let this = Arc::clone(&self);
        let on = job_id.clone();
        let running = tokio::spawn(async move {
            let _held = held;
            this.checks_run_again(&on, &step, stopped_by).await
        });
        match running.await {
            Ok(came_to) => came_to,
            Err(_) => Err(Adrift::RecheckAbandoned {
                job: job_id.clone(),
            }),
        }
    }

    /// Refuse an act that would take the worktree while its Checks run again.
    pub(crate) fn not_while_checks_run_again(&self, job: &Job) -> Result<(), Adrift> {
        match self.rechecking().holds(job.id()) {
            true => Err(Adrift::ChecksRunningAgain {
                job: job.id().clone(),
            }),
            false => Ok(()),
        }
    }

    /// The step a failed Check stopped, and the trigger it stopped on.
    ///
    /// **The Checks are read off the store, as `crate::stuck` reads them**, so
    /// the offer and this refusal answer from one record.
    async fn failed_on_a_check(&self, job: &Job) -> Result<(StepId, StepLevelTrigger), Adrift> {
        let refused = |why| Adrift::CannotRerunChecks {
            job: job.id().clone(),
            why,
        };
        if job.status() != JobStatus::AwaitingRepair {
            return Err(refused(Unrecheckable::NotHeldForRepair {
                status: job.status(),
            }));
        }
        let (step, stopped_by) = job
            .stopped_on()
            .ok_or_else(|| refused(Unrecheckable::NoStepStopped))?;
        let step = step.clone();
        if stopped_by.trigger() != EscalationTrigger::GateFailure {
            return Err(refused(Unrecheckable::NotACheck {
                trigger: stopped_by.trigger(),
            }));
        }
        let runs = self
            .store()
            .lock()
            .await
            .step_checks(job.id())
            .map_err(Adrift::Reading)?;
        let failed = runs
            .iter()
            .filter(|(id, _)| *id == step)
            .flat_map(|(_, checks)| checks.iter())
            .any(|check| !check.outcome.advances());
        if !failed {
            return Err(refused(Unrecheckable::ChecksPassed));
        }
        Ok((step, stopped_by))
    }

    /// Whether this Job's slot holds a Drone, heard or not.
    async fn a_drone_stands_on(&self, job_id: &JobId) -> bool {
        let Some(slot) = self.slot_of(job_id).await else {
            return false;
        };
        let held = slot.lock().await;
        held.as_ref().is_some_and(|at_work| at_work.is(job_id))
    }

    /// The re-run itself, on the task [`rerun_checks`](Fleet::rerun_checks)
    /// spawned.
    ///
    /// **Everything is read as the stopped run left it**, since the slot went
    /// with the Drone: the attempt, the spend and the declared scope are that
    /// run's, and `diff_nonempty` is its recorded outcome — passed says the step
    /// moved, anything else or no row says it did not.
    async fn checks_run_again(
        &self,
        job_id: &JobId,
        step: &StepId,
        stopped_by: StepLevelTrigger,
    ) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        // Asked again on the task, for a move that landed after the press read it.
        self.failed_on_a_check(&job).await?;
        let worktree = self.surviving_worktree(&job)?;
        let submission = self.submitted_already(&job, step).await?;
        let Some(at) = AtStep::named(job.workflow(), step, &worktree) else {
            return Err(Adrift::NoSuchStep {
                job: job_id.clone(),
                step: Some(step.clone()),
            });
        };
        let served = self.served_by(&job)?;
        let judging = self
            .judging(&job, &served)
            .map_err(|cause| Adrift::NotConfigurable {
                job: job_id.clone(),
                cause,
            })?;
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(job_id)
            .map_err(Adrift::Reading)?;
        let plan = self
            .store()
            .lock()
            .await
            .work_plan(job_id)
            .map_err(Adrift::Reading)?;
        let recorded = crate::work_plan::with_the_plan(recorded, job.workflow(), plan.as_ref());
        let attempt = self
            .store()
            .lock()
            .await
            .step_attempt(job_id, step)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        let spent = self
            .store()
            .lock()
            .await
            .step_spent(job_id, step)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        let declared = self
            .store()
            .lock()
            .await
            .step_plans(job_id)
            .map_err(Adrift::Reading)?
            .into_iter()
            .find(|declared| declared.step_id == *step && declared.attempt == attempt)
            .map(|declared| declared.paths);
        let moved = self
            .store()
            .lock()
            .await
            .step_checks_every_attempt(job_id)
            .map_err(Adrift::Reading)?
            .iter()
            .filter(|run| run.step_id == *step && run.attempt == attempt)
            .flat_map(|run| run.record.iter())
            .any(|check| check.name == DIFF_NONEMPTY && check.outcome == CheckOutcome::Passed);
        let announcing = self.announcing(&served, &job, step, attempt);
        let ports = self.port_map(&job).await;
        let port_env = self.port_env(&job).await;
        let refusal_policy = self
            .store()
            .lock()
            .await
            .when_refused(job_id)
            .unwrap_or_default();
        let tolerated = self
            .store()
            .lock()
            .await
            .tolerated_criteria()
            .unwrap_or_default();
        let ruling = rule_on(
            at.on_attempt(attempt, spent),
            Request::of(&job),
            &submission,
            declared.as_ref(),
            &Lifted::of(&job),
            Began::AsRecorded(moved),
            &recorded,
            self.work(),
            self.budget(),
            &self
                .checks_room_for(&job, crate::places::Asking::Gate)
                .await,
            &judging,
            &Keeping::of(served.records_root(), &job.handle()),
            self.gating_policies(&served),
            &announcing,
            &ports,
            &port_env,
            refusal_policy,
            &tolerated,
            plan.as_ref(),
            None,
        )
        .await;
        let ruling = self
            .guarded_against_unpushed_delivery(job_id, at.step(), ruling)
            .await?;

        self.recorded_checks(job_id, &job.handle(), step, attempt, &ruling)
            .await?;
        self.kept_timings(&job, announcing.timings()).await;
        drop(announcing);
        self.recorded_judgments(job_id, step, &ruling).await?;
        self.recorded_evidence(job_id, step, &submission, &ruling)
            .await?;
        self.recorded_gaming(job_id, step, &ruling).await?;
        self.noted_rechecked(job_id, step, &ruling);
        self.noted_undecided(job_id, step, &ruling);
        if matches!(ruling, Ruling::Failed { .. } | Ruling::HandedBack { .. }) {
            return self.load(job_id).await;
        }
        self.carried_on(&ruling, job_id, step, stopped_by, &worktree)
            .await
    }

    /// Take the Job out of `awaiting_repair` and carry the ruling out.
    ///
    /// **A pass with a step left re-queues, as an override does**, rather than
    /// going through `act_on`: that arm starts the next Drone in the slot, and a
    /// slot opened here is not admission's, so a press would put a Drone past
    /// `concurrency-cap`. Every other ruling starts no Drone and is `act_on`'s.
    async fn carried_on(
        &self,
        ruling: &Ruling,
        job_id: &JobId,
        step: &StepId,
        stopped_by: StepLevelTrigger,
        worktree: &Worktree,
    ) -> Result<Job, Adrift> {
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        // The Checks took minutes, and a person may have killed the Job or
        // taken it to Pilot meanwhile. Nothing is carried onto a Job that left.
        if job.status() != JobStatus::AwaitingRepair {
            return Err(Adrift::CannotRerunChecks {
                job: job_id.clone(),
                why: Unrecheckable::NotHeldForRepair {
                    status: job.status(),
                },
            });
        }
        let job = self.move_job(&job, Target::Running, Actor::Human).await?;
        let job = self
            .move_step_by(&job, step, StepTarget::Rechecking(stopped_by), Actor::Human)
            .await?;
        if let Ruling::Advanced { .. } = ruling {
            let job = self.move_step(&job, step, StepTarget::Advanced).await?;
            drop(working);
            return self.move_job(&job, Target::Queued, Actor::Human).await;
        }
        self.act_on(ruling, job_id, step, &mut working).await?;
        drop(working);
        // After `act_on` and reloaded, for `crate::settling`'s reason.
        if matches!(ruling, Ruling::HeldForReview { .. }) {
            let held = self.load(job_id).await?;
            self.compose_review_at_gate(&held, step, worktree).await;
        }
        self.load(job_id).await
    }

    /// Write the re-run into the Job's own log, with what it came to.
    ///
    /// **Fields, for `crate::regating`'s reason**, and on every press, including
    /// the one that moved nothing.
    fn noted_rechecked(&self, job: &JobId, step: &StepId, ruling: &Ruling) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person ran the step's Checks again on the work as it stands",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("came_to", FieldValue::Str(came_to(ruling).to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}
