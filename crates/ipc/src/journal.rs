//! What Fleet did to a Job, as the Job's own log recorded it.
//!
//! **The third voice, and it reached nowhere.** `ActivityLog` is specified as
//! one stream carrying the Drone's turns, Armada's injected turns and Fleet's
//! own events. Two travelled [`observe_job`](crate::TurnMessage); the third
//! went to `.armada/logs/<handle>.jsonl` and was read by nothing, so a Job with
//! no Drone on it drew a blank panel for the whole of preparation. **The
//! missing half of a surface designed whole**, not a new record.
//!
//! **A note is not the envelope.** `run_id`, `target`, `span`, `workspace` and
//! `component` are joining keys for somebody with `jq`, and none of them is
//! what a person watching a Job is asking. `fleet::journal` is the conversion,
//! and where a field added to the envelope is carried or left behind.
//!
//! **Every note names who.** [`LogNote::by`] is on the wire and is [`Voice`],
//! the same closed set a turn carries, so a surface folding the two streams
//! into one column attributes both the same way. Fleet is the only writer of a
//! Job's log today, which is exactly why the field is stated.
//!
//! **Every note opens to its payload.** [`NotedField`] carries the envelope's
//! `fields`, because a Fleet event drawn as grey prose in a column of openable
//! entries is a second-class citizen in a stream claiming one grammar.
//!
//! Nothing here is bounded per note: every `msg` and every value is Fleet's own
//! writing, so there is nothing to cut and no second route to fetch.

use serde::{Deserialize, Serialize};

use crate::ids::{DroneId, Instant, JobId, StepId};
use crate::turn::Voice;
use crate::version::ProtocolVersion;

/// How bad a note is, in the envelope's own five.
///
/// **The spelling is `core_model::Level::as_wire`'s and not a second one.**
/// This enum's `serde` name is checked against it by a test in
/// `crates/ipc/src/tests`, so the two cannot drift without something failing —
/// which is the only honest way to mirror a set whose authority is a type this
/// crate cannot `impl` a deserializer for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteLevel {
    Trace,
    Debug,
    /// **The default**, and the reading of a line whose level this Bridge does
    /// not know. A level nobody can place is still a thing that happened, and
    /// dropping the note would lose the event to keep the adjective.
    #[default]
    Info,
    Warn,
    Error,
}

/// One name-and-value out of a note's `fields`.
///
/// **Values are strings, whatever the envelope held.** `FieldValue` is a
/// number, a flag, a string or a list of them, and a surface draws all four the
/// same way — as text beside a name. A tagged union on the wire would make
/// every reader branch to reach a string it was always going to render.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotedField {
    pub name: String,
    pub value: String,
}

/// One line of `.armada/logs/<handle>.jsonl`, as a viewer is shown it.
///
/// **Absent, never present-and-null**, the rule `crate::detail` states and
/// `docs/concepts/log-envelope.md` owns: a client handed `step: null` cannot
/// tell "this belongs to no step" from "Fleet forgot to stamp it", and the
/// first of those is the whole reason this stream exists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogNote {
    /// When Fleet wrote it. The envelope's `ts`.
    pub at: Instant,
    /// Whose note this is. See the module: stated, never assumed.
    pub by: Voice,
    pub level: NoteLevel,
    /// The one line, exactly as Fleet wrote it. **Never carries an interpolated
    /// id** — that is the envelope's own rule, and it is why the ids below are
    /// fields rather than something a surface parses out of the sentence.
    pub msg: String,
    /// The step the note was written under.
    ///
    /// **Absent is the case this stream was built for.** Fleet cutting a
    /// worktree, running a repository's preparation commands or reclaiming a
    /// Job belongs to no step — there is no step running yet, and attaching
    /// these to the one about to start would read as a step that has begun when
    /// it has not. So a note with no step is a Job-level note, and the surface
    /// draws it in a section of the Job's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    /// The Drone the note is about, where it is about one. A retry is a second
    /// `drone_id` under one Job, exactly as the envelope has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone: Option<DroneId>,
    /// What the note opens to. Empty is a note that carried no structured data,
    /// and the row draws closed rather than offering to open nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<NotedField>,
}

/// `get_job_log`: what Fleet did to one Job, read back once as a window that
/// says it is one.
///
/// **The backfill without the tail.** [`JournalMessage`] is the lines already
/// written followed by the ones that come next, and only the second half needs
/// a socket. The stream is Bridge's because an agent cannot be interrupted
/// mid-turn — reasoning about the tail, which left the settled half reachable
/// by nobody who was not watching a screen.
///
/// **A second operation and never a widening of `observe_job_log`**, whose row
/// would then read `Yes` for its backfill and `No` for its tail and mean two
/// things at once.
///
/// **[`crate::RunOutput`]'s shape, one file over**: the tail of the record,
/// numbered as the record numbers it, with the facts that say it is a window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobLog {
    pub job_id: JobId,
    /// Where it was read from, relative to `ManifestSummary::records_root`.
    /// [`crate::CheckOutput::path`]'s field and its reason: a reader drawing
    /// the path beside the reading takes it off the answer rather than
    /// composing it and hoping the two agree.
    pub path: String,
    /// The window, oldest note first. **The tail**, which is
    /// [`crate::RunOutput`]'s choice one file over: what a person opens a Job's
    /// log to find out is what Fleet did to it last.
    pub notes: Vec<LogNote>,
    /// Which note [`notes`](JobLog::notes)`[0]` is in the whole log, counted
    /// from one. **The log's own numbering and never the window's**, because a
    /// citation names the record.
    pub from_note: u32,
    /// How many notes the log holds. Counted by reading it, so it is exact
    /// even where the window is not the whole.
    pub total_notes: u32,
    /// Lines the log holds that would not decode, and so are notes in no
    /// count above.
    ///
    /// **Counted rather than dropped in silence**, which is the rule the
    /// stream's reader already keeps: every line here was an event, and a gap
    /// nobody was told about reads as a Job nothing was happening to.
    pub undecodable: u32,
    /// What the log weighs, in bytes.
    pub bytes: u64,
    /// Whether [`notes`](JobLog::notes) is all of them.
    ///
    /// **Stated rather than inferred from `from_note == 1`**, which is
    /// [`crate::CheckOutput::whole`]'s reason: a surface deriving completeness
    /// from another field would call a partial answer whole the first time that
    /// field's meaning moved.
    pub whole: bool,
    /// The log is there and Fleet could not read the whole of it.
    ///
    /// **[`Quiet::Unreadable`] as a settled answer**, and never a Job with no
    /// log yet — that one is ordinary and comes back with no notes at all. What
    /// was read is still carried: a refusal here would throw away the notes
    /// that did read, on the one call somebody makes when something is wrong.
    pub unreadable: bool,
}

/// The first message on a Job's log socket: what the reader is about to be
/// handed, and what it left behind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalOpened {
    pub protocol_version: ProtocolVersion,
    pub job_id: JobId,
    /// Older notes the first read left out, because the window is bounded.
    ///
    /// **Said rather than implied**, for [`crate::Opened`]'s reason: a
    /// shortened history nobody was told about reads as the whole one.
    pub skipped: u64,
}

/// Why the stream stopped.
///
/// One value, because one thing produces one. **A kind exists when something
/// produces it** — `docs/practices/protocol.md` — so this is not stubbed with
/// the endings that might one day exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quiet {
    /// The log is there and Fleet could not read it. **Not a Job with no log
    /// yet**, which is ordinary and answers with no notes at all.
    Unreadable,
}

/// The end of the stream, said rather than left.
///
/// A socket that simply stops is indistinguishable from one that broke — the
/// same argument [`crate::Closed`] carries, and the reason both sockets on this
/// seam end with a sentence rather than a `close` frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalClosed {
    pub because: Quiet,
}

/// One message on a Job's log socket.
///
/// **Not [`crate::TurnMessage`], and a socket of its own.** That one is a
/// Drone's transcript and exists only while a Drone is writing: `observe_job`
/// answers `nothing_writing` and closes on exactly the Job this stream is for.
/// Folding these notes into it would also mean a new `Saw` variant, which
/// `docs/practices/protocol.md` makes a **major** bump — an old Bridge's
/// `switch` has no arm for it and falls into the one it does have.
///
/// **Not `/events` either.** That stream is one drop-oldest channel of fixed
/// capacity carrying every Job, and an eviction there is a full resync of every
/// Job rather than a lost row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum JournalMessage {
    Opened(JournalOpened),
    /// A note on the wire is its own fields beside `"message": "note"` rather
    /// than nested under a key — the tag is internal, as it is on
    /// [`crate::TurnMessage`], so one line is one flat object a person can read
    /// with `jq` and no wrapper to reach through.
    Note(LogNote),
    Closed(JournalClosed),
}
