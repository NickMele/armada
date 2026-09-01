//! The inside of the loop: taking an approved Job, running it, and ending it.
//!
//! Split from [`daemon`](mod@crate::daemon), which is what Fleet *is* — the
//! seams it is assembled from and the things it can be asked. This is what
//! happens to one Job in a slot, and the only file in the workspace that calls
//! `Job::transition` and `Job::transition_step`. **Which** Job gets a slot is
//! [`admitting`](mod@crate::admitting)'s; a Job that has run before is
//! [`readmitting`](mod@crate::readmitting)'s.
//!
//! # The order in `dispatch` is the specification
//!
//! `queued -> running` happens **first**, before the worktree and the Drone,
//! and that is not the order it reads as. The registry forces it. A step
//! cannot be started from `queued`, because the inner machine advances only
//! beneath `running`; and `queued`'s outbound edges give a disk that will not
//! give up a worktree **no expressible destination**, where `running` has every
//! one.
//!
//! # Nothing here removes a worktree, on any path
//!
//! Not on a failed Check, a kill or an interruption. No method in this
//! workspace could: `Vcs` has no removal and the reason is written on the
//! trait. A failed Job's branch is exactly as its Drone left it, which is what
//! "a person reads the branch" depends on.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{
    Actor, Branch, EscalationTrigger, Job, JobId, StepId, StepLevelTrigger, StepTarget, Target,
    Transitioned,
};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::briefing::{Declaring, Opening};
use crate::crossing::{Cleared, Crossed, Produced};
use crate::daemon::Fleet;
use crate::drone::{aftermath, Aftermath, Ending, Left};
use crate::gate::{apply, Ruling};
use crate::session::{LiveSession, Occasion};
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
    /// Take one approved Job all the way to a running Drone.
    ///
    /// **Two Jobs are queued and only one of them is new.** A Job whose branch
    /// is already written has run before: a person answered it at a human gate
    /// or acted on it while it was escalated, and it went back in the queue
    /// rather than straight to a Drone. Its worktree, its branch and every
    /// earlier step's work are on disk, and starting it from the first step
    /// would re-run work that was already accepted.
    ///
    /// The branch is the discriminator and it is exact, not a heuristic: it is
    /// written once, here, and of the three statuses `queued` is reachable from
    /// only `awaiting_approval` has no branch — `awaiting_review` and
    /// `escalated` are both downstream of this line.
    ///
    /// Every failure below leaves the Job `escalated` rather than `running`,
    /// and returns the cause. A person decides; Fleet does not retry, and does
    /// not put the Job back in the queue for itself to fail on again.
    pub(crate) async fn dispatch(
        &self,
        job: Job,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        if job.branch().is_some() {
            return self.readmitted(job, working).await;
        }
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
                self.stopped_before_a_drone(&job, EscalationTrigger::NoWorktree)
                    .await?;
                return Err(Adrift::NoWorktree {
                    job: job_id,
                    cause: Box::new(cause),
                });
            }
        };

        // **What the repository says has to be true of a worktree before work
        // starts in one.** Here and nowhere else because this is the only
        // `create_worktree` in the workspace, which is what makes "once per
        // worktree" a property of the call site rather than of a record Fleet
        // would have to keep. `crate::preparing` holds the rest.
        self.prepared(&job, &worktree).await?;

        // Every attachment the Job carries, copied again into this fresh
        // worktree — under `.armada/attachments/`, where `job_brief` names it
        // and a Drone's own tools can open it. Before the first-turn prompt is
        // assembled, which is what makes the path `job_brief` writes one the
        // Drone can actually read.
        if let Err((filename, cause)) = copy_attachments(&job, &worktree) {
            // `no_worktree` and not a trigger of its own. A worktree missing
            // the files the brief tells the Drone to open is not one work can
            // start in, which is the same state the line above leaves behind
            // and the same person's to fix.
            self.stopped_before_a_drone(&job, EscalationTrigger::NoWorktree)
                .await?;
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
        self.put_a_drone_on(&job, &step, worktree, Opening::fresh(), working)
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
            // **The Drone ends here, and a fresh one starts the next step on
            // the same worktree.** It used to be told and carried on: same
            // process, same session, same accumulated transcript, with the last
            // step of a Job — the one whose work lands — paying for every step
            // before it. `crate::boundary` owns the order the ending happens
            // in, and each part of that order answers a failure.
            //
            // **`tell` is not read on this arm any more.** There is no session
            // to inject a verdict into; what the verdict *said* crosses as
            // `Cleared`, re-tensed for a Drone that was not there — see
            // `crate::crossing`, which argues why a rendered turn cannot simply
            // be moved into an opening brief.
            //
            // **Nothing here rebases and nothing reads a baseline.** Both are
            // inside `put_a_drone_on`, which is the one funnel every spawn goes
            // through, and both were already there for the restart path. What
            // the catch-up came to rides the opening brief because there is
            // nowhere else for it to go.
            Ruling::Advanced { .. } => {
                let job = self.load(job_id).await?;
                // Read before the step moves: the block the next Drone gets
                // names the part that just cleared, by the label the frozen
                // workflow gives it.
                let passed = self.declared_step(&job, step)?.clone();
                let job = self.move_step(&job, step, StepTarget::Advanced).await?;
                let next = self.step_after(&job, step)?;
                // **The step that just dispatched, and children still going.**
                // A Drone put on the next step now would hold the slot its own
                // children need to finish — at a bound of two, a parent and one
                // child fill it. So the Job goes back in the queue with this
                // step advanced and the next one not entered, its Drone stands
                // down, and `admit_next` holds it there until every child is
                // terminal. `crate::readmitting` puts the Drone back.
                //
                // **Asked about `step` and not about `next`**, which is what
                // makes it survive the workflow becoming a loop. See the
                // predicate's own doc in `crate::sub_dispatch`.
                if self.dispatched_and_waits(&job, step).await? {
                    self.move_job(&job, Target::Queued, Actor::Fleet).await?;
                    self.stood_down(job_id, working).await?;
                    return Ok(());
                }
                let job = self.move_step(&job, &next, StepTarget::Running).await?;
                // Every step's evidence as the record holds it, read after
                // `crate::settling` wrote this step's. `Produced::before`
                // takes the one strictly-earlier row it wants out of it.
                let recorded = self
                    .store()
                    .lock()
                    .await
                    .step_evidence(job_id)
                    .map_err(Adrift::Reading)?;
                let crossed = Crossed::nothing()
                    .and_produced(Produced::before(job.workflow(), &next, &recorded))
                    .and_cleared(Cleared::checked(&passed));
                self.crossed_onto(&job, &next, crossed, working).await
            }
            // The whole of what finishing a Job is, including the commit that
            // makes its branch mergeable, is `landing`'s.
            Ruling::Finished { tell, .. } => self.finish(ruling, tell, job_id, step, working).await,
            // The Job moves to the gate and its Drone ends there. The step
            // stays `running` — it is what the person is standing at, and
            // `approve_review` advances it from there while the Job is still at
            // the gate.
            //
            // **A person's review costs no fleet time**, and that is what the
            // ending is for. The work passed the machine gates, which is what
            // ends a Drone; keeping the session so that a `request_changes`
            // could cost a turn rather than a respawn also kept the working
            // slot, and one Job held the only slot for four hours and fifty-six
            // minutes doing nothing while a person read it. The slot is freed
            // in this turn and `admit_next` may give it to the next Job.
            //
            // **The cost is that `request_changes` cannot inject anything.**
            // There is no Drone to give the note to, so `#207` gives it
            // somewhere to wait: the note goes onto the Job and the Drone
            // re-admission puts back on the step opens with it. Nothing here
            // softens the ending, because a Drone kept alive for that one path
            // would keep the slot for it too.
            //
            // The Job moves first, so the departure is published over a record
            // that already says where the Job stands.
            Ruling::HeldForReview { .. } => {
                let job = self.load(job_id).await?;
                self.applied(&job, ruling).await?;
                self.stood_down(job_id, working).await?;
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
            // instructions would produce the same work. **The refusal
            // reprompt the contract specifies is not injected here and no
            // longer waits to be** — the step stopping is what ends the
            // Drone, so `expected` and `produced` reach the opening brief of
            // the Drone a person restarts the step with instead. `#204`.
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
        let (job_id, spawned_on, drone_id) = at_work.drone();
        let heard = at_work.heard();
        // **Before anything moves the Job**, because the slot is what holds the
        // events and the arms below take it. Recording twice is harmless — the
        // spend row is keyed on the Drone — so `boundary::stood_down` folding
        // the same run again costs nothing and neither of the two has to know
        // about the other. See `crate::allowance`.
        self.record_spend(&job_id, &drone_id, &at_work.spent(&self.now()))
            .await?;
        // The status is read before the ending is folded, because an escalated
        // Job keeps its Drone: a process that is gone no longer proves the Job
        // was working, and asking one that already stopped to stop again is the
        // move the machine refuses.
        let standing = self.load(&job_id).await?;
        let after = aftermath(standing.status(), &Ending::of(&heard), self.left(&job_id));
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
                self.dropped_with_the_job(&job_id, self.empty_the_inbox(&job_id));
            }
            // The idle Drone of a Job a person is already holding. Its going is
            // the only fact, and it is what turns a redirect into a restart.
            Aftermath::AlreadyStopped => {
                self.drone_left(&job_id, &spawned_on).await?;
                working.take();
                // Reachable, unlike its neighbour: a Job that stopped while its
                // Drone was still submitting leaves evidence with no step to be
                // against. It goes, and the Job's log says it went.
                self.dropped_with_the_job(&job_id, self.empty_the_inbox(&job_id));
            }
            Aftermath::TheGateDecides => {}
        }
        Ok(Some(after))
    }

    /// Whether this Job's Drone left anything for the gate to rule on.
    ///
    /// **This Job's and not the inbox's**, which is `#50` arriving on a
    /// question that used to have one answer for all of Fleet: a Drone that
    /// exits having submitted nothing must not be read as having left evidence
    /// because some other Job's Drone did.
    pub(crate) fn left(&self, job: &JobId) -> Left {
        if self.evidence_waiting_for(job) > 0 {
            Left::Evidence
        } else {
            Left::Nothing
        }
    }

    /// Inject the gate's outcome into the live session.
    ///
    /// **Reached only where the Drone is still there afterwards**: a hand-back,
    /// which is the same step going round again in the same process, and the
    /// last step of a Job, where there is no next step to spawn onto and the
    /// turn goes to the process that finished the work. A step boundary reaches
    /// `crate::boundary` instead, because there is no session to inject into.
    ///
    /// `declaring` is `None` at both — a hand-back re-asks for no plan it never
    /// cleared, and a Job that has finished asks its Drone for nothing.
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
        // Written down before the send, not after: a turn the pipe would not
        // take is still a turn Armada composed and a person still has to read
        // it to know what the Drone was — or was not — told.
        at_work.instructed(Occasion::Outcome, turn.text());
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
        if let Some(job_id) = ended {
            let dropped = self.empty_the_inbox(&job_id);
            self.dropped_with_the_job(&job_id, dropped);
        }
    }

    /// Pause the Job for a person, holding its worktree as-is, before any
    /// process existed.
    ///
    /// **The trigger is a parameter because the answer differs and the fact
    /// does not.** Every caller is upstream of the spawn, so on all of them
    /// nothing is running and nothing is missing; what changes is who fixes it
    /// — `no_worktree` the disk or the repository, `not_configurable` the
    /// Manifest or the model roster, `would_not_start` the environment the
    /// daemon runs in. That is what a person reads off the badge, and it is all
    /// they get before they open the Job. All three are Job-level, so
    /// [`core_model::StepLevelTrigger::of`] keeps them out of `last_verdict`.
    ///
    /// **It was one trigger, `interrupted`, until 2026-08-31**, and this
    /// method's doc used to say outright that no trigger named an
    /// infrastructure failure at dispatch. Three do now. `interrupted` means a
    /// Job marked running has no matching OS process, so borrowing it here sent
    /// whoever read it after a dead Drone on a Job that had never spawned one —
    /// the third time this defect was found in a week, after `gate_failure`'s
    /// verb and `not_prepared`'s split.
    ///
    /// It is not a home for `interrupted` itself. The two sites that raise it
    /// legitimately — a Drone that vanished, and a submission for a Job outside
    /// the slot — have a process to be missing, which is the one thing no
    /// caller here does.
    pub(crate) async fn stopped_before_a_drone(
        &self,
        job: &Job,
        trigger: EscalationTrigger,
    ) -> Result<(), Adrift> {
        self.move_job(job, Target::Escalated(trigger), Actor::Fleet)
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
