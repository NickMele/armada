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
//! variant added to `DroneEvent` fails to compile. Three kinds map from none
//! at all; [`Voice`] tells them apart.
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
//! them; no rail ever did, so a Drone's spend was reachable from nowhere.

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

impl TranscriptRow {
    /// The row without the parts the file keeps and a viewer is not sent.
    ///
    /// Today that is one field: the whole of a long call argument. **The
    /// narrowing is by field and not by row**, which is what [`Shown`] alone
    /// could not do — a `called` row is shown, and the file's copy of a
    /// fourteen-thousand-character heredoc is not.
    ///
    /// It is called twice, and deliberately. [`Shown`] calls it so nothing can
    /// reach the socket carrying a whole, and the backfill calls it as it reads
    /// so a history of two thousand rows never holds two thousand arguments in
    /// memory to strip them one at a time on the way out.
    pub fn for_a_viewer(mut self) -> TranscriptRow {
        if let Saw::Called { whole, .. } = &mut self.saw {
            *whole = None;
        }
        self
    }
}

/// Who a row is. The three actors a step's story has.
///
/// A step is a conversation: Armada opens it with an instruction, the Drone
/// works, and Fleet runs the Checks and reads what came out. **Only the middle
/// one was written down**, so a surface drawing the story could say what the
/// Drone did and nothing about what it had been asked or what was made of it.
/// [`Saw::Instructed`], [`Saw::Checked`] and [`Saw::Produced`] are the rows
/// Fleet authors, and this is what tells them from the Drone's.
///
/// **They go in the transcript rather than on a channel of their own.** It is
/// already per-step, already durable, already read back in order — so an
/// instruction and the turns it produced arrive interleaved, which is the whole
/// of what a reader is looking for. A second record would have to be merged
/// against this one by instant, and two clocks is how that goes wrong.
///
/// **Not [`Actor`](crate::Actor).** That one names who caused a *transition* —
/// a person, Helm or Fleet — and answers "who is accountable". This names who
/// is speaking, where the distinction that matters is between what Armada told
/// the Drone and what Fleet did about the result. No registry declares either
/// set as this one, so it is a plain enum for [`Silence`]'s reason.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Voice {
    /// Armada speaking into the Drone's session: the instruction a step opened
    /// with, a person's redirect carried in, an answer to a question, a nudge.
    ///
    /// **Two rows carry it and they are not duplicates.**
    /// [`Saw::Instructed`] is what Fleet wrote and sent, with the occasion it
    /// sent it for; a [`Saw::Said`] in this voice is the Drone's own stream
    /// replaying the same words back off its input channel, which is where a
    /// turn the harness itself wrote also lands. A surface asking what Armada
    /// told a Drone reads the first; one drawing the conversation in order
    /// draws both.
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
    /// The Drone reached for a tool, and what it reached for it with.
    ///
    /// **Three fields about one argument, because a row and an opened row ask
    /// different things.** `detail` is the line; `detail_length` is how much
    /// there is; `whole` is the rest, kept by the file and never sent.
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
        /// How many characters the argument had, before anything was cut.
        ///
        /// **What `truncated` on its own could not say.** A flag makes a row
        /// report an absence; a size makes it report a proportion, so an opened
        /// row reads *showing 200 of 14,320 characters* and offers the rest
        /// through `get_call`.
        ///
        /// **`None` is a row written before this field existed**, whose true
        /// size nobody can recover — the same absence
        /// [`step`](TranscriptRow::step) carries, for the same reason, and not
        /// an argument measured at nought.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail_length: Option<usize>,
        /// The argument as the Drone sent it, uncollapsed.
        ///
        /// **The file's, never a viewer's.** [`Shown`] drops it, so the socket
        /// carries the bounded row whatever the argument's size — the stream is
        /// bounded and lossy by design, and a row big enough to evict its
        /// neighbours loses the short form too. What a person opens instead
        /// comes back over HTTP, once, for the one call they asked about.
        ///
        /// Present exactly where `truncated` is true and the row was written
        /// since the file kept it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        whole: Option<String>,
    },
    Answered {
        call: String,
        failed: bool,
    },
    /// Prose that crossed the session. **Whose is
    /// [`TranscriptRow::by`](TranscriptRow::by) and not a field here** — the
    /// row already stamps every kind, and a second speaker on one variant would
    /// be two answers to one question.
    ///
    /// Every one of these used to be the Drone's, because the decoder discarded
    /// the channel the line arrived on. So the brief a step opened with was
    /// written down as the Drone's own prose and drew as the longest thing on
    /// the pane — `adapter_traits::Speaker` is where that fact is now carried,
    /// and `fleet::transcript::row` is where it becomes this row's voice.
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
        /// Which lines of `text` its writer wrote as block headings,
        /// zero-based, in order.
        ///
        /// **The one fact about the turn's shape that only the writer has.**
        /// `fleet::briefing` writes every block of a brief as a heading, a
        /// blank line, then the body, and that shape is known in the `format!`
        /// that writes it and nowhere afterwards. A surface handed the text
        /// alone can only guess — the first line of a block, or a line in
        /// capitals — and both guesses are wrong on text `briefing` already
        /// writes: its baseline opens with prose, and what the part before
        /// produced opens its block with a sentence. So a short body line
        /// would draw as a heading and nothing would catch it.
        ///
        /// **Line indices rather than the heading strings.** The two are
        /// written in one act from one string, so an index names exactly one
        /// line, where a set of strings would also mark a body line that
        /// happened to repeat a heading.
        ///
        /// **Empty is a turn with no headed blocks, and also a row written
        /// before this field existed** — the two are the same to a reader, and
        /// deliberately: what an older row rendered as is what an unheaded turn
        /// renders as. Unlike [`step`](TranscriptRow::step) there is nothing
        /// unrecoverable in the absence, because every turn but the opening
        /// brief is one block of prose.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        headings: Vec<usize>,
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
///
/// **It narrows fields as well as kinds.** Every row that reaches this is put
/// through [`TranscriptRow::for_a_viewer`], so the whole of a long call
/// argument stays in the file whatever the caller hands over — the socket's
/// bound is a property of the type rather than of somebody remembering.
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

/// One tool call's arguments, as the record holds them.
///
/// **The other half of [`Saw::Called`], and the reason a cut row is not a dead
/// end.** The socket carries a line and a size; this carries the argument, and
/// it is asked for once, by a person who opened one row. The split is the one
/// `get_diff` already makes against `job.files_changed`: the cheap fact streams
/// and the bytes are fetched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallArguments {
    /// The tool, as its own vocabulary spells it.
    pub tool: String,
    /// The call id the row carried, which is what was asked for.
    pub call: String,
    /// The argument, as the Drone sent it — whitespace and newlines intact,
    /// because a heredoc read as one line is not the thing that was run.
    pub arguments: String,
    /// Whether [`arguments`](CallArguments::arguments) is all of it.
    ///
    /// **False only where the record itself is short**: a row written before
    /// the file kept the whole carries the bounded line and nothing behind it.
    /// It is stated rather than inferred from the two lengths agreeing, because
    /// a surface that inferred it would call a partial answer complete on any
    /// row where the count was also missing.
    pub whole: bool,
    /// How many characters the argument had, where the record knows.
    ///
    /// `None` is an old row again — not an argument of no length. A surface
    /// holding `whole: false` and `length: None` has what there is and no way
    /// to say how much is missing, and says that rather than inventing a size.
    pub length: Option<usize>,
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
            | Saw::Unreadable { .. } => Ok(Shown(row.for_a_viewer())),
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
