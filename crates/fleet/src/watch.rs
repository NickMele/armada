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
    /// The task holds a pipe and nothing else, so aborting it loses nothing:
    /// whatever had been read is already in `heard`, and a Job whose Drone has
    /// been terminated has no further lines coming.
    fn drop(&mut self) {
        self.reader.abort();
    }
}
