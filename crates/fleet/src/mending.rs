//! Recovering a Job the old `Agree` arm of `crate::asking` could leave
//! answered, stopped, and never escalated — `#733`.
//!
//! **The one shape this repairs.** Agreeing with a refusal used to stop the
//! step, then ask for `awaiting_review -> escalated` with `gate_failure` — an
//! edge `interrupted` alone owns — so the move failed after the question
//! cleared and the step stopped. Every ruling that stops a step also
//! escalates the Job in the same breath (`crate::gate::apply`'s own header),
//! so a stopped current step beneath `awaiting_review` is this bug's
//! signature and nothing else's. Narrowed further on the trigger, so a future
//! defect with a different one is not silently swept in here.
//! `01M28XMQNW0027QG51YJ54G1B9` is the real Job this was written for.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, Job, JobId, JobStatus, Level, StepId, StepState,
    StepVerdict, Target,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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
    /// Finish every Job the old `Agree` arm stranded, once, at boot.
    ///
    /// **Called from `reconcile`, before the Drone-adoption pass reads the
    /// same rows** — updated in place for that reason: a pass reading a stale
    /// `awaiting_review` next to a live pointer would ask the wrong question
    /// about it.
    pub(crate) async fn mended_wedged_reviews(
        &self,
        jobs: &mut [Job],
    ) -> Result<Vec<JobId>, Adrift> {
        let mut mended = Vec::new();
        for slot in jobs.iter_mut() {
            let Some(step) = wedged_by_the_old_agree_bug(slot) else {
                continue;
            };
            if self.judge_question_of(slot.id()).await.is_some() {
                continue;
            }
            let job = self.move_job(slot, Target::Running, Actor::Fleet).await?;
            self.noted_mended(job.id(), &step);
            let job = self
                .move_job(
                    &job,
                    Target::Escalated(EscalationTrigger::GateFailure),
                    Actor::Fleet,
                )
                .await?;
            mended.push(job.id().clone());
            *slot = job;
        }
        Ok(mended)
    }

    fn noted_mended(&self, job: &JobId, step: &StepId) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "an earlier answer stopped this step and never escalated the Job; \
             Fleet finished the move on boot",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str());
        self.noted_in_the_log(job, &envelope);
    }
}

/// The one shape the old `Agree` arm could leave behind. `None` on every
/// other Job, including one legitimately `awaiting_review` — that state's
/// current step is always `awaiting_human`, never `stopped`.
fn wedged_by_the_old_agree_bug(job: &Job) -> Option<StepId> {
    if job.status() != JobStatus::AwaitingReview {
        return None;
    }
    let step = job.current_step()?;
    if step.state() != StepState::Stopped {
        return None;
    }
    match step.last_verdict() {
        Some(StepVerdict::Failed(trigger))
            if trigger.trigger() == EscalationTrigger::GateFailure =>
        {
            Some(step.step_id().clone())
        }
        _ => None,
    }
}
