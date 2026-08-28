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

use adapter_traits::{AgentHarness, DroneEvent};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::task::JoinHandle;

use crate::transcript::Tap;

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

/// Where in a Drone's stream a step began.
///
/// **Opaque, and there is no arithmetic on it.** A step's own turn count is
/// asked of the transcript through [`Watching::turns_since`] rather than
/// subtracted from a running total by the caller — because only the stream can
/// say which invocation a turn count belongs to, and the caller that subtracted
/// is the one that got it wrong. There is no cumulative total to subtract from
/// anywhere in this file, which is what makes that mistake unavailable rather
/// than discouraged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepMark {
    /// How many events had been read when the step began. `0` is the start of
    /// the stream, which is where a Drone's first step begins.
    seen: usize,
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

    /// Where the stream stands now, so a step can be measured from here.
    ///
    /// Taken by the caller at the instant the step began. It is a position and
    /// not a count, so it cannot go stale: the events before it stay before it
    /// however long the step runs.
    pub fn mark(&self) -> StepMark {
        StepMark {
            seen: self
                .heard
                .lock()
                .expect("the transcript is not held across a panic")
                .events
                .len(),
        }
    }

    /// How many times the Drone has come to rest. **Not a verdict** — it says a
    /// directive was acted on, never what was said in answer.
    ///
    /// **Still not a per-event accessor.** It folds under the lock and answers
    /// a number, so there is no way to reach a Drone's claim through it — the
    /// same property [`events`](Watching::events) has by returning everything.
    pub fn boundaries(&self) -> usize {
        self.heard
            .lock()
            .expect("the transcript is not held across a panic")
            .events
            .iter()
            .filter(|event| matches!(event, DroneEvent::Ended { .. }))
            .count()
    }

    /// The Drone's own turns since a mark, over the invocations that began
    /// after it.
    ///
    /// **An `Ended` before the first `Started` past the mark is not counted.**
    /// The harness reports a turn count only when an invocation finishes, and
    /// the one a step submits from finishes *after* Fleet has advanced — so the
    /// first count past a boundary is the previous step's, arriving late. A
    /// baseline taken at the boundary and subtracted from it lands that whole
    /// count on the new step, and did: a step was poked for thrashing
    /// twenty-three seconds in having spent no turns of its own.
    /// [`DroneEvent::Started`] is the stream saying a new invocation began —
    /// spike 4's second `init`, and every capture under `testkit` has one.
    ///
    /// **The counts are summed, because `turns` is per invocation.**
    /// `clarification-exhausted.ndjson` reads 5, 2, 3, 2 across one session
    /// rather than 5, 7, 10, 12, and a step that spans several has spent all.
    ///
    /// **It reads `0` while a step's first invocation is still running**, which
    /// is the harness not having said rather than a low number — and quiet is
    /// the safe direction for a tripwire whose next stage spends a call.
    pub fn turns_since(&self, mark: StepMark) -> u32 {
        let heard = self
            .heard
            .lock()
            .expect("the transcript is not held across a panic");
        turns_over(&heard.events, mark)
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

/// The fold itself, on a slice, so it can be exercised against a sequence
/// rather than against a pipe whose timing a test cannot hold still.
pub(crate) fn turns_over(events: &[DroneEvent], mark: StepMark) -> u32 {
    let mut begun = false;
    let mut taken = 0;
    for event in events.iter().skip(mark.seen) {
        match event {
            DroneEvent::Started { .. } => begun = true,
            DroneEvent::Ended { turns, .. } if begun => taken += turns,
            _ => {}
        }
    }
    taken
}

impl StepMark {
    /// A mark after this many events. **Test-only**: the running system takes
    /// one from [`Watching::mark`] and never counts to it.
    #[cfg(test)]
    pub(crate) fn after(seen: usize) -> StepMark {
        StepMark { seen }
    }
}

impl Drop for Watching {
    /// Stop reading when the Job that was being read is over.
    ///
    /// The task holds a pipe and nothing else, so aborting it loses nothing:
    /// whatever had been read is already in `heard`, and a Job whose Drone has
    /// been terminated has no further lines coming.
    fn drop(&mut self) {
        self.reader.abort();
    }
}
