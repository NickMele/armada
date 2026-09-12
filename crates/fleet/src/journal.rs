//! A Job's own log, read back off the disk and put on the wire.
//!
//! **The other side of the file the whole of Fleet writes.**
//! `crate::transcript::note` is called from nineteen modules — a worktree cut,
//! a preparation run, a scope widened, a Job reclaimed — and every one of those
//! lines went nowhere. This reads them, so nothing over there changes and a
//! module that learns to write a new line is on the stream at once.
//!
//! **A byte offset, not a row count.** The file is append-only, so where a
//! reader stopped is a position in it — which is what makes a viewer's cursor
//! lossless rather than a count of what it missed. **A partial line is never
//! consumed**: a reader can arrive between a write and its flush, so the cursor
//! only advances to the last newline seen and the half-line is read whole next
//! pass.
//!
//! **The redaction is here.** An envelope carries joining keys a person
//! watching a Job is not asking for; [`ipc::LogNote`] is the narrower thing,
//! and this conversion is where a field added to the envelope is carried or
//! deliberately left behind.

use std::collections::{BTreeMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use api::{Journal, Reading, Window};
use core_model::Level;
use ipc::{DroneId, Instant, LogNote, NoteLevel, NotedField, StepId, Voice};
use serde::Deserialize;

use crate::transcript::log_of;

/// How many notes the first pass hands a viewer.
///
/// A Job's log has no size limit and a viewer is a window. What is left out is
/// counted and said, because a shortened history nobody was told about reads as
/// the whole one — `crate::transcript::HISTORY`'s rule, one file over.
///
/// Smaller than that number on purpose. A transcript is a Drone's whole output
/// and this is Fleet's own narration of one Job, which is two orders of
/// magnitude fewer lines; a window this size holds every note of any Job that
/// has ever run here.
pub const NOTES: usize = 512;

/// The Job logs, read where Fleet keeps them.
///
/// Holds this repository's records root and nothing else. It is handed to
/// the listener at startup and is the only thing on that side that knows
/// `.armada/logs/` exists.
pub struct JobLogs {
    records_root: String,
}

impl JobLogs {
    pub fn under(records_root: impl Into<String>) -> JobLogs {
        JobLogs {
            records_root: records_root.into(),
        }
    }
}

impl Journal for JobLogs {
    fn read(&self, handle: &str, from: u64) -> Reading {
        read_from(&self.records_root, handle, from)
    }

    fn window(&self, handle: &str) -> Window {
        window_of(&self.records_root, handle)
    }
}

/// Everything appended after `from`, and where the next pass starts.
///
/// **A log that is not there is nothing at `from`**, not a fault. A Job at the
/// approval gate has written no line, and neither has one proposed before any
/// of this existed.
fn read_from(records_root: &str, handle: &str, from: u64) -> Reading {
    let at = log_of(records_root, handle);
    let Ok(mut file) = File::open(&at) else {
        return nothing(from);
    };
    if file.seek(SeekFrom::Start(from)).is_err() {
        return Reading {
            notes: Vec::new(),
            from,
            skipped: 0,
            unreadable: true,
        };
    }
    // Whole lines only. `read_line` keeps the newline it stopped at, so a chunk
    // arriving without one is the tail of a write in flight and is left where
    // it is for the next pass to read whole.
    let mut reader = BufReader::new(&mut file);
    let mut read = from;
    let mut kept: VecDeque<LogNote> = VecDeque::new();
    let mut skipped = 0u64;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(taken) => {
                if !line.ends_with('\n') {
                    break;
                }
                read += taken as u64;
                // A line that will not decode is counted rather than dropped
                // silently: it was an event, and a gap nobody was told about
                // reads as a Job nothing was happening to.
                match ipc::decode::<Line>("a Job log line", line.as_bytes()) {
                    Ok(entry) => {
                        if kept.len() == NOTES {
                            kept.pop_front();
                            skipped += 1;
                        }
                        kept.push_back(LogNote::from(entry));
                    }
                    Err(_) => skipped += 1,
                }
            }
            Err(_) => {
                return Reading {
                    notes: Vec::from(kept),
                    from: read,
                    skipped,
                    unreadable: true,
                }
            }
        }
    }
    Reading {
        notes: Vec::from(kept),
        from: read,
        skipped,
        unreadable: false,
    }
}

/// The whole log, once: the last [`NOTES`] notes and the facts that say so.
///
/// **One pass, and the same [`NOTES`] bound the stream's first read uses.** A
/// second number here would be a second answer to *how much of a Job's log is
/// a reading*, and there is only one such question; what this adds over
/// [`read_from`] is the count of everything the window left in front of it, so
/// a caller can say which note it is looking at rather than only that some are
/// missing.
///
/// **Bounded by notes and not also by bytes**, which is where this parts from
/// `rehearsing::records::output`. A run's log is a test runner's output and a
/// single line of it can be arbitrarily long; every line here is Fleet's own
/// writing about one Job, which `ipc::journal` already argues is why nothing
/// on this stream is cut per note.
fn window_of(records_root: &str, handle: &str) -> Window {
    let path = relative_log(handle);
    let at = log_of(records_root, handle);
    // A log that is not there is an empty window, never a fault — [`read_from`]'s
    // rule, and for its reason: a Job at the approval gate has written no line.
    let Ok(file) = File::open(&at) else {
        return empty(path);
    };
    let bytes = file.metadata().map(|held| held.len()).unwrap_or_default();
    let mut reader = BufReader::new(file);
    let mut kept: VecDeque<LogNote> = VecDeque::new();
    let (mut total, mut from_note, mut undecodable, mut unreadable) = (0u32, 1u32, 0u32, false);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            // A write in flight, for [`read_from`]'s reason: the tail of a line
            // with no newline on it yet is not a line, and this read ends here
            // rather than decoding half an object.
            Ok(_) if !line.ends_with('\n') => break,
            Ok(_) => match ipc::decode::<Line>("a Job log line", line.as_bytes()) {
                Ok(entry) => {
                    total = total.saturating_add(1);
                    kept.push_back(LogNote::from(entry));
                    if kept.len() > NOTES {
                        kept.pop_front();
                        from_note = from_note.saturating_add(1);
                    }
                }
                Err(_) => undecodable = undecodable.saturating_add(1),
            },
            // What was read is answered, with the fact that it is not all of
            // it. A refusal would throw away the notes that did read, on the
            // one call somebody makes when something is already wrong.
            Err(_) => {
                unreadable = true;
                break;
            }
        }
    }
    Window {
        notes: Vec::from(kept),
        from_note,
        total_notes: total,
        undecodable,
        bytes,
        unreadable,
        path,
    }
}

/// A Job that has written no line: the answer, not an error.
fn empty(path: String) -> Window {
    Window {
        notes: Vec::new(),
        from_note: 1,
        total_notes: 0,
        undecodable: 0,
        bytes: 0,
        unreadable: false,
        path,
    }
}

/// The log's path relative to the records root, as `get_job_log` answers with
/// it. **The one spelling of the layout stays in [`log_of`]**; this is the same
/// three components with the root left off, because a path on the wire that
/// named somebody's home directory would put it in every reader's session.
fn relative_log(handle: &str) -> String {
    format!(".armada/logs/{handle}.jsonl")
}

fn nothing(from: u64) -> Reading {
    Reading {
        notes: Vec::new(),
        from,
        skipped: 0,
        unreadable: false,
    }
}

/// One line of `.armada/logs/<handle>.jsonl`, as this reads it back.
///
/// **Only the fields a viewer is shown.** `crate::transcript::row::Line` is the
/// writing half and carries the whole envelope; this is deliberately narrower,
/// and unknown fields are ignored so a field added to the envelope cannot stop
/// the read — the same discipline every DTO on the seam keeps.
#[derive(Debug, Deserialize)]
struct Line {
    ts: String,
    /// Read as a string and placed below. **Absent or unknown reads as
    /// `info`** — a level nobody can place is still an event that happened, and
    /// refusing the line to keep the adjective would lose the thing a person
    /// came to see. Typing it as the enum would do exactly that.
    #[serde(default)]
    level: Option<String>,
    msg: String,
    #[serde(default)]
    drone_id: Option<String>,
    #[serde(default)]
    step_id: Option<String>,
    #[serde(default)]
    fields: BTreeMap<String, Field>,
}

impl From<Line> for LogNote {
    fn from(line: Line) -> LogNote {
        LogNote {
            at: Instant::carried(line.ts),
            // **Fleet's, and stated rather than inferred.** Fleet is the sole
            // writer of a Job's log — the envelope's `component` is `fleet` on
            // every line in it — and the field is on the wire so a surface
            // folding this stream in beside a Drone's turns attributes both
            // from the wire rather than from where it fetched them.
            by: Voice::Fleet,
            level: level_of(line.level.as_deref()),
            msg: line.msg,
            step: line.step_id.map(StepId::carried),
            drone: line.drone_id.map(DroneId::carried),
            fields: line
                .fields
                .into_iter()
                .map(|(name, value)| NotedField {
                    name,
                    value: value.drawn(),
                })
                .collect(),
        }
    }
}

/// The level a line was written at.
///
/// **Each arm reads its spelling from `core_model::Level`**, which is the
/// authority `docs/concepts/log-envelope.md` gives it, so this maps between two
/// sets rather than inventing a third. Anything else is `info`: see
/// [`Line::level`].
fn level_of(spelled: Option<&str>) -> NoteLevel {
    let Some(spelled) = spelled else {
        return NoteLevel::Info;
    };
    if spelled == Level::Trace.as_wire() {
        NoteLevel::Trace
    } else if spelled == Level::Debug.as_wire() {
        NoteLevel::Debug
    } else if spelled == Level::Warn.as_wire() {
        NoteLevel::Warn
    } else if spelled == Level::Error.as_wire() {
        NoteLevel::Error
    } else {
        NoteLevel::Info
    }
}

/// A `fields` value as the file holds it. Untagged, because the file is written
/// untagged for a person reading it with `jq`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Field {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Field>),
}

impl Field {
    /// The value as a surface draws it.
    ///
    /// **One string, whatever the type was.** A tagged union on the wire would
    /// make every reader branch to reach a string it was always going to
    /// render — and the four scalar shapes are already indistinguishable once
    /// they are text beside a name.
    fn drawn(self) -> String {
        match self {
            Field::Bool(flag) => flag.to_string(),
            Field::Int(number) => number.to_string(),
            Field::Float(number) => number.to_string(),
            Field::Str(text) => text,
            // Joined rather than nested. A list in a field is a set of paths or
            // a set of names, and both read as a line.
            Field::List(values) => values
                .into_iter()
                .map(Field::drawn)
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

impl<H, V, W> crate::Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The reader for this Fleet's Job logs, for whoever builds the listener.
    ///
    /// **Handed out rather than the repository root**, so nothing outside this
    /// crate learns that `.armada/logs/` is where they are — which is the same
    /// line [`crate::serving`] holds for every other answer Fleet gives.
    pub fn job_logs(&self) -> JobLogs {
        JobLogs::under(&self.host().records_root)
    }
}
