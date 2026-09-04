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
    Actor, FrozenWorkflow, IllegalStepTransition, Job, ResolvedStep, Resumption, StepEvidence,
    StepId, StepState, StepTarget, Target,
};

use crate::adrift::Adrift;
use crate::briefing::Opening;
use crate::crossing::{Cleared, Crossed, Produced};
use crate::daemon::Fleet;
use crate::working::Working;

/// What a re-queued Job is owed: the step to work, and how the Drone opens.
///
/// **One variant per shape the inner machine can be in**, read off the current
/// step's state rather than off a column that remembers which button was
/// pressed.
///
/// | The current step reads | What put it back | The Drone gets |
/// |---|---|---|
/// | `running` | an approval, or changes asked for | the next step, or the same one again |
/// | `stopped` | a restart | the same step, told what stopped it |
/// | `advanced`, no dispatch | an override | the step after it, told a person cleared this one |
/// | `advanced`, it dispatched | Fleet, once the children finished | the step after it, told what they came to |
///
/// The two `running` cases are told apart by the waiting note, which is
/// `request_changes`'s alone — that test predates this file and is unchanged.
/// The two `advanced` cases are told apart by the frozen workflow, which is the
/// only thing that knows whether a step was allowed to create Jobs.
///
/// **Three of the four are a person and the fourth is not**, which is what
/// [`Owed::resumption`] answers `None` for.
pub(crate) enum Owed {
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
    /// The step is `advanced` and it dispatched Jobs — the machine cleared it,
    /// the Job stood its Drone down so its children could have the slot, and
    /// every child has now finished.
    ///
    /// **The same two step ids as [`Owed::Overruled`] and a different
    /// sentence.** What the Drone is told about the part before it is the whole
    /// difference: nobody read that part, a gate did, and telling a fresh Drone
    /// a person accepted it would be the record saying something that did not
    /// happen.
    AfterADispatch { dispatched: StepId, next: StepId },
}

impl Owed {
    /// This shape as the registry spells it, with the step ids taken out.
    ///
    /// **The wire's half of the same partition**, so a Board row and the Drone
    /// re-admission actually puts on cannot come from two readings. `Room::hold`
    /// is the same arrangement one file over.
    ///
    /// # `None` is a real answer, and it is not the absence of a fourth word
    ///
    /// [`Resumption`] names **a person's act**: all three of its values are
    /// something somebody did. A parent re-queued because it dispatched Jobs
    /// and is waiting on them was put back by Fleet, and nobody did anything —
    /// so the honest answer is that nothing resumed it, and the row reads as
    /// the ordinary queued Job it is, with `blocked_by_dependency` as its
    /// reason.
    ///
    /// **Do not add a variant for it.** `Resumption` is a strict `wire_enum!`,
    /// so a fourth value is a major protocol bump — and the thing that would be
    /// bought is a word for something that is not a resumption in the first
    /// place. A variant added here that is Fleet's own doing, rather than a
    /// person's, wants `None` for the same reason.
    ///
    /// A Drone waiting on an answer never reaches this at all: its Job stays
    /// `running` and is never re-queued, so `crate::questioning` is the second
    /// case this rule covers and it covers it by not arriving.
    pub(crate) fn resumption(&self) -> Option<Resumption> {
        match self {
            Owed::Standing { .. } => Some(Resumption::Reviewed),
            Owed::Restarted { .. } => Some(Resumption::Restarted),
            Owed::Overruled { .. } => Some(Resumption::Overruled),
            Owed::AfterADispatch { .. } => None,
        }
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
    // **Here rather than in `crate::serving`, where it landed.** That file's
    // header keeps it the `Daemon` trait impl and nothing else; this is a
    // helper calling `owed` below it, and it put that file over 900.
    /// Which act a person took to put this Job back in the queue, where one
    /// did.
    ///
    /// **`readmitting::Fleet::owed` and nothing beside it.** That is the
    /// function re-admission itself calls to decide which step a Drone goes
    /// back on, so a row saying "restarted" and the Drone that arrives cannot
    /// disagree — the same rule `queued_reason` follows against `admit_next`.
    ///
    /// **Nothing is read for a Job that is not `queued`**, and the record is
    /// already in hand for one that is, so this costs no store call at all.
    ///
    /// **A refusal reads as "nobody put this Job back".** On a `queued` Job it
    /// means the current step is `not_started` or missing, which is a Job that
    /// arrived from `awaiting_approval` and has never run. The refusal that
    /// matters is still re-admission's, made at the spawn: this renders nothing
    /// where that would refuse, and never offers a word where it would not.
    ///
    /// **A shape that resolves and answers `None` reads the same way** and means
    /// something different: re-admission knows which step this Job is owed, and
    /// no person is why it waits. A parent that dispatched Jobs and stood its
    /// Drone down is the case — [`Owed::resumption`] holds why that is not a
    /// fourth word.
    pub(crate) fn resumption(&self, job: &Job) -> Option<core_model::Resumption> {
        if job.status() != core_model::JobStatus::Queued {
            return None;
        }
        self.owed(job).ok().and_then(|owed| owed.resumption())
    }

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
        // Read before the Job moves. A worktree that has been reclaimed is a
        // Job whose earlier steps' work is not on disk, and there is nothing to
        // put a Drone back onto — the same refusal `restart_step` makes, asked
        // before anything else is done rather than after.
        let worktree = match self.surviving_worktree(&job) {
            Ok(worktree) => worktree,
            Err(cause) => {
                // **Admitted, then stopped, and the record says both.** The Job
                // is `queued` here — `admit_next` reads no other kind — and
                // `queued -> escalated` is the edge `dependency_failed` owns,
                // which by the edge table's own rule accepts that trigger and
                // no other. So this arm could not escalate at all: until
                // 2026-08-31 it asked for `interrupted`, got the machine's
                // `WrongTrigger` back, returned *that* in place of the missing
                // worktree, and left the Job `queued` for admission to fail on
                // again every turn. Moving through `running` is what makes an
                // escalation reachable from here, and it is true — Fleet took
                // the slot and then found nothing in it to work in.
                let job = self.move_job(&job, Target::Running, Actor::Fleet).await?;
                self.stopped_before_a_drone(&job, core_model::EscalationTrigger::NoWorktree)
                    .await?;
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
        // **Read only after a dispatch**, and it is a board read: every other
        // act here pays nothing for it. `None` on the other three is the
        // ordinary boundary carrying nothing.
        let dispatched = match &owed {
            Owed::AfterADispatch { .. } => self.dispatched_jobs(&job_id).await?,
            _ => None,
        };
        let crossed = carried_across(&job, &step, &owed, &recorded).and_dispatched(dispatched);
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
    ///
    /// **`serving` reads it too, and reads the refusal as "nobody put this Job
    /// back".** That is what the error means on a Job at `queued`: it arrived
    /// from `awaiting_approval` and has never run, so there is no act to name.
    /// The refusal is still re-admission's — a read renders nothing where a
    /// spawn would refuse, and never the other way round.
    pub(crate) fn owed(&self, job: &Job) -> Result<Owed, Adrift> {
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
                // Which of the two an advanced step is, read off the frozen
                // workflow rather than off a column: a step that dispatched is
                // one the definition gave the dispatching role, and a person
                // cannot have overruled a gate that never refused.
                Some(next)
                    if job
                        .workflow()
                        .step(&step)
                        .is_some_and(ResolvedStep::may_dispatch_jobs) =>
                {
                    Ok(Owed::AfterADispatch {
                        dispatched: step,
                        next: next.id().clone(),
                    })
                }
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
    /// **This is where a run begins**, and it is why three of the four shapes
    /// arrive with a step still to move: see the module header.
    async fn entered(&self, job: Job, owed: &Owed) -> Result<(Job, StepId), Adrift> {
        match owed {
            Owed::Standing { step, .. } => Ok((job, step.clone())),
            Owed::Restarted { step } => {
                let job = self.move_step(&job, step, StepTarget::Running).await?;
                Ok((job, step.clone()))
            }
            Owed::Overruled { next, .. } | Owed::AfterADispatch { next, .. } => {
                // `entering` for `crate::dispatch`'s reason: the step a
                // forward walk arrives at is already `running` where a loop
                // came round to it, and the two entrances walk different edges.
                let entering = self.entering(&job, next);
                let job = self.move_step(&job, next, entering).await?;
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
        //
        // The part after a dispatch is not here either, and it is the one case
        // left out for the opposite reason: the part before it *was* cleared, by
        // the machine, and that is `Cleared::checked` two arms down rather than
        // `Cleared::reviewed`.
        Owed::Standing { .. } | Owed::Restarted { .. } | Owed::AfterADispatch { .. } => None,
    };
    if let Owed::AfterADispatch { dispatched, .. } = owed {
        if let Some(passed) = job.workflow().step(dispatched) {
            return crossed.and_cleared(Cleared::checked(passed));
        }
    }
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
