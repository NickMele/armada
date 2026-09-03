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
    /// **The step moves beneath `escalated`, and it is the only move that
    /// may** — `core_model::overruled_while_frozen` is that exception and says
    /// why. It cannot be deferred: it *is* the act. That separates it from
    /// `restart_step`, which defers its step move entirely, because entering
    /// `running` opens a run and a run belongs to a Drone.
    ///
    /// **A workflow with a step left goes back in the queue**, at
    /// `escalated -> queued`, and `crate::readmitting` spawns when
    /// `concurrency-cap` has room. Until #50's follow-on this act opened a slot
    /// of its own and consulted no bound. **An override of the last step
    /// finishes the Job here instead**, because no Drone is owed and there is
    /// nothing for the bound to bound.
    ///
    /// **Neither is refused because the cap is spent.** The act lands, and a
    /// Job that waits says `waiting_on_resources`.
    pub async fn override_verdict(
        &self,
        job_id: &JobId,
        overruling: &Overruling,
    ) -> Result<Job, Adrift> {
        // Opened rather than looked up: the Job whose Drone has gone has no
        // slot in the roster, and `completed` may still have work to land
        // through one. It holds nothing on the path that re-queues, and
        // `Slots::sweep` forgets it the moment this call lets it go.
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        let (step, overruled) = self.overridable(&job).await?;
        // Read before anything moves, and discarded. A worktree that is gone
        // is a Job whose earlier steps' work is not on disk, and what is being
        // asked for there is a redispatch — the same refusal `restart_step`
        // makes, made before the Job has been half-moved rather than after.
        // `crate::readmitting` reaches the same directory again at the spawn.
        self.surviving_worktree(&job)?;
        let passed = self.declared_step(&job, &step)?.clone();
        let next = job.workflow().after(&step).cloned();

        // **First, and beneath `escalated`.** See the note above: this move is
        // the act, and the Job's own move is what follows from it rather than
        // the other way round.
        let job = self
            .move_step_by(&job, &step, StepTarget::Overridden(overruled), Actor::Human)
            .await?;
        // After the move and before anything else can fail, so the reason a
        // person gave is on the record whatever happens next.
        self.noted_override(job_id, &step, overruled, overruling);

        if next.is_none() {
            // Nothing is owed a Drone, so nothing waits on the bound. The Job
            // takes the edge it has always taken from here, and `completed`
            // lands the work through the slot above and ends the Drone.
            let told = OutcomeTurn::approved(&passed, None);
            let job = self.move_job(&job, Target::Running, Actor::Human).await?;
            let done = self.completed(&job, &told, job_id, &mut working).await?;
            // **`completed` does not admit and says so**: it holds a slot, and
            // taking the roster while holding one is the lock order reversed.
            // This is the caller doing it, which is what that note asks for and
            // what this path had been missing — a Job that finished by override
            // left the bound spent until some later turn noticed.
            drop(working);
            self.admit_next().await?;
            return Ok(done);
        }
        // **The overridden part's Drone ends here, before the Job waits.** The
        // part is settled and there is no more work in it, so a process left
        // standing would spend a place against `concurrency-cap` on a Job that
        // is only queued — which is the overrun this change exists to close,
        // arriving from the other side. `crate::boundary` used to do it, on the
        // way to a spawn this act no longer makes; `stood_down` is that same
        // ordering, called directly.
        self.stood_down(job_id, &mut working).await?;
        // The actor is **human**, for `crate::reviewing`'s reason: a person
        // took the Job out of `escalated`, and Fleet only decides which turn it
        // gets a process back. What the next Drone is told — that a person
        // settled the part before — is `crate::readmitting`'s to assemble, off
        // the `advanced` step this call leaves behind.
        self.move_job(&job, Target::Queued, Actor::Human).await?;
        // **After the slot is let go**, which is the lock order `crate::slots`
        // states: admission takes the roster, and a caller holding a slot must
        // not reach for it. Inline, exactly as an approval re-admits inline.
        drop(working);
        self.admit_next().await?;
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
    ///
    /// **Which is why `awaiting_repair` is refused twice over.** A Job held for
    /// a spent retry budget carries a step stopped on `gate_failure`, the same
    /// trigger a Judge refusal writes — so the status test above is what keeps
    /// it out, and the Check reading would keep it out on its own. `build`
    /// failing is not a matter of opinion, and `#208` did not make it one.
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
