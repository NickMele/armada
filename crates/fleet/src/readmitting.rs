//! Putting a Drone back on a Job a person answered, once the bound has room.
//!
//! Split from [`dispatch`](mod@crate::dispatch), which starts a Job that has
//! never run. The branch on the record is the discriminator between the two.
//!
//! # Four acts arrive here and none of them spawned for itself
//!
//! `approve_review`, `request_changes`, `restart_step` and `override_verdict`
//! each end with the Job at `queued` and no Drone. That is the whole of what
//! `settings.concurrency-cap` bounding Drones means: **a person's act is
//! accepted whatever the cap holds, and admission is the only thing that starts
//! one.** The first two always took `awaiting_review -> queued`; the second two
//! take `escalated -> queued`, which is #50's follow-on and where the registry
//! records the reversal.
//!
//! **Which step, and what the Drone is told, is read off the step's own state**
//! rather than remembered — each act leaves the inner machine in a shape only
//! it produces, and [`Owed`] is that partition.
//!
//! # A step enters `running` when a Drone starts on it, and never earlier
//!
//! `store::attempt` counts entries into `running` as a step's runs, so a
//! restart that moved its step when the button was pressed would open a run
//! with no Drone in it. The one move *not* deferred is the override, a
//! person's verdict rather than a run — `core_model::overruled_while_frozen`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, FrozenWorkflow, IllegalStepTransition, Job, ResolvedStep, StepEvidence, StepId,
    StepState, StepTarget, Target,
};

use crate::adrift::Adrift;
use crate::briefing::Opening;
use crate::crossing::{Cleared, Crossed, Produced};
use crate::daemon::Fleet;
use crate::working::Working;

/// What a re-queued Job is owed: the step to work, and how the Drone opens.
///
/// **Three variants for the three shapes the inner machine can be in**, read
/// off the current step's state rather than off a column that remembers which
/// button was pressed.
///
/// | The current step reads | The act was | The Drone gets |
/// |---|---|---|
/// | `running` | an approval, or changes asked for | the next step, or the same one again |
/// | `stopped` | a restart | the same step, told what stopped it |
/// | `advanced` | an override | the step after it, told a person cleared this one |
///
/// The two `running` cases are told apart by the waiting note, which is
/// `request_changes`'s alone — that test predates this file and is unchanged.
enum Owed {
    /// The step is `running` — a person answered at a human advance gate, and
    /// the step moves the act already made are made.
    ///
    /// `cleared` is the part before this one where a person accepted it, and
    /// `None` where they sent it back: `request_changes` did not advance
    /// anything, so the part before is one nobody just acted on and "read by a
    /// person and accepted" over it is a sentence the record does not support.
    Standing { step: StepId, cleared: bool },
    /// The step is `stopped` — a person restarted it. It is entered at the
    /// spawn, and the Drone opens with what stopped it.
    Restarted { step: StepId },
    /// The step is `advanced` — a person overruled the verdict that stopped it.
    /// The step after it is entered at the spawn.
    Overruled { advanced: StepId, next: StepId },
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
    /// Put a Drone back on a Job a person answered.
    ///
    /// **The slot is the whole reason this exists.** A human gate stands its
    /// Drone down, and an escalated Job's has usually gone already — so by the
    /// time a person answers, the bound may well be spent on somebody else's
    /// Job. The Job goes back in the queue and arrives here when there is room,
    /// which is why none of the four acts is a resume.
    ///
    /// **The Job's own move is `Actor::Fleet` on every path**, including the
    /// two that follow a person's decision. The decision was recorded when it
    /// was made; what Fleet decides is the moment, and that is exactly what
    /// this row says.
    pub(crate) async fn readmitted(
        &self,
        job: Job,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        let job_id = job.id().clone();
        let owed = self.owed(&job)?;
        // Before the Job moves. A worktree that has been reclaimed is a Job
        // whose earlier steps' work is not on disk, and there is nothing to put
        // a Drone back onto — the same refusal `restart_step` makes, made
        // before the status has moved rather than after.
        let worktree = match self.surviving_worktree(&job) {
            Ok(worktree) => worktree,
            Err(cause) => {
                self.interrupt(&job).await?;
                return Err(cause);
            }
        };
        // **Before the spawn, because `drone_spawned` refuses over a live
        // pointer.** `crossed_onto` used to answer for this on the two paths a
        // person's advance took, through `stood_down`; those paths come here
        // now, and the record can still name a Drone on the step being taken —
        // a Fleet that died holding one leaves exactly that. A no-op on the
        // ordinary path, costing one load.
        self.every_exit_recorded(&job_id).await?;
        // Read before anything moves, for the reason every act that reaches a
        // spawn reads it there: a record that would not open should refuse
        // rather than escalate a Job it had already half-moved.
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(&job_id)
            .map_err(Adrift::Reading)?;
        // **On a restart, before the step re-enters `running`.** The row keeps
        // the verdict it stopped on across that move, but the judgments and the
        // gaming flags are filed by attempt, and entering `running` is what
        // mints the next one.
        let stopped = match &owed {
            Owed::Restarted { step } => Some(self.what_stopped(&job, step).await?),
            _ => None,
        };

        let job = self.move_job(&job, Target::Running, Actor::Fleet).await?;
        let (job, step) = self.entered(job, &owed).await?;
        let crossed = carried_across(&job, &step, &owed, &recorded);
        let opening = match stopped {
            // **`resuming`, which no other path here takes.** The Drone is as
            // new as a fresh one and knows only what stopped *this* part.
            Some(stopped) => Opening::resuming(stopped),
            // **`fresh` on the other three.** `Stopped` is read off
            // `last_verdict` and the Judge's answers, and a step a person
            // cleared or sent back stopped at none of them.
            None => Opening::fresh(),
        };
        self.put_a_drone_on(&job, &step, worktree, opening.carrying(crossed), working)
            .await
    }

    /// Which shape the inner machine is in, and therefore which act left it.
    ///
    /// **Every other state is a refusal.** A `queued` Job whose current step is
    /// `not_started` or `retrying` was left there by nothing this file knows
    /// about, and putting a Drone on a guess is how a Job comes to be worked at
    /// the wrong step.
    fn owed(&self, job: &Job) -> Result<Owed, Adrift> {
        let job_id = job.id().clone();
        let Some(step) = job.current_step_id().cloned() else {
            return Err(Adrift::NoSuchStep {
                job: job_id,
                step: None,
            });
        };
        let state = job
            .step(&step)
            .map(|row| row.state())
            .ok_or_else(|| Adrift::NoSuchStep {
                job: job_id.clone(),
                step: Some(step.clone()),
            })?;
        match state {
            StepState::Running => Ok(Owed::Standing {
                cleared: job.redirect_waiting().is_none(),
                step,
            }),
            StepState::Stopped => Ok(Owed::Restarted { step }),
            StepState::Advanced => match job.workflow().after(&step) {
                Some(next) => Ok(Owed::Overruled {
                    advanced: step,
                    next: next.id().clone(),
                }),
                // Unreachable through `override_verdict`, which finishes a Job
                // whose last step it overruled rather than re-queueing it — no
                // Drone is owed for a workflow with nothing left in it.
                None => Err(Adrift::NoSuchStep {
                    job: job_id,
                    step: Some(step),
                }),
            },
            other => Err(Adrift::IllegalStepMove(IllegalStepTransition::NoSuchEdge {
                step_id: step,
                from: other,
                to: StepState::Running,
            })),
        }
    }

    /// Enter the step the Drone is about to be put on, where it is not there
    /// already.
    ///
    /// **This is where a run begins**, and it is why two of the three shapes
    /// arrive with a step still to move: see the module header.
    async fn entered(&self, job: Job, owed: &Owed) -> Result<(Job, StepId), Adrift> {
        match owed {
            Owed::Standing { step, .. } => Ok((job, step.clone())),
            Owed::Restarted { step } => {
                let job = self.move_step(&job, step, StepTarget::Running).await?;
                Ok((job, step.clone()))
            }
            Owed::Overruled { next, .. } => {
                let job = self.move_step(&job, next, StepTarget::Running).await?;
                Ok((job, next.clone()))
            }
        }
    }
}

/// What the fresh Drone is handed about the parts before the one it is taking.
///
/// A free function: it reads the frozen workflow and the evidence rows and
/// touches no Fleet state, which is [`the_part_before`]'s reason and the same
/// one `copy_attachments` has.
fn carried_across(
    job: &Job,
    step: &StepId,
    owed: &Owed,
    recorded: &[(StepId, StepEvidence)],
) -> Crossed {
    let crossed = Crossed::nothing().and_produced(Produced::before(job.workflow(), step, recorded));
    // **`Cleared::reviewed` says a person read the part before and accepted
    // it**, so exactly the two acts where that happened carry one.
    let cleared = match owed {
        Owed::Standing {
            step,
            cleared: true,
        } => the_part_before(job.workflow(), step),
        // The overruled part, named directly rather than counted back from the
        // next one: an override is precisely a person accepting it.
        Owed::Overruled { advanced, .. } => job.workflow().step(advanced),
        // A restart re-runs the part that stopped, and nobody just accepted the
        // one before it — its gate may have been auto. And a step sent back
        // across a gate was not accepted at all; the person's own note is what
        // says why the Drone is there, and `put_a_drone_on` folds it in.
        Owed::Standing { .. } | Owed::Restarted { .. } => None,
    };
    match cleared {
        Some(passed) => crossed.and_cleared(Cleared::reviewed(passed)),
        None => crossed,
    }
}

/// The step immediately before this one in the Job's frozen workflow.
///
/// `None` on the first step, which has no part before it. A free function
/// rather than a method: it touches no Fleet state, and it answers the same
/// question `Produced::before` answers about evidence — asked here about the
/// declaration, which is what `Cleared` is worded from.
fn the_part_before<'a>(workflow: &'a FrozenWorkflow, at: &StepId) -> Option<&'a ResolvedStep> {
    let steps = workflow.steps();
    let here = steps.iter().position(|step| step.id() == at)?;
    steps.get(here.checked_sub(1)?)
}
