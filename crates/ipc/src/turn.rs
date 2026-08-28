//! One Drone's turns: what the transcript holds, and what a viewer is sent.
//!
//! # One declaration, two readers
//!
//! [`TranscriptRow`] is written to `.armada/transcripts/<drone-id>.jsonl` by
//! Fleet and read back from it by the backfill. It is here rather than beside
//! the writer because the live view sends the same shape, and two declarations
//! would be two vocabularies that agree until one of them changes.
//!
//! **The mapping from `adapter_traits::DroneEvent` is not here.** `ipc` depends
//! on `core-model` and nothing else by design, so the conversion stays in
//! `fleet`, beside the loop that holds the event — which is also where a
//! variant added to `DroneEvent` fails to compile.
//!
//! # A withheld row has no constructor
//!
//! `docs/concepts/observe.md` withholds three kinds from a viewer that the file
//! still records. [`Shown`] is the narrowing, and it is a type rather than a
//! filter at the call site: a row the design withholds cannot be put on the
//! wire by somebody forgetting to check.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::event::Missed;
use crate::ids::{Instant, JobId, StepId};
use crate::version::ProtocolVersion;

/// One line of `.armada/transcripts/<drone-id>.jsonl`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptRow {
    /// When Fleet's line loop saw it, not when it reached the disk.
    pub ts: Instant,
    /// The step that was running when Fleet saw it — **not the step the Job is
    /// on when the row is read back**. One Drone works several steps, so a row
    /// that took its label at spawn would say the first step for the whole of a
    /// four-step Job.
    ///
    /// A label on a row and never a range: a step can run more than once, so
    /// the same id may appear, stop appearing, and appear again.
    ///
    /// **`None` is a row written before this field existed**, whose true step
    /// nobody can recover. It is not a step that was unknown at the time, and
    /// nothing relabels it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    #[serde(flatten)]
    pub saw: Saw,
}

/// The row vocabulary. Tagged `event` rather than `kind`, because
/// `Unrecognised` already carries a `kind` and a key cannot be two things.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Saw {
    Started {
        session: String,
        model: String,
        mcp_servers: usize,
    },
    Called {
        tool: String,
        call: String,
        /// What the call was on — the path, the command, the pattern. Bounded
        /// where it is built, in `adapter_traits::CallDetail`; empty where the
        /// vocabulary had no name for the tool's arguments.
        detail: String,
        /// The detail was longer than a row carries. **Said rather than
        /// implied** — a command can legitimately end in an ellipsis.
        truncated: bool,
    },
    Answered {
        call: String,
        failed: bool,
    },
    Said {
        text: String,
    },
    Refused {
        tool: String,
        call: String,
        because: String,
    },
    /// Dispatch gating rather than this Job's business. Recorded, withheld.
    QuotaMoved {
        window: String,
        status: String,
    },
    /// The Job's rail states the cost and the turn count, so a row restating
    /// them would be a second answer to a question already answered.
    Ended {
        turns: u32,
        cost_micros: u64,
        refusals: usize,
    },
    Unrecognised {
        kind: String,
    },
    Unreadable {
        line: String,
        why: String,
    },
    /// The sink saying how much of the record is not there. **Never a
    /// `DroneEvent`**, and a viewer's losses are its subscription's rather than
    /// this one's — which is why it is withheld.
    Missed {
        rows: u64,
    },
}

/// A row the design shows a viewer.
///
/// [`Shown::of`] is the only way to make one and it refuses what
/// `observe.md` withholds, so the wire cannot carry a withheld row by
/// oversight. Decoding goes through the same check, so a peer cannot mint one
/// either.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TranscriptRow", into = "TranscriptRow")]
pub struct Shown(TranscriptRow);

impl Shown {
    /// The row where a viewer may see it, `None` where it is withheld.
    pub fn of(row: TranscriptRow) -> Option<Shown> {
        Shown::try_from(row).ok()
    }

    pub fn row(&self) -> &TranscriptRow {
        &self.0
    }
}

/// A row that is recorded and not shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Withheld;

impl fmt::Display for Withheld {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("a row the transcript keeps and a viewer is not shown")
    }
}

impl TryFrom<TranscriptRow> for Shown {
    type Error = Withheld;

    /// **No `_` arm.** A kind added to [`Saw`] fails to compile here rather
    /// than defaulting into a viewer's window or out of it.
    fn try_from(row: TranscriptRow) -> Result<Shown, Withheld> {
        match row.saw {
            Saw::QuotaMoved { .. } | Saw::Ended { .. } | Saw::Missed { .. } => Err(Withheld),
            Saw::Started { .. }
            | Saw::Called { .. }
            | Saw::Answered { .. }
            | Saw::Said { .. }
            | Saw::Refused { .. }
            | Saw::Unrecognised { .. }
            | Saw::Unreadable { .. } => Ok(Shown(row)),
        }
    }
}

impl From<Shown> for TranscriptRow {
    fn from(shown: Shown) -> TranscriptRow {
        shown.0
    }
}

/// One message on a Job's Observe socket.
///
/// **Not [`crate::StreamMessage`], and not on `/events`.** That stream is one
/// drop-oldest channel carrying every Job, so a transcript row on it would
/// evict the state changes a Board is drawn from and cost a full resync each
/// time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum TurnMessage {
    /// The first message on every connection, before any row.
    Opened(Opened),
    Row(Shown),
    /// The subscription fell behind and lost rows. **Not the sink's loss** —
    /// what the sink dropped is in the file as a `missed` row and is not shown.
    Missed(Missed),
    /// Nothing more is coming, and why. Sent before the socket closes, because
    /// a socket that simply ends is indistinguishable from one that broke.
    Closed(Closed),
}

/// What this connection is, and what it is about to send.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opened {
    /// Restated on the socket so a client that reached it without reading the
    /// runtime file still learns what it is talking to.
    pub protocol_version: ProtocolVersion,
    pub job_id: JobId,
    /// Whether a Drone was writing rows when this opened. `false` is ordinary:
    /// the Job may not have been dispatched, or its Drone may have gone with
    /// the Fleet that spawned it.
    pub live: bool,
    /// Older rows the history left out. The backfill is bounded, and a viewer
    /// that is not told would read a truncated history as the whole one.
    pub skipped: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Closed {
    pub because: Silence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Silence {
    /// The Drone that was writing has finished. The history is complete.
    DroneEnded,
    /// Nothing was writing when this opened, so the history is all there is.
    NothingWriting,
}
