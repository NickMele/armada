//! Reading a Drone's transcript while it runs, so that its ending means
//! something.
//!
//! # Why the stream is read at all, given the gate does not read it
//!
//! Nothing a Drone says gates its own step — `verification` has no type that
//! can be built from a message. But `crate::aftermath` needs an
//! [`Ending`](crate::Ending), and an `Ending` is a fold over the events the
//! Drone emitted: whether it called anything, and how often it was refused. The
//! three answers that fold produces — `blocked_by_policy`, `silent`, `stalled`
//! — are three different things for a person to do, and a Fleet that never read
//! the stream could only ever say `interrupted`.
//!
//! So the transcript is read for exactly one purpose: to know what a Drone's
//! silence was made of. It is never read for a fact that advances anything.
//!
//! # EOF is the signal, not a poll
//!
//! The task below ends when the pipe closes, which is when the child's stdout
//! is gone. That is a fact the operating system delivers rather than one Fleet
//! discovers by asking on a timer, and it is why nothing here has an interval.
//! Whether the process itself is also gone is `crate::holder_of`'s question —
//! `Ending::Reported`'s own comment is explicit that a terminating event is a
//! turn boundary and not a lifetime.
//!
//! # A line that will not decode is still evidence of life
//!
//! `AgentHarness::read` is total and answers `Unreadable` rather than an error,
//! and nothing here filters those out. A run full of undecodable lines is not a
//! silent run, and folding it as one would send a person to rephrase a prompt
//! that was working.

use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, DroneEvent};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::task::JoinHandle;

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
#[derive(Debug)]
pub struct Watching {
    heard: Arc<Mutex<Heard>>,
    reader: JoinHandle<()>,
}

impl Watching {
    /// Start reading, on a task of its own.
    ///
    /// The harness is shared rather than borrowed because the task outlives the
    /// call: decoding a line is the harness's, and the harness is the only
    /// thing that knows what a line means.
    pub fn reading<H>(transcript: ChildStdout, harness: Arc<H>) -> Watching
    where
        H: AgentHarness + Send + Sync + 'static,
    {
        let heard = Arc::new(Mutex::new(Heard::default()));
        let filling = Arc::clone(&heard);
        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(transcript).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let read = harness.read(&line);
                let mut held = filling
                    .lock()
                    .expect("the transcript is not held across a panic");
                held.events.extend(read);
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
