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
use crate::drone::{Ending, Started};
use crate::footprint::Publishing;
use crate::questioning::Question;
use crate::session::{DroneSession, LiveSession, Occasion};
use crate::transcript::{Tap, Taps};
use crate::watch::Watching;
use store::DroneSpend;
use verification::NotConverging;

/// The Job being worked, and everything holding it up.
///
/// **One of several.** This said there was no second one, and that being held
/// in an `Option` was the whole of what "Fleet is busy" meant, until #50 gave
/// Fleet a slot per Job — [`Slots`](crate::slots::Slots) is the roster now, and
/// how many may run at once is a bound rather than the shape of a type. The
/// invariant that survives is narrower and belongs to the slot rather than to
/// the fleet: one Drone, on one step, in one worktree, and the four are put
/// together once by [`Working::holding`].
pub(crate) struct Working {
    job: JobId,
    /// The Drone in the slot. The same id its transcript is named by, and what
    /// `step`'s `assigned_drone` holds while this slot is full.
    drone: DroneId,
    /// Which step of the frozen workflow the Drone was told to do.
    ///
    /// **It never moves**, and there is no method here that would move it. A
    /// Drone belongs to a step, so a slot that outlived a step boundary would
    /// be a process working one step under a record naming another — which is
    /// what the second field beside this one used to hold apart. The boundary
    /// ends the Drone and builds a new slot instead; see
    /// [`stood_down`](Working::stood_down).
    step: StepId,
    worktree: Worktree,
    session: DroneSession,
    transcript: Watching,
    /// The same sinks the reader task fans a Drone's lines out to, held here so
    /// that what **Armada and Fleet** did reaches the record too.
    ///
    /// **A second handle rather than a second channel.** The rows belong in the
    /// transcript beside the turns they caused — an instruction and what the
    /// Drone did with it are one story — and a record of Fleet's own acts kept
    /// anywhere else would have to be merged back against this one by instant.
    ///
    /// It is a clone of the list the reader holds, not a share of it: nothing
    /// here is mutable, and both ends offer to the same `Arc`.
    taps: Vec<Arc<dyn Tap>>,
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
    /// When the Drone was last heard from, as the injected clock read it on the
    /// turn Fleet noticed. **Sampled per turn rather than stamped per event**:
    /// the transcript is read on a task of its own, which holds no clock, and
    /// nothing in this crate reads one outside `crate::clock`. A quarter-second
    /// loop against a threshold in minutes makes the sampling error nothing.
    heard_at: Timestamp,
    /// What [`Progress::heard`](crate::Progress::heard) read when `heard_at`
    /// was taken. The comparison that says whether anything has arrived since.
    heard: usize,
    /// What [`Progress::turned`](crate::Progress::turned) read when a person
    /// redirected this Drone on a Job **no step had stopped on** — the
    /// `stalled` shape, where the Job is `escalated` and the step is still
    /// running.
    ///
    /// **`Some` is a Job waiting for its Drone to prove it heard.** The
    /// redirect went down the pipe and the Job was left `escalated` on purpose:
    /// a Job that returned to `running` on the act of sending would read as
    /// recovered whether or not anything woke up, and the one case worth
    /// telling apart is a Drone that never does. `None` on every other Job and
    /// on every other redirect — where a step *had* stopped, the two machines
    /// move together and there is nothing outstanding.
    ///
    /// **The reading is taken before the write, never after.** The answer can
    /// arrive between the write and the next statement, and a baseline taken
    /// after it would have the answer already inside it and read as a Drone
    /// that never turned. That is [`rested`](Working::rested)'s hazard, and it
    /// costs the same care. The instant is kept beside the baseline because the
    /// wait is a fact a person is owed — `Fleet::redirect_awaited` serves it.
    answering: Option<Awaiting>,
    /// The question this Drone asked and nobody has answered yet.
    ///
    /// **`Some` is a Drone that is waiting rather than working**, and it is the
    /// only thing that tells the two apart: the process is alive, the Job is
    /// `running`, the step is `running`, and nothing is arriving. Both vigils
    /// read this and decline, exactly as they decline on evidence sitting at
    /// the gate — see `crate::questioning`.
    ///
    /// Held here and written to no column, for
    /// [`JudgeInFlight`](ipc::JudgeInFlight)'s reason: it is only ever true
    /// now. A Fleet that restarts loses the Drone that asked, and the Job it
    /// asked on is escalated as `interrupted`, so a stored question would
    /// outlive the only process that could act on the answer.
    ///
    /// **One at a time.** `Fleet::ask_question` refuses a second while one is
    /// held, because a Drone that could stack questions would be holding a
    /// conversation and a queue is a thing a person answers out of order.
    asked: Option<Question>,
    /// How many liveness pokes this step has spent.
    ///
    /// **The step's budget, not the episode's.** A Drone that answers a poke
    /// and then goes quiet again has spent one either way — resetting on an
    /// answer would let a Drone that says one word every two minutes and does
    /// nothing else stay under the counter for ever.
    pokes: u32,
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
    /// When Fleet started running this step's Checks for the Drone, where it
    /// is doing so now. **`Some` is the whole of what "the clocks are
    /// suspended" means** — see [`Working::quiet_for`] and
    /// [`Working::running_for`], which are the only two readers.
    ///
    /// It is also the refusal that stops two dry runs overlapping: a second
    /// `cargo build` in one worktree is two processes fighting over one target
    /// directory, and neither answer would be about the work.
    checking_since: Option<Timestamp>,
    /// How long this step has already spent inside Fleet's own Check runs.
    /// Subtracted from the wall clock, because a Drone waiting on Fleet is not
    /// a Drone failing to converge.
    checked_for: Duration,
    /// How many dry runs this step has asked for. **The step's budget**, like
    /// the pokes — and unlike the pokes it is not refunded by anything the
    /// Drone does, because what it bounds is money rather than patience.
    dry_runs: u32,
}

/// A redirect that has gone into the session and has not been answered: what
/// [`Progress::turned`](crate::Progress::turned) read the moment before it went
/// down the pipe, and when it did.
///
/// **One value because they are one act.** Held apart, a redirect could be
/// outstanding with no instant to serve, which is the reading
/// `JobDetail.redirecting` must not have.
struct Awaiting {
    turned: usize,
    sent_at: Timestamp,
}

/// A Drone that has been ended, and everything the slot that held it was
/// holding.
///
/// **The slot is gone by the time this exists**, which is what makes it a
/// value rather than three accessors: [`Working::stood_down`] consumes the
/// slot, so there is no arrangement of these fields that could be read off a
/// process still running.
pub(crate) struct StoodDown {
    pub(crate) job: JobId,
    /// The step the Drone was on, which is the step whose `assigned_drone`
    /// names it.
    pub(crate) step: StepId,
    pub(crate) drone: DroneId,
    /// The worktree the Drone was working in. **It outlives the process**:
    /// nothing in this workspace removes one, so the next step's Drone is put
    /// on this same directory and the branch is what carries the work across.
    pub(crate) worktree: Worktree,
    /// What the whole run folded to, read after the pipe closed. The last lines
    /// before an exit are in it because the drain waited for them.
    pub(crate) ending: Ending,
    /// What the run cost the Job, folded from the same drained stream. **Not
    /// part of [`Ending`]**: how a run finished and what it cost are different
    /// questions, and a Drone that vanished still spent whatever it spent.
    pub(crate) spent: DroneSpend,
    /// What signalling the Drone came to. **An error is not a failure to
    /// report**: it is a process already gone, or one the operating system
    /// would not signal, and neither is anything a caller can do more about.
    /// It is carried so the Job's log can say which.
    pub(crate) terminated: Result<(), std::io::Error>,
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
        let each = taps.each();
        Working {
            job,
            drone,
            step,
            worktree,
            session: started.session,
            transcript: Watching::reading(started.transcript, harness, each.clone()),
            taps: each,
            _complaints: started.complaints,
            declared: None,
            drifted: Vec::new(),
            step_began: at.clone(),
            calls_before: 0,
            rested_before: 0,
            chain: Chain::Working,
            heard_at: at,
            heard: 0,
            answering: None,
            asked: None,
            pokes: 0,
            publishing: Publishing::default(),
            entered_with: None,
            checking_since: None,
            checked_for: Duration::ZERO,
            dry_runs: 0,
        }
    }

    /// End the Drone in this slot, and read the rest of what it said.
    ///
    /// **It consumes the slot.** A `Working` whose process has been ended is a
    /// slot that lies about a Drone — it would still answer `session()`,
    /// `heard()` and `standing()` — so there is no version of this taking
    /// `&mut self`. What the caller needs afterwards is [`StoodDown`], and
    /// recording the exit is the caller's: this type reaches no store.
    ///
    /// The order of the three acts is `crate::boundary`'s subject and each of
    /// them answers a failure the one before it causes. In one line each:
    /// dropping a slot signals nothing and the child is `setsid`-detached;
    /// [`Watching`]'s `Drop` aborts the reader over whatever the pipe still
    /// held; and [`Ending::of`] over a stream still being read is a fold over
    /// a prefix, missing the terminating event at the end of it.
    pub(crate) async fn stood_down(mut self, at: &Timestamp) -> StoodDown {
        let terminated = self.session.terminate().await;
        self.transcript.drained().await;
        let events = self.transcript.events();
        let ending = Ending::of(&events);
        // **Folded after the drain, like the ending is**, and for the same
        // reason: what the Drone said on its way out is the last thing it said,
        // and a terminating line read before the pipe closed is a cost read off
        // a prefix. Recording it is the caller's, as recording the exit is.
        let spent = crate::allowance::spent(&events, elapsed(&self.step_began, at));
        StoodDown {
            job: self.job,
            step: self.step,
            drone: self.drone,
            worktree: self.worktree,
            ending,
            spent,
            terminated,
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

    /// Which Job, which step, and which Drone. The triple the exit event needs
    /// — a Drone belongs to a step, so the step is part of naming it — cloned
    /// together so no borrow of the slot outlives the read.
    ///
    /// **One step, not two.** The pointer an exit clears is on the step the
    /// Drone was put on, and that is the step it is still on: a slot does not
    /// outlive a boundary, so the step it was spawned on and the step it is
    /// working cannot come apart.
    pub(crate) fn drone(&self) -> (JobId, StepId, DroneId) {
        (self.job.clone(), self.step.clone(), self.drone.clone())
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
        self.step_began = at.clone();
        self.calls_before = self.transcript.progress().calls;
        self.chain = Chain::Working;
        // The pokes go back with the chain, and for the same reason: a person
        // has just spoken into the session, so what the Drone did before they
        // did is not what its answer to them is measured from.
        self.listening(at);
        // **The dry runs do not go back.** The pokes are patience and a person
        // has just spent some of theirs; a Check run is minutes of a machine,
        // and a redirect is not a refund. `checked_for` is cleared only because
        // `step_began` moved above, so the time it accounted for is already
        // outside the window.
        self.checked_for = Duration::ZERO;
    }

    /// Fleet has started running this step's Checks for the Drone, at this
    /// instant.
    ///
    /// **The clocks suspend from here.** The Drone is not working and not
    /// speaking, and both of those are things it would otherwise be counted
    /// for — `#58` suspends the silence clock while evidence sits at the gate
    /// for the same reason, and this is the same mechanism on a different
    /// trigger.
    pub(crate) fn checking(&mut self, at: Timestamp) {
        self.checking_since = Some(at);
        self.dry_runs += 1;
    }

    /// The run has finished, at this instant. **The clocks start again from
    /// here** rather than from where they were: the Drone has been waiting, and
    /// the silence it owes an answer for begins when it gets one.
    pub(crate) fn checked(&mut self, at: Timestamp) {
        if let Some(began) = self.checking_since.take() {
            self.checked_for += elapsed(&began, &at);
        }
        self.waiting(at);
    }

    /// Whether a dry run is in flight. **The refusal a second call gets**, and
    /// the switch [`quiet_for`](Working::quiet_for) reads.
    pub(crate) fn is_checking(&self) -> bool {
        self.checking_since.is_some()
    }

    /// How many dry runs this step has spent.
    pub(crate) fn dry_runs(&self) -> u32 {
        self.dry_runs
    }

    /// How long of the window ending at `now` was Fleet running Checks.
    fn suspended_for(&self, now: &Timestamp) -> Duration {
        match &self.checking_since {
            Some(began) => self.checked_for + elapsed(began, now),
            None => self.checked_for,
        }
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
    ///
    /// `in_plan` is the declared plan as the look that produced `why` found it.
    pub(crate) fn reporting(
        &mut self,
        asked_at: Timestamp,
        rested_before: usize,
        why: NotConverging,
        in_plan: Vec<RepoPath>,
    ) {
        self.rested_before = rested_before;
        self.chain = Chain::Reporting {
            asked_at,
            why,
            in_plan,
        };
    }

    /// The grace is spent and the Drone is still writing inside its plan, so it
    /// is given another one from this instant, measured against this reading.
    ///
    /// **It keeps the finding and the rest baseline.** Nothing about the look
    /// has changed — what changed is that the citation is being answered — and
    /// re-reading `rested_before` here would count the Drone as never having
    /// been asked.
    pub(crate) fn still_reporting(&mut self, asked_at: Timestamp, in_plan: Vec<RepoPath>) {
        if let Chain::Reporting { why, .. } = &self.chain {
            self.chain = Chain::Reporting {
                asked_at,
                why: why.clone(),
                in_plan,
            };
        }
    }

    /// The step stopped and the Job escalated.
    pub(crate) fn stopped(&mut self) {
        self.chain = Chain::Stopped;
    }

    /// How long the step has been running, by the instant handed in — **not
    /// counting the time Fleet spent running the step's Checks for it.**
    ///
    /// The wall-clock tripwire is a question about the Drone, and a Drone
    /// blocked on a tool call Fleet is servicing is not doing anything the
    /// tripwire is looking for. Without the subtraction, a `cargo build` this
    /// capability exists to offer would push an honest step over a ceiling set
    /// against steps that could not ask for one.
    pub(crate) fn running_for(&self, now: &Timestamp) -> Duration {
        elapsed(&self.step_began, now).saturating_sub(self.suspended_for(now))
    }

    /// How long the Drone has said nothing, by the instant handed in.
    ///
    /// **It samples, which is why it is `&mut`.** The reading it takes is what
    /// the next one is compared against, so asking the question is what keeps
    /// the answer true — there is no separate writer to forget to call, and no
    /// way to read this without the reading being recorded.
    ///
    /// Zero on the turn anything arrived, of any kind. See
    /// [`Progress::heard`](crate::Progress::heard) for why it is every kind.
    ///
    /// **Zero, too, while Fleet is running the step's Checks for it.** A Drone
    /// inside a tool call Fleet has not answered yet is quiet the way a Drone
    /// whose evidence is at the gate is quiet — waiting on Fleet, and unable to
    /// say anything until Fleet is done. The reading is still taken so that
    /// what arrives during the run is not counted as silence afterwards.
    pub(crate) fn quiet_for(&mut self, now: &Timestamp) -> Duration {
        if self.is_checking() {
            self.waiting(now.clone());
            return Duration::ZERO;
        }
        let heard = self.transcript.progress().heard;
        if heard > self.heard {
            self.heard = heard;
            self.heard_at = now.clone();
        }
        elapsed(&self.heard_at, now)
    }

    /// Start the silence clock again, and give the step its pokes back.
    ///
    /// **A new step, or the same step handed back by a person.** Both are
    /// moments the Drone has just been given something to do, and neither owes
    /// an answer for the time before it.
    fn listening(&mut self, at: Timestamp) {
        self.waiting(at);
        self.pokes = 0;
    }

    /// Start the silence clock again **without** returning a poke.
    ///
    /// For the turns where the Drone owes nothing: its evidence is at the gate,
    /// or the Job is not `running` and the liveness clock is suspended by the
    /// registry's own rule. Quiet is what a Drone waiting on Fleet or on a
    /// person looks like, and charging it for that is how a tripwire learns to
    /// fire on the honest case.
    pub(crate) fn waiting(&mut self, at: Timestamp) {
        self.heard = self.transcript.progress().heard;
        self.heard_at = at;
    }

    /// The question this Drone is waiting on, where there is one.
    pub(crate) fn asked(&self) -> Option<&Question> {
        self.asked.as_ref()
    }

    /// Hold the question the Drone just asked.
    ///
    /// **Nothing here checks that none is held.** `Fleet::ask_question` does,
    /// under this slot's lock and before it mints an id, because the refusal it
    /// answers with names the question already outstanding — which is a value
    /// this method has no way to return.
    pub(crate) fn asks(&mut self, asked: Question) {
        self.asked = Some(asked);
    }

    /// The question has been answered and the answer is down the pipe.
    ///
    /// **After the write, never before.** A session that would not take the
    /// answer leaves the question standing, so a person can answer again rather
    /// than being told there is nothing to answer on a Drone that never heard.
    pub(crate) fn answered_question(&mut self) {
        self.asked = None;
    }

    /// How many pokes this step has spent.
    pub(crate) fn pokes(&self) -> u32 {
        self.pokes
    }

    /// The Drone has been poked, at this instant. **The clock restarts**, so
    /// the next poke — or the escalation — is a fresh silence rather than the
    /// same one read twice.
    pub(crate) fn poked(&mut self, at: Timestamp) {
        self.waiting(at);
        self.pokes += 1;
    }

    /// What [`Progress::turned`](crate::Progress::turned) reads now. The
    /// baseline a redirect is held against, and it is asked for **before** the
    /// instruction goes down the pipe.
    pub(crate) fn turned(&self) -> usize {
        self.transcript.progress().turned
    }

    /// A person's redirect has gone into the session and the Job is held at
    /// `escalated` until the Drone answers it.
    ///
    /// Both are handed in rather than read here, because the write has already
    /// happened by the time anything can call this — see
    /// [`answering`](Working::answering). The instant is when the instruction
    /// went into the session and not when the Drone read it: nothing on this
    /// side of the pipe can say the second.
    ///
    /// **A second redirect replaces the first.** Nothing bounds how often a Job
    /// may be redirected, and a baseline kept from the earlier one would let a
    /// turn taken before the new instruction answer it.
    pub(crate) fn awaiting_answer(&mut self, turned: usize, at: Timestamp) {
        self.answering = Some(Awaiting {
            turned,
            sent_at: at,
        });
    }

    /// When the outstanding redirect went into the session, where one is.
    /// **`None` is a Job with nothing outstanding** — the same reading
    /// [`turned_since_redirect`](Working::turned_since_redirect) takes, asked by
    /// a reader rather than by the vigil.
    pub(crate) fn awaiting_since(&self) -> Option<&Timestamp> {
        self.answering.as_ref().map(|awaiting| &awaiting.sent_at)
    }

    /// Whether the Drone has taken a turn since a redirect was put to it.
    ///
    /// **`false` where no redirect is outstanding**, which is every Job but the
    /// one a person has just spoken to — so the question costs a lock and a
    /// comparison and reaches no store.
    ///
    /// **`turned` and not `heard`.** A `tool_progress` heartbeat is a Drone
    /// that never stopped working rather than one that read what a person said,
    /// and counting it would move the Job back to `running` on a Drone that is
    /// wedged inside the same call it was wedged in when the vigil caught it.
    pub(crate) fn turned_since_redirect(&self) -> bool {
        self.answering
            .as_ref()
            .is_some_and(|awaiting| self.transcript.progress().turned > awaiting.turned)
    }

    /// The outstanding redirect has been answered, and nothing is waiting on
    /// this Drone.
    pub(crate) fn answered(&mut self) {
        self.answering = None;
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

    /// Write down something Armada or Fleet did, into this step's own record.
    ///
    /// **Never awaits and never fails**, for the reason
    /// [`Tap`](crate::transcript::Tap) says: this is called from the loop that
    /// advances the Job, and a record that could hold it up would make watching
    /// a Job change its outcome. A row the sinks will not take is counted as
    /// missed exactly as a Drone's is.
    ///
    /// **It is not a send.** Nothing here reaches the Drone; the caller has
    /// already spoken to the session, or has decided not to, and this says what
    /// happened. Pairing the two in one method was rejected on the failure it
    /// hides — a turn that did not go down the pipe still belongs in the record,
    /// and `crate::silence` counts a poke that failed to write as spent.
    pub(crate) fn told(&self, by: ipc::Voice, saw: ipc::Saw) {
        for tap in &self.taps {
            tap.noted(by, saw.clone());
        }
    }

    /// Write down a turn Armada put into this session, whole.
    ///
    /// The one caller shape: every send site has the rendered text in hand and
    /// drops it, which is why the brief a step opened with was recoverable from
    /// nowhere once the process had gone.
    pub(crate) fn instructed(&self, occasion: Occasion, text: &str) {
        self.told(
            ipc::Voice::Armada,
            ipc::Saw::Instructed {
                occasion: occasion.as_wire().to_string(),
                text: text.to_string(),
            },
        );
    }

    pub(crate) fn transcript_ended(&self) -> bool {
        self.transcript.transcript_ended()
    }

    /// Everything the Drone said. What `Ending::of` folds, and the only thing
    /// anybody asks a transcript.
    pub(crate) fn heard(&self) -> Vec<DroneEvent> {
        self.transcript.events()
    }

    /// What this Drone's run has cost the Job so far.
    ///
    /// **A second fold over the same events `Ending::of` reads**, and not a
    /// field on `Ending`: what the run cost and how it finished are different
    /// questions, and a `Vanished` Drone still spent whatever it spent before
    /// the stream stopped. The fold itself is `crate::allowance::spent`, which
    /// is where the reason cost and turns fold differently is written down.
    ///
    /// The wall clock is measured from `step_began`, which is when this slot
    /// was opened — a `Working` is built once per spawn, so that is the Drone's
    /// own start and not the step's across a restart.
    pub(crate) fn spent(&self, now: &Timestamp) -> DroneSpend {
        crate::allowance::spent(&self.heard(), elapsed(&self.step_began, now))
    }
}
