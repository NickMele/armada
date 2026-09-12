//! The inside of the loop: taking an approved Job, running it, and ending it.
//!
//! Split from [`daemon`](mod@crate::daemon) — the seams Fleet is assembled from and what it can
//! be asked. This is what happens to one Job in a slot, and the only file that calls
//! `Job::transition` and `Job::transition_step`. **Which** Job gets a slot is
//! [`admitting`](mod@crate::admitting)'s; a Job that has run before is [`readmitting`](mod@crate::readmitting)'s.
//!
//! **The order in `dispatch` is the specification.** `queued -> running` happens first, before
//! the worktree and the Drone — the registry forces it: a step cannot start from `queued`
//! because the inner machine advances only beneath `running`, and `queued`'s outbound edges
//! give a disk that will not give up a worktree no expressible destination.
//!
//! **Nothing here removes a worktree, on any path** — not on a failed Check, a kill or an
//! interruption. No method in this workspace could: `Vcs` has no removal, and a failed Job's
//! branch is exactly as its Drone left it, which is what "a person reads the branch" depends on.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{
    Actor, Branch, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, Level, StepId,
    StepLevelTrigger, StepState, StepTarget, Target, Transitioned,
};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::briefing::Opening;
use crate::crossing::{Cleared, Crossed, Produced};
use crate::daemon::Fleet;
use crate::drone::{aftermath, Aftermath, Ending, Left};
use crate::gate::{apply, Ruling};
use crate::session::{LiveSession, Occasion};
use crate::terms::Declaring;
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
    /// **Two Jobs are queued and only one of them is new.** A Job whose branch is already
    /// written has run before: a person answered it at a human gate or acted on it while it
    /// was escalated. Its worktree, its branch and every earlier step's work are on disk, and
    /// starting it from the first step would re-run work that was already accepted.
    ///
    /// The branch is the discriminator and it is exact, not a heuristic: written once, here,
    /// and of the three statuses `queued` is reachable from, only `awaiting_approval` has no
    /// branch — `awaiting_review` and `escalated` are both downstream of this line.
    ///
    /// Every failure below leaves the Job `escalated` rather than `running`, and returns the
    /// cause. A person decides; Fleet does not retry, and does not requeue itself to fail again.
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
            WorktreeSpec::for_job(&self.host().repo_root, &job.handle()).map_err(|cause| {
                Adrift::Unworkable {
                    job: job_id.clone(),
                    cause,
                }
            })?;
        // **With the Manifest's base on it**, which `Vcs` cannot put there —
        // `adapters` may not read a Manifest. Without it this step's readings
        // measure from the checkout's HEAD and later steps' from the base.
        let worktree = match self.vcs().create_worktree(&spec) {
            Ok(worktree) => self.based(worktree),
            Err(cause) => {
                self.stopped_before_a_drone(&job, EscalationTrigger::NoWorktree)
                    .await?;
                return Err(Adrift::NoWorktree {
                    job: job_id,
                    cause: Box::new(cause),
                });
            }
        };

        // **Claimed before preparation, so a `setup.requires` command already
        // sees its own `${port.NAME}`.** Here and nowhere else, for
        // `create_worktree`'s own reason above: "once per worktree" is a
        // property of this call site rather than of a record Fleet would have
        // to keep. See `crate::ports`.
        self.claimed_ports(&job).await?;

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
    /// **The reading `diff_nonempty` is decided against**, taken at the moment a step starts
    /// so the gate compares the step's own work rather than the branch's — `WorkProduct`
    /// measures from the commit the branch was cut from, crediting predecessors' work too.
    ///
    /// **After the rebase, on every path, since every path has one now.** A rebase's move is
    /// inherited rather than done: a conflicting one leaves markers, a clean one replays onto a
    /// base that itself moved, and reading before it would make git's output the next step's
    /// work. This used to except a Job's first step and an approved one; `#150` and `#180`
    /// closed both.
    ///
    /// **A failure leaves the step with no baseline, deliberately** — a reading that did not
    /// happen is not a worktree that did not move, so nothing here stores an empty footprint;
    /// the gate reads `None` as nothing known to have moved and fails the check.
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
        // **Before the Job or the step moves**, which is `crate::settling`'s
        // rule about the four records it writes ahead of this call and
        // [`reap`](Fleet::reap)'s about the spend in particular. Every arm below
        // publishes at least one move and a client re-reads the Job on it, so a
        // spend folded afterwards is a Job that reads as costing less than it
        // did. See [`paid_so_far`](Fleet::paid_so_far).
        self.paid_so_far(working).await?;
        match ruling {
            // **The Drone ends here, and a fresh one starts the next step on the same
            // worktree.** It used to be told and carried on with the same process/session; now
            // the last step of a Job pays for every step before it — `crate::boundary` owns
            // the ending order.
            //
            // **`tell` is not read on this arm any more.** There is no session to inject a
            // verdict into; what it *said* crosses as `Cleared`, re-tensed for a Drone that was
            // not there — see `crate::crossing`.
            //
            // **Nothing here rebases and nothing reads a baseline** — both are inside
            // `put_a_drone_on`, the one funnel every spawn goes through, already there for the
            // restart path. The catch-up rides the opening brief because there is nowhere else
            // for it to go.
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
                let entering = self.entering(&job, &next);
                let job = self.move_step(&job, &next, entering).await?;
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
            // The Job moves to the gate and its Drone ends there. **The step moves first and
            // holds at `awaiting_human`** — `#522` — since the inner machine advances beneath
            // both statuses; otherwise a client would see a Job at `awaiting_review` whose
            // current step still said a Drone was working it.
            //
            // **A person's review costs no fleet time**, and that is what the ending is for.
            // The work passed the machine gates, which is what ends a Drone; keeping the
            // session so `request_changes` could cost a turn rather than a respawn also kept
            // the working slot — one Job held the only slot for four hours and fifty-six
            // minutes doing nothing while a person read it. The slot frees this turn.
            //
            // **The cost is that `request_changes` cannot inject anything.** There is no Drone
            // to give the note to, so `#207` gives it somewhere to wait: the note goes onto the
            // Job, and the Drone re-admission puts back on the step opens with it.
            Ruling::HeldForReview { held, .. } => {
                let job = self.load(job_id).await?;
                let job = self
                    .move_step(&job, step, StepTarget::HeldForReview)
                    .await?;
                // **Only where the hold is not the one a person expects.** A
                // `human_always` step holding for a person is the commonest
                // event in the fleet and says nothing; a step whose repository
                // asked for automation and did not get it is a file somebody
                // has to fix, and the only place that can be said is here —
                // `crate::gate` reaches no store and writes nothing.
                if let Some(said) = held.worth_saying() {
                    self.noted_the_hold(&job, step, said);
                }
                self.applied(&job, ruling).await?;
                self.stood_down(job_id, working).await?;
                Ok(())
            }
            // A refusal `crate::asking` says to ask a person about rather than
            // stop the step over. The step moves; the Drone does not — see
            // that module for why.
            Ruling::Questioned { .. } => {
                let job = self.load(job_id).await?;
                let job = self
                    .move_step(&job, step, StepTarget::HeldForReview)
                    .await?;
                self.applied(&job, ruling).await?;
                self.asked_the_judge_question(&job, step, ruling).await?;
                Ok(())
            }
            // The gate failed and there is budget left. **Nothing about the Job moves** — it
            // is still `running`, the Drone still holds its session and context, and only the
            // step goes round again.
            //
            // Two step moves, because `retrying` is a pair of edges and not a resting place: the
            // first writes `retrying` with the trigger (a machine handoff, not a person
            // restarting a stopped step), and the second is the entry `store::attempt` counts —
            // without it the next run's checks would overwrite this one's.
            //
            // **Nothing re-reads the baseline and nothing re-asks for a plan.** The step was
            // entered once and this is still that entry: a baseline taken here would make
            // `diff_nonempty` ask whether *this attempt* wrote something rather than the step,
            // and a second `declare_plan_at` would spend a turn asking a Drone to restate a
            // plan it never left.
            Ruling::HandedBack { tell, retrying, .. } => {
                let job = self.load(job_id).await?;
                let job = self
                    .move_step(&job, step, StepTarget::Retrying(*retrying))
                    .await?;
                self.move_step(&job, step, StepTarget::Running).await?;
                self.tell(job_id, tell, None, working).await
            }
            // Four stops, one shape: the work stops here, the Drone is not told, and `apply`
            // decides which status and which trigger. `Suspect` joins them because a person is
            // asked either way; what differs is the claim, which is the trigger's to say.
            // **A refusal does not reach the retry budget** — resubmitting under the same
            // instructions would produce the same work. **Its reprompt is not injected here any
            // more** — the step stopping ends the Drone, so `expected` and `produced` reach the
            // opening brief of the Drone a person restarts the step with instead. `#204`.
            //
            // **`CouldNotDecide` is the fourth, and it is not a verdict.** The shape is shared
            // and the claim is not: the other three weighed the work, and this one is Fleet
            // saying it could not read what it needed to weigh it with. The alternative is what
            // it used to do — leave the Job `running` for the liveness clock to find — and
            // only a stopped step is one `crate::resume` can put a person back on. What could
            // not be read is written by `crate::settling` before this runs.
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
                // failed one's. `running -> awaiting_repair` is guarded on
                // `no_step_running`, so this order is now the machine's rather
                // than only this file's.
                let job = match stopping(ruling) {
                    Some(why) => self.move_step(&job, step, StepTarget::Stopped(why)).await?,
                    None => job,
                };
                self.applied(&job, ruling).await?;
                // Terminated without a turn, and the worktree is kept — on the
                // one ruling here that leaves a person to answer at their own
                // pace. A spent budget frees the slot in this turn, as a human
                // gate does; a refusal keeps its Drone alive and idle, which is
                // what makes a redirect cost no respawn. `Ruling::ends_the_drone`.
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
    /// A terminate that fails is a process already gone or one the OS will not signal: the
    /// slot is already free, and the Job has already moved.
    ///
    /// **What it spent goes onto the Job, and for a fortnight it did not.** This is the ending
    /// [`Ruling::Finished`] takes and, since `#397`, the one a Job whose gate-failure attempts
    /// are spent takes to `awaiting_repair`. Its Drone was signalled and dropped without a
    /// fold, so the Job's record showed every Drone's cost but its last, and `#51`'s cap was
    /// reading a number that was short. `Fleet::stood_down_paying` is the pairing, shared with
    /// the two endings that always had it.
    ///
    /// The ids are read before the slot is consumed, so a spend that will not write still
    /// leaves a departure that can be. Neither refusal can return — six callers end a Drone as
    /// part of moving a Job that has already moved — so both go into that Job's own log.
    pub(crate) async fn end_the_drone(&self, working: &mut Option<Working>) {
        let ended = match working.take() {
            Some(at_work) => {
                let (job_id, step, _) = at_work.drone();
                if let Err(why) = self.stood_down_paying(at_work).await {
                    self.noted_adrift(&why);
                }
                // A departure nobody could write down is what leaves a Board
                // showing a Drone on a Job that has none, and it used to leave
                // nothing behind at all.
                if let Err(why) = self.drone_left(&job_id, &step).await {
                    self.noted_adrift(&why);
                }
                Some(job_id)
            }
            None => None,
        };
        if let Some(job_id) = ended {
            let dropped = self.empty_the_inbox(&job_id);
            self.dropped_with_the_job(&job_id, dropped);
        }
    }

    /// Pause the Job for a person, holding its worktree as-is, before any process existed.
    ///
    /// **The trigger is a parameter because the answer differs and the fact does not.** Every caller is
    /// upstream of the spawn, so nothing is running and nothing is missing on any of them; what changes is
    /// who fixes it — `no_worktree` the disk or repository, `not_configurable` the Manifest or model
    /// roster, `would_not_start` the daemon's own environment. All three are Job-level, kept out of
    /// `last_verdict` by [`core_model::StepLevelTrigger::of`].
    ///
    /// **It was one trigger, `interrupted`, until 2026-08-31.** `interrupted` means a Job marked running
    /// has no matching OS process, so borrowing it here sent whoever read it after a dead Drone on a Job
    /// that had never spawned one — the third such defect found in a week, after `gate_failure`'s verb
    /// and `not_prepared`'s split.
    ///
    /// It is not a home for `interrupted` itself: the two sites that legitimately raise it have a
    /// process to be missing, which is the one thing no caller here does.
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
            // **After teardown, never on a timer.** `docs/concepts/fleet.md`,
            // *Teardown, then release*: the servers Fleet holds for the Job are
            // stopped and waited for first — `crate::servers` — and everything
            // else Armada spawned in the worktree was a process-group kill that
            // already happened. `escalated` is excluded by `is_terminal()`, so
            // an interrupted Job keeps its span, and its servers, until a
            // person answers it.
            self.stopped_servers_of(moved.job.id()).await;
            self.released_ports(&moved.job).await;
        }
        self.publish(ipc::Event::JobStateChanged((&moved.event).into()));
        Ok(moved.job)
    }

    /// A Job as an event publish carries it, with the reason its last transition stored.
    ///
    /// **`From<&core_model::Job>` cannot make this redaction.** That conversion is right the
    /// instant a Job is created, advances a step or gains or loses a Drone while still
    /// `running` — none of those carry a reason. It stops being right once `escalated`: the
    /// reason is what `escalation` in `packages/screens/src/render.ts` reads to draw the
    /// dead-end render, a client replaces its whole row on every one of these events rather
    /// than patching it, and no later event puts the reason back while the Job sits `escalated`.
    ///
    /// **`queued_reason`, `budget_hold`, `asking` and `resumption` stay `None`, for `From`'s
    /// own reason** — nothing publishes an event about a `queued` Job, so there is no board to
    /// read those from here either.
    pub(crate) async fn published(&self, job: &Job) -> Result<ipc::JobSummary, Adrift> {
        let reason = self.last_reason(job.id()).await?;
        Ok(ipc::JobSummary::of(
            job,
            reason.as_ref(),
            None,
            None,
            false,
            None,
        ))
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

    /// The target that enters a step, given where that step already stands.
    ///
    /// **One place asks it, because a forward walk cannot tell by looking.** Every step of every
    /// linear workflow enters as [`StepTarget::Running`]; a step a loop has come round to has not
    /// been dispatched into, since a return leaves the emitting step where it stood, and entering
    /// it again is [`StepTarget::Revisited`]. The two walk different edges the machine refuses
    /// each in the other's place, so a call site choosing by hand can be wrong.
    ///
    /// **Two states answer `Revisited` and they are the same fact** — routing leaves the
    /// emitter `running` where it was mechanical and `awaiting_human` where a person at the gate
    /// asked for it; reading only the first left the loop's second pass one move short.
    ///
    /// **It never invents a return.** A step that has `advanced` answers [`StepTarget::Running`]
    /// here and the machine refuses it (`StepAlreadyAdvanced`): a return needs the step that
    /// routed it, and this cannot see one.
    pub(crate) fn entering(&self, job: &Job, step: &StepId) -> StepTarget {
        match job.step(step).map(|row| row.state()) {
            Some(StepState::Running | StepState::AwaitingHuman) => StepTarget::Revisited,
            _ => StepTarget::Running,
        }
    }

    /// The same move, said by somebody other than Fleet.
    ///
    /// **Only what a person did needs it.** Every step move Fleet derives — a gate ruling, a
    /// dispatch, a reap — is Fleet's, and [`move_step`](Fleet::move_step) is the spelling. An
    /// override is a person advancing a step the gate refused, and the actor is the whole
    /// content of that row: a `stopped -> advanced` recorded against Fleet would say Fleet
    /// overruled itself.
    ///
    /// **`crate::reviewing` is the other caller, and the actor tells its row from the loop's.**
    /// Both reach [`StepTarget::Revisited`] and neither carries a payload; Fleet on that row is
    /// a loop coming round, and a person on it is a person sending the work back — nothing
    /// else distinguishes them.
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
        // The row whole, so a client replaces it rather than re-reading it —
        // with the reason its last transition stored, for `published`'s
        // reason: a redirect can revisit a step on a Job still `escalated`
        // the instant this fires, ahead of the `move_job` that returns it to
        // `running` a line below the caller.
        let summary = self.published(&moved.job).await?;
        self.publish(ipc::Event::JobStepAdvanced(ipc::JobStepAdvanced::of(
            &moved.event,
            summary,
        )));
        Ok(moved.job)
    }

    /// The line saying why a step gated on a policy is holding, where the
    /// reason is not the one a person would read off the workflow file.
    ///
    /// **[`Level::Warn`], and it is the only hold in the fleet that gets one.**
    /// Nothing failed and the work is fine, so this is not a verdict — but the
    /// repository asked for something it is not getting, and the fix is in a
    /// file rather than in the Job. A line at `Info` would sit among the step's
    /// ordinary traffic and be read by nobody.
    ///
    /// **A log line that will not write does not undo the hold**, for
    /// `noticing::logged`'s reason: the Job is where it should be either way,
    /// and this is the account of why.
    fn noted_the_hold(&self, job: &Job, step: &StepId, said: &'static str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("step", FieldValue::Str(step.as_str().to_string()))
        .with_field(
            "advance_gate",
            FieldValue::Str("manifest_rule:review_gate".to_string()),
        );
        self.logged(job.id(), envelope);
    }
}

/// Why the step stops, on each of the four rulings that end the work on it.
///
/// **[`Ruling::stops_the_step`] answers three, and the fourth is spelled here rather than
/// folded into it.** `gate::apply` reads that method as the trigger to escalate on — "the
/// rulings that escalate are exactly the rulings that stop the step" — and a gate failure
/// escalates nothing, because the Job is over.
///
/// A failure spells `gate_failure`, the trigger a hand-back already writes: the same tier
/// failed, and what differs is whether there was budget left. Without this, `#179` — the Job
/// reached `completed_failed` while its `tests` step stayed `running` with a null verdict, so
/// the only record that the step had failed was the Check run itself.
///
/// `pub` for one caller outside the loop: `acceptance`'s bench restates this ordering because
/// a hermetic test cannot reach `act_on`. It restates the order and **not** the decision — a
/// second spelling of `gate_failure` over there is how the two would come to disagree.
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
