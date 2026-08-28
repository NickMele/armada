//! Overruling a verdict: a person reads a refusal, disagrees, and the stopped
//! step advances with the work that was already done left standing.
//!
//! **The fifth act on an escalated Job**, and the only one that keeps the work.
//! `docs/concepts/job.md` has the table and the argument — including which
//! triggers this lifts, which it will not, and why an unappealable verdict is
//! worse than no verdict at all. `crates/ipc/operations.toml` keys it
//! `override_verdict`.
//!
//! Two things here are not in either document because they are about this code.
//! [`overridable`](Fleet::overridable) reads the recorded Check runs rather than
//! resting on the tier ordering that makes a failed Check unreachable — a guard
//! that holds by an argument about ordering stops holding the day the ordering
//! moves. And whether a Drone is there decides only *how* the Job carries on,
//! never whether the act applies, which is the whole of what separates this from
//! `crate::resume`.
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    ResolvedStep, StepId, StepLevelTrigger, StepState, StepTarget, StepVerdict, Target,
};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::briefing::{self, Declaring};
use crate::daemon::Fleet;
use crate::transcript;

/// Whether a person may overrule this verdict.
///
/// **An exhaustive `match` and not a list.** The constant this replaced was a
/// constant so that a second entry could not be added quietly; a slice would
/// give that up, and a match keeps it the other way round — a trigger minted in
/// the registry does not compile until somebody writes its arm here, which is
/// the moment the argument gets had.
///
/// `gate_failure` is the Judge refusing a criterion, and was the only one until
/// 2026-08-28. `evidence_suspect` is the gaming check saying the evidence is
/// not to be trusted, and it was refused here on the grounds that it is a claim
/// about *how* the step was satisfied rather than about whether it was. That
/// distinction is real and it is not the one that decides this: the owner's
/// rule is that anything a machine decides, a person can overrule, and a gaming
/// flag is a machine reading a diff and inferring intent — which is the kind of
/// call a person is most likely to be right about and the machine least. It
/// moved for that, and nothing moved with it.
///
/// **`gate_undecided` in particular did not.** It is not a verdict a person is
/// disagreeing with; it is the machine saying it could not read the artifact,
/// so there is nothing ruled to overrule. Whether it should be answerable some
/// other way is open and is not this.
///
/// The rest are refused because nothing weighed the work at all: a Check that
/// hit its bound, evidence too large to read, a loop that did not converge, a
/// Drone denied a tool, a Drone going in circles. The Job-level triggers cannot
/// reach here — [`StepLevelTrigger::of`] refuses them, so no `last_verdict`
/// carries one — and they are listed rather than caught by a wildcard so that
/// the exhaustiveness above is real.
fn overrulable(overruled: StepLevelTrigger) -> bool {
    match overruled.trigger() {
        EscalationTrigger::GateFailure | EscalationTrigger::EvidenceSuspect => true,
        EscalationTrigger::GateUndecided
        | EscalationTrigger::BlockedByPolicy
        | EscalationTrigger::CheckTimeout
        | EscalationTrigger::EvidenceTooLarge
        | EscalationTrigger::LoopCap
        | EscalationTrigger::Thrashing => false,
        EscalationTrigger::DependencyFailed
        | EscalationTrigger::FanOut
        | EscalationTrigger::HatchUnbidden
        | EscalationTrigger::Interrupted
        | EscalationTrigger::ResourceExhausted
        | EscalationTrigger::Silent
        | EscalationTrigger::Stalled => false,
    }
}

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
        // Read before anything moves. A worktree that is gone is a Job whose
        // earlier steps' work is not on disk, and what is being asked for there
        // is a redispatch — the same refusal `restart_step` makes, made before
        // the Job has been half-moved rather than after.
        let onwards = self.surviving_worktree(&job)?;
        let passed = self.declared_step(&job, &step)?.clone();
        let next = job.workflow().after(&step).cloned();

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
        match working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            // The session is still there, holding its context. An escalated Job
            // keeps its Drone, so this is the ordinary case and it costs a turn
            // rather than a respawn — which is the whole of what makes this
            // cheaper than the restart it replaces.
            true => self.carried_on(job_id, &next, &told, &mut working).await?,
            // The Drone has gone: the Fleet that held it restarted, or the
            // process ended on its own. A fresh one takes the *next* step on
            // the worktree the last one left, which is `restart_step`'s
            // machinery pointed one step further along. Doing nothing here
            // would leave a `running` Job with no process on it, which the
            // reaper escalates as `interrupted` a moment later.
            false => {
                let brief =
                    briefing::first_turn(&job, job.workflow(), next.id()).map_err(|cause| {
                        Adrift::NotConfigurable {
                            job: job_id.clone(),
                            cause,
                        }
                    })?;
                self.put_a_drone_on(&job, next.id(), onwards, brief, &mut working)
                    .await?;
            }
        }
        self.load(job_id).await
    }

    /// Tell the Drone that is there, and move the slot on to the next step.
    ///
    /// The same three things `approve_review` does at a human gate, in the same
    /// order and for the same reasons: `now_on` clears what the step being left
    /// had declared, `marked` reads the baseline the next step's
    /// `diff_nonempty` is decided against, and the ask goes out with the
    /// acceptance because an unasked Drone declares nothing.
    async fn carried_on(
        &self,
        job_id: &JobId,
        next: &ResolvedStep,
        told: &OutcomeTurn,
        working: &mut Option<crate::working::Working>,
    ) -> Result<(), Adrift> {
        if let Some(at_work) = working.as_mut() {
            at_work.now_on(next.id().clone(), self.now());
        }
        self.marked(working);
        self.tell(job_id, told, Declaring::at(next).as_ref(), working)
            .await
    }

    /// The step a person may overrule, and the verdict they are overruling.
    ///
    /// Four things have to hold, and each refusal names a different act as the
    /// one that applies. The Job is escalated; a step of it stopped; the
    /// trigger that stopped it is one [`overrulable`] admits, which is a
    /// machine having ruled rather than a machine having been unable to; and no
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
        let stopped = job
            .steps()
            .iter()
            .find(|step| step.state() == StepState::Stopped)
            .ok_or_else(|| Adrift::NoStepStopped {
                job: job.id().clone(),
            })?;
        let step = stopped.step_id().clone();
        // `stopped` is the state and `failed(<trigger>)` is the verdict, and it
        // is the verdict that says what to disagree with. A stopped step with
        // no verdict on it is a row nothing could ever say why about, which the
        // machine does not admit.
        let Some(StepVerdict::Failed(overruled)) = stopped.last_verdict() else {
            return Err(Adrift::NoStepStopped {
                job: job.id().clone(),
            });
        };
        if !overrulable(overruled) {
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
            .find(|check| !check.outcome.passed());
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
