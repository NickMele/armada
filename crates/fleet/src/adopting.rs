//! A Drone that outlived the Fleet which spawned it, and what taking it back
//! can honestly mean.
//!
//! # What survives a Fleet restart, and what cannot
//!
//! `crate::detach` calls `libc::setsid()` on every spawn, so a Drone leads a
//! session of its own. That is what makes an orphan possible; it is not what
//! makes the conversation resumable.
//!
//! | | Survives | Why |
//! |---|---|---|
//! | The process and its group | **Yes** | It is nobody's child now |
//! | The worktree, the branch, the uncommitted work | **Yes** | Nothing removes one on a process ending |
//! | The record, the Job log, the transcript file | **Yes** | Fleet wrote them as it went |
//! | The Drone's own calls into Fleet | **Yes**, once the pid index is restored | They arrive on a fresh connection to the same loopback port, which `crate::peer` attributes by pid and port pair |
//! | Fleet reading its transcript | **No** | A pipe, whose read end went with the process that held it |
//! | Fleet speaking to it | **No** | The same pipe the other way, and the Drone saw end-of-file when Fleet died |
//!
//! **So adoption is of the process and the record, never of the pipe.** An
//! adopted Drone can finish its step and be gated, because evidence arrives
//! over the loopback and nothing a Drone says gates its own step. It cannot be
//! handed back to, poked, redirected or told a verdict.

use std::io;
use std::num::NonZeroU32;

use core_model::{DroneId, JobId, StepId, Timestamp};
use store::DroneProcess;
use verification::OutcomeTurn;

use crate::converging::ReportNow;
use crate::group::end_the_group;
use crate::process::{holder_of, Holder, StartedAt};
use crate::questioning::Answer;
use crate::resume::Redirection;
use crate::session::{DroneSession, LiveSession};
use crate::silence::Poke;
use crate::terms::{Declaring, Redeclaring};

/// What Fleet says when something asks an adopted Drone to listen.
///
/// One sentence for all six, because there is one reason and a caller that
/// could tell them apart would be a caller deciding which of them to retry.
const NOTHING_TO_SPEAK_INTO: &str = "this Drone was adopted after a Fleet restart, so Fleet holds \
                                     no pipe into it: the write end went with the process that \
                                     spawned it and nothing can reopen another process's stdin";

/// A Drone this Fleet did not spawn, taken back over.
///
/// **The capability, and it is deliberately small.** There is no `tell`, no
/// `poke`, no `redirect`, no `interrupt` and no `answer` on this type — not a
/// refusal at runtime, but no method to call. What Fleet can still do with an
/// orphan is ask whether it is there and end it, and those are the two methods.
///
/// The seven-method [`LiveSession`] a slot exposes is [`Session`]'s, which is
/// where the refusal is written once for every caller that holds a Drone
/// without knowing which kind it is.
#[derive(Clone, Debug)]
pub struct Adopted {
    job: JobId,
    /// The step whose `assigned_drone` this process is behind, carried so the
    /// record and the process cannot come apart while the slot holds it.
    step: StepId,
    /// The Drone the transcript is named by. **The join back to what it had
    /// already said**, which is the only part of this run Fleet still has.
    drone: DroneId,
    pid: NonZeroU32,
    /// When the process started, as recorded at the spawn. Half the identity —
    /// see this module's header.
    started_at: StartedAt,
    /// The stretch nothing observed.
    gap: Gap,
}

/// The turns that went past with nobody reading them.
///
/// **Both ends are readings and neither is a guess.** `from` is the instant on
/// the last row of the Drone's own transcript, which is by construction the
/// last line the previous Fleet read; `until` is this Fleet's clock at the
/// adoption. What happened between them is not recoverable from anywhere, and
/// this type carries no field that pretends otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    /// The last thing Fleet heard. **`None` where the transcript holds no
    /// readable row** — a Drone that was spawned and had said nothing yet, or a
    /// file that will not read. That is a wider gap rather than a narrower one,
    /// and it is left absent rather than filled in with the spawn instant,
    /// which would claim a reading nobody took.
    pub from: Option<Timestamp>,
    /// When this Fleet picked the Drone back up.
    pub until: Timestamp,
}

impl Adopted {
    /// Take back a process the record names, having already proved it is the
    /// same one. [`reattaching`] is the only thing that can produce the proof,
    /// which is why this is not public.
    pub(crate) fn of(recorded: &DroneProcess, pid: NonZeroU32, gap: Gap) -> Adopted {
        Adopted {
            job: recorded.job_id.clone(),
            step: recorded.step_id.clone(),
            drone: recorded.drone_id.clone(),
            pid,
            started_at: StartedAt::carried(recorded.started_at.clone()),
            gap,
        }
    }

    pub fn job(&self) -> &JobId {
        &self.job
    }

    pub fn step(&self) -> &StepId {
        &self.step
    }

    pub fn drone(&self) -> &DroneId {
        &self.drone
    }

    pub fn pid(&self) -> u32 {
        self.pid.get()
    }

    pub fn gap(&self) -> &Gap {
        &self.gap
    }

    /// Whether the process has gone, **and it is still the same process**.
    ///
    /// The reading `crate::dispatch::reap` takes every turn for an adopted
    /// Drone, and it is [`crate::holder_of`] rather than the `waitid` a spawned
    /// Drone gets: `waitid` answers about a child, and this one is not Fleet's
    /// child — there is nothing to collect and nothing to be a zombie. The
    /// argument that made `holder_of` the wrong probe for a spawned Drone
    /// therefore does not reach here.
    ///
    /// **It re-checks the identity on every reading, and that is what it costs
    /// a fork for.** A pid that came round as somebody else's process reads as
    /// gone, which is the answer that ends the adoption rather than one that
    /// signals a stranger. A `ps` per turn per adopted Drone is the price, paid
    /// only on a road that exists because Fleet crashed.
    ///
    /// **A probe that would not run is not a Drone that stopped.** It comes
    /// back as the `io::Error` the caller escalates on, never as `true`.
    pub(crate) fn exited(&self) -> Result<bool, io::Error> {
        match self.still_ours()? {
            Some(_) => Ok(false),
            None => Ok(true),
        }
    }

    /// The process at this pid, if it is still the one that was adopted.
    fn still_ours(&self) -> Result<Option<StartedAt>, io::Error> {
        match holder_of(self.pid.get()).map_err(io::Error::other)? {
            Holder::Held(now) if now == self.started_at => Ok(Some(now)),
            _ => Ok(None),
        }
    }

    /// End it, and everything it started.
    ///
    /// **The fallback, and the only act on an adopted Drone that changes
    /// anything.** `kill_drone` and `kill_job` reach it through [`Session`]
    /// exactly as they reach a spawned Drone's `terminate`.
    ///
    /// **The identity is proved immediately before the signal, never once at
    /// adoption.** A spawned Drone's group is safe to signal because Fleet
    /// holds an uncollected child on the pid; there is no child here, so the
    /// proof is the reading — and a pid that is no longer this Drone's is left
    /// alone and said so.
    ///
    /// **`Ok` means the group was signalled, not that the process is gone.**
    /// Fleet cannot wait on something it did not spawn, so there is no wait to
    /// do and nothing to collect; the confirmation is the next turn's
    /// [`exited`](Adopted::exited). Saying more than that would be the one lie
    /// this whole module exists to avoid.
    async fn terminate(&self) -> Result<(), io::Error> {
        match self.still_ours()? {
            Some(_) => {
                end_the_group(self.pid);
                Ok(())
            }
            None => Err(io::Error::other(
                "the process at this Drone's pid is no longer the one that was adopted, so \
                 nothing was signalled — a group signal at a recycled pid ends somebody else's \
                 work",
            )),
        }
    }
}

/// The Drone in a slot: one Fleet started, or one it took back over.
///
/// **The seam, and the whole of what a caller has to know about the
/// difference.** Every act that speaks into a session goes through
/// [`LiveSession`] and gets the same six-way answer for an adopted Drone: there
/// is nothing to speak into. `crate::silence` already records a poke that would
/// not send, and `crate::converging` already abandons a turn over a directive
/// that would not go, so an adopted Drone degrades onto roads that exist and
/// are tested rather than onto a new one.
#[derive(Debug)]
pub enum Session {
    /// Fleet started this process and holds both its pipes.
    Spawned(DroneSession),
    /// Fleet found this process on the record and proved it was the same one.
    Adopted(Adopted),
}

impl Session {
    /// The process. What `crate::peer` indexes and what a person is shown.
    pub fn pid(&self) -> u32 {
        match self {
            Session::Spawned(session) => session.pid(),
            Session::Adopted(adopted) => adopted.pid(),
        }
    }

    /// The orphan behind this session, where it is one.
    ///
    /// **What reads it is the record**, not a caller deciding what to do: an
    /// act that has to branch on this is an act that should be a method on
    /// [`LiveSession`] instead.
    pub fn adopted(&self) -> Option<&Adopted> {
        match self {
            Session::Spawned(_) => None,
            Session::Adopted(adopted) => Some(adopted),
        }
    }

    /// Whether the Drone has gone, by whichever reading its kind allows. See
    /// [`DroneSession::exited`] and [`Adopted::exited`] for why they differ.
    pub(crate) async fn exited(&self) -> Result<bool, io::Error> {
        match self {
            Session::Spawned(session) => session.exited().await,
            Session::Adopted(adopted) => adopted.exited(),
        }
    }
}

/// **Six refusals and one act.** Nothing an adopted Drone is asked to listen to
/// can be delivered, and the one thing Fleet can still do to it is end it.
impl LiveSession for Session {
    type Error = io::Error;

    async fn tell(
        &self,
        turn: &OutcomeTurn,
        declaring: Option<&Declaring>,
    ) -> Result<(), io::Error> {
        match self {
            Session::Spawned(session) => session.tell(turn, declaring).await,
            Session::Adopted(_) => Err(io::Error::other(NOTHING_TO_SPEAK_INTO)),
        }
    }

    async fn notice(&self, drifted: &Redeclaring) -> Result<(), io::Error> {
        match self {
            Session::Spawned(session) => session.notice(drifted).await,
            Session::Adopted(_) => Err(io::Error::other(NOTHING_TO_SPEAK_INTO)),
        }
    }

    async fn redirect(&self, instruction: &Redirection) -> Result<(), io::Error> {
        match self {
            Session::Spawned(session) => session.redirect(instruction).await,
            Session::Adopted(_) => Err(io::Error::other(NOTHING_TO_SPEAK_INTO)),
        }
    }

    async fn interrupt(&self, directive: &ReportNow) -> Result<(), io::Error> {
        match self {
            Session::Spawned(session) => session.interrupt(directive).await,
            Session::Adopted(_) => Err(io::Error::other(NOTHING_TO_SPEAK_INTO)),
        }
    }

    async fn answer(&self, answer: &Answer) -> Result<(), io::Error> {
        match self {
            Session::Spawned(session) => session.answer(answer).await,
            Session::Adopted(_) => Err(io::Error::other(NOTHING_TO_SPEAK_INTO)),
        }
    }

    async fn poke(&self, nudge: &Poke) -> Result<(), io::Error> {
        match self {
            Session::Spawned(session) => session.poke(nudge).await,
            Session::Adopted(_) => Err(io::Error::other(NOTHING_TO_SPEAK_INTO)),
        }
    }

    async fn terminate(&self) -> Result<(), io::Error> {
        match self {
            Session::Spawned(session) => session.terminate().await,
            Session::Adopted(adopted) => adopted.terminate().await,
        }
    }
}

/// What the record's Drone turns out to be, once the machine has been asked.
///
/// **Three answers and no fourth**, which is the correction this whole change
/// makes to `Fleet::reconcile`: it used to have one, `Ending::Vanished`, stated
/// about every Drone the restarting Fleet had not started.
#[derive(Debug)]
pub enum Reattachment {
    /// The process is there and is the same one. It is Fleet's again.
    Adopted(Box<Adopted>),
    /// Nothing holds the pid, or what holds it started at a different instant
    /// and is therefore a different process. **The Drone is gone**, and this is
    /// the answer reconciliation always gave — now having asked.
    Gone,
    /// The machine could not be asked, or the record does not say enough to
    /// ask. **Not folded into [`Gone`](Reattachment::Gone)**: a probe that
    /// failed is not a Drone that stopped, and answering "gone" on no evidence
    /// is how a live Drone gets left unowned. The fallback the owner kept
    /// applies — the step is stopped and a person decides — and this carries
    /// the sentence that says which of the two happened and why.
    Unknown { because: String },
}

/// Whether the Drone the record names is still there.
///
/// **The probe, and it is a free function so a test can reach it without a
/// Fleet.** `now` is injected like every instant in this crate; `last_heard` is
/// the transcript's own last row, read by the caller because reading a file is
/// not this function's job.
pub fn reattaching(
    recorded: &DroneProcess,
    last_heard: Option<Timestamp>,
    now: Timestamp,
) -> Reattachment {
    let Some(pid) = NonZeroU32::new(recorded.pid) else {
        // The store refuses a pid of zero, so this is a file written by
        // something else. It is `Unknown` and not `Gone` for the reason
        // `Unknown` gives: nothing was asked, so nothing is known.
        return Reattachment::Unknown {
            because: String::from("the recorded pid is zero, which names no process"),
        };
    };
    match holder_of(pid.get()) {
        Ok(Holder::Held(now_started)) if now_started.as_str() == recorded.started_at => {
            Reattachment::Adopted(Box::new(Adopted::of(
                recorded,
                pid,
                Gap {
                    from: last_heard,
                    until: now,
                },
            )))
        }
        // Held by something that started at a different instant. **The pid came
        // round**, which is the case a bare pid could not tell from an adoption
        // — and the one where acting on it would signal a stranger.
        Ok(Holder::Held(_)) | Ok(Holder::Vacant) => Reattachment::Gone,
        Err(probe) => Reattachment::Unknown {
            because: format!("{probe}"),
        },
    }
}
