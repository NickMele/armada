//! A step's Checks while the gate is running them, and the socket a running
//! Check's log is read over as it grows.
//!
//! **A view of the run, never a second record of it.** What the gate rules on
//! and what `job_step_checks` holds are written after the ruling, exactly as
//! they were before any of this crossed. Nothing here is stored, nothing here
//! is read back by the gate, and all of it is gone the moment the ruling is
//! written down — [`JudgeInFlight`](crate::JudgeInFlight)'s terms one tier
//! along.
//!
//! # Not a step state
//!
//! `domain/step-states.toml` declares six, a seventh is a variant the other
//! side matches on, and it would be the wrong fact anyway: a step whose gate is
//! running its Checks is still `running`, and it stops without moving. So this
//! rides beside the state, which is `job.judging`'s shape and its reason.
//!
//! # Three states and no word for any of them
//!
//! A Check is waiting, running or finished, and **the fields say which**:
//! no [`started_at`](CheckUnderway::started_at) is waiting, a start and no
//! [`ran`](CheckUnderway::ran) is running, and a `ran` is finished. A closed
//! set spelled beside them would be a second statement of the same fact, and
//! the two would disagree the first time one of them was set without the other.

use serde::{Deserialize, Serialize};

use crate::checks::CheckRun;
use crate::ids::{Instant, JobId};
use crate::version::ProtocolVersion;

/// A step's Checks, while the gate is running them.
///
/// **Present from the moment the gate starts its Checks until the ruling is
/// written down**, and not only while a Check is running. A Check that finished
/// three minutes before the Judge answered is a result a person can read in
/// those three minutes, and `check_runs` does not hold it until the ruling
/// does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecksUnderway {
    /// Which run of the step these belong to, counted from one. The same
    /// ordinal [`CheckRun::attempt`] carries, so a surface narrowing to the
    /// current run narrows this the same way.
    pub attempt: u32,
    /// Every Check the step declares, in the step's order — waiting ones
    /// included, because the shape of what is still coming is part of reading
    /// a running gate.
    pub checks: Vec<CheckUnderway>,
}

/// One declared Check, as the gate has it right now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckUnderway {
    /// The Manifest Check's name, or the built-in's kind where it names none.
    /// The word [`CheckRun::name`] carries, so the two line up.
    pub name: String,
    /// When it started. **Absent while it waits** — for one of the gate's
    /// slots, or for the Commands it `requires`. A surface counts the elapsed
    /// time from here; nothing ticks on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Instant>,
    /// How long it took, in milliseconds, once it has finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<u64>,
    /// What it came to, once it has finished: the row the ruling will write,
    /// spelled exactly as [`CheckRun`] spells it. **Absent while it waits or
    /// runs.** Its `output_path` is absent too — the live log is
    /// [`output_path`](CheckUnderway::output_path) below, and the recorded one
    /// does not exist until the ruling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ran: Option<CheckRun>,
    /// Where this Check's log is being written as it runs, relative to the
    /// repository root. **The last component is what `observe_check_output`
    /// takes**, the way `get_check_output` takes a recorded one's.
    ///
    /// Absent on a Check that runs no command, on one still waiting, and where
    /// the file could not be opened — the Check runs either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
}

/// One message on a running Check's log socket.
///
/// **`observe_job_log`'s shape, one file over.** What the log already holds,
/// then what is appended to it, then a sentence saying why it stopped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum OutputMessage {
    Opened(OutputOpened),
    Lines(OutputLines),
    Closed(OutputClosed),
}

/// The first message: whose log this is, and what the first read left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputOpened {
    pub protocol_version: ProtocolVersion,
    pub job_id: JobId,
    /// The Check whose log this is, spelled as [`CheckUnderway::name`] spells
    /// it. **Off the answer rather than off the row that was pressed**, for
    /// `CheckOutput::name`'s reason.
    pub name: String,
    /// Which run of the step is writing it.
    pub attempt: u32,
    /// Where it is being written, relative to the repository root.
    pub path: String,
    /// Older lines the first read left out, because the window is bounded.
    /// **Said rather than implied**: a shortened log nobody was told about
    /// reads as the whole one.
    pub skipped: u64,
}

/// Lines written since the last message, oldest first, verbatim.
///
/// **Whole lines only, until the end.** A line still being written is held
/// back until its newline arrives, so a reader never shows half a line that
/// the next message would have to take back. The last line of a Check that
/// ended without a newline comes on the final pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputLines {
    pub lines: Vec<String>,
}

/// The end of the stream, said rather than left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputClosed {
    pub because: OutputEnded,
}

/// Why a running Check's log stopped arriving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputEnded {
    /// The Check ended, and every line it wrote has been sent.
    Finished,
    /// The file is there and could not be read.
    Unreadable,
}
