//! Asking the gate again, on the evidence already submitted.
//!
//! # It is not an override, and the distinction is the whole act
//!
//! `crate::overruling` lifts a decision: a machine weighed the work and a
//! person disagrees. [`Ruling::CouldNotDecide`] is the machine saying it
//! weighed nothing, so there is nothing to disagree with — `overrulable`
//! refuses `gate_undecided` for that, and advancing on it would pass a step
//! nothing ruled on. This asks the question that failed to be asked. The
//! Drone's work is still on the branch and its evidence still on the record, so
//! the second reading is the first one made again. Where the cause was
//! permanent it fails again and says so.
//!
//! # It re-runs once, on a person's say-so, and never loops
//!
//! `#156` recorded that a worktree momentarily unreadable and a Judge that
//! cannot be handed a patch arrive here as the same value, and that guessing
//! between them would be the machine producing an answer it does not have.
//! Nothing here retries; a person presses this and presses it again or does not.
//!
//! # It does not spend the retry budget
//!
//! A failed Check hands the step back to its Drone inside `retry_limit`, which
//! counts *runs of the step*. A gate re-run is not one: no Drone works and
//! nothing it did is redone. See [`rerun_gate`](Fleet::rerun_gate).

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    StepId, StepTarget, Target,
};
use verification::{Claimed, NotClaimed, Request, ShownBy, Submission};

use crate::adrift::Adrift;
use crate::at_step::AtStep;
use crate::daemon::Fleet;
use crate::gate::{rule_on, Ruling};
use crate::keeping::Keeping;
use crate::transcript;

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
    /// Run the gate again over the evidence the step already submitted.
    ///
    /// **The act for `gate_undecided`, and the only one.** It takes no reason:
    /// nothing is being disagreed with, so there is nothing a sentence could
    /// record that the second reading will not say for itself.
    ///
    /// # Everything is read before anything moves
    ///
    /// The whole reading happens while the Job is still `escalated` and the
    /// step still `stopped`, and two things follow. **A read that fails leaves
    /// the Job exactly where it was**, rather than half-moved into `running`
    /// with no Drone for the reaper to find. And **the attempt is the
    /// Drone's**, because `store::step_attempt` counts entries into `running`
    /// and nothing has entered one — so `may_hand_back` is asked about the run
    /// the Drone worked, and the four records land under it, replacing the
    /// first reading's partial rows for the run they are both about.
    ///
    /// # A second `CouldNotDecide` moves nothing
    ///
    /// The Job is already `escalated` and the step already `stopped` on
    /// `gate_undecided`, which is where `act_on` would put them. Skipping the
    /// two moves keeps the permanent case free: a person may press this as
    /// often as they like without the record filling with runs the Drone never
    /// had.
    pub async fn rerun_gate(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let Some(slot) = self.slot_of(job_id).await else {
            return Err(Adrift::NotStandingThere {
                job: job_id.clone(),
            });
        };
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        let step = self.undecided_step(&job)?;
        // **The slot, and it is a requirement rather than a convenience.** The
        // baseline `diff_nonempty` is decided against is what the worktree held
        // when the step began, and it lives here and nowhere else. Without it
        // the second reading would decide on nothing known to have moved and
        // answer a different question from the first — which is the guess this
        // act exists to not make.
        let Some(at_work) = working.as_ref().filter(|at_work| at_work.is(job_id)) else {
            return Err(Adrift::NotStandingThere {
                job: job_id.clone(),
            });
        };
        let (_, standing, worktree) = at_work.standing();
        if standing != step {
            return Err(Adrift::NotStandingThere {
                job: job_id.clone(),
            });
        }
        let declared = at_work.declared().cloned();
        let entered_with = at_work.entered_with().cloned();

        let submission = self.submitted_already(&job, &step).await?;
        let Some(at) = AtStep::named(job.workflow(), &step, &worktree) else {
            return Err(Adrift::NoSuchStep {
                job: job_id.clone(),
                step: Some(step),
            });
        };
        let judging = self
            .judging(job_id)
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
        // The Drone's run, read before the resume below adds one. See this
        // function's own note on why it is read here.
        let attempt = self
            .store()
            .lock()
            .await
            .step_attempt(job_id, &step)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        // And the pass's own spend beside it, for `crate::settling`'s reason:
        // the budget the second reading is weighed against is the one the
        // Drone's run belongs to, and a return would have reset it.
        let spent = self
            .store()
            .lock()
            .await
            .step_spent(job_id, &step)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        // The same call `crate::settling` makes, with the same arguments read
        // off the same places. Nothing here is assembled differently, because
        // an easier reading would not be the reading that failed.
        let ruling = rule_on(
            at.on_attempt(attempt, spent),
            Request::of(&job),
            &submission,
            declared.as_ref(),
            entered_with.as_ref(),
            &recorded,
            self.work(),
            self.budget(),
            &judging,
            &Keeping::of(&self.host().repo_root, job_id),
        )
        .await;

        self.recorded_checks(job_id, &step, attempt, &ruling)
            .await?;
        self.recorded_judgments(job_id, &step, &ruling).await?;
        self.recorded_evidence(job_id, &step, &submission, &ruling)
            .await?;
        self.recorded_gaming(job_id, &step, &ruling).await?;
        self.noted_reread(job_id, &step, &ruling);
        self.noted_undecided(job_id, &step, &ruling);
        // The permanent case, and nothing moves for it.
        if ruling.undecided().is_some() {
            return self.load(job_id).await;
        }

        // The two moves in the one order the machines admit, and the actor is
        // **human** on both: Fleet re-runs no gate of its own accord. This is
        // `crate::resume`'s pair, spelled here because what follows it is a
        // ruling rather than a Drone.
        let job = self.move_job(&job, Target::Running, Actor::Human).await?;
        self.move_step_by(&job, &step, StepTarget::Running, Actor::Human)
            .await?;
        match ruling.hands_back() {
            // **The step has already re-entered `running` and must not do it
            // twice.** `act_on` writes `retrying` and then `running` for a
            // hand-back, which is right when the step was running to begin
            // with and wrong here: the resume above *is* this run's entry, and
            // a second one would count one run of the Drone's work as two and
            // take a hand-back off `retry_limit` for a re-run that spent none.
            // What that costs is the `retrying` row, and the line this wrote
            // into the Job's log above carries the same fact.
            Some(_) => {
                if let Some(told) = ruling.tell().cloned() {
                    self.tell(job_id, &told, None, &working).await?;
                }
            }
            None => self.act_on(&ruling, job_id, &step, &mut working).await?,
        }
        self.load(job_id).await
    }

    /// The step whose gate could not decide, and nothing else.
    ///
    /// Four things hold, and each refusal names the act that applies instead.
    /// The Job is `escalated`; a step of it stopped; that step carries a
    /// verdict; and the verdict is `gate_undecided`.
    ///
    /// **The last is [`overrulable`]'s refusal read from the other side.** That
    /// match admits `gate_failure` and `evidence_suspect` because a machine
    /// ruled, and refuses `gate_undecided` because none did. This admits
    /// exactly what that refuses, so the two acts partition the triggers rather
    /// than overlapping — and a trigger belonging to neither is refused by
    /// both, which is the honest answer for a Check that hit its bound or a
    /// loop that did not converge.
    ///
    /// [`overrulable`]: crate::overruling
    fn undecided_step(&self, job: &Job) -> Result<StepId, Adrift> {
        if job.status() != JobStatus::Escalated {
            return Err(Adrift::NotResumable {
                job: job.id().clone(),
                status: job.status(),
            });
        }
        // The one reading, shared with `crate::overruling` and with the
        // classification `core_model::Stuck` makes — the state and the
        // `failed(<trigger>)` verdict together, never one without the other.
        let (step, stopped_by) = job.stopped_on().ok_or_else(|| Adrift::NoStepStopped {
            job: job.id().clone(),
        })?;
        let step = step.clone();
        if stopped_by.trigger() != EscalationTrigger::GateUndecided {
            return Err(Adrift::NotUndecided {
                job: job.id().clone(),
                step,
                trigger: stopped_by.trigger(),
            });
        }
        Ok(step)
    }

    /// The submission the gate already had, read back off the record.
    ///
    /// **Not a fresh one, and there is no path here that could ask for one.**
    /// The Drone is idle and holds no obligation: it submitted, the gate could
    /// not read what it needed, and asking again would spend a turn to be told
    /// what the record already holds. `store::step_evidence` answers the
    /// step's latest run, which is the run that stopped.
    async fn submitted_already(&self, job: &Job, step: &StepId) -> Result<Submission, Adrift> {
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(job.id())
            .map_err(Adrift::Reading)?;
        let evidence = recorded
            .into_iter()
            .find(|(id, _)| id == step)
            .map(|(_, evidence)| evidence)
            .ok_or_else(|| Adrift::NothingToRuleOn {
                job: job.id().clone(),
                step: step.clone(),
            })?;
        // The same four fields, back through the same constructor. A row that
        // will not pass it was written by something that did not share it,
        // which is a corrupt record rather than a Drone that said nothing —
        // and either way there is no evidence to rule on.
        Submission::submitted(
            evidence.evidence_type,
            Claimed(&evidence.claimed),
            ShownBy(&evidence.shown_by),
            NotClaimed(&evidence.not_claimed),
        )
        .map_err(|_| Adrift::NothingToRuleOn {
            job: job.id().clone(),
            step: step.clone(),
        })
    }

    /// Write the re-run into the Job's own log, with what it came to.
    ///
    /// **Fields, never an interpolated message**, for `crate::settling`'s
    /// reason: `came_to` is what a query groups on, so "asked again and still
    /// could not read it" is countable against "asked again and ruled". That
    /// count is the only measure there will be of how often the cause was
    /// transient, which is the question `#156` left open and refused to guess
    /// at.
    ///
    /// Written before the moves and on every re-run, including the one that
    /// changes nothing — a press that produced no transition is exactly the
    /// press a person needs to see happened.
    fn noted_reread(&self, job: &JobId, step: &StepId, ruling: &Ruling) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person asked the gate again on the evidence already submitted",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("came_to", FieldValue::Str(came_to(ruling).to_string()));
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}

/// What the second reading came to, in one word a query can group on.
///
/// **A vocabulary rather than a rendering.** `undecided` is the one that says
/// the cause was permanent; the rest say the gate answered, and which answer it
/// gave is already on the step. A wildcard here would let a new ruling arrive
/// spelled as something it is not, so the match is exhaustive.
fn came_to(ruling: &Ruling) -> &'static str {
    match ruling {
        Ruling::Advanced { .. } => "advanced",
        Ruling::Finished { .. } => "finished",
        Ruling::HeldForReview { .. } => "held_for_review",
        Ruling::HandedBack { .. } => "handed_back",
        Ruling::Failed { .. } => "failed",
        Ruling::Refused { .. } => "refused",
        Ruling::Suspect { .. } => "suspect",
        Ruling::NotWhatTheStepAsked(_) => "not_what_the_step_asked",
        Ruling::CouldNotDecide { .. } => "undecided",
    }
}
