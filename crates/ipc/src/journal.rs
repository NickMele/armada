//! What Fleet did to a Job, as the Job's own log recorded it.
//!
//! # The third voice, and it reached nowhere
//!
//! `packages/components/src/compositions/ActivityLog/ActivityLog.tsx` opens by
//! saying the log is one stream carrying the Drone's turns, Armada's injected
//! turns and Fleet's own events. Two of the three travelled
//! [`observe_job`](crate::TurnMessage); the third was written to
//! `.armada/logs/<job-id>.jsonl` and read by nothing. So a Job with no Drone on
//! it — cutting a worktree, installing dependencies, being reclaimed — showed
//! an empty activity log for however long that took, which is exactly the span
//! somebody opens it for. **This is the missing half of a surface that was
//! designed whole**, not a new record: `docs/concepts/log-envelope.md` already
//! owns the line and Fleet already writes it.
//!
//! # Notes, not envelopes
//!
//! A [`LogNote`] is not the envelope. The envelope carries `run_id`, `target`,
//! `span`, `workspace` and a `component` that names the emitter — machine
//! joining keys for somebody reading the file with `jq`, and none of them is
//! what a person watching a Job is asking. What crosses is the instant, the
//! voice, how bad it is, the one line, the ids that place it, and the fields
//! flattened to strings a surface can draw. The conversion is Fleet's, in
//! `fleet::journal`, and it is where a field added to the envelope is either
//! carried or deliberately left behind.
//!
//! # Every note names who
//!
//! [`LogNote::by`] is on the wire rather than assumed by the reader, and it is
//! [`Voice`] — the same closed set an `observe_job` row carries — so a surface
//! folding the two streams into one column attributes both the same way.
//! Fleet is the only writer of a Job's log today, which is exactly why the
//! field is stated: a value everybody agrees on by convention is the one that
//! goes wrong silently when a second writer arrives.
//!
//! # A note opens to its fields
//!
//! Every other entry in the activity log opens to its payload. A Fleet event
//! rendered as a line of grey prose would be a second-class citizen in a stream
//! whose whole claim is one grammar, so [`NotedField`] carries the envelope's
//! `fields` as name-and-value pairs — the pid, the branch, the paths, the
//! counts — and the surface draws them as the payload the row opens to.
//!
//! # Nothing here is bounded per note
//!
//! `Saw::Called` cuts a long argument because a Drone chooses its size. Every
//! `msg` and every field value on this stream is Fleet's own writing, from a
//! `format!` Fleet holds, so there is nothing to cut and no second route to
//! fetch the rest from.

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

/// One line of `.armada/logs/<job-id>.jsonl`, as a viewer is shown it.
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
