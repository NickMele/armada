//! Starting a Drone, and what its ending means for the Job.
//!
//! **Fleet builds the environment; nobody inherits one.** [`environment`] is
//! the whole of what a Drone gets and every variable in it was named there.
//! Nothing here reads `std::env`: the two values a Drone needs are parameters
//! resolved by the composition root, for the same reason a timestamp is — a
//! function reading its own inputs from the process cannot be tested or replayed.
//!
//! **No credential of any kind is on that list**, which is the second half of
//! "a Drone cannot push" — the first being that no [`Grant`] can express one. A
//! shell that reached a push anyway would have no `SSH_AUTH_SOCK`, no
//! `GIT_ASKPASS`, no forge token and nobody to ask, and the attempt is a
//! `DroneEvent::Refused` in the transcript either way.
//!
//! **The prompt goes in on stdin, not argv.** `ps` prints a same-uid child's
//! argument list on darwin 27, so argv is public to every process on the
//! machine. The first turn goes to the child's stdin instead, the same channel
//! a later turn is injected through — [`start`] holds why the write is checked.
//!
//! **The environment is not a hiding place either, and this module used to say
//! it was.** `ps -Eww` prints a same-uid process's environment on darwin 27,
//! measured against a live Drone in
//! `docs/spikes/011-what-can-one-drone-reach.md`. Nothing here depended on the
//! old claim, so nothing moves; it is corrected because a false statement about
//! what is visible is the kind a later design rests on.

use std::error::Error;
use std::fmt;

use adapter_traits::{
    AgentHarness, DroneEvent, DroneHandle, DroneSpawnConfig, Environment, SpawnConfigRefused,
};
use core_model::{EscalationTrigger, JobStatus, Target};
use tokio::process::{ChildStderr, ChildStdout};

use crate::session::{DroneSession, Turn};
use crate::Detached;

/// The two host facts a Drone needs, named by whoever resolved them.
///
/// **Not read from this process.** The composition root reads them once and
/// hands them down, so a test can plant something else and every other caller
/// gets the same two values rather than whatever its own shell had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostPaths<'a> {
    /// Where the Drone looks for programs. **Fleet's choice, not Fleet's own
    /// `PATH`** — a Drone that inherited it would find whatever the operator
    /// happened to have in front, which is a different toolchain on two
    /// machines and a different one again after a shell profile changes.
    pub path: &'a str,
    /// Who the operator is, as the agent CLI insists on being told.
    ///
    /// Not decoration and not provenance: the CLI refuses to authenticate
    /// without it, however readable its credentials are. See `environment`.
    pub user: &'a str,
    /// The home directory the agent CLI reads its own credentials from.
    ///
    /// **This is the confinement's known floor and it is written down as one.**
    /// Passing it is what lets the Drone authenticate at all; it is also what
    /// lets the CLI read the operator's skills, plugins, subagents and session
    /// hooks. `--strict-mcp-config` bounds MCP servers and bounds none of
    /// those, which is the open question on Drone rather than something this
    /// module quietly solved.
    pub home: &'a str,
}

/// Everything a Drone's process will hold.
///
/// Four variables, and each one is here because something breaks without it.
/// Adding a fifth is a deliberate edit to this function, which is the point:
/// the list is a diff, not a default.
///
/// `env_clear` appears nowhere in v1's production code, so a token exported in
/// the operator's shell reached every Drone v1 started — while v1's own handoff
/// table rejected "Armada's own environment" as a channel for exactly that
/// reason. It is the only place v1's Drone spawn was worse than its check
/// spawn.
pub fn environment(host: HostPaths<'_>) -> Result<Environment, SpawnConfigRefused> {
    Environment::nothing()
        .and("PATH", host.path)?
        .and("HOME", host.home)?
        // Deterministic text handling. Without it the child takes the C locale
        // on one machine and the operator's on another, and a diff of the same
        // work differs by how a byte was collated.
        .and("LANG", "en_US.UTF-8")?
        // The CLI renders progress differently into a pipe than into a
        // terminal. Saying so once here beats a heuristic reading the stream.
        .and("TERM", "dumb")?
        // **Without this the agent CLI is not logged in**, and says so on its
        // first turn instead of working. Its credentials are in the macOS
        // Keychain, and the Keychain is readable without this — the same
        // `security` read succeeds either way, so nothing here is an access
        // problem. The CLI simply will not authenticate with no `USER`.
        //
        // Measured against a real Drone: same binary, same `HOME`, `USER` the
        // only difference. `LOGNAME` does not substitute.
        .and("USER", host.user)
}

/// A Drone that is running.
///
/// The three halves a caller needs and no more: who it is, how to speak to it,
/// and what it is saying. **There is no field through which the process itself
/// can be reached** — ending one is [`DroneSession::terminate`], held by the
/// caller allowed to end it.
#[derive(Debug)]
pub struct Started {
    pub handle: DroneHandle,
    pub session: DroneSession,
    /// The transcript. Read a line at a time and given to
    /// `AgentHarness::read`.
    pub transcript: ChildStdout,
    /// Whatever the CLI itself complains about, which is not the transcript and
    /// is never parsed. Kept so a Drone that failed to start for its own
    /// reasons has something a person can read.
    pub complaints: ChildStderr,
}

/// Start a Drone against a prepared worktree, detached, and give it its first
/// turn.
///
/// The order matters and is the order v1 got wrong: the process exists before
/// anything is written to it, and the write is checked before this returns, so
/// a caller that gets an `Ok` has a Drone that has been told what to do. A
/// caller that gets an `Err` has one that is gone — every failure below ends
/// with nothing running.
///
/// **The write is the one place this module can be killed by a child.** v1
/// wrote its own handoff with `let _ = pipe.write_all(…)` while SIGPIPE was at
/// `SIG_DFL` process-wide, so a child that exec'd and exited before reading
/// would take the parent down with exit 141, mid-spawn, after the record was
/// already written — and `let _ =` cannot catch it, because the signal arrives
/// before `write` returns. Here the result is matched, a broken pipe is its own
/// variant, and **nothing in this workspace restores SIGPIPE**, so the write
/// returns `EPIPE` rather than a signal. `a_drone_that_dies_before_it_is_told`
/// is the test that would fail if any of that changed.
pub async fn start<H>(
    harness: &H,
    config: &DroneSpawnConfig,
) -> Result<Started, DroneNotStarted<H::Error>>
where
    H: AgentHarness,
{
    let launch = harness
        .render(config)
        .map_err(|why| DroneNotStarted::NotRendered { why })?;

    let mut child = Detached::launching(&launch)
        .piping_input()
        .capturing_output()
        .spawn()
        .map_err(|cause| DroneNotStarted::NotSpawned {
            program: String::from(launch.program()),
            cause,
        })?;

    let pid = child.id().ok_or(DroneNotStarted::ExitedImmediately)?;
    let (Some(input), Some(transcript), Some(complaints)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        return Err(DroneNotStarted::ExitedImmediately);
    };

    let session = DroneSession::holding(pid, input, child);
    session
        .say(&Turn::first(config.prompt().as_str()))
        .await
        .map_err(|cause| match cause.kind() {
            std::io::ErrorKind::BrokenPipe => DroneNotStarted::DiedBeforeItWasTold,
            _ => DroneNotStarted::NotTold { cause },
        })?;

    Ok(Started {
        handle: DroneHandle::started(pid),
        session,
        transcript,
        complaints,
    })
}

/// Why no Drone is running.
///
/// **Every variant means nothing is running**, which is what makes them all the
/// same thing to the Job and different things to a person. A rendering refusal
/// is an `armada.yml` to fix; a spawn failure is a binary to install; a broken
/// pipe is a CLI that started and died, and its own stderr says why.
#[derive(Debug)]
pub enum DroneNotStarted<E> {
    /// The harness would not turn the configuration into a process.
    NotRendered { why: E },
    /// The operating system would not start it.
    NotSpawned {
        program: String,
        cause: std::io::Error,
    },
    /// It was gone before Fleet could read a pid or take its streams.
    ExitedImmediately,
    /// **The child exited before it read its first turn.**
    ///
    /// Its own variant because it is the one v1 could not survive: with SIGPIPE
    /// at its default the same event killed the parent instead of returning an
    /// error, and the run record was already written by then.
    DiedBeforeItWasTold,
    /// The first turn could not be written. A short write lands here too:
    /// `write_all` fails rather than reporting how much went, so a truncated
    /// turn is an error and never a Drone that was told half a task.
    NotTold { cause: std::io::Error },
}

impl<E: fmt::Display> fmt::Display for DroneNotStarted<E> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DroneNotStarted::NotRendered { why } => {
                write!(out, "the Drone could not be configured: {why}")
            }
            DroneNotStarted::NotSpawned { program, cause } => {
                write!(out, "{program} would not start: {cause}")
            }
            DroneNotStarted::ExitedImmediately => {
                out.write_str("the Drone was gone before Fleet could hold on to it")
            }
            DroneNotStarted::DiedBeforeItWasTold => out.write_str(
                "the Drone exited before it read its first turn — read its own \
                 stderr, which is what says why",
            ),
            DroneNotStarted::NotTold { cause } => {
                write!(out, "the Drone's first turn could not be written: {cause}")
            }
        }
    }
}

impl<E: Error + 'static> Error for DroneNotStarted<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DroneNotStarted::NotRendered { why } => Some(why),
            DroneNotStarted::NotSpawned { cause, .. } | DroneNotStarted::NotTold { cause } => {
                Some(cause)
            }
            DroneNotStarted::ExitedImmediately | DroneNotStarted::DiedBeforeItWasTold => None,
        }
    }
}

/// How a Drone's run finished.
///
/// **Not how it did.** There is no `Succeeded`, because the harness reported
/// exit 0, `is_error` false and a success subtype for a run that accomplished
/// nothing — four agreeing signals, all wrong. What did or did not pass is the
/// gate's, decided from evidence and Fleet's own Checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ending {
    /// The stream carried a terminating event.
    ///
    /// **This is a turn boundary, not a lifetime.** One process emitted a
    /// terminating event, took an injected turn, and emitted another — so this
    /// is an ending only once the process is also gone, which
    /// `crate::holder_of` answers and this enum does not.
    Reported {
        /// How many calls the Drone was refused, over the whole run.
        ///
        /// **A run total, so it classifies nothing on its own** — read it with
        /// `reached_after_refusal`, which is what `crate::aftermath` matches
        /// on and why.
        refusals: usize,
        /// Whether the Drone reached for anything at all.
        called_something: bool,
        /// Whether the Drone reached for something after the last call it was
        /// refused — equivalently, whether the last reach in the stream got
        /// through.
        ///
        /// **`refusals` is what gives it a subject.** With nothing refused
        /// there is no refusal to have reached past, and the field says only
        /// that the run's last call was allowed.
        reached_after_refusal: bool,
    },
    /// The process is gone and no terminating event ever arrived.
    Vanished,
}

impl Ending {
    /// Fold a run's events into an ending, given that the process is gone.
    ///
    /// Reads the stream once and keeps what the classification needs.
    /// `Unreadable` lines are **not** skipped: a line that did not decode still
    /// proves the Drone was producing output, so a run full of them is not a
    /// silent one.
    pub fn of(events: &[DroneEvent]) -> Ending {
        let mut reported = None;
        let mut called_something = false;
        let mut reached_after_refusal = false;
        for event in events {
            match event {
                DroneEvent::Ended { refusals, .. } => reported = Some(*refusals),
                DroneEvent::Called { .. } => {
                    called_something = true;
                    reached_after_refusal = true;
                }
                // A refused call emits `Called` and then `Refused`, so this
                // takes back the reach the arm above just recorded. What the
                // flag holds at the end of the stream is whether the Drone
                // went on reaching past the last thing it was refused.
                DroneEvent::Refused { .. } => {
                    called_something = true;
                    reached_after_refusal = false;
                }
                _ => {}
            }
        }
        match reported {
            Some(refusals) => Ending::Reported {
                refusals,
                called_something,
                reached_after_refusal,
            },
            None => Ending::Vanished,
        }
    }
}

/// Whether the Drone's run ended with work still running behind it.
///
/// **Not part of [`Ending`], and the separation is the point.** `Ending` says
/// how the run finished and `aftermath` turns that into one of three answers; a
/// fourth field there would put a question about *whether to reap at all* into
/// the function that decides *what a reap means*. This is asked first, by
/// `crate::silence::at_rest`, and where it answers true nothing is folded.
///
/// **The last count wins and there is no pairing.** `BackgroundWork` is the
/// harness re-stating its whole outstanding set, so the last one in the stream
/// is the set as it stood when the run ended — a Drone that started three
/// subagents and saw all three land emitted a final zero and is not waiting.
///
/// **Why a Drone gets into this state at all.** A background task's report
/// arrives as a notification that wakes the session; a headless Drone that ends
/// its turn has nothing left to wake. So it says "I will wait for the agent's
/// findings", ends its turn, and the report can never come. Reaping it there
/// loses the whole step to a wait that was never going to end, which is Job
/// `01M21BKVPW002DC0ATD1X9T0VF` and what this reading exists for.
pub fn awaiting_background(events: &[DroneEvent]) -> bool {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            DroneEvent::BackgroundWork { outstanding } => Some(*outstanding > 0),
            _ => None,
        })
        .unwrap_or(false)
}

/// Whether the Drone left anything for the gate to rule on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Left {
    /// Evidence is in the inbox. It may still fail; that is a different
    /// question and a different decision.
    Evidence,
    Nothing,
}

/// What follows from a Drone being gone.
///
/// **There is no variant meaning "the Job stays running and nothing happens".**
/// That is the state the milestone step is about: v1's Jobs sat at `running`
/// with no process behind them, and nothing in this enum can produce it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Aftermath {
    /// Evidence is waiting, so the gate rules and the Drone being gone changes
    /// nothing about that. The Job is not stuck: something is queued that will
    /// move it.
    TheGateDecides,
    /// The Job moves, now, to this.
    JobMoves(Target),
    /// The Job had already stopped, so its Drone going takes nothing further
    /// with it. **The escalation already recorded stands** — a second one
    /// would be the `escalated -> escalated` move the machine refuses, which
    /// is what made ending an escalated Job's Drone look like the easy answer.
    ///
    /// What it changes is which act a person has: the Drone was there and is
    /// not, so this Job is restartable rather than redirectable.
    AlreadyStopped,
}

/// What a dead Drone means for its Job. Each answer is a different act:
///
/// | Status | Ending | Left | Answer |
/// | --- | --- | --- | --- |
/// | working | anything | evidence | the gate rules |
/// | working | reported, refused and never reached again | nothing | `blocked_by_policy` — widen the allowlist, do not rephrase |
/// | working | reported, called nothing | nothing | `silent` — rephrase, then redispatch |
/// | working | reported, called things | nothing | `stalled` — it worked and never submitted, refused on the way or not |
/// | working | vanished | nothing | `interrupted` — the process died; a person decides |
/// | stopped | anything | anything | nothing moves; the Job is now restartable |
///
/// **A refusal count is a run total, so it classifies nothing on its own.** A
/// Drone refused early and still calling tools at the end carried on past the
/// allowlist a person would be sent to widen; what decides is whether it
/// reached again after the last refusal.
///
/// **Every answer but the first two leaves the Job escalated, which is not
/// terminal** — the registry wins over the milestone step's word: `escalated`
/// holds the worktree and the port span until a person answers, and answering
/// is what takes it terminal. What the step asks is that the Job never stays
/// `running`, and no path here leaves it there.
///
/// **`status` is the first question, not a guard bolted on.** An escalated Job
/// keeps its Drone alive and idle, so a dying Drone is no longer proof the Job
/// was working, and stopping one twice is `escalated -> escalated`, refused.
pub fn aftermath(status: JobStatus, ending: &Ending, left: Left) -> Aftermath {
    if status != JobStatus::Running {
        return Aftermath::AlreadyStopped;
    }
    if left == Left::Evidence {
        return Aftermath::TheGateDecides;
    }
    Aftermath::JobMoves(Target::Escalated(match ending {
        Ending::Reported {
            refusals,
            reached_after_refusal: false,
            ..
        } if *refusals > 0 => EscalationTrigger::BlockedByPolicy,
        Ending::Reported {
            called_something: false,
            ..
        } => EscalationTrigger::Silent,
        Ending::Reported { .. } => EscalationTrigger::Stalled,
        Ending::Vanished => EscalationTrigger::Interrupted,
    }))
}
