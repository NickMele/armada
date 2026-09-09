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
//! # What is here, and what is in the modules beside it
//!
//! Here: the invariant and the lifetime. Beside it: the state another module
//! keeps in the slot because it can only be read under the same lock, one
//! module per reader, each of them stating its own half.
mod answering;
mod converging;
mod dry_run;
mod saying;
mod scope;
mod silence;

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use std::time::Duration;

use adapter_traits::{AgentHarness, Footprint, Worktree};
use core_model::{DeclaredPaths, DroneId, JobId, RepoPath, StepId, Timestamp};
use tokio::process::ChildStderr;

use crate::adopting::{Adopted, Session};
use crate::converging::{elapsed, Chain};
use crate::drone::{Ending, Started};
use crate::footprint::Publishing;
use crate::questioning::Question;
use crate::session::LiveSession;
use crate::silence::Liveness;
use crate::transcript::{Tap, Taps};
use crate::watch::{Drained, Watching};
use crate::working::answering::Awaiting;
use store::DroneSpend;

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
    /// The Drone, and which kind it is. **A Fleet that restarted holds
    /// [`Session::Adopted`]** — the same slot, the same acts offered, and every
    /// one that speaks refused because there is no pipe to speak into. See
    /// `crate::adopting`.
    session: Session,
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
    ///
    /// **`None` on an adopted Drone**, whose stderr went with the Fleet that
    /// held it. Nothing here can reopen it, and the Drone has been writing into
    /// a closed pipe since that moment — which is one of the reasons an orphan
    /// is unlikely to survive long, and is not a reason to pretend Fleet still
    /// has the far end.
    _complaints: Option<ChildStderr>,
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
    /// The pair that says whether the Drone was handed the directive, and when.
    /// Zero and `None` on a step nobody has told to report — and `None` for as
    /// long as the Drone is inside a call. See `crate::working::converging`.
    handed_before: usize,
    handed_at: Option<Timestamp>,
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
    /// What [`Progress::boundaries`](crate::Progress::boundaries) read when
    /// Armada last put a turn into this session.
    ///
    /// **The baseline that tells a Drone at rest from a Drone that owes an
    /// answer.** A terminating event is a turn boundary and not a lifetime —
    /// `Ending::Reported` says so — because Armada injects turns and the same
    /// process runs again. Which of the two a boundary is depends entirely on
    /// whether anything is outstanding for it, and that is not a question the
    /// transcript can answer on its own.
    ///
    /// **Written before the send at every site**, which is what makes it
    /// exact rather than probable: [`Working::instructed`] is called by every
    /// caller that speaks into a session, and every one of them calls it
    /// before it writes. A baseline taken afterwards would have the answer
    /// inside it — [`rested`](Working::rested)'s hazard, and it costs the same
    /// care.
    ///
    /// An atomic because `instructed` is `&self`, like the [`told`] it is
    /// built on: a slot is read through a shared reference while a turn goes
    /// down the pipe, and none of the six senders holds it mutably.
    ///
    /// [`told`]: Working::told
    told_after: AtomicUsize,
    /// How many liveness pokes this step has spent.
    ///
    /// **The step's budget, not the episode's.** A Drone that answers a poke
    /// and then goes quiet again has spent one either way — resetting on an
    /// answer would let a Drone that says one word every two minutes and does
    /// nothing else stay under the counter for ever.
    pokes: u32,
    /// How long this Drone may say nothing, and how many nudges it gets:
    /// **this step's, resolved when the slot was made.**
    ///
    /// A field rather than a reading, because `Fleet::watch_silence` compares
    /// against the threshold before it touches the store — a per-step value
    /// fetched off the record would put a store read on every turn of every
    /// healthy Drone, which is the reading the whole vigil is arranged to
    /// avoid. [`Liveness::at`] is the resolution and it runs once, here, over
    /// the same frozen step this slot was spawned against.
    ///
    /// **It cannot go stale, because a slot does not outlive a step.** The
    /// boundary ends the Drone and builds a new slot, which resolves again —
    /// so a Job of four steps holds four of these in turn, and never a sum of
    /// them.
    liveness: Liveness,
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
    ///
    /// **Read [`drained`](StoodDown::drained) beside it.** Over a stream cut
    /// short this is a fold over a prefix, and the fold has no way to say so.
    pub(crate) ending: Ending,
    /// Whether the drain reached the end of the pipe or gave up on it.
    ///
    /// **Beside the ending rather than inside it**, for the reason `Ending` has
    /// no `Succeeded`: how a run finished is what the Drone said, and this is
    /// how much of what it said Fleet managed to hear.
    pub(crate) drained: Drained,
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
        // This step's, already resolved — see the field. A parameter and not a
        // builder, for the reason `Liveness` has no `Default`: there is no
        // value to hold until somebody remembers to set one, and a slot
        // holding the wrong patience is a Job that escalates for no reason a
        // person can see.
        liveness: Liveness,
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
            session: Session::Spawned(started.session),
            transcript: Watching::reading(started.transcript, harness, each.clone()),
            taps: each,
            _complaints: Some(started.complaints),
            declared: None,
            drifted: Vec::new(),
            step_began: at.clone(),
            calls_before: 0,
            rested_before: 0,
            handed_before: 0,
            handed_at: None,
            chain: Chain::Working,
            heard_at: at,
            heard: 0,
            answering: None,
            asked: None,
            told_after: AtomicUsize::new(0),
            pokes: 0,
            liveness,
            publishing: Publishing::default(),
            entered_with: None,
            checking_since: None,
            checked_for: Duration::ZERO,
            dry_runs: 0,
        }
    }

    /// Take a slot back over a Drone this Fleet did not spawn.
    ///
    /// **The same invariant [`holding`](Working::holding) keeps, assembled from
    /// what a restart still has.** Which Job, which step, which worktree and
    /// which process all come off the record; what is missing is the two pipes,
    /// and the two fields that held them say so rather than being faked —
    /// [`Watching::unheard`] for the transcript, `None` for the complaints.
    ///
    /// **The taps are a fresh handle onto the same file.** A transcript is
    /// named by the `drone_id`, which has not changed, so Fleet appends to what
    /// the Drone had already written instead of opening a second file that
    /// splits one run in two. `Fleet::adopted` writes the gap row through them
    /// before anything else.
    ///
    /// **Every counter starts at zero and that is a decision, not an
    /// oversight.** `calls_before`, `rested_before` and `pokes` are readings of
    /// what Fleet observed, and Fleet observed none of the gap — carrying a
    /// count forward would measure this step against a norm using a number
    /// nobody took. What is *not* zeroed is the spend, which is on the record
    /// rather than in the slot: money the Drone spent is a fact about the world
    /// and survives the restart in `job_drone_spend`. It is an undercount by
    /// whatever the gap cost, because the harness reports a run's cost on the
    /// terminating line that went into the dead pipe.
    pub(crate) fn adopting(
        adopted: Adopted,
        worktree: Worktree,
        taps: Taps,
        liveness: Liveness,
        at: Timestamp,
    ) -> Working {
        let each = taps.each();
        Working {
            job: adopted.job().clone(),
            drone: adopted.drone().clone(),
            step: adopted.step().clone(),
            worktree,
            session: Session::Adopted(adopted),
            transcript: Watching::unheard(),
            taps: each,
            _complaints: None,
            declared: None,
            drifted: Vec::new(),
            step_began: at.clone(),
            calls_before: 0,
            rested_before: 0,
            handed_before: 0,
            handed_at: None,
            chain: Chain::Working,
            heard_at: at,
            heard: 0,
            answering: None,
            asked: None,
            told_after: AtomicUsize::new(0),
            pokes: 0,
            liveness,
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
    /// this type reaches no store: recording the exit and the spend is its one
    /// caller's, `Fleet::stood_down_paying`, for `#398`'s reason.
    ///
    /// The order of the three acts is `crate::boundary`'s subject and each of
    /// them answers a failure the one before it causes. In one line each:
    /// dropping a slot signals nothing and the child is `setsid`-detached;
    /// [`Watching`]'s `Drop` aborts the reader over whatever the pipe still
    /// held; and [`Ending::of`] over a stream still being read is a fold over
    /// a prefix, missing the terminating event at the end of it.
    ///
    /// **The drain is bounded and its answer is carried out.** Signalling the
    /// Drone does not close its stdout where a tool it spawned inherited the
    /// same write end, so the pipe outlives the process and this is on the turn
    /// loop's own task — see [`Watching::drained`].
    pub(crate) async fn stood_down(mut self, at: &Timestamp) -> StoodDown {
        let terminated = self.session.terminate().await;
        let drained = self.transcript.drained().await;
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
            drained,
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

    /// Whether the Drone has exited, **and reap it if it has** — which now
    /// signals the Drone's process group before collecting the child, so a
    /// tool it left running goes with it. See [`DroneSession::exited`].
    pub(crate) async fn exited(&self) -> Result<bool, std::io::Error> {
        self.session.exited().await
    }

    pub(crate) fn session(&self) -> &Session {
        &self.session
    }
}
