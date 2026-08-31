//! Overruling a verdict: a person reads a refusal, disagrees, and the stopped
//! step advances with the work that was already done left standing.
//!
//! **The fifth act on an escalated Job**, and the only one that keeps the work.
//! `docs/concepts/job.md` has the table and the argument — including which
//! triggers this lifts, which it will not, and why an unappealable verdict is
//! worse than no verdict at all. `crates/ipc/operations.toml` keys it
//! `override_verdict`.
//!
//! Which triggers this lifts is [`StepLevelTrigger::overrulable`], which sits
//! beside the vocabulary rather than here: the classification in
//! `core_model::Stuck` has to answer the same question, and two exhaustive
//! matches over one set is how a button and the sentence beside it come to
//! disagree.
//!
//! Two things here are not in either document because they are about this code.
//! [`overridable`](Fleet::overridable) reads the recorded Check runs rather than
//! resting on the tier ordering that makes a failed Check unreachable — a guard
//! that holds by an argument about ordering stops holding the day the ordering
//! moves. And whether a Drone is there decides nothing at all any more: the act
//! applies either way and the Job carries on the same way either way, because
//! the overridden part's Drone is ended and a fresh one takes the next part.
//! That a live session exists is still what separates `crate::resume`'s two
//! acts from each other; it stopped separating anything here.
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, FieldValue, Job, JobId, JobStatus, Level, StepId, StepLevelTrigger,
    StepTarget, Target,
};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::crossing::{Cleared, Crossed, Produced};
use crate::daemon::Fleet;
use crate::transcript;

/// Why a person says the verdict is wrong. **Never empty.**
///
/// A type of its own rather than `resume::Redirection`, which is structurally
/// the same string: that one is delivered to a Drone as a turn and this one
/// never leaves the record. The Drone did nothing wrong and is told only that
/// the step was accepted.
///
/// It is required rather than optional because a person disagreeing with a
/// Judge is the strongest signal there is that a criterion is mis-stated, and a
/// count of overrides with no reasons beside it gives the rate and never the
/// cause. An override that says nothing is also how the act this module keeps
/// visible becomes the one somebody reaches for to quiet a gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overruling(String);

impl Overruling {
    /// `None` where there is nothing in it a person could later read.
    pub fn saying(reason: &str) -> Option<Overruling> {
        let said = reason.trim();
        (!said.is_empty()).then(|| Overruling(said.to_string()))
    }

    pub fn text(&self) -> &str {
        &self.0
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
    /// Overrule the Judge and let the Job go on from where it stopped.
    ///
    /// The actor is **human** on both moves. Fleet overrules nothing of its own
    /// accord — disagreeing with a verdict is exactly the part a person was
    /// escalated to.
    ///
    /// The order is `crate::resume`'s and for its reason: the inner machine is
    /// frozen beneath `escalated`, so the Job moves first and the step moves
    /// beneath the status it arrives in.
    pub async fn override_verdict(
        &self,
        job_id: &JobId,
        overruling: &Overruling,
    ) -> Result<Job, Adrift> {
        let mut working = self.slot().lock().await;
        let job = self.load(job_id).await?;
        let (step, overruled) = self.overridable(&job).await?;
        // Read before anything moves, and discarded. A worktree that is gone
        // is a Job whose earlier steps' work is not on disk, and what is being
        // asked for there is a redispatch — the same refusal `restart_step`
        // makes, made before the Job has been half-moved rather than after.
        // `crossed_onto` reaches the same directory again once the Drone that
        // was on it has been stood down, because until then the slot is holding
        // the reading that answers for free.
        self.surviving_worktree(&job)?;
        let passed = self.declared_step(&job, &step)?.clone();
        let next = job.workflow().after(&step).cloned();
        // **Read before anything moves.** A record that would not open would
        // otherwise escalate an override that had already moved the Job and
        // the step. One query buys a refusal that leaves the Job exactly where
        // the person found it.
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(job_id)
            .map_err(Adrift::Reading)?;

        let job = self.move_job(&job, Target::Running, Actor::Human).await?;
        let job = self
            .move_step_by(&job, &step, StepTarget::Overridden(overruled), Actor::Human)
            .await?;
        // After both moves and before anything else can fail, so the reason a
        // person gave is on the record whatever the resume does next.
        self.noted_override(job_id, &step, overruled, overruling);

        let told = OutcomeTurn::approved(&passed, next.as_ref());
        let Some(next) = next else {
            return self.completed(&job, &told, job_id, &mut working).await;
        };
        let job = self.move_step(&job, next.id(), StepTarget::Running).await?;
        // **One arm, because the two used to differ over a fact that no longer
        // decides anything.** It asked whether a process happened to be there:
        // where one was it was told and carried on, and where one was not a
        // fresh Drone was put on the next step. A Drone belongs to a step now,
        // so the second is what happens either way — the Drone that worked the
        // overridden part is ended whether it was standing or already gone, and
        // `crate::boundary` is the one place that order is written.
        //
        // The two facts a fresh Drone is owed are the ones the missing process
        // would have held: what the overridden part produced, and that a person
        // settled it.
        let crossed = Crossed::nothing()
            .and_produced(Produced::before(job.workflow(), next.id(), &recorded))
            .and_cleared(Cleared::reviewed(&passed));
        self.crossed_onto(&job, next.id(), crossed, &mut working)
            .await?;
        self.load(job_id).await
    }

    /// The step a person may overrule, and the verdict they are overruling.
    ///
    /// Four things have to hold, and each refusal names a different act as the
    /// one that applies. The Job is escalated; a step of it stopped; the
    /// trigger that stopped it is one [`StepLevelTrigger::overrulable`] admits,
    /// which is a machine having ruled rather than a machine having been unable
    /// to; and no
    /// Check the gate ran on that step failed.
    ///
    /// **The last is read out of the store and not inferred.** A refusal
    /// implies the mechanical tier held, so ordinarily it is redundant — and a
    /// guard that is redundant by an argument about tier ordering is a guard
    /// that stops holding the day the ordering changes.
    async fn overridable(&self, job: &Job) -> Result<(StepId, StepLevelTrigger), Adrift> {
        if job.status() != JobStatus::Escalated {
            return Err(Adrift::NotResumable {
                job: job.id().clone(),
                status: job.status(),
            });
        }
        // `Job::stopped_on` is the one reading, shared with `crate::regating`
        // and with the classification: the state and the `failed(<trigger>)`
        // verdict together, because a stopped row with no verdict is a row
        // nothing could ever say why about.
        let (step, overruled) = job.stopped_on().ok_or_else(|| Adrift::NoStepStopped {
            job: job.id().clone(),
        })?;
        let step = step.clone();
        if !overruled.overrulable() {
            return Err(Adrift::NotTheJudges {
                job: job.id().clone(),
                step,
                trigger: overruled.trigger(),
            });
        }
        self.every_check_passed(job, &step).await?;
        Ok((step, overruled))
    }

    /// What the gate's Checks did on this step, read back off the record.
    ///
    /// **The first failure names itself**, because a person told a Check failed
    /// needs to know which one to go and run.
    ///
    /// A skipped Check is not one: `advances` asks whether anything failed, and
    /// a Check whose paths the step never touched is not a reason to refuse an
    /// override.
    async fn every_check_passed(&self, job: &Job, step: &StepId) -> Result<(), Adrift> {
        let runs = self
            .store()
            .lock()
            .await
            .step_checks(job.id())
            .map_err(Adrift::Reading)?;
        let failed = runs
            .iter()
            .filter(|(id, _)| id == step)
            .flat_map(|(_, checks)| checks.iter())
            .find(|check| !check.outcome.advances());
        match failed {
            None => Ok(()),
            Some(check) => Err(Adrift::CheckDidNotPass {
                job: job.id().clone(),
                step: step.clone(),
                check: check.name.clone(),
            }),
        }
    }

    /// Write the override into the Job's own log.
    ///
    /// **Fields, never an interpolated message**, for the reason
    /// `crate::settling` gives about a decline: `overruled` is what a query
    /// groups on and it is a trigger spelling from the registry, so overrides
    /// stay countable against the refusals they answer. The person's words are
    /// carried whole beside it, because the count says the rate and only the
    /// sentence says the cause.
    ///
    /// A log line that will not write does not stop the act. The step has
    /// already moved and the move is the record; this is the half no column
    /// holds.
    fn noted_override(
        &self,
        job: &JobId,
        step: &StepId,
        overruled: StepLevelTrigger,
        overruling: &Overruling,
    ) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "a person overruled the gate and the step advanced",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field(
            "overruled",
            FieldValue::Str(overruled.as_wire().to_string()),
        )
        .with_field("said", FieldValue::Str(overruling.text().to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}
