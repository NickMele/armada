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

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{
    Actor, Branch, DependencyDirection, EscalationTrigger, Job, JobId, JobStatus, StepId,
    StepLevelTrigger, StepTarget, Target, Transitioned,
};
use store::Moved;
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::briefing::{Declaring, Opening};
use crate::daemon::Fleet;
use crate::drone::{aftermath, Aftermath, Ending, Left};
use crate::gate::{apply, Ruling};
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
        let standing: BTreeMap<JobId, JobStatus> = loaded
            .jobs
            .iter()
            .map(|job| (job.id().clone(), job.status()))
            .collect();
        let mut waiting = Vec::new();
        for job in loaded.jobs {
            if job.status() != JobStatus::Queued {
                continue;
            }
            // Approved and still waiting on a peer. It is skipped rather than
            // reordered — the queue is by approval and this is not a Job's turn
            // being taken, it is a Job that has nothing to work against yet.
            if !clear_to_run(&job, &standing) {
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

        // The brief is `put_a_drone_on`'s to assemble, because the catch-up it
        // runs first is part of what a Drone is told. A worktree cut a moment
        // ago is ordinarily not behind anything, so this path is where the
        // funnel costs one `standing` call and announces nothing — and where
        // the repository's HEAD is not the base, a Drone that would otherwise
        // have started two commits back is told so on its first turn.
        self.put_a_drone_on(&job, &step, worktree, Opening::Fresh, working)
            .await
    }

    /// Read what the worktree holds now, and hold it as this step's baseline.
    ///
    /// **The reading `diff_nonempty` is decided against**, taken at the moment
    /// a step starts so that what the gate compares is the step's own work
    /// rather than the branch's. `WorkProduct` measures from the commit the
    /// branch was cut from, which credits every step with everything its
    /// predecessors wrote.
    ///
    /// **After the rebase, on every path — because every path has one now.**
    /// What a rebase moves is inherited rather than done: a conflicting one
    /// leaves markers and a clean one replays the branch onto a base that
    /// itself moved. Both are content, so a baseline read before it makes git's
    /// output the next step's work, and a Drone that resolved nothing passes
    /// `diff_nonempty` on the markers it was handed. This used to except a
    /// Job's first step and an approved one; `#150` and `#180` closed both.
    ///
    /// **A failure leaves the step with no baseline, and that is deliberate.**
    /// A reading that did not happen is not a worktree that did not move, so
    /// there is no arm here that stores an empty footprint — the gate reads
    /// `None` as nothing known to have moved and fails the check. An unread
    /// baseline must not be able to advance a step, which is
    /// `Changed::nothing`'s rule applied one level up.
    pub(crate) fn marked(&self, working: &mut Option<Working>) {
        let Some(at_work) = working.as_ref() else {
            return;
        };
        let (_, _, worktree) = at_work.standing();
        let Ok(footprint) = self.work().footprint(&worktree) else {
            return;
        };
        if let Some(at_work) = working.as_mut() {
            at_work.entering_with(footprint);
        }
    }

    /// The Job move a ruling implies, and the step move it implies, in the one
    /// order the two machines admit.
    pub(crate) async fn act_on(
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
                // **Read before the slot moves and sent with the verdict.**
                // `now_on` clears the declaration the last step carried, so a
                // Drone that is not asked here is a Drone working a scoped step
                // having declared nothing — which the gate then fails it for.
                let asked = job.workflow().step(&next).and_then(Declaring::at);
                if let Some(at_work) = working.as_mut() {
                    at_work.now_on(next, self.now());
                }
                // The boundary catch-up is `delivery`'s. It is told either way
                // — a Drone that never heard the step advanced would sit there,
                // and a base that would not read is not its fault.
                let caught_up = self.caught_up(working).await;
                // **After the rebase and before the Drone is told**, which is
                // the whole of the window this reading has. It cannot be
                // earlier than `now_on`, which clears the last step's baseline;
                // it cannot be later than `tell`, or the first thing the Drone
                // does on hearing the verdict is folded into what it started
                // from. The rebase sits inside that window and belongs on the
                // inherited side of it — see [`marked`](Fleet::marked). Read
                // whatever the catch-up came to, including an error: a rebase
                // that would not run leaves a worktree in some state, and this
                // step starts from the one that is there.
                self.marked(working);
                let tell = tell.clone().and(caught_up.as_ref().ok().cloned().flatten());
                self.tell(job_id, &tell, asked.as_ref(), working).await?;
                caught_up.map(|_| ())
            }
            // The whole of what finishing a Job is, including the commit that
            // makes its branch mergeable, is `landing`'s.
            Ruling::Finished { tell, .. } => self.finish(ruling, tell, job_id, step, working).await,
            // The Job moves and **nothing else does**. The step stays `running`
            // — it is what the person is standing at, and `approve_review`
            // advances it from there while the Job is still at the gate. The
            // Drone is neither told nor ended: it holds its context so that a
            // `request_changes` costs a turn rather than a respawn, and the
            // person's answer is the next thing its session hears.
            Ruling::HeldForReview { .. } => {
                let job = self.load(job_id).await?;
                self.applied(&job, ruling).await?;
                Ok(())
            }
            // The gate failed and there is budget left. **Nothing about the
            // Job moves** — it is still `running`, the Drone still holds its
            // session and its context, and the only thing that happens is the
            // step going round again.
            //
            // Two step moves, because `retrying` is a pair of edges and not a
            // resting place. The first writes `retrying` with the trigger, so
            // the log says this entry into `running` was the machine handing
            // work back rather than a person restarting a stopped step. The
            // second is the entry itself, and it is what `store::attempt`
            // counts — without it the next run's checks and judgments would
            // overwrite this one's and the record would read as one attempt.
            // `step_machine` argues the shape at length.
            //
            // **Nothing re-reads the baseline and nothing re-asks for a plan.**
            // The step was entered once and this is still that entry: a
            // baseline taken here would make `diff_nonempty` ask whether *this
            // attempt* wrote something rather than whether the step did, and a
            // second `declare_plan_at` would spend a turn asking a Drone to
            // restate a plan it never left.
            Ruling::HandedBack { tell, retrying, .. } => {
                let job = self.load(job_id).await?;
                let job = self
                    .move_step(&job, step, StepTarget::Retrying(*retrying))
                    .await?;
                self.move_step(&job, step, StepTarget::Running).await?;
                self.tell(job_id, tell, None, working).await
            }
            // Four stops, one shape: the work stops here, the Drone is not
            // told, and `apply` decides which status and which trigger.
            // `Suspect` joins them because a person is being asked either way —
            // what differs is the claim being made, which is the trigger's to
            // say. **A refusal does not reach the retry budget**, though the
            // budget now exists: it answers a mechanical failure, and what
            // sends a step back to its Drone is a check that ran and said no.
            // A refusal says the work runs and is not what was asked for,
            // which is a person's to answer — resubmitting under the same
            // instructions would produce the same work. The refusal reprompt
            // the prompt contract specifies is still unwritten.
            //
            // **`CouldNotDecide` is the fourth, and it is not a verdict.** The
            // shape is shared and the claim is not: the other three weighed the
            // work, and this one is Fleet saying it could not read what it
            // needed to weigh it with. It joins them because the alternative is
            // what it used to do — leave the Job `running` for the liveness
            // clock to find, with nothing anywhere saying why — and because
            // only a stopped step is one `crate::resume` can put a person back
            // on. What could not be read is written into the Job's log by
            // `crate::settling`, before this runs.
            Ruling::Failed { .. }
            | Ruling::Refused { .. }
            | Ruling::Suspect { .. }
            | Ruling::CouldNotDecide { .. } => {
                let job = self.load(job_id).await?;
                // **Before the Job moves, and it cannot be after.** The inner
                // machine is frozen beneath every status but `running` and
                // `awaiting_review`, so a step stopped after the Job left
                // `running` would be refused and `last_verdict` would stay
                // unwritten — which is exactly what left an escalated Job's
                // step reading `running` with nothing saying why, and then a
                // failed one's. `running -> completed_failed` is guarded on
                // `no_step_running`, so this order is now the machine's rather
                // than only this file's.
                let job = match stopping(ruling) {
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
            // Nothing moves, and it is the one ruling left that moves nothing.
            // `NotWhatTheStepAsked` asks the Drone again — and **nothing is
            // sent**, because `Ruling::tell` answers `None` for it and
            // `verification` has no turn for a resubmission. That gap is real
            // and is named in this crate's report.
            //
            // It stays here for the reason the arm above stopped holding it: a
            // submission of the wrong kind spent no Check and derived no
            // artifact, so nothing was read that could have failed to be read.
            Ruling::NotWhatTheStepAsked(_) => Ok(()),
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
        // The step the Drone was **put on**, which is where its pointer is —
        // not the step the slot has advanced to beneath it.
        let (job_id, spawned_on, _) = at_work.drone();
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
                self.drone_left(&job_id, &spawned_on).await?;
                let job = self.load(&job_id).await?;
                self.move_job(&job, target.clone(), Actor::Fleet).await?;
                working.take();
                // Ordinarily nothing: `left` answers `Left::Evidence` when a
                // submission is waiting and this arm is not the one reached.
                // Said out loud on the arm it is not reached from, because the
                // one drop nobody wrote down is the defect this pair closes.
                self.dropped_with_the_job(&job_id, self.empty_the_inbox());
            }
            // The idle Drone of a Job a person is already holding. Its going is
            // the only fact, and it is what turns a redirect into a restart.
            Aftermath::AlreadyStopped => {
                self.drone_left(&job_id, &spawned_on).await?;
                working.take();
                // Reachable, unlike its neighbour: a Job that stopped while its
                // Drone was still submitting leaves evidence with no step to be
                // against. It goes, and the Job's log says it went.
                self.dropped_with_the_job(&job_id, self.empty_the_inbox());
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

    /// Inject the gate's outcome into the live session, with whatever the step
    /// it moves the Drone on to asks for.
    ///
    /// **Only ever reached from the advance path.** `Ruling::tell` answers
    /// `None` on every ruling that is not an advance, so there is no call here
    /// that could deliver a verdict to a Drone about to be terminated.
    ///
    /// `declaring` is `None` where the next step asks for no plan, and where
    /// there is no next step at all — a Job that has finished asks its Drone
    /// for nothing.
    pub(crate) async fn tell(
        &self,
        job_id: &JobId,
        turn: &OutcomeTurn,
        declaring: Option<&Declaring>,
        working: &Option<Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(());
        };
        at_work
            .session()
            .tell(turn, declaring)
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
        let ended = match working.take() {
            Some(at_work) => {
                let (job_id, step, _) = at_work.drone();
                // This one cannot return — six callers end a Drone as part of
                // moving a Job that has already moved — so the refusal goes
                // into that Job's own log rather than into a discard. A
                // departure nobody could write down is what leaves a Board
                // showing a Drone on a Job that has none, and it used to leave
                // nothing behind at all.
                if let Err(why) = self.drone_left(&job_id, &step).await {
                    self.noted_adrift(&why);
                }
                let _ = at_work.session().terminate().await;
                Some(job_id)
            }
            None => None,
        };
        let dropped = self.empty_the_inbox();
        if let Some(job_id) = ended {
            self.dropped_with_the_job(&job_id, dropped);
        }
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

    /// The write, the footprint a terminal Job is owed, and the publish — in
    /// that order.
    ///
    /// **The footprint sits between them deliberately.** A client refetches the
    /// Job on the event, so recording after the publish would leave a window
    /// where the Job reads as finished with nothing to say about what it
    /// touched — the exact absence [`kept_footprint`](Fleet::kept_footprint)
    /// exists to end, narrowed to a race instead of removed. It costs one
    /// repository read on the transition that ends a Job and nothing on any
    /// other.
    ///
    /// [`kept_footprint`](Fleet::kept_footprint) answers nothing and refuses
    /// nothing: the move has already landed, and a Job that ended is over
    /// whether or not its worktree could be read.
    async fn record(&self, moved: Transitioned) -> Result<Job, Adrift> {
        self.store()
            .lock()
            .await
            .record_transition(&moved)
            .map_err(Adrift::Writing)?;
        if moved.job.status().is_terminal() {
            self.kept_footprint(&moved.job).await;
        }
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
        self.move_step_by(job, step, to, Actor::Fleet).await
    }

    /// The same move, said by somebody other than Fleet.
    ///
    /// **Only one act needs it.** Every step move above follows from something
    /// Fleet derived — a gate ruling, a dispatch, a reap — so Fleet is the
    /// actor and [`move_step`](Fleet::move_step) is the spelling. An override
    /// is a person advancing a step the gate refused, and the actor is the
    /// whole content of that row: the log cannot reconstruct afterwards who
    /// disagreed with the Judge, and a `stopped -> advanced` recorded against
    /// Fleet would say Fleet overruled itself.
    pub(crate) async fn move_step_by(
        &self,
        job: &Job,
        step: &StepId,
        to: StepTarget,
        by: Actor,
    ) -> Result<Job, Adrift> {
        let moved = job
            .transition_step(step, to, by, self.now())
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

/// Why the step stops, on each of the four rulings that end the work on it.
///
/// **[`Ruling::stops_the_step`] answers three, and the fourth is spelled here
/// rather than folded into it.** `gate::apply` reads that method as the trigger
/// to escalate on — "the rulings that escalate are exactly the rulings that
/// stop the step" is the sentence it is written under — and a gate failure
/// escalates nothing, because the Job is over. Folding the two together would
/// have the one ruling that ends a Job derive an escalation for it.
///
/// A failure spells `gate_failure`, the trigger a hand-back already writes: the
/// same tier failed, and what differs is whether there was budget left to
/// answer it. Without this, #179 — the Job reached `completed_failed` while its
/// `tests` step stayed `running` with a null verdict, so the only record that
/// the step had failed was the Check run itself, and `resume::resumable` finds
/// a step to act on by looking for the stopped one.
/// `pub` for one caller outside the loop: `acceptance`'s bench restates this
/// ordering because a hermetic test cannot reach `act_on`, which speaks to a
/// live session. It restates the order and **not** the decision — a second
/// spelling of `gate_failure` over there is how the two would come to disagree.
pub fn stopping(ruling: &Ruling) -> Option<StepLevelTrigger> {
    match ruling {
        Ruling::Failed { .. } => StepLevelTrigger::of(EscalationTrigger::GateFailure),
        other => other.stops_the_step(),
    }
}

/// Whether every Job this one waits on has finished, and finished well.
///
/// `pub(crate)` because `serving` renders the same fact as a label. **One
/// answer, not two** — a Board saying a Job is blocked while admission
/// disagrees is worse than a Board saying nothing.
///
/// **`completed_success` and nothing weaker.** A dependent admitted after a
/// failed upstream would do its work against a base that never landed, which is
/// the half-landed upstream the linked-DAG shape exists to prevent.
///
/// A peer that is not in the list at all counts as unfinished. An edge pointing
/// at a Job that was retained out is a sequence nothing can establish, and of
/// the two answers only "run it" cannot be taken back.
pub(crate) fn clear_to_run(job: &Job, standing: &BTreeMap<JobId, JobStatus>) -> bool {
    job.dependencies()
        .iter()
        .filter(|edge| edge.direction == DependencyDirection::DependsOn)
        .all(|edge| standing.get(&edge.peer) == Some(&JobStatus::CompletedSuccess))
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
