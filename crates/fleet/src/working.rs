//! The Job being worked: one slot, and everything holding it up.
//!
//! # A type on its own, because it has an invariant of its own
//!
//! Four things stay together for a Job to be workable — which Job, at which
//! step, the process, and the stream it is talking on — put together once, by
//! [`Working::holding`]; no constructor takes three. [`Working::standing`]
//! hands three back owned rather than borrowed: the slot is behind a lock, and
//! a borrow outliving the read would stop the gate clearing the slot while
//! holding what it read, which is what a step that ends a Job does.
//!
//! # No pid and no `drone_id`, and that is a gap rather than a decision
//!
//! `core_model::Job` has no writer for `assigned_drone` — the store refuses to
//! reconstruct that column for want of an event that sets it — so the process
//! working a Job is recorded nowhere a person can see. Dispatch mints a
//! `drone_id` to name the transcript and this does not keep it: a field read by
//! nobody reads as a field that is working, and the Job log line naming that
//! file is what connects a Job to its Drone.
//!
//! # There is no method that returns one event
//!
//! [`Working::heard`] answers with the whole run, because what anybody asks a
//! transcript is what it folds to. A per-event accessor would invite reading a
//! Drone's claim, which the gate exists to refuse.
use std::sync::Arc;

use std::time::Duration;

use adapter_traits::{AgentHarness, DroneEvent, Footprint, Worktree};
use core_model::{DeclaredPaths, DroneId, JobId, RepoPath, StepId, Timestamp};
use tokio::process::ChildStderr;

use crate::converging::{elapsed, Chain};
use crate::drone::Started;
use crate::footprint::Publishing;
use crate::session::DroneSession;
use crate::transcript::{StepLabel, Taps};
use crate::watch::Watching;
use verification::NotConverging;

/// The Job being worked, and everything holding it up.
///
/// **There is no second one.** It is held in an `Option`, and that `Option`
/// being `Some` is the whole of what "Fleet is busy" means.
pub(crate) struct Working {
    job: JobId,
    /// The Drone in the slot. The same id its transcript is named by, and what
    /// `assigned_drone` holds while this slot is full.
    drone: DroneId,
    /// Which step of the frozen workflow the Drone was told to do.
    step: StepId,
    /// The same answer as `step`, held where the transcript's sinks can read
    /// it. **Two places rather than one because the sinks are on the far side
    /// of the reader task**, and a step moved only here left every row after a
    /// Job's first advance claiming the first step. [`Working::now_on`] is the
    /// only writer of either.
    labelling: StepLabel,
    worktree: Worktree,
    session: DroneSession,
    transcript: Watching,
    /// Whatever the CLI complains about. **Never parsed**, and held rather than
    /// dropped: dropping it closes the pipe, and a Drone writing to a closed
    /// stderr takes a signal for it.
    _complaints: ChildStderr,
    /// Where the Drone said this step's work would be. **`None` until it
    /// declares**, which is a different answer from an empty declaration.
    declared: Option<DeclaredPaths>,
    /// Every file seen changed outside the declaration while the step ran, in
    /// the order first seen. **It does not fail the step** — the Drone may
    /// declare again — and it survives a revert, which is the only thing the
    /// live check sees that the gate cannot.
    drifted: Vec<RepoPath>,
    /// When the step in this slot started, as the injected clock read it. What
    /// the wall-clock tripwire is measured from.
    step_began: Timestamp,
    /// Fleet's own call count when the step started, so a step's own count is
    /// a subtraction rather than a second counter to keep true.
    ///
    /// **True at the instant it is taken**, which the harness's `turns` was
    /// not: see [`Progress::calls`](crate::Progress::calls).
    calls_before: u32,
    /// How many times the Drone had come to rest by the moment it was told to
    /// report. The baseline the forced report is read against.
    rested_before: usize,
    /// Where this step stands in the thrashing chain.
    chain: Chain,
    /// When the worktree was last read for the live file list, and what was
    /// last published from it.
    publishing: Publishing,
    /// What the worktree held when this step began.
    ///
    /// **The baseline `diff_nonempty` is decided against.** `WorkProduct` reads
    /// the branch — everything since the commit it was cut from — which is the
    /// right question for a Job and the wrong one for a step: every step after
    /// the first one that writes anything inherits its predecessor's files.
    /// Armada shipped that, and a step that wrote no code advanced on the scope
    /// note the step before it had committed.
    ///
    /// `None` where Fleet never saw the step start, which the gate reads as
    /// nothing known to have moved.
    ///
    /// **A redirect does not clear it.** `resumed` restarts the chain and the
    /// step's clock; the step itself is carrying on, and re-reading here would
    /// discard the work it had already done as though some other step had done
    /// it.
    ///
    /// It is not persisted. A Fleet that restarts mid-step reads a fresh
    /// baseline when it puts a Drone back on the worktree, so the step is then
    /// measured from where it was picked up rather than from where it began.
    /// That fails closed — work already done stops counting toward the step
    /// that did it — which is the direction an unknown baseline has to fail
    /// in.
    entered_with: Option<Footprint>,
}

impl Working {
    /// The taps are a constructor argument rather than something switched on
    /// later: a Job is worked with its transcript being written, or the
    /// dispatch that would have started it has already failed.
    pub(crate) fn holding<H>(
        job: JobId,
        drone: DroneId,
        step: StepId,
        worktree: Worktree,
        started: Started,
        harness: Arc<H>,
        taps: Taps,
        at: Timestamp,
    ) -> Working
    where
        H: AgentHarness + Send + Sync + 'static,
    {
        // Taken before `each`, which consumes the taps: the label is the one
        // part of them that outlives the reader task.
        let labelling = taps.label();
        Working {
            job,
            drone,
            step,
            labelling,
            worktree,
            session: started.session,
            transcript: Watching::reading(started.transcript, harness, taps.each()),
            _complaints: started.complaints,
            declared: None,
            drifted: Vec::new(),
            step_began: at,
            calls_before: 0,
            rested_before: 0,
            chain: Chain::Working,
            publishing: Publishing::default(),
            entered_with: None,
        }
    }

    /// Which Job, at which step, in which worktree. The three the gate needs,
    /// cloned together so no borrow of the slot outlives the read.
    pub(crate) fn standing(&self) -> (JobId, StepId, Worktree) {
        (self.job.clone(), self.step.clone(), self.worktree.clone())
    }

    pub(crate) fn is(&self, job: &JobId) -> bool {
        self.job == *job
    }

    /// Which Job and which Drone. The pair the exit event needs, cloned
    /// together so no borrow of the slot outlives the read.
    pub(crate) fn drone(&self) -> (JobId, DroneId) {
        (self.job.clone(), self.drone.clone())
    }

    /// Move to the next step, **and forget the last one's plan**. A
    /// declaration is about one step; carrying it forward would let step two's
    /// footprint be measured against step one's promise.
    ///
    /// **What is cleared here has to be asked for again**, by the caller that
    /// moved the step, in the turn the Drone gets for the new one — see
    /// [`crate::briefing::Declaring`]. A Drone whose plan is cleared and who is
    /// not asked has no reason to declare, and the gate then fails it for a
    /// call nobody made.
    ///
    /// **The transcript is told here and nowhere else.** Its sinks stamp each
    /// row with the step as they build it, so a row written after this call
    /// carries the new step and one already built carries the old — which is
    /// what the label is for, since a step can advance mid-turn.
    pub(crate) fn now_on(&mut self, step: StepId, at: Timestamp) {
        self.labelling.now_on(step.clone());
        self.step = step;
        self.declared = None;
        self.drifted.clear();
        // Cleared rather than replaced, because reading the worktree needs the
        // seam and this type holds none. The caller that moved the step reads
        // the new baseline and hands it back through `entering_with`; until it
        // does, the step has no baseline and the gate fails closed.
        self.entered_with = None;
        self.step_began = at;
        self.calls_before = self.transcript.progress().calls;
        self.chain = Chain::Working;
        // A new step is a new pen holder. Keeping the memo would let step two's
        // first reading match step one's last and publish nothing, leaving a
        // watcher reading a list attributed to the step before.
        self.publishing = Publishing::default();
    }

    /// The step this Drone is on has been resumed by a person.
    ///
    /// **The chain starts again and the declaration does not.** A redirect
    /// leaves the step and its plan exactly where they were — what is stale is
    /// how long the step has been running and the look it already spent, and a
    /// Drone that thrashed once and was steered can thrash again.
    ///
    /// **The step's baseline does not move either.** A redirect is the same
    /// step being done again, so what it entered with is still what it entered
    /// with — remeasuring here would hand the step whatever it had written
    /// before the person spoke, and it would then pass `diff_nonempty` on
    /// nothing.
    pub(crate) fn resumed(&mut self, at: Timestamp) {
        self.step_began = at;
        self.calls_before = self.transcript.progress().calls;
        self.chain = Chain::Working;
    }

    /// Record what the Drone declared for the step it is on. **Replaces**, so a
    /// plan that turned out wrong is corrected by declaring again rather than
    /// by widening one that already exists.
    pub(crate) fn declares(&mut self, paths: DeclaredPaths) {
        self.declared = Some(paths);
        self.drifted.clear();
    }

    pub(crate) fn declared(&self) -> Option<&DeclaredPaths> {
        self.declared.as_ref()
    }

    /// Record what the worktree held as this step began.
    pub(crate) fn entering_with(&mut self, footprint: Footprint) {
        self.entered_with = Some(footprint);
    }

    /// The baseline this step's work is measured against.
    pub(crate) fn entered_with(&self) -> Option<&Footprint> {
        self.entered_with.as_ref()
    }

    /// What has been seen outside the plan so far. The third tripwire, and the
    /// observation the mid-step look is given.
    pub(crate) fn off_plan(&self) -> &[RepoPath] {
        &self.drifted
    }

    /// The Drone's own tool calls since this step started.
    pub(crate) fn calls_this_step(&self) -> u32 {
        self.transcript
            .progress()
            .calls
            .saturating_sub(self.calls_before)
    }

    /// Whether the Drone has come to rest since it was told to report.
    ///
    /// **The fact that it happened, never what was said.** A terminating event
    /// is the Drone finishing a turn, which is what complying with a stop looks
    /// like from outside — and reading the words would be reading self-report.
    pub(crate) fn came_to_rest(&self) -> bool {
        self.transcript.progress().boundaries > self.rested_before
    }

    pub(crate) fn chain(&self) -> &Chain {
        &self.chain
    }

    /// The step's one look has been spent.
    pub(crate) fn looked(&mut self) {
        self.chain = Chain::Looked;
    }

    /// How many times the Drone has come to rest so far.
    ///
    /// Read **before** the directive goes down the pipe and handed back to
    /// [`reporting`](Working::reporting): the answer can arrive between the
    /// write and the next statement, and a baseline taken after it would count
    /// the reply as having been there all along.
    pub(crate) fn rested(&self) -> usize {
        self.transcript.progress().boundaries
    }

    /// The Drone has been told to report, from this instant.
    pub(crate) fn reporting(
        &mut self,
        asked_at: Timestamp,
        rested_before: usize,
        why: NotConverging,
    ) {
        self.rested_before = rested_before;
        self.chain = Chain::Reporting { asked_at, why };
    }

    /// The step stopped and the Job escalated.
    pub(crate) fn stopped(&mut self) {
        self.chain = Chain::Stopped;
    }

    /// How long the step has been running, by the instant handed in.
    pub(crate) fn running_for(&self, now: &Timestamp) -> Duration {
        elapsed(&self.step_began, now)
    }

    /// Add paths seen outside the plan, and answer with the ones that are new.
    /// Empty on every turn after the first that saw them, which is what keeps
    /// the live check from saying the same thing every tick.
    pub(crate) fn drifting(&mut self, seen: Vec<RepoPath>) -> Vec<RepoPath> {
        let fresh: Vec<RepoPath> = seen
            .into_iter()
            .filter(|path| !self.drifted.contains(path))
            .collect();
        self.drifted.extend(fresh.iter().cloned());
        fresh
    }

    /// When the worktree was last read for the live file list. **Mutable, and
    /// there is no read-only view of it**: every question anybody asks it is
    /// asked in the course of deciding whether to read again, which is a
    /// decision that records itself.
    pub(crate) fn publishing(&mut self) -> &mut Publishing {
        &mut self.publishing
    }

    /// Whether the Drone has exited, **and reap it if it has**. See
    /// [`DroneSession::exited`].
    pub(crate) async fn exited(&self) -> Result<bool, std::io::Error> {
        self.session.exited().await
    }

    pub(crate) fn session(&self) -> &DroneSession {
        &self.session
    }

    pub(crate) fn transcript_ended(&self) -> bool {
        self.transcript.transcript_ended()
    }

    /// Everything the Drone said. What `Ending::of` folds, and the only thing
    /// anybody asks a transcript.
    pub(crate) fn heard(&self) -> Vec<DroneEvent> {
        self.transcript.events()
    }
}
