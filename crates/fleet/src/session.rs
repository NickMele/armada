//! Speaking to a Drone that is already running, and ending one.
//!
//! # The mechanism, and where it came from
//!
//! Spike 4 established that Fleet can inject a turn into a live session — the
//! harness reads one JSON object per line on stdin with the stream held open,
//! and re-emits each message when it is consumed, which is what made the
//! latency measurable rather than inferred. Three runs, delivered in 1.59s
//! mid-task and 2.85s idle.
//!
//! [`DroneSession`] is that, and the first turn a Drone is ever given goes down
//! the same pipe — see `crate::drone`, which is the only thing that opens one.
//!
//! # Every write is checked, and a truncated one is a failure
//!
//! `write_all` either writes the whole turn or fails; there is no path here
//! that reports how much of a turn went. v1's equivalent read a short payload
//! as an empty grant, so a partial write became a silent absence of authority
//! and the run then failed naming a secret rather than the pipe. The shape of
//! that defect is what this module refuses: a half-delivered instruction is an
//! error, never a Drone that was told part of something.
//!
//! # What the spike also established, and what it costs this trait
//!
//! **Delivery waits for the current turn to end.** A message injected while the
//! Drone is inside a tool call is consumed when that call returns — measured at
//! 33.14s against a 40-second command, of which none was latency. For the gate
//! that cost is zero: a Drone that has just submitted evidence is between turns
//! by definition, which is exactly the moment the gate speaks.
//!
//! # Two methods, and neither can start anything
//!
//! [`tell`](LiveSession::tell) and [`terminate`](LiveSession::terminate). There
//! is no spawn, no respawn and no restart, because the gate must not be able to
//! produce a Drone — and no way to remove a worktree, because nothing in this
//! workspace can.

use std::future::Future;
use std::io;

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;
use verification::OutcomeTurn;

/// A Drone's live session, from the gate's side.
pub trait LiveSession {
    /// Errors this implementation can raise. Named by the implementation, so a
    /// caller can tell a session that has already ended from one that would not
    /// accept the write.
    type Error;

    /// Inject a turn. The Drone reads it at the next turn boundary.
    ///
    /// Called only where the step advanced. A failed step ends the Job, and the
    /// Drone is terminated rather than told — see `verification::OutcomeTurn`.
    ///
    /// Asynchronous because the pipe is: a synchronous signature would either
    /// block the runtime on a child that has stopped reading, or need a second
    /// path for the same write that `crate::drone` already makes.
    fn tell(&self, turn: &OutcomeTurn) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// End the Drone.
    ///
    /// **The worktree is untouched.** Removal is driven by Job retention and
    /// never by a process ending, and there is no method in this workspace that
    /// could remove one anyway. The branch is left exactly as the Drone left
    /// it, which is what "a person reads the branch" depends on.
    fn terminate(&self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// One turn, as the harness reads it: a JSON object on a line of its own.
///
/// **The only shape that goes down the pipe.** There is no method taking a
/// prepared string, so nothing can put an arbitrary line into a Drone's input —
/// the first turn and every injected one are built here from text.
#[derive(Debug, Serialize)]
pub struct Turn {
    #[serde(rename = "type")]
    kind: &'static str,
    message: Message,
}

#[derive(Debug, Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

impl Turn {
    /// The task, at the start of a session.
    pub fn first(prompt: &str) -> Turn {
        Turn::of(prompt)
    }

    /// What the gate decided, injected into a session already running.
    pub fn outcome(turn: &OutcomeTurn) -> Turn {
        Turn::of(turn.text())
    }

    fn of(content: &str) -> Turn {
        Turn {
            kind: "user",
            message: Message {
                role: "user",
                content: String::from(content),
            },
        }
    }
}

/// A Drone that is running, held by the one component allowed to speak to it.
///
/// **No spawn, no respawn, no restart.** The gate must not be able to produce a
/// Drone, so this type carries no way to make one — it is handed a process that
/// already exists and can only speak to it or end it.
///
/// The child is held so it can be reaped: a terminated process that nobody
/// waits on is a zombie, and a zombie holds the pid that `crate::holder_of`
/// uses to answer whether a Drone is still there.
#[derive(Debug)]
pub struct DroneSession {
    pid: u32,
    input: Mutex<ChildStdin>,
    child: Mutex<Child>,
}

impl DroneSession {
    /// Hold a Drone that has already been started. `crate::drone::start` is the
    /// only caller, because it is the only thing that can produce the halves.
    pub(crate) fn holding(pid: u32, input: ChildStdin, child: Child) -> DroneSession {
        DroneSession {
            pid,
            input: Mutex::new(input),
            child: Mutex::new(child),
        }
    }

    /// The process. Fleet's own record of a running Job names it, and
    /// `crate::holder_of` is what turns it back into "is that still the same
    /// process".
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Whether the Drone has exited, **and reap it if it has**.
    ///
    /// The two halves are one call on purpose. `try_wait` is the thing that
    /// collects a finished child, and a child nobody collects is a zombie that
    /// still holds its pid — so a caller that asked without reaping would be
    /// told "still there" forever, by the very state its asking was meant to
    /// detect.
    ///
    /// This is why the aftermath does **not** go through
    /// [`holder_of`](crate::holder_of): that answers about a pid this process
    /// does not own, which is the runtime file's question and not this one's.
    /// For a child Fleet is holding, the operating system has an exact answer
    /// and it is here.
    pub(crate) async fn exited(&self) -> Result<bool, io::Error> {
        self.child
            .lock()
            .await
            .try_wait()
            .map(|status| status.is_some())
    }

    /// Write one turn, whole, and flush it.
    ///
    /// Returns the `io::Error` rather than a wrapped one because every caller
    /// here distinguishes on `kind()` — a broken pipe is a Drone that is gone
    /// and anything else is a pipe that would not take the write. **A partial
    /// write is inside the `Err`**: `write_all` does not report how far it got,
    /// so there is no success value that means "some of it".
    pub(crate) async fn say(&self, turn: &Turn) -> Result<(), io::Error> {
        let mut line = ipc::encode(turn)
            .map_err(|why| io::Error::new(io::ErrorKind::InvalidData, why.to_string()))?;
        line.push('\n');

        let mut input = self.input.lock().await;
        input.write_all(line.as_bytes()).await?;
        input.flush().await
    }
}

impl LiveSession for DroneSession {
    type Error = io::Error;

    async fn tell(&self, turn: &OutcomeTurn) -> Result<(), io::Error> {
        self.say(&Turn::outcome(turn)).await
    }

    /// End the Drone and reap it.
    ///
    /// **Killing is exclusively something a caller asks for.** Fleet never
    /// auto-kills: anything escalated is paused with its worktree held as-is,
    /// and this is reached from the two rulings that end a Job and from a
    /// person's own Kill.
    async fn terminate(&self) -> Result<(), io::Error> {
        let mut child = self.child.lock().await;
        child.start_kill()?;
        child.wait().await.map(|_| ())
    }
}
