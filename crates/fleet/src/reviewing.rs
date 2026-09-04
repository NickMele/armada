//! The three answers a person gives at a human gate.
//!
//! **Every one is a transition, and none writes a status.** Approving advances
//! the step on the inner machine and then moves the Job on the outer one;
//! requesting changes re-queues it at the same step; rejecting ends it. All
//! three go through `Job::transition`, so a review is a row in the same log as
//! everything else, and the actor on it is **human**.
//!
//! **All three refuse anywhere but `awaiting_review`.**
//! `awaiting_approval -> rejected` is a legal edge and it is the *dispatch*
//! gate's — `deny_dispatch`, a different act on a Job that never ran, and a
//! reject that took it would be two operations sharing one route. What puts a
//! Job at this gate is a step gated `advance_gate: human_always` whose tiers
//! all held: `crate::gate` rules `HeldForReview`, `apply` takes the edge, and
//! the step stays `running`.
//!
//! **The step advances before the Job moves, and it has to.**
//! `ADVANCING_STATUSES` is `running` and `awaiting_review`, so the inner
//! machine freezes the moment the Job leaves the gate. Both step moves an
//! approval makes are therefore made while it is still standing there, and
//! `current_step_id` is what re-admission reads to know where to put the Drone.
//!
//! **The gate holds no Drone and no slot**, so a person's review costs no fleet
//! time: both answers that keep the Job re-queue rather than resume.
//! `job-statuses.toml`'s `awaiting_review` row says the same.
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{
    Actor, Component, Envelope, Job, JobId, JobStatus, Level, RedirectWaiting, StepId, StepTarget,
    Target,
};
use std::path::Path;
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::gate::{self, SentBack};
use crate::resume::Redirection;
use crate::session::{LiveSession, Occasion};
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
    /// Take the work. **The counterpart to [`approve`](Fleet::approve)**, at
    /// the other end of the Job: that one releases a Job to spawn, this one
    /// accepts what it produced. It needs no Drone and there is never one to
    /// need — the decision is the person's, recorded whatever the slot holds.
    ///
    /// The step advances, and then one of two things happens. A gate on the
    /// last step commits the work, delivers the branch and records
    /// `completed_success` — the same three things [`finish`](Fleet::finish)
    /// does, because a Job whose branch is uncommitted is correct, verified and
    /// unmergeable. A workflow with a step left goes back in the **queue**.
    ///
    /// **Approving is not a resume, and the slot is why.** The gate stood its
    /// Drone down when it opened, and the slot it freed is very often another
    /// Job's by the time a person answers; a Job that assumed one here would
    /// either fail or take it from underneath the Job in it. So both step moves
    /// are made where the inner machine is still live, the Job takes
    /// `awaiting_review -> queued`, and `crate::dispatch`'s re-admission puts a
    /// Drone back on it on the first turn a slot is free.
    ///
    /// **The branch is caught up on the far side of the decision**, inside
    /// [`put_a_drone_on`](Fleet::put_a_drone_on). An approval is a verdict on
    /// the step that was worked, not an authorisation to merge, so the rebase
    /// moves the tree the *next* step starts from rather than what was read.
    /// Rebasing on the way *in* buys nothing: the base moves while a person
    /// reads, and a conflict would put markers into the diff being judged.
    pub async fn approve_review(&self, job_id: &JobId) -> Result<Job, Adrift> {
        // **A Job at a human gate has no Drone**, because the gate stood it
        // down so a person's review costs no fleet time — so ordinarily there
        // is no slot at all and this is an empty one. It is opened rather than
        // looked up because `completed` may still have work to land through it,
        // and it is closed again below whatever happens.
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        let step = self.at_the_gate(&job)?;
        let passed = self.declared_step(&job, &step)?.clone();
        let next = job.workflow().after(&step).cloned();
        // Before the Job moves and never after: the inner machine is frozen
        // beneath every status but the two that advance, and `awaiting_review`
        // is one of them only until this call leaves it.
        let job = self.move_step(&job, &step, StepTarget::Advanced).await?;
        let told = OutcomeTurn::approved(&passed, next.as_ref());
        let Some(next) = next else {
            let done = self.completed(&job, &told, job_id, &mut working).await?;
            // **After the slot is let go**, which is the lock order
            // `crate::slots` states: admission takes the roster, and a caller
            // holding a slot must not reach for it.
            drop(working);
            self.admit_next().await?;
            return Ok(done);
        };
        // The next step is entered here for the same reason, and it is what
        // re-admission reads to know where to put the Drone: `current_step_id`
        // moves when a step enters `running` and nothing else moves it. Which
        // target enters it is `entering`'s to say — a loop that came round
        // arrives at a step already being worked.
        let entering = self.entering(&job, next.id());
        let job = self.move_step(&job, next.id(), entering).await?;
        // The actor is **human**. A person took the Job out of the gate; Fleet
        // only decides which turn it gets a process back.
        self.move_job(&job, Target::Queued, Actor::Human).await?;
        // Inline, exactly as `approve` dispatches inline off the other gate —
        // so a fleet with nothing else to do starts the next step now rather
        // than on the next tick, and a busy one leaves the Job in the queue
        // where a person can see it waiting.
        //
        // The `told` above is still built and still goes to the Drone on the
        // path where there is no next step: a Job that finished tells the
        // process that finished it. There is nobody to tell here. That a person
        // accepted the part crosses as a block in the next Drone's opening
        // brief, built by `crate::dispatch`'s re-admission.
        drop(working);
        self.admit_next().await?;
        self.load(job_id).await
    }

    /// Send the work back with a note. **The worktree and every step so far
    /// survive**: the Job is worked again at the same step and the note is what
    /// it is worked against. The step does not move, which is the whole
    /// difference between this and an approval.
    ///
    /// **A Drone that is there is told, and nothing waits**: a turn injected
    /// into a live session, the Job back to `running`, no record of the words.
    /// It is the branch a gate does not reach today and is kept anyway, because
    /// "there is a process" is a question about the slot rather than about
    /// where the Job stands.
    ///
    /// **A gate with no Drone writes the note down and re-queues.** The note
    /// goes onto the Job, the Job takes `awaiting_review -> queued`, and
    /// `crate::dispatch`'s re-admission puts a fresh Drone on the same step
    /// with the note in its opening brief — `crate::spawning` delivers it and
    /// clears it. **Not `running`**, which is what this used to refuse over: a
    /// Job put straight back to `running` with no process on it escalates as
    /// `interrupted` a moment later. The slot the gate freed is very often
    /// another Job's by the time a person answers, which is why this and
    /// [`approve_review`](Fleet::approve_review) take the same edge.
    ///
    /// **Whichever runs, the other must not** — a note both injected and
    /// written down is one a Drone reads twice. Where a Job has no worktree
    /// left to put a Drone on there is nowhere for the note to go at all, and
    /// it refuses before anything moves: [`Adrift::NoDroneToTell`].
    ///
    /// **The re-queue ends the pass and the injection does not**, which is
    /// `#418` and is the same split. Told into a live session, the run carries
    /// on: one Drone, one session, and a resubmission inside a run supersedes
    /// rather than opening a new one. Written down, the Drone that answers the
    /// note is a *different* process — the gate stood the first one down and
    /// its session cannot be reopened — so the work after the note is somebody
    /// else's, and filing it under the first one's ordinal writes over the only
    /// account of what the note was about. So the second path takes
    /// [`StepTarget::Revisited`] and the first takes nothing.
    ///
    /// **It opens a pass and does not spend one.** `store::step_spent` resets
    /// at that edge, as `retry_count`'s registry row says a re-entry as
    /// designed must — nothing failed here, and a Job that could die of being
    /// reviewed is the fix overshooting into the defect beside it.
    pub async fn request_changes(&self, job_id: &JobId, note: &Redirection) -> Result<Job, Adrift> {
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        let step = self.at_the_gate(&job)?;
        // **Where the verdict goes is the step's declaration, not this
        // method's.** `crate::gate` answers it with no store and no worktree,
        // the way it answers every other verdict; what is left here is writing
        // the answer down. A step that declares no `verdict_routing` answers
        // `ToTheSameStep`, which is every step of every linear workflow and is
        // the whole of what this act did before a loop could be declared.
        let declared = self.declared_step(&job, &step)?.clone();
        let pass = self
            .store()
            .lock()
            .await
            .step_iteration(job_id, &step)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        let answered = match gate::sent_back(&declared, pass) {
            SentBack::ToTheSameStep => false,
            SentBack::ToAnEarlierStep(target) => {
                self.route_back(&job, &step, &target, note, &mut working)
                    .await?;
                true
            }
            SentBack::NowhereLeft(spent) => {
                self.loop_is_spent(&job, &step, spent, &mut working).await?;
                true
            }
        };
        if answered {
            // **After the slot is let go**, which is the lock order
            // `crate::slots` states and the reason neither call above admits
            // for itself: admission takes the roster, and a caller holding a
            // slot must not reach for it. A Job re-queued with the lock still
            // held sat at `queued` until the next tick.
            drop(working);
            self.admit_next().await?;
            return self.load(job_id).await;
        }
        if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            let job = self.move_job(&job, Target::Running, Actor::Human).await?;
            self.said(job_id, note, &working).await?;
            return Ok(job);
        }
        // Before anything moves, so a refusal leaves the Job at the gate rather
        // than half-answered — the property this method had when it refused
        // every time.
        if self.surviving_worktree(&job).is_err() {
            return Err(Adrift::NoDroneToTell {
                job: job_id.clone(),
            });
        }
        // The note is written before the Job moves and never after. A move that
        // landed with the write still to come would put the Job in the queue
        // with the person's words nowhere, which is the failure the whole
        // refusal existed to prevent, arriving one line later.
        let waiting = self.hold_the_note(&job, note, Said::AtTheGate).await?;
        // **The pass ends here, and the record has to say so** — `#418`. The
        // step does not move, so nothing entered `running` and
        // `store::attempt` had nothing to count; the fresh Drone wrote its
        // Checks, evidence and judgments over the ones the note was about, and
        // a step sent back twice read as a step that passed first time.
        // `StepTarget::Revisited` is the boundary `#263` gave the loop for the
        // same reason, reaching the road a person walks.
        //
        // **While the Job is still `awaiting_review`**, which is in
        // `ADVANCING_STATUSES` only until the move below leaves it — the order
        // `approve_review` keeps for both of its step moves.
        let waiting = self
            .move_step_by(&waiting, &step, StepTarget::Revisited, Actor::Human)
            .await?;
        // The actor is **human**, for `approve_review`'s reason: a person took
        // the Job out of the gate, and Fleet only decides which turn it gets a
        // process back.
        self.move_job(&waiting, Target::Queued, Actor::Human)
            .await?;
        // Inline, exactly as an approval re-admits inline — so a fleet with
        // nothing else to do starts the step again now rather than on the next
        // tick.
        drop(working);
        self.admit_next().await?;
        self.load(job_id).await
    }

    /// Send the work back to an earlier step, as the workflow's own
    /// `verdict_routing` says.
    ///
    /// **The shape is [`approve_review`](Fleet::approve_review)'s and not
    /// [`request_changes`](Fleet::request_changes)'s**, because what happens is
    /// an advance in reverse: a step move made while the inner machine is still
    /// live, then `awaiting_review -> queued`, then re-admission putting a fresh
    /// Drone where `current_step_id` now points. The note rides along, because a
    /// person asking for another draft said what to change and the Drone that
    /// writes it is the one that needs to hear it.
    ///
    /// **The step that moves is the one being redone, and it names the gate.**
    /// `StepTarget::Returned` carries the emitting step so the row says whose
    /// `iteration_count` this pass belongs to; the gate itself does not move and
    /// stays `running`, which is how a step at a human gate already reads.
    ///
    /// **A Drone in the slot is ended.** The gate ordinarily stood it down, so
    /// there is usually none — but a process still standing on the step the Job
    /// is leaving has nothing left to do, and re-admission is what puts one on
    /// the step the work went back to.
    ///
    /// **Admission is the caller's**, and it is not an omission: this runs
    /// beneath the slot lock, and taking the roster while holding one is the
    /// lock order reversed — see [`completed`](Fleet::completed), which says the
    /// same thing.
    async fn route_back(
        &self,
        job: &Job,
        gate: &StepId,
        target: &StepId,
        note: &Redirection,
        working: &mut Option<crate::working::Working>,
    ) -> Result<Job, Adrift> {
        // Before anything moves, for `request_changes`'s reason: a refusal
        // leaves the Job at the gate rather than half-answered. There is
        // nowhere for the next pass to happen without a worktree.
        if self.surviving_worktree(job).is_err() {
            return Err(Adrift::NoDroneToTell {
                job: job.id().clone(),
            });
        }
        if working.as_ref().is_some_and(|at_work| at_work.is(job.id())) {
            self.end_the_drone(working).await;
        }
        // While the Job is still `awaiting_review`, which is in
        // `ADVANCING_STATUSES` only until the move below leaves it.
        let job = self
            .move_step_by(
                job,
                target,
                StepTarget::Returned(gate.clone()),
                Actor::Human,
            )
            .await?;
        let waiting = self.hold_the_note(&job, note, Said::AtTheGate).await?;
        self.move_job(&waiting, Target::Queued, Actor::Human).await
    }

    /// The loop did not converge, and the pass the verdict asked for is one the
    /// step's `iteration_cap` does not have.
    ///
    /// **Nothing failed**, which is the whole content of the trigger: a plan on
    /// its sixth draft under a cap of five is not a Drone that got it wrong
    /// five times, and `retry_count` is untouched.
    ///
    /// # Three moves, and every one of them a declared edge
    ///
    /// **`awaiting_review -> escalated` is `interrupted`'s alone.**
    /// `job-transitions.toml` gives that edge one `escalation_trigger`, an
    /// edge's identity there is its `(from, to)` pair, and `xtask` compares the
    /// two — so a second trigger on it is a change to the registry, and not one
    /// this wave has a ruling for.
    ///
    /// What is left is the road a person's answer already takes.
    /// [`request_changes`](Fleet::request_changes) moves
    /// `awaiting_review -> running` where a Drone is there to hear the note, so
    /// that edge already means "the person answered and the Job is back on the
    /// machine". It is true here too: they asked for another pass and the
    /// machine refused it. Then the step stops and the Job takes the default
    /// `running -> escalated`, which every trigger without an edge of its own
    /// fires — the step first, because the inner machine freezes the moment the
    /// Job leaves an advancing status.
    async fn loop_is_spent(
        &self,
        job: &Job,
        gate: &StepId,
        spent: core_model::StepLevelTrigger,
        working: &mut Option<crate::working::Working>,
    ) -> Result<Job, Adrift> {
        if working.as_ref().is_some_and(|at_work| at_work.is(job.id())) {
            self.end_the_drone(working).await;
        }
        // The actor is the person: they answered, and leaving the gate is what
        // their answer did. What happened next is Fleet's and is recorded as
        // Fleet's, two rows down.
        let job = self.move_job(job, Target::Running, Actor::Human).await?;
        let job = self
            .move_step_by(&job, gate, StepTarget::Stopped(spent), Actor::Fleet)
            .await?;
        // **Fleet is the actor here and the person is not.** They asked for
        // another pass; what refused it is the workflow's own bound, read by
        // Fleet. A row saying a person escalated the Job would say they gave up
        // on it.
        self.move_job(&job, Target::Escalated(spent.trigger()), Actor::Fleet)
            .await
    }

    /// Put a person's note on the Job, for the opening brief of the next Drone
    /// that starts on it.
    ///
    /// **One road, two entrances.** `request_changes` writes at a human gate
    /// and [`restart_step`](Fleet::restart_step) writes on a step that stopped;
    /// both are a person saying something to a Drone that does not exist yet,
    /// and both are answered by the same column, the same spawn and the same
    /// clearing. A second writer assembling this for itself would be a second
    /// road to one destination, and the note's whole lifetime rule lives on the
    /// road.
    ///
    /// **It refuses a second note over an undelivered first**, which is
    /// `core_model::RedirectAlreadyWaiting` and not a rule invented here: the
    /// refusal carries the held note back, so the person is left holding both
    /// sets of words. Nothing reaches it from the gate — the Job leaves
    /// `awaiting_review` in that same call, under the slot lock — and a restart
    /// reaches it whenever a spawn failed after a note had been written.
    ///
    /// **The write lands before the caller moves anything**, and the caller's
    /// job is to keep it that way.
    pub(crate) async fn hold_the_note(
        &self,
        job: &Job,
        note: &Redirection,
        said: Said,
    ) -> Result<Job, Adrift> {
        let waiting =
            job.redirect_waits(waiting_note(note))
                .map_err(|held| Adrift::NoteAlreadyWaiting {
                    job: job.id().clone(),
                    held,
                })?;
        self.store()
            .lock()
            .await
            .record_redirect_waiting(&waiting)
            .map_err(Adrift::Writing)?;
        self.noted_waiting(job.id(), &waiting, said);
        Ok(waiting)
    }

    /// Write into the Job's own log that a person spoke and nobody was there.
    ///
    /// **The first of the pair, and `crate::spawning` writes the second.** The
    /// owner's ruling is that the record says a redirect was *delivered* rather
    /// than merely written — which needs both lines, because one of them alone
    /// cannot tell "you were told" from "nobody was there".
    ///
    /// It carries the words. They are the person's own and they are already on
    /// the record; a log line that said only that a note existed would send a
    /// reader to the column to find out what it was.
    fn noted_waiting(&self, job: &JobId, waiting: &Job, said: Said) {
        let words = waiting
            .redirect_waiting()
            .map(|note| note.text())
            .unwrap_or_default();
        let stood = said.clause();
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            format!("{stood}, and the note is waiting for the next Drone: \"{words}\""),
        )
        .in_job(job.as_ulid().clone());
        // A log line that will not write does not undo the write that matters,
        // for `resume::noted_roused`'s reason: the column is the record.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// A verdict on the work: the Job is over and its Drone is ended.
    ///
    /// **Terminal, which is what makes it the hard stop.** `rejected` is a
    /// judgement on what was produced, unlike [`kill_job`](Fleet::kill_job),
    /// which clears a Job off the Board and carries no verdict at all. The
    /// worktree is left where it is, as every ending leaves it.
    pub async fn reject(&self, job_id: &JobId) -> Result<Job, Adrift> {
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                self.end_the_drone(&mut working).await;
            }
        }
        let job = self.load(job_id).await?;
        self.at_the_gate(&job)?;
        let rejected = self.move_job(&job, Target::Rejected, Actor::Human).await?;
        // The bound has room and a queued Job is entitled to it, the same as
        // after a kill. A rejection that left the place held would stop the
        // board on a Job that is over.
        self.admit_next().await?;
        Ok(rejected)
    }

    /// The last step passed, so the work lands and the Job is over.
    ///
    /// The order is [`finish`](Fleet::finish)'s and for its reasons: the commit
    /// comes before the Job is recorded complete, so a `completed_success` on
    /// the Board is a Job whose work is on its branch, and the failure is held
    /// until the Drone has been told and the slot freed.
    pub(crate) async fn completed(
        &self,
        job: &Job,
        told: &OutcomeTurn,
        job_id: &JobId,
        working: &mut Option<crate::working::Working>,
    ) -> Result<Job, Adrift> {
        let landed = self.land_and_deliver(job, working).await;
        let job = self
            .move_job(job, Target::CompletedSuccess, Actor::Human)
            .await?;
        let said = self.tell(job_id, told, None, working).await;
        self.end_the_drone(working).await;
        // **Admission is the caller's**, and it is not an omission: this holds
        // a slot, and taking the roster while holding one is the lock order
        // reversed. Both callers admit once they have let the slot go.
        landed?;
        said?;
        Ok(job)
    }

    /// The step a person is standing at, or why there is not one.
    ///
    /// **The status is the whole test.** A Job at `awaiting_review` is at a
    /// human gate by definition — the status carries no reason and there is
    /// nothing else to read — and the step is the one the Job's cursor names.
    fn at_the_gate(&self, job: &Job) -> Result<StepId, Adrift> {
        if job.status() != JobStatus::AwaitingReview {
            return Err(Adrift::NotUnderReview {
                job: job.id().clone(),
                status: job.status(),
            });
        }
        job.current_step_id()
            .cloned()
            .ok_or_else(|| Adrift::NoSuchStep {
                job: job.id().clone(),
                step: None,
            })
    }

    /// The frozen workflow's declaration of a step, which is what a turn is
    /// worded from. A step the workflow does not name is a fault in Fleet, not
    /// a review that can be answered.
    pub(crate) fn declared_step<'a>(
        &self,
        job: &'a Job,
        step: &StepId,
    ) -> Result<&'a core_model::ResolvedStep, Adrift> {
        job.workflow().step(step).ok_or_else(|| Adrift::NoSuchStep {
            job: job.id().clone(),
            step: Some(step.clone()),
        })
    }

    /// Say the person's words into the session the slot is holding.
    ///
    /// The same turn a redirect sends, because it is the same act on the
    /// Drone's side: a person read the work and wrote what to do about it, and
    /// the Agent Prompt Contract gives that turn no wording of Fleet's.
    async fn said(
        &self,
        job_id: &JobId,
        note: &Redirection,
        working: &Option<crate::working::Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Err(Adrift::NoDroneToTell {
                job: job_id.clone(),
            });
        };
        at_work.instructed(Occasion::Redirect, note.text());
        at_work
            .session()
            .redirect(note)
            .await
            .map_err(|cause| Adrift::NotTold {
                job: job_id.clone(),
                cause,
            })
    }

    /// What a Job's worktree holds, where it still has one.
    ///
    /// **`None` is "there is nothing to read"** — a Job at the approval gate, or
    /// one whose worktree has been reclaimed — and it is a different answer from
    /// a reading that found no change. A directory that will not open is an
    /// error rather than either.
    pub(crate) fn worktree_of(&self, job: &Job) -> Result<Option<Worktree>, Adrift> {
        let spec = WorktreeSpec::for_job(&self.host().repo_root, job.id().as_str())
            .map_err(Adrift::NoReadingWorktree)?;
        if !Path::new(&spec.worktree_path()).is_dir() {
            return Ok(None);
        }
        let branch = job
            .branch()
            .map(|branch| branch.as_str().to_string())
            .unwrap_or_else(|| spec.branch());
        Ok(Some(Worktree::at(spec.worktree_path(), branch)))
    }
}

/// Where a person was standing when they left the note.
///
/// **It reaches the log and nothing else.** The record holds one note and does
/// not say which act wrote it, because nothing reads that — the Drone is handed
/// the words, and a column recording the entrance would be a fact with one
/// reader and two ways to be wrong. What a person reading the Job's own log
/// needs is the sentence, and the two sentences are genuinely different: one
/// says a gate had nobody at it, and one says a person asked for another
/// attempt and said what to change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Said {
    /// At a human advance gate, through `request_changes`.
    AtTheGate,
    /// On a step that stopped, through [`restart_step`](Fleet::restart_step).
    Restarting,
}

impl Said {
    /// The clause the log line opens with. It states what happened and never
    /// what will — the delivery is `crate::spawning`'s line, and this one has
    /// to read the same whether or not that one ever comes.
    fn clause(self) -> &'static str {
        match self {
            Said::AtTheGate => "changes were asked for at the gate with no Drone there to hear it",
            Said::Restarting => {
                "a person restarted the step and said what to do differently, with no Drone \
                 there to hear it"
            }
        }
    }
}

/// The person's words, as the record holds them.
///
/// **Two types for one string, and the conversion is total.** `Redirection` is
/// what the wire decoded and refuses a blank; `RedirectWaiting` is what the
/// `jobs` row holds and refuses one too. Neither can be constructed empty, so
/// this cannot fail — and the `expect` is unreachable rather than unchecked.
fn waiting_note(note: &Redirection) -> RedirectWaiting {
    RedirectWaiting::saying(note.text())
        .expect("a Redirection is never blank, so neither is the note it becomes")
}
