//! Reading a Drone's transcript while it runs, so its ending means something.
//!
//! # Why the stream is read at all, given the gate does not read it
//!
//! Nothing a Drone says gates its own step. But `crate::aftermath` needs an
//! [`Ending`](crate::Ending) — a fold over what the Drone emitted: whether it
//! called anything, and how often it was refused. Those three answers,
//! `blocked_by_policy`, `silent` and `stalled`, are three different things for
//! a person to do, and a Fleet that never read the stream could only ever say
//! `interrupted`. An `Unreadable` line is kept rather than filtered: a run full
//! of them is not a silent run.
//!
//! # EOF is the signal, not a poll
//!
//! The task ends when the pipe closes, which the operating system delivers
//! rather than Fleet asking on a timer. Whether the *process* is also gone is
//! `crate::holder_of`'s question: a terminating event is a turn boundary, not
//! a lifetime.
//!
//! # One read, then a fan-out
//!
//! Everything else that wants the transcript is fed from here rather than from
//! the pipe, and **the parser is served first on every line**. A `Tap` gets
//! what the harness already decoded, never the line and never a decoder of its
//! own. `docs/concepts/observe.md` is the design.
use core::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, DroneEvent, Speaker};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::task::JoinHandle;

use crate::transcript::Tap;

/// How long [`Watching::drained`] waits for the pipe to close.
///
/// **It is not a guess at how long reading takes.** The drain is called after
/// the Drone has been signalled and waited on, so the process that was writing
/// is gone and what is left is whatever the kernel is holding — at most one
/// pipe buffer, read at memory speed. On every healthy boundary this returns in
/// under a millisecond and the bound is never reached.
///
/// **What the bound is for is a writer that is not gone**, which is the whole
/// of `#211`: a tool the Drone spawned inherited the same write end, so the
/// pipe stays open for as long as that tool runs and the drain would otherwise
/// wait on a process nothing observed. Two seconds is three orders of magnitude
/// above the healthy case and is paid once per step boundary by the turn loop,
/// which is one task walking every Job — so every millisecond past it is the
/// whole fleet standing still.
const DRAIN: Duration = Duration::from_secs(2);

/// What a Drone has said so far, and whether it has stopped saying anything.
///
/// **There is no method that returns one event.** The only question anybody
/// asks of a transcript is what the whole run folds to, so the whole run is
/// what comes back — a per-event accessor would be an invitation to read a
/// Drone's claim, which is the thing the gate exists to refuse.
#[derive(Debug, Default)]
struct Heard {
    events: Vec<DroneEvent>,
    ended: bool,
}

/// How far a run has got, in the only four numbers anything asks for.
///
/// **None of them is a verdict and none can become one.** `calls` is Fleet's
/// own count of what the Drone did and is compared against a step norm;
/// `boundaries` is how many times the Drone came to rest, which is what says a
/// directive was acted on rather than what it said in answer; `heard` is how
/// many events arrived at all, which is what says the Drone is still there; and
/// `turned` is how many of those the Drone produced itself, which is what says
/// it acted on something a person told it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// How many tool calls the Drone has made this session.
    ///
    /// **Cumulative and live.** A `Called` arrives as the Drone makes it, so a
    /// step's own count is this minus its reading when the step began — and
    /// that reading is true at the instant it is taken, which is the whole
    /// reason the count is here rather than on the harness's own number.
    ///
    /// **Not the harness's `turns`.** That arrives only on `Ended`, and the
    /// harness ends an invocation when a step ends, so at a step boundary the
    /// finishing step's count has not been reported yet and the baseline reads
    /// its predecessor's. It also resets per invocation rather than
    /// accumulating, so subtracting a baseline from it underflows. Both halves
    /// of that were load-bearing: a step was told to stop and report
    /// twenty-three seconds in, charged with the 69 turns of the step before
    /// it.
    ///
    /// Calls stand in for turns because they track them. Measured over the 32
    /// invocations this repository has recorded, `calls / turns` runs 0.92–0.99
    /// for any step past fifteen turns, which is the only range a norm of sixty
    /// can care about.
    pub calls: u32,
    /// How many terminating events have arrived.
    pub boundaries: usize,
    /// How many events have arrived, **of every kind**, including the ones this
    /// vocabulary has no name for and the lines that did not decode.
    ///
    /// **The liveness reading, and it has to count everything.** `calls` is the
    /// wrong number for it twice over: a Drone stuck inside one long command
    /// makes no further calls and is not silent, and a Drone that has stopped
    /// makes none either — which is the failure mode
    /// `docs/spikes/009-how-long-does-a-step-take.md` measured, where a frozen
    /// call counter read as a step that was merely early.
    ///
    /// A Drone inside a long command is not quiet: the harness emits a
    /// `tool_progress` heartbeat every thirty seconds while a tool runs, which
    /// this vocabulary has no variant for and therefore counts as
    /// `Unrecognised`. Counting only what Armada names would have thrown away
    /// the one signal that separates a Drone running `cargo test` from a Drone
    /// that has stopped running anything.
    pub heard: usize,
    /// How many events the **Drone itself** produced — a tool it reached for, a
    /// refusal it walked into, prose it wrote, a turn it finished.
    ///
    /// **The narrower reading, and the one that says a redirect was taken up.**
    /// [`heard`](Progress::heard) has to count everything, because a Drone that
    /// is merely working through a long command is not silent and its heartbeat
    /// is the only proof — but a heartbeat is a Drone that never stopped, not a
    /// Drone that read what a person said and started again. So the two
    /// questions take two counters, and the wider one cannot answer the
    /// narrower.
    ///
    /// **Four kinds, and the four that are absent are absent on purpose.**
    /// `Started` is the harness opening a session, `Answered` is a tool
    /// replying rather than the Drone acting, `QuotaMoved` is the harness's
    /// window, and `Unrecognised` is where that heartbeat lands. `Unreadable`
    /// is a line nobody could attribute. None of them is the Drone taking a
    /// turn.
    pub turned: usize,
}

/// How much of the transcript the drain got.
///
/// **Returned rather than discarded, because the two are different records.**
/// A run read to end-of-file and a run whose tail was cut short both fold
/// through [`crate::Ending`], and the fold cannot tell them apart: a
/// terminating event that never arrived and one that was still in the pipe are
/// both `Vanished`, which `crate::aftermath` reads as `interrupted` and
/// escalates a Job for. Replacing a hang with a truncation is only an
/// improvement if the truncation says so, so this is what the caller writes
/// down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drained {
    /// The pipe closed. Everything the Drone wrote has been read.
    ToTheEnd,
    /// The wait was cut short. **Something other than the Drone still holds
    /// the write end** — a tool that inherited it and outlived the kill — so
    /// the transcript may be missing whatever was written after the last line
    /// read, including the terminating event.
    CutShort {
        /// How long was waited before giving up, so the record names the
        /// number rather than a reader having to find the constant.
        waited: Duration,
    },
}

/// A transcript being read.
pub struct Watching {
    heard: Arc<Mutex<Heard>>,
    reader: JoinHandle<()>,
}

impl fmt::Debug for Watching {
    /// By hand, because a `Tap` is a trait object and a `Recording` holds a
    /// clock. What a reader wants from this is how much has been heard.
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let heard = self.heard.lock().expect("not held across a panic");
        out.debug_struct("Watching")
            .field("events", &heard.events.len())
            .field("ended", &heard.ended)
            .finish()
    }
}

impl Watching {
    /// Start reading, on a task of its own, and fan every line out.
    ///
    /// The harness is shared rather than borrowed because the task outlives the
    /// call: decoding a line is the harness's, and the harness is the only
    /// thing that knows what a line means.
    ///
    /// **The parser takes the line before any tap sees it**, and the lock is
    /// released before the taps are offered anything — so a tap cannot hold the
    /// lock that `Ending::of` reads through.
    pub fn reading<H>(transcript: ChildStdout, harness: Arc<H>, taps: Vec<Arc<dyn Tap>>) -> Watching
    where
        H: AgentHarness + Send + Sync + 'static,
    {
        let heard = Arc::new(Mutex::new(Heard::default()));
        let filling = Arc::clone(&heard);
        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(transcript).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let read = harness.read(&line);
                filling
                    .lock()
                    .expect("the transcript is not held across a panic")
                    .events
                    .extend(read.iter().cloned());
                for tap in &taps {
                    tap.saw(&read);
                }
            }
            filling
                .lock()
                .expect("the transcript is not held across a panic")
                .ended = true;
        });
        Watching { heard, reader }
    }

    /// Whether the pipe has closed. **Not whether the process is gone** — see
    /// this module's comment, and `Ending::Reported`'s.
    pub fn transcript_ended(&self) -> bool {
        self.heard
            .lock()
            .expect("the transcript is not held across a panic")
            .ended
    }

    /// How far the run has got, as four counts.
    ///
    /// **Still not a per-event accessor.** It folds under the lock and answers
    /// numbers, so there is no way to reach a Drone's claim through it — the
    /// same property [`events`](Watching::events) has by returning everything.
    pub fn progress(&self) -> Progress {
        let heard = self
            .heard
            .lock()
            .expect("the transcript is not held across a panic");
        heard
            .events
            .iter()
            .fold(Progress::default(), |mut so_far, event| {
                // A catch-all is right here and wrong in `transcript::row`.
                // That one maps every kind onto the wire and must fail to
                // compile when a kind is added; this one counts two of them,
                // and a new kind is not a third thing to count.
                match event {
                    DroneEvent::Called { .. } => {
                        so_far.calls += 1;
                        so_far.turned += 1;
                    }
                    DroneEvent::Ended { .. } => {
                        so_far.boundaries += 1;
                        so_far.turned += 1;
                    }
                    // **The Drone's own prose only.** A turn Armada put into
                    // the session comes back on the same stream, and counting
                    // it made `Working::turned_since_redirect` true the moment
                    // the harness echoed the redirect — before the Drone had
                    // read it, on a baseline taken before it was sent.
                    DroneEvent::Said {
                        by: Speaker::Drone, ..
                    }
                    | DroneEvent::Refused { .. } => so_far.turned += 1,
                    _ => {}
                }
                // Outside the match, deliberately: every event is one, and an
                // arm that had to be added for a new kind would be an arm
                // somebody could forget — which would read as a Drone falling
                // silent the moment the harness grew an event.
                so_far.heard += 1;
                so_far
            })
    }

    /// Read to the end of the pipe, bounded, and only then let the reader go.
    ///
    /// **The half of a step boundary that [`Drop`] cannot do.** Dropping a
    /// `Watching` aborts the reader where it stands, so whatever the operating
    /// system had already buffered on the Drone's stdout is thrown away — and
    /// what is buffered at an exit is the last thing the Drone said, which is
    /// the interesting part. Every sink is fed from that task, so an aborted
    /// reader is also a transcript file missing its own final rows.
    ///
    /// **Call it after the process is gone, never before.** This returns when
    /// the pipe closes, and a pipe closes when the *last* writer to it does —
    /// a live Drone is a writer, so awaiting this over one waits out the run.
    ///
    /// **And the last writer need not be the Drone**, which is `#211`: a tool
    /// it spawned inherited the same write end and is not something Fleet ever
    /// observed, so an unbounded wait here is the turn loop stopped on a
    /// process with no timeout, no escalation and no status. Hence the bound,
    /// and hence [`Drained`] rather than nothing — an ending cut short must not
    /// read as a Drone that said nothing, and the caller writes down which.
    ///
    /// Awaited by `&mut` rather than by value so that the events stay readable
    /// afterwards: what the drain was for is the fold that comes next.
    pub async fn drained(&mut self) -> Drained {
        // The task holds a pipe and a lock it never carries across an await, so
        // the inner `Err` is a panic inside it — in which case whatever had
        // been read is already in `heard` and there is nothing further to wait
        // for. Either way the pipe reached its end.
        match tokio::time::timeout(DRAIN, &mut self.reader).await {
            Ok(_) => Drained::ToTheEnd,
            // The reader is left running rather than aborted here. `Drop` is
            // what stops it, and dropping it closes Fleet's read end — so the
            // tool still writing gets `EPIPE` on its next write instead of a
            // pipe that fills and blocks it forever.
            Err(_) => Drained::CutShort { waited: DRAIN },
        }
    }

    /// Every event so far, in the order the Drone emitted them. What
    /// `Ending::of` folds.
    pub fn events(&self) -> Vec<DroneEvent> {
        self.heard
            .lock()
            .expect("the transcript is not held across a panic")
            .events
            .clone()
    }
}

impl Drop for Watching {
    /// Stop reading when the Job that was being read is over.
    ///
    /// **The backstop, and not the ordinary path.** It used to be the whole of
    /// how a reader ended, on the argument that whatever had been read was
    /// already in `heard` — which is true of what had been *read* and says
    /// nothing about what was sitting in the pipe unread. A Drone's last lines
    /// arrive in the moments around its exit, so aborting there loses precisely
    /// the ones a person opens the transcript for.
    ///
    /// [`drained`](Watching::drained) is what a deliberate ending calls first,
    /// and this is then a no-op over a task that has already finished. What is
    /// left for this to catch is a slot dropped without one — a Fleet shutting
    /// down, or a test walking away.
    fn drop(&mut self) {
        self.reader.abort();
    }
}
