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
//! `write_all` either writes the whole turn or fails; nothing here reports how
//! much of a turn went. v1's equivalent read a short payload as an empty grant,
//! so a partial write became a silent absence of authority and the run failed
//! naming a secret rather than the pipe. A half-delivered instruction is an
//! error, never a Drone that was told part of something.
//! # What the spike also established, and what it costs this trait
//!
//! **Delivery waits for the current turn to end.** A message injected while the
//! Drone is inside a tool call is consumed when that call returns — measured at
//! 33.14s against a 40-second command, of which none was latency. For the gate
//! that cost is zero: a Drone that has just submitted evidence is between turns
//! by definition, which is exactly the moment the gate speaks.
//!
//! # Seven methods, and none of them can start anything
//!
//! [`tell`](LiveSession::tell), [`notice`](LiveSession::notice),
//! [`redirect`](LiveSession::redirect), [`interrupt`](LiveSession::interrupt),
//! [`answer`](LiveSession::answer), [`poke`](LiveSession::poke),
//! [`terminate`](LiveSession::terminate). There
//! is no spawn, no respawn and no restart, because the gate must not be able to
//! produce a Drone — and no way to remove a worktree, because nothing in this
//! workspace can. A restart is `crate::resume`'s, and it reaches a spawn rather
//! than this trait.
//!
//! Each carries a different authorship, which is why one method taking text
//! would be wrong: a verdict Fleet reached, something Fleet observed while the
//! step ran, a person's own words, Fleet's directive at the third stage of the
//! thrashing chain, the answer a person picked from a set the Drone offered, and
//! Fleet asking a quiet Drone whether it is still there.

use std::future::Future;
use std::io;

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;
use verification::OutcomeTurn;

use crate::briefing::{Declaring, Redeclaring};
use crate::converging::ReportNow;
use crate::questioning::Answer;
use crate::resume::Redirection;
use crate::silence::Poke;

/// A Drone's live session, from the gate's side.
pub trait LiveSession {
    /// Errors this implementation can raise. Named by the implementation, so a
    /// caller can tell a session that has already ended from one that would not
    /// accept the write.
    type Error;

    /// Inject a turn. The Drone reads it at the next turn boundary.
    ///
    /// **Two callers, and neither crosses a step boundary.** A hand-back is the
    /// same step going round again under the same process, and a Job that
    /// finished has no next step to put a fresh Drone on. Everywhere else the
    /// Drone is ended: a failed step ends the Job, and an advance ends the
    /// Drone that worked the step — what the verdict said reaches the next one
    /// as a block in its opening brief, not as a turn.
    ///
    /// `declaring` is the ask the step being started makes of its Drone, where
    /// it makes one. **It travels with the verdict rather than as a turn of its
    /// own**, because a step boundary is one moment and a second injected
    /// message would spend a second turn to say the other half of it. It is
    /// `Option` rather than a separate method for the reason
    /// [`crate::briefing::Declaring::at`] returns one: most steps ask nothing,
    /// and a caller cannot tell which without reading the step.
    ///
    /// Asynchronous because the pipe is: a synchronous signature would either
    /// block the runtime on a child that has stopped reading, or need a second
    /// path for the same write that `crate::drone` already makes.
    fn tell(
        &self,
        turn: &OutcomeTurn,
        declaring: Option<&Declaring>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Inject what the live check saw, so the Drone can act on it.
    ///
    /// **A separate method from [`tell`](LiveSession::tell) because there is no
    /// verdict and no boundary to ride.** Drift is found in the middle of a
    /// step, and the turn the Drone would otherwise hear about it on is the one
    /// after its gate — by which time the declaration it could have fixed has
    /// already been measured.
    ///
    /// **And not [`interrupt`](LiveSession::interrupt)**, which is a directive
    /// to stop and report. This one asks for nothing but the tool call that
    /// makes a plan true, and a Drone that ignores it has done nothing wrong;
    /// see [`crate::briefing::Redeclaring`].
    fn notice(&self, drifted: &Redeclaring)
        -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Inject a person's instruction. The Drone reads it at the next turn
    /// boundary, like every other injected turn.
    ///
    /// **A separate method from [`tell`](LiveSession::tell) because the two
    /// carry different authorship.** `tell` carries a verdict Fleet reached and
    /// can only be produced by the advance path; this carries words a person
    /// wrote, and `docs/contracts/agent-prompt.md` gives it no wording of its
    /// own for that reason. One method taking either would let the gate send
    /// arbitrary text.
    fn redirect(
        &self,
        instruction: &Redirection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Interrupt with a directive to stop and report.
    ///
    /// **Stage three of the thrashing chain and nothing else.** It is not
    /// [`redirect`](LiveSession::redirect) because no person wrote it, and not
    /// [`tell`](LiveSession::tell) because no gate ruled — see `crate::converging`.
    fn interrupt(
        &self,
        directive: &ReportNow,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Hand the waiting Drone a person's answer to the question it asked.
    ///
    /// **A separate method from [`redirect`](LiveSession::redirect) because the
    /// two carry different authorship**, the same argument that keeps `redirect`
    /// apart from [`tell`](LiveSession::tell). A redirect is a person's own
    /// words and Fleet adds none; this is Fleet's own sentence built around a
    /// label the Drone itself offered, and there is no way to send other text
    /// under it — [`Answer`] has no constructor taking one.
    ///
    /// One method taking either would let a person's free text arrive as though
    /// it were an answer to a closed question, which is the conversation
    /// `crate::questioning` exists instead of.
    fn answer(&self, answer: &Answer) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Ask a Drone that has said nothing whether it is still there.
    ///
    /// **The liveness nudge and nothing else** — see `crate::silence`. It is
    /// not [`interrupt`](LiveSession::interrupt): that one tells a Drone to
    /// stop, and this one is explicit that a Drone which is working should
    /// carry on. Reading them as one method would make the poke an
    /// interruption, which is what it must never be.
    fn poke(&self, nudge: &Poke) -> impl Future<Output = Result<(), Self::Error>> + Send;

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

    /// What the gate decided, injected into a session already running, and the
    /// ask the step it moves the Drone on to makes.
    ///
    /// **One turn for the whole boundary.** Both halves are Fleet's own and
    /// both are about the same instant: what happened to the part just
    /// finished, and what the next part wants before it starts. Splitting them
    /// would cost a second turn boundary to deliver the half a Drone acts on
    /// first.
    ///
    /// `declaring` is `Some` on no path that survives: a step boundary spawns
    /// now, and the ask is in the opening brief. It stays because the pairing
    /// is the shape of an outcome turn rather than a caller's arrangement of
    /// two.
    pub fn outcome(turn: &OutcomeTurn, declaring: Option<&Declaring>) -> Turn {
        match declaring {
            Some(asked) => Turn::of(&format!("{}\n\n{}", turn.text(), asked.text())),
            None => Turn::of(turn.text()),
        }
    }

    /// What the live check saw, injected into a session already running.
    ///
    /// **Its own turn, and this is the one place that is right.** The step
    /// boundary has a verdict for the notice to ride and this moment has
    /// nothing: the drift is found while the step runs, and holding it until
    /// something else goes down the pipe would deliver it after the gate has
    /// already measured the plan it was offering to correct.
    pub fn noticing(drifted: &Redeclaring) -> Turn {
        Turn::of(drifted.text())
    }

    /// What a person said, injected into a session already running.
    ///
    /// **Verbatim, and Fleet adds nothing.** The Drone's baseline already told
    /// it a verdict arrives as a later turn carrying the reason, so this turn
    /// lands where the Drone is waiting for one — and framing it would be
    /// Fleet authoring copy the prompt contract deliberately leaves to the
    /// person.
    pub fn redirection(instruction: &Redirection) -> Turn {
        Turn::of(instruction.text())
    }

    /// Fleet's own stop-and-report directive, injected into a session already
    /// running. The wording is `ReportNow`'s and there is no way to send other
    /// text under it.
    pub fn reporting(directive: &ReportNow) -> Turn {
        Turn::of(directive.text())
    }

    /// A person's answer to the question this Drone asked, injected into a
    /// session already running.
    ///
    /// The wording is [`Answer`]'s and there is no way to send other text
    /// under it — which is the difference from [`redirection`](Turn::redirection),
    /// where a person's own words are exactly what travels.
    pub fn answering(answer: &Answer) -> Turn {
        Turn::of(answer.text())
    }

    /// Fleet's own liveness nudge, injected into a session that has said
    /// nothing for a while. The wording is `Poke`'s and there is no way to send
    /// other text under it.
    pub fn poking(nudge: &Poke) -> Turn {
        Turn::of(nudge.text())
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

    async fn tell(
        &self,
        turn: &OutcomeTurn,
        declaring: Option<&Declaring>,
    ) -> Result<(), io::Error> {
        self.say(&Turn::outcome(turn, declaring)).await
    }

    async fn notice(&self, drifted: &Redeclaring) -> Result<(), io::Error> {
        self.say(&Turn::noticing(drifted)).await
    }

    async fn redirect(&self, instruction: &Redirection) -> Result<(), io::Error> {
        self.say(&Turn::redirection(instruction)).await
    }

    async fn interrupt(&self, directive: &ReportNow) -> Result<(), io::Error> {
        self.say(&Turn::reporting(directive)).await
    }

    async fn answer(&self, answer: &Answer) -> Result<(), io::Error> {
        self.say(&Turn::answering(answer)).await
    }

    async fn poke(&self, nudge: &Poke) -> Result<(), io::Error> {
        self.say(&Turn::poking(nudge)).await
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
