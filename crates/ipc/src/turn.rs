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
//! `docs/concepts/observe.md` withholds two kinds from a viewer that the file
//! still records — dispatch gating, and the sink's own losses. [`Shown`] is the
//! narrowing, and it is a type rather than a filter at the call site: a row the
//! design withholds cannot be put on the wire by somebody forgetting to check.
//!
//! A third was withheld until somebody checked the reason. [`Saw::Ended`]'s
//! cost and turn count were held back because the Job's rail was said to state
//! them, no rail ever did, and nothing else on the wire carries either — so
//! what a Drone spent was reachable from nowhere.
//!
//! # Three voices, because the record was always three and carried one
//!
//! A step is a conversation: Armada opens it with an instruction, the Drone
//! works, and Fleet runs the Checks and reads what came out. Only the middle
//! one was written down, so a surface drawing the step's story could say what
//! the Drone did and nothing about what it had been asked or what was made of
//! it. [`Voice`] is which of the three a row is, and the rows Fleet authors
//! itself are [`Saw::Instructed`], [`Saw::Checked`] and [`Saw::Produced`].
//!
//! **The transcript is where they go, and not a new channel.** It is already
//! per-step and per-Drone, already durable, and already read back in order by
//! the backfill — so an instruction and the turns it produced arrive
//! interleaved, which is the whole of what a reader is trying to see. A second
//! record would have to be merged against this one by instant, and two clocks
//! is how the merge goes wrong.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::checks::CheckRun;
use crate::event::{ChangedFile, Missed};
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
    /// Whose row this is.
    ///
    /// **Defaulted rather than optional, and the default is the truth.** Every
    /// row written before this field existed decoded from a Drone's own output,
    /// so a file read back without it is read back correctly — which is not a
    /// convenience, it is the reason a `bool`-style absence is safe here where
    /// [`step`](TranscriptRow::step)'s is not. It is always written, so nothing
    /// on the wire has to guess.
    #[serde(default)]
    pub by: Voice,
    #[serde(flatten)]
    pub saw: Saw,
}

/// Who a row is. The three actors a step's story has.
///
/// **Not [`Actor`](crate::Actor).** That one names who caused a *transition* —
/// a person, Helm or Fleet — and is the answer to "who is accountable". This
/// names who is speaking in a transcript, where the interesting distinction is
/// between what Armada told the Drone and what Fleet did about the result. A
/// registry declares neither set as this one, so it is a plain enum for
/// [`Silence`]'s reason.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Voice {
    /// Armada speaking into the Drone's session: the instruction a step opened
    /// with, a person's redirect carried in, an answer to a question, a nudge.
    Armada,
    /// The Drone's own output, decoded. **The default**, because every row
    /// written before this field existed is one.
    #[default]
    Drone,
    /// Fleet acting on the Job around the Drone: a Check it ran, the reading it
    /// took of the worktree.
    Fleet,
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
    /// The last row a Drone writes: what the run cost, how many turns it took,
    /// and how many of them the harness refused.
    ///
    /// **Spend crosses here and nowhere else.** A Job's rail carries a step's
    /// state and how long it took, and no DTO in this crate has a cost field —
    /// so a Job that burned a dollar over forty turns and produced nothing read
    /// exactly like one that gave up in four.
    ///
    /// **A run's own total and never a Job's.** A Job that retried has a row
    /// per Drone, and adding them up is the reader's — which keeps the Job-wide
    /// figure a question somebody decides rather than a number this crate
    /// invents.
    Ended {
        turns: u32,
        cost_micros: u64,
        refusals: usize,
    },
    /// A turn Armada put into the Drone's session, whole.
    ///
    /// **Chapter one of a step's story, and it was reachable from nowhere.**
    /// `session::Turn` is the only shape that goes down the pipe and it was
    /// written and dropped, so what a step opened with — the brief, the
    /// deliverable, what moved on the base — existed on this machine only
    /// inside a process that had already exited.
    ///
    /// The text is not bounded. It is Fleet's own rendering of a template it
    /// holds, not something a Drone chose the size of, and truncating the one
    /// thing the Drone was asked to do would make the row unreadable for
    /// exactly the question it exists to answer.
    Instructed {
        /// Which turn this was — `opening`, `outcome`, `redirect`, `answer`,
        /// `drift`, `report`, `poke` — spelled as `crates/fleet/src/session.rs`
        /// names the constructor that built it.
        ///
        /// A string rather than a closed set, for the reason
        /// [`JudgeInFlight::look`](crate::JudgeInFlight) is one: **no registry
        /// declares this set.** It is decided by the code that sends the turns,
        /// and a mirrored enum here would be a second authority for a list that
        /// has exactly one.
        occasion: String,
        /// What the Drone was told, exactly as it was told it.
        text: String,
    },
    /// One declared Check, as Fleet ran it.
    ///
    /// **The Drone never runs these and that is the point of them** — a Drone
    /// reporting its own tests is a claim rather than a result — so a Check
    /// appears nowhere in a Drone's own output and the activity log had no way
    /// to show that anything mechanical had happened at all.
    ///
    /// It carries a [`CheckRun`], which is the same value
    /// [`StepDetail::check_runs`](crate::StepDetail) carries: one vocabulary
    /// for what a Check did, whether it is read on the step panel or in the log.
    Checked {
        run: CheckRun,
    },
    /// What the step's work came to, as Fleet read the worktree.
    ///
    /// **A reading taken at a step boundary, and the only per-step one there
    /// is.** `JobFootprint` is the Job's whole work at the instant it stopped
    /// and `job.files_changed` is a live reading with no boundary attached, so
    /// neither could say which step produced what. This row is stamped with its
    /// step like every other, which is what makes the answer per-step without a
    /// column for it.
    ///
    /// **The names, never the bytes** — [`ChangedFile`]'s own rule. What
    /// changed inside a file is `get_diff`, on the one route that spends it.
    Produced {
        files: Vec<ChangedFile>,
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
            Saw::QuotaMoved { .. } | Saw::Missed { .. } => Err(Withheld),
            Saw::Started { .. }
            | Saw::Called { .. }
            | Saw::Answered { .. }
            | Saw::Said { .. }
            | Saw::Refused { .. }
            | Saw::Ended { .. }
            | Saw::Instructed { .. }
            | Saw::Checked { .. }
            | Saw::Produced { .. }
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
