//! Speaking to a Drone that is already running, and ending one.
//!
//! **The mechanism is spike 4's.** Fleet can inject a turn into a live session:
//! the harness reads one JSON object per line on stdin with the stream held
//! open, and re-emits each message when it is consumed, which is what made the
//! latency measurable rather than inferred — three runs, delivered in 1.59s
//! mid-task and 2.85s idle. [`DroneSession`] is that, and the first turn a
//! Drone is ever given goes down the same pipe; `crate::drone` is the only
//! thing that opens one.
//!
//! **Delivery waits for the current turn to end.** A message injected while the
//! Drone is inside a tool call is consumed when that call returns — measured at
//! 33.14s against a 40-second command, of which none was latency. For the gate
//! that cost is zero: a Drone that has just submitted evidence is between turns
//! by definition, which is exactly the moment the gate speaks.
//!
//! **Every write is checked, and a truncated one is a failure.** `write_all`
//! either writes the whole turn or fails, and nothing here reports how much of
//! one went. v1's equivalent read a short payload as an empty grant, so a
//! partial write became a silent absence of authority and the run failed naming
//! a secret rather than the pipe. A half-delivered instruction is an error,
//! never a Drone that was told part of something.

use std::future::Future;
use std::io;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;
use verification::OutcomeTurn;

use crate::converging::ReportNow;
use crate::group::{end_the_group, run_is_over};
use crate::questioning::Answer;
use crate::resume::Redirection;
use crate::silence::Poke;
use crate::terms::{Declaring, Redeclaring};

/// A Drone's live session, from the gate's side.
///
/// **Seven methods, and none of them can start anything.** There is no spawn,
/// no respawn and no restart, because the gate must not be able to produce a
/// Drone — and no way to remove a worktree, because nothing in this workspace
/// can. A restart is `crate::resume`'s and it reaches a spawn rather than this
/// trait.
///
/// Each method carries a different authorship, which is why one method taking
/// text would be wrong: a verdict Fleet reached, something Fleet observed while
/// the step ran, a person's own words, Fleet's directive at the third stage of
/// the thrashing chain, the answer a person picked from a set the Drone
/// offered, and Fleet asking a quiet Drone whether it is still there.
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
    /// [`crate::terms::Declaring::at`] returns one: most steps ask nothing,
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
    /// see [`crate::terms::Redeclaring`].
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

    /// End the Drone, and everything it left running.
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

/// Which of Armada's turns a transcript row is.
///
/// **One variant per [`Turn`] constructor above, and it lives here for that
/// reason** — the set is decided by what Fleet can say to a Drone, and this is
/// the file that decides it. It crosses the wire as a string, for
/// `ipc::JudgeInFlight::look`'s reason: no registry declares the set, so a
/// closed set restated in `ipc` would be a second authority for a list that has
/// exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occasion {
    /// The brief a step opened with. **[`Turn::first`], and it is the one a
    /// person most often wants** — what the Drone was asked to do.
    Opening,
    /// What the gate decided, injected into a session already running.
    Outcome,
    /// A person's own words, carried in verbatim.
    Redirect,
    /// A person's answer to a question the Drone asked.
    Answer,
    /// What the live scope check saw.
    Drift,
    /// Fleet's stop-and-report directive.
    Report,
    /// Fleet's liveness nudge, after a silence.
    Poke,
}

impl Occasion {
    pub fn as_wire(&self) -> &'static str {
        match self {
            Occasion::Opening => "opening",
            Occasion::Outcome => "outcome",
            Occasion::Redirect => "redirect",
            Occasion::Answer => "answer",
            Occasion::Drift => "drift",
            Occasion::Report => "report",
            Occasion::Poke => "poke",
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
    /// Whether the child has been collected. **Read and written only under the
    /// `child` lock**, which is what orders it, so the accesses below are
    /// relaxed rather than carrying an ordering the mutex already gives.
    reaped: AtomicBool,
}

impl DroneSession {
    /// Hold a Drone that has already been started. `crate::drone::start` is the
    /// only caller, because it is the only thing that can produce the halves.
    pub(crate) fn holding(pid: u32, input: ChildStdin, child: Child) -> DroneSession {
        DroneSession {
            pid,
            input: Mutex::new(input),
            child: Mutex::new(child),
            reaped: AtomicBool::new(false),
        }
    }

    /// The process. Fleet's own record of a running Job names it, and
    /// `crate::holder_of` is what turns it back into "is that still the same
    /// process".
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Whether the Drone has exited, **and reap it if it has** — ending
    /// whatever it left in its process group first.
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
    ///
    /// **The third half is `#371`.** The incident that filed it was a Drone
    /// whose run *ended*: it wrote its own terminating row, exited, and the
    /// tool it had started was there an hour later holding the same pipe.
    /// Nothing calls [`terminate`](LiveSession::terminate) on an ordinary
    /// completion, so what is in the group when this answers `true` is an
    /// orphan by definition — ending it is the reaping this method is named
    /// for. **Signalled before the child is collected**, because collecting is
    /// what frees the pid the group is named by.
    pub(crate) async fn exited(&self) -> Result<bool, io::Error> {
        let mut child = self.child.lock().await;
        if let Some(group) = self.group() {
            if run_is_over(group) {
                end_the_group(group);
            }
        }
        let ended = child.try_wait()?.is_some();
        if ended {
            // Collecting it hands the pid back to the operating system, so from
            // here the group it named is no longer this Drone's to signal. See
            // [`DroneSession::group`].
            self.reaped.store(true, Ordering::Relaxed);
        }
        Ok(ended)
    }

    /// The process group to signal, while it is still certainly this Drone's.
    ///
    /// `crate::detach` spawns every Drone through `libc::setsid()`, which makes
    /// the child a leader of a new session **and of a new process group whose
    /// id is its own pid** — so the pid Fleet holds is the group id, and the
    /// group holds this Drone and whatever it started and did not detach.
    ///
    /// `None` once the child has been collected: the pid is the operating
    /// system's to hand out again from that moment, and a pid that comes round
    /// as another Drone's would turn one kill into a kill of somebody else's
    /// work. **The claim is this type's rather than the caller's** — nothing
    /// about the order `crate::dispatch` reaps and ends in has to stay true for
    /// the group id to mean what it says here.
    fn group(&self) -> Option<NonZeroU32> {
        if self.reaped.load(Ordering::Relaxed) {
            return None;
        }
        NonZeroU32::new(self.pid)
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
    ///
    /// **The group first and the pid second.** A single-pid kill ends the
    /// Drone and leaves the tools it started, which sit in the process group
    /// `setsid` made for it — still running, still holding the Drone's stdout,
    /// watched by nobody. On 2 Sep one outlived its Fleet by an hour and was
    /// found by hand.
    ///
    /// **It makes a cut-short drain rarer and cannot make it impossible**, so
    /// [`Watching::drained`](crate::Watching::drained)'s bound is not something
    /// this replaces: a tool that calls `setsid` itself leaves the group, and
    /// then no signal reaches it. Reading this as a guarantee that the pipe
    /// closes is the one mistake to avoid here.
    async fn terminate(&self) -> Result<(), io::Error> {
        let mut child = self.child.lock().await;
        if let Some(group) = self.group() {
            end_the_group(group);
        }
        // Sent anyway, and not folded into the group signal above: the pid is
        // what this type is certain of, and a Drone that somehow left its own
        // group would otherwise be waited on having been signalled by nothing.
        child.start_kill()?;
        let ended = child.wait().await;
        self.reaped.store(true, Ordering::Relaxed);
        ended.map(|_| ())
    }
}
