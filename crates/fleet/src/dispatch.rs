//! The inside of the loop: taking an approved Job, running it, and ending it.
//!
//! Split from [`daemon`](mod@crate::daemon) because the two are different
//! subjects. That file is what Fleet *is* — the slot, the seams it is assembled
//! from, and the five things it can be asked. This one is what happens to one
//! Job while it is in the slot, and it is the only file in the workspace that
//! calls `Job::transition` and `Job::transition_step`.
//!
//! # The order in `dispatch` is the specification
//!
//! `queued -> running` happens **first**, before the worktree and before the
//! Drone, and that is not the order it reads as. It is forced by two things the
//! registry already decided. The inner machine advances only beneath `running`,
//! so a step cannot be started from `queued`. And `queued`'s only outbound
//! edges are `awaiting_approval`, `killed`, `running` and `escalated` with the
//! trigger `dependency_failed` — so a disk that will not give up a worktree has
//! **no expressible destination** from `queued`, and every one it does have
//! from `running`.
//!
//! # Nothing here removes a worktree, on any path
//!
//! Not on a failed Check, not on a kill, not on an interruption. There is no
//! method in this workspace that could: `Vcs` has no removal and the reason is
//! written on the trait. A failed Job's branch is exactly as its Drone left it,
//! which is what "a person reads the branch" depends on.

use adapter_traits::{
    AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec,
};
use core_model::{
    Actor, Branch, EscalationTrigger, Job, JobId, JobStatus, StepId, StepTarget, Target,
    Transitioned,
};
use store::Moved;
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::at_step::AtStep;
use crate::briefing;
use crate::daemon::Fleet;
use crate::drone::{aftermath, Aftermath, Ending, Left};
use crate::gate::{apply, rule_on, Ruling};
use crate::session::LiveSession;
use crate::working::Working;

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
    /// Start the next approved Job, if the slot is free.
    ///
    /// **A queue of one.** The first line is the whole of what "one Job at a
    /// time" means here, and there is nothing above it deciding an order among
    /// several — because there is never more than one running to decide
    /// between.
    pub(crate) async fn admit_next(
        &self,
        working: &mut Option<Working>,
    ) -> Result<Option<JobId>, Adrift> {
        if working.is_some() {
            return Ok(None);
        }
        let Some(job) = self.next_queued().await? else {
            return Ok(None);
        };
        let job_id = job.id().clone();
        self.dispatch(job, working).await?;
        Ok(Some(job_id))
    }

    /// The Job that has been waiting longest, by when it was approved.
    ///
    /// Ordered by the sequence of the event that put it at `queued`, not by id
    /// and not by creation: two Jobs created in either order can be approved in
    /// the other, and the person who approved first is entitled to expect
    /// theirs to run first.
    ///
    /// A row that would not rebuild is not dispatchable, and is reported by
    /// every read that returns a list rather than being silently completed
    /// here.
    async fn next_queued(&self) -> Result<Option<Job>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        let mut waiting = Vec::new();
        for job in loaded.jobs {
            if job.status() != JobStatus::Queued {
                continue;
            }
            waiting.push((self.approved_at(job.id()).await?, job));
        }
        waiting.sort_by_key(|(seq, _)| *seq);
        Ok(waiting.into_iter().next().map(|(_, job)| job))
    }

    /// When the Job was released to run, as the log's own sequence.
    ///
    /// A Job with no such event was **created** at `queued` — a sub-dispatch,
    /// approved as part of its parent — and is therefore older than anything
    /// that had to be approved on its own.
    async fn approved_at(&self, job_id: &JobId) -> Result<i64, Adrift> {
        let events = self
            .store()
            .lock()
            .await
            .events_for(job_id)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        Ok(events
            .iter()
            .rev()
            .find(|event| {
                matches!(
                    event.moved(),
                    Moved::Job {
                        to: JobStatus::Queued,
                        ..
                    }
                )
            })
            .map(|event| event.seq())
            .unwrap_or(i64::MIN))
    }

    /// Take one approved Job all the way to a running Drone.
    ///
    /// Every failure below leaves the Job `escalated` rather than `running`,
    /// and returns the cause. A person decides; Fleet does not retry, and does
    /// not put the Job back in the queue for itself to fail on again.
    async fn dispatch(&self, job: Job, working: &mut Option<Working>) -> Result<(), Adrift> {
        let job_id = job.id().clone();
        // The Job's own copy, never the file. A workflow edited while this Job
        // sat at the approval gate declares what it declared then.
        let Some(first) = job.workflow().steps().first() else {
            return Err(Adrift::NoSuchStep {
                job: job_id,
                step: None,
            });
        };
        let step = first.id().clone();

        let job = self.move_job(&job, Target::Running, Actor::Fleet).await?;

        let spec =
            WorktreeSpec::for_job(&self.host().repo_root, job_id.as_str()).map_err(|cause| {
                Adrift::Unworkable {
                    job: job_id.clone(),
                    cause,
                }
            })?;
        let worktree = match self.vcs().create_worktree(&spec) {
            Ok(worktree) => worktree,
            Err(cause) => {
                self.interrupt(&job).await?;
                return Err(Adrift::NoWorktree {
                    job: job_id,
                    cause: Box::new(cause),
                });
            }
        };

        // Every attachment the Job carries, copied again into this fresh
        // worktree — under `.armada/attachments/`, where `job_brief` names it
        // and a Drone's own tools can open it. Before the first-turn prompt is
        // assembled, which is what makes the path `job_brief` writes one the
        // Drone can actually read.
        if let Err((filename, cause)) = copy_attachments(&job, &worktree) {
            self.interrupt(&job).await?;
            return Err(Adrift::AttachmentUnreadable {
                job: job_id,
                filename,
                cause,
            });
        }

        // Read from the worktree, not derived from the id: a branch a reader
        // recomputes cannot be renamed and cannot say what happened. `Err` is
        // unreachable — `WorktreeSpec` refuses an empty job id.
        let job = match Branch::new(worktree.branch()) {
            Ok(branch) => self.branded(&job, branch).await?,
            Err(_) => job,
        };

        let job = self.move_step(&job, &step, StepTarget::Running).await?;

        let brief = match briefing::first_turn(&job, job.workflow(), &step) {
            Ok(brief) => brief,
            Err(cause) => {
                self.interrupt(&job).await?;
                return Err(Adrift::NotConfigurable { job: job_id, cause });
            }
        };
        self.put_a_drone_on(&job, &step, worktree, brief, working)
            .await
    }

    /// Run the gate over one waiting submission, and do what it says.
    pub(crate) async fn settle(
        &self,
        working: &mut Option<Working>,
    ) -> Result<Option<Ruling>, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(None);
        };
        let (job_id, step, worktree) = at_work.standing();
        let Some(landed) = self.take_evidence() else {
            return Ok(None);
        };
        // A submission from the idle Drone of a Job that has already stopped.
        // The gate ruled on this step and the inner machine is frozen beneath
        // `escalated`, so a second ruling has nowhere to be written. It is
        // dropped rather than held: a held one would be ruled on by the turn a
        // redirect produces, and the person's instruction would be answered by
        // a verdict on work done before they wrote it.
        if self.load(&job_id).await?.status() != JobStatus::Running {
            return Ok(None);
        }
        // The tool is bound to a Job at construction and the inbox is emptied
        // when a Job ends, so this cannot be a submission about some other Job.
        // Kept because the alternative to a guard here is a gate ruling on one
        // Job's step from another Job's evidence.
        if landed.job != job_id {
            return Ok(None);
        }

        let job = self.load(&job_id).await?;
        let Some(at) = AtStep::named(job.workflow(), &step, &worktree) else {
            return Err(Adrift::NoSuchStep {
                job: job_id,
                step: Some(step),
            });
        };
        // Assembled here rather than inside the gate: a Judge call
        // authenticates as Fleet, and a value that could not be built is a
        // configuration failure against this Job rather than a verdict.
        let judging = self.judging().map_err(|cause| Adrift::NotConfigurable {
            job: job_id.clone(),
            cause,
        })?;
        let declared = at_work.declared().cloned();
        // Read before the gate rather than inside it: `rule_on` reaches no
        // database, and a baseline is a row like any other.
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(&job_id)
            .map_err(Adrift::Reading)?;
        let ruling = rule_on(
            at,
            &landed.submission,
            declared.as_ref(),
            &recorded,
            self.work(),
            self.budget(),
            &judging,
        )
        .await;
        // Before the Job or the step moves. A recorded result the transition
        // then failed to make is readable; a transition whose evidence was
        // never written down is a verdict with no trace.
        self.recorded_checks(&job_id, &step, &ruling).await?;
        self.recorded_judgments(&job_id, &step, &ruling).await?;
        self.recorded_evidence(&job_id, &step, &landed.submission, &ruling)
            .await?;
        self.recorded_gaming(&job_id, &step, &ruling).await?;
        self.act_on(&ruling, &job_id, &step, working).await?;
        Ok(Some(ruling))
    }

    /// The Job move a ruling implies, and the step move it implies, in the one
    /// order the two machines admit.
    async fn act_on(
        &self,
        ruling: &Ruling,
        job_id: &JobId,
        step: &StepId,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        match ruling {
            Ruling::Advanced { tell, .. } => {
                let job = self.load(job_id).await?;
                let job = self.move_step(&job, step, StepTarget::Advanced).await?;
                let next = self.step_after(&job, step)?;
                self.move_step(&job, &next, StepTarget::Running).await?;
                if let Some(at_work) = working.as_mut() {
                    at_work.now_on(next);
                }
                // The boundary catch-up is `delivery`'s. It is told either way
                // — a Drone that never heard the step advanced would sit there,
                // and a base that would not read is not its fault.
                let caught_up = self.caught_up(working).await;
                let tell = tell.clone().and(caught_up.as_ref().ok().cloned().flatten());
                self.tell(job_id, &tell, working).await?;
                caught_up.map(|_| ())
            }
            // The whole of what finishing a Job is, including the commit that
            // makes its branch mergeable, is `landing`'s.
            Ruling::Finished { tell, .. } => self.finish(ruling, tell, job_id, step, working).await,
            // Three endings, one shape: the work stops here, the Drone is not
            // told, and `apply` decides which status and which trigger.
            // `Suspect` joins them because a person is being asked either way —
            // what differs is the claim being made, which is the trigger's to
            // say. The refusal reprompt the prompt contract specifies arrives
            // with the retry ledger, which is what would give a Drone somewhere
            // to go with a citation; until then it goes to the person.
            Ruling::Failed { .. } | Ruling::Refused { .. } | Ruling::Suspect { .. } => {
                let job = self.load(job_id).await?;
                // **Before the Job moves, and it cannot be after.** The inner
                // machine is frozen beneath every status but `running` and
                // `awaiting_review`, so a step stopped after the escalation
                // would be refused and `last_verdict` would stay unwritten —
                // which is exactly what left an escalated Job's step reading
                // `running` with nothing saying why.
                let job = match ruling.stops_the_step() {
                    Some(why) => self.move_step(&job, step, StepTarget::Stopped(why)).await?,
                    None => job,
                };
                self.applied(&job, ruling).await?;
                // Terminated without a turn, and the worktree is kept — on the
                // two rulings that end the Job. A refusal keeps its Drone
                // alive and idle, which is what makes a redirect cost no
                // respawn. See `Ruling::ends_the_drone`.
                if ruling.ends_the_drone() {
                    self.end_the_drone(working).await;
                }
                Ok(())
            }
            // Neither moves anything. `NotWhatTheStepAsked` asks the Drone
            // again — and **nothing is sent**, because `Ruling::tell` answers
            // `None` for it and `verification` has no turn for a resubmission.
            // That gap is real and is named in this crate's report.
            Ruling::NotWhatTheStepAsked(_) | Ruling::CouldNotDecide { .. } => Ok(()),
        }
    }

    /// The step that follows this one in **the Job's own** frozen workflow.
    fn step_after(&self, job: &Job, step: &StepId) -> Result<StepId, Adrift> {
        job.workflow()
            .after(step)
            .map(|next| next.id().clone())
            .ok_or_else(|| Adrift::NoSuchStep {
                job: job.id().clone(),
                step: Some(step.clone()),
            })
    }

    /// What follows from a Drone that is gone.
    ///
    /// **Three things have to be true before this decides anything**: the pipe
    /// closed, the process exited, and whether evidence is waiting. The middle
    /// one is separate from the first because a terminating event is a turn
    /// boundary and not a lifetime — a Drone that reported and then took an
    /// injected turn would otherwise be reaped mid-step.
    ///
    /// It is `DroneSession::exited` and **not** `crate::holder_of`, and that is
    /// a correction this file makes to its own first draft: `holder_of` asks
    /// `ps`, and `ps` reports a zombie as held. A child nobody has waited on is
    /// exactly a zombie, so the probe would have answered "still running"
    /// forever about a Drone that had finished.
    pub(crate) async fn reap(
        &self,
        working: &mut Option<Working>,
    ) -> Result<Option<Aftermath>, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(None);
        };
        if !at_work.transcript_ended() {
            return Ok(None);
        }
        if !at_work.exited().await.map_err(|cause| Adrift::NotReaped {
            job: at_work.standing().0,
            cause,
        })? {
            return Ok(None);
        }
        let job_id = at_work.standing().0;
        let heard = at_work.heard();
        // The status is read before the ending is folded, because an escalated
        // Job keeps its Drone: a process that is gone no longer proves the Job
        // was working, and asking one that already stopped to stop again is the
        // move the machine refuses.
        let standing = self.load(&job_id).await?;
        let after = aftermath(standing.status(), &Ending::of(&heard), self.left());
        match &after {
            Aftermath::JobMoves(target) => {
                // The departure first, so the Job's move is published over a
                // record that already says no Drone is on it.
                self.drone_left(&job_id).await;
                let job = self.load(&job_id).await?;
                self.move_job(&job, target.clone(), Actor::Fleet).await?;
                working.take();
                self.empty_the_inbox();
            }
            // The idle Drone of a Job a person is already holding. Its going is
            // the only fact, and it is what turns a redirect into a restart.
            Aftermath::AlreadyStopped => {
                self.drone_left(&job_id).await;
                working.take();
                self.empty_the_inbox();
            }
            Aftermath::TheGateDecides => {}
        }
        Ok(Some(after))
    }

    /// Whether the Drone left anything for the gate to rule on.
    pub(crate) fn left(&self) -> Left {
        if self.evidence_waiting() > 0 {
            Left::Evidence
        } else {
            Left::Nothing
        }
    }

    /// Inject the gate's outcome into the live session.
    ///
    /// **Only ever reached from the advance path.** `Ruling::tell` answers
    /// `None` on every ruling that is not an advance, so there is no call here
    /// that could deliver a verdict to a Drone about to be terminated.
    pub(crate) async fn tell(
        &self,
        job_id: &JobId,
        turn: &OutcomeTurn,
        working: &Option<Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(());
        };
        at_work
            .session()
            .tell(turn)
            .await
            .map_err(|cause| Adrift::NotTold {
                job: job_id.clone(),
                cause,
            })
    }

    /// End the Drone and free the slot. **The worktree is untouched.**
    ///
    /// A terminate that fails is a process already gone or one the operating
    /// system will not signal, and there is nothing further to do about either:
    /// the slot is already free, and the Job has already moved.
    ///
    /// The exit is recorded before the signal is sent, because the slot is
    /// taken here and the id goes with it — and a `drone.exited` that never
    /// landed would leave the Board showing a Drone on a Job that has none.
    pub(crate) async fn end_the_drone(&self, working: &mut Option<Working>) {
        if let Some(at_work) = working.take() {
            let (job_id, _) = at_work.drone();
            self.drone_left(&job_id).await;
            let _ = at_work.session().terminate().await;
        }
        self.empty_the_inbox();
    }

    /// Pause the Job for a person, holding its worktree as-is.
    ///
    /// `interrupted` is the trigger the registry gives for "a Job marked
    /// running has no matching OS process", which is literally what a worktree
    /// that would not be created and a Drone that would not start both leave
    /// behind. **No trigger names an infrastructure failure at dispatch**, and
    /// that gap is named in this crate's report rather than papered over with a
    /// trigger that means something else.
    pub(crate) async fn interrupt(&self, job: &Job) -> Result<(), Adrift> {
        self.move_job(
            job,
            Target::Escalated(EscalationTrigger::Interrupted),
            Actor::Fleet,
        )
        .await
        .map(|_| ())
    }

    /// Write the branch the worktree was made on. **No event and nothing
    /// published**: a worktree is not a transition, and the column is the
    /// field's authority.
    async fn branded(&self, job: &Job, branch: Branch) -> Result<Job, Adrift> {
        let job = job.on_branch(branch);
        self.store()
            .lock()
            .await
            .record_branch(&job)
            .map_err(Adrift::Writing)?;
        Ok(job)
    }

    /// Move the Job, write the event, publish it. **The only path.**
    pub(crate) async fn move_job(&self, job: &Job, to: Target, by: Actor) -> Result<Job, Adrift> {
        let moved = job
            .transition(to, by, self.now())
            .map_err(Adrift::IllegalMove)?;
        self.record(moved).await
    }

    /// The Job move a ruling implies, where it implies one.
    pub(crate) async fn applied(&self, job: &Job, ruling: &Ruling) -> Result<(), Adrift> {
        match apply(job, ruling, self.now()) {
            Some(moved) => {
                self.record(moved.map_err(Adrift::IllegalMove)?).await?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    async fn record(&self, moved: Transitioned) -> Result<Job, Adrift> {
        self.store()
            .lock()
            .await
            .record_transition(&moved)
            .map_err(Adrift::Writing)?;
        self.publish(ipc::Event::JobStateChanged((&moved.event).into()));
        Ok(moved.job)
    }

    /// Move one step of the frozen workflow, write it to the same log, and
    /// publish it. **The only path**, like [`move_job`](Fleet::move_job).
    ///
    /// The publish is after the write: announcing a move that then failed to
    /// land tells every client something that did not happen. It published
    /// nothing until now, and a Job running through four steps emitted one
    /// event and nothing after it.
    pub(crate) async fn move_step(
        &self,
        job: &Job,
        step: &StepId,
        to: StepTarget,
    ) -> Result<Job, Adrift> {
        let moved = job
            .transition_step(step, to, Actor::Fleet, self.now())
            .map_err(Adrift::IllegalStepMove)?;
        self.store()
            .lock()
            .await
            .record_step_transition(&moved)
            .map_err(Adrift::Writing)?;
        // The row whole, so a client replaces it rather than re-reading it.
        self.publish(ipc::Event::JobStepAdvanced(ipc::JobStepAdvanced::of(
            &moved.event,
            ipc::JobSummary::from(&moved.job),
        )));
        Ok(moved.job)
    }
}

/// Copy every attachment the Job carries into this worktree, under
/// `.armada/attachments/<filename>` — the path `briefing::job_brief` names, so
/// what the brief points at is there by the time a Drone reads it.
///
/// A free function rather than a method: it touches no Fleet state, and the
/// error it returns names the one attachment that failed rather than a whole
/// `Adrift` variant, which is `dispatch`'s own business to build — the same
/// split `Adrift::from_delivery` draws for a different seam.
fn copy_attachments(job: &Job, worktree: &Worktree) -> Result<(), (String, std::io::Error)> {
    if job.attachments().is_empty() {
        return Ok(());
    }
    let dir = std::path::Path::new(worktree.path())
        .join(".armada")
        .join("attachments");
    std::fs::create_dir_all(&dir).map_err(|cause| (String::new(), cause))?;
    for attachment in job.attachments() {
        std::fs::copy(&attachment.storage_ref, dir.join(&attachment.filename))
            .map_err(|cause| (attachment.filename.clone(), cause))?;
    }
    Ok(())
}
