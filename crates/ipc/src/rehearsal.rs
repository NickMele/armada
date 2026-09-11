//! A person's run of one Manifest entry in a Job's worktree: what the run sheet
//! lists, what streams while one is going, and what a finished one left.
//!
//! **A rehearsal, never a verdict.** Nothing here is a Check row or Evidence,
//! and no field moves a Job — Journey 9, *A passing Check leaves no verdict
//! behind*. The shapes are fitted to the run sheet's props (#609).

use serde::{Deserialize, Serialize};

use crate::event::{ChangedFile, Missed};
use crate::ids::{Instant, JobId};
use crate::underway::{OutputClosed, OutputLines};
use crate::version::ProtocolVersion;

/// `get_run_sheet`: what can be run in this Job's worktree, and the facts the
/// sheet's header and notices draw from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSheet {
    pub job_id: JobId,
    /// The Commands `setup.requires` names, in its order.
    pub setup: Vec<RunEntry>,
    /// Every Check, in the order the Manifest declares them.
    pub checks: Vec<RunEntry>,
    /// Every Command `setup.requires` does not name.
    pub commands: Vec<RunEntry>,
    /// When the Manifest file was last changed by a commit made at or before
    /// the Job was created. Absent where no commit in reach touched it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_edited_at: Option<Instant>,
    /// Absent worktree, nothing runs: the sheet's Run is disabled and says why.
    pub worktree_on_disk: bool,
    /// The worktree's own `armada.yml` declares something other than these
    /// lists, so a person can be offered its version instead.
    pub worktree_differs: bool,
    /// Why the worktree's own `armada.yml` could not be read, where it could not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_unreadable: Option<String>,
    /// A Drone is working in the tree: a run shares it, and Undo is refused.
    pub drone_working: bool,
    /// The run in flight on this Job, if one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<RunUnderway>,
    /// The Commands declaring `serve`, with this Job's instance of each. Not
    /// in `commands`: a server is started with `start_server`, never
    /// `start_run`. Since protocol 10.9.
    #[serde(default)]
    pub servers: Vec<crate::servers::ServerEntry>,
}

/// One Check or Command, as the sheet lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEntry {
    pub name: String,
    /// The `run` line, exactly as it was declared.
    pub run: String,
    /// Whether it declares `narrow`.
    pub narrows: bool,
    /// The command a narrowed run resolves to against what this Job changed.
    /// Absent where it declares no `narrow`, and where nothing the Job changed
    /// feeds it — the whole tree is then the only run there is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrow_run: Option<String>,
    /// The Commands that run first, in order.
    pub requires: Vec<String>,
    pub expect_exit_code: i64,
    /// Said, never enforced: a person's own run needs no approval.
    pub destructive: bool,
    /// Taken from what the Job froze at creation. `false` is read from the
    /// Manifest Fleet holds now, because the Job froze nothing for this entry.
    pub frozen: bool,
}

/// `start_run`'s body. **A name, never a command line**: what runs is what the
/// Manifest declares, so nothing a caller sends reaches a process as argv.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartRun {
    pub name: String,
    /// Run what `narrow` resolves to, rather than the whole tree.
    #[serde(default)]
    pub narrowed: bool,
    /// Run the worktree's own `armada.yml` rather than what the Job froze.
    #[serde(default)]
    pub worktree_version: bool,
}

/// A run that has started and not finished. `start_run`'s answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunUnderway {
    /// The run's own id. What `stop_run`, `undo_run` and `get_run_output` take.
    pub id: String,
    pub job_id: JobId,
    pub name: String,
    /// The command line that is running, narrowed where it was.
    pub command: String,
    pub narrowed: bool,
    /// Elapsed time is counted from here; nothing ticks on the wire.
    pub started_at: Instant,
}

/// One finished run, as its directory under `.armada` records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub job_id: JobId,
    pub name: String,
    pub command: String,
    pub narrowed: bool,
    pub worktree_version: bool,
    /// Whether the entry came from what the Job froze. See [`RunEntry::frozen`].
    pub frozen: bool,
    /// The Commands that ran first, in order.
    pub required: Vec<String>,
    pub started_at: Instant,
    pub ended_at: Instant,
    pub duration_ms: u64,
    /// Absent where there was no code: a signal, a budget, a spawn that failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub expect_exit_code: i64,
    /// How it ended, in a sentence. **Unhued on every surface**: a rehearsal.
    pub ended: String,
    /// A person pressed Stop.
    pub stopped: bool,
    /// What the run wrote, read from the tree before and after it.
    pub changed: Vec<ChangedFile>,
    /// Why the change could not be read, where it could not. Undo is then
    /// unavailable, and an empty `changed` means nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_unreadable: Option<String>,
    /// A Drone was working in the tree during the run, so `changed` may hold
    /// its edits as well as the run's.
    pub shared_with_drone: bool,
    /// Where the tree before the run is kept. Absent: nothing to undo from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    /// When a person undid it. A run is undone once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undone_at: Option<Instant>,
    /// The log, relative to `ManifestSummary::records_root`.
    pub log: String,
}

/// `list_runs`: a Job's earlier runs, newest first — and the directories that
/// would not read, said rather than dropped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunList {
    pub job_id: JobId,
    pub runs: Vec<RunRecord>,
    pub unreadable: Vec<UnreadableRun>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreadableRun {
    pub id: String,
    pub why: String,
}

/// `get_run_output`: the tail of one run's log, and a statement that it is one.
/// `CheckOutput`'s shape, for the same reader.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub lines: Vec<String>,
    /// The file's own numbering of `lines[0]`, from one.
    pub from_line: u32,
    pub total_lines: u32,
    pub bytes: u64,
    pub whole: bool,
}

/// `stop_run`'s and `undo_run`'s body: which run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedRun {
    pub id: String,
}

/// One message on a run's socket, `observe_run`.
///
/// **`observe_job`'s shape, one subject over**: what the log holds, then what
/// the run prints next, a count where the bound dropped some, and a sentence
/// saying why it stopped. Output never rides `/events`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum RunMessage {
    Opened(RunOpened),
    Lines(OutputLines),
    /// The viewer fell behind and this many live messages were dropped. The
    /// log keeps them; `get_run_output` reads it.
    Missed(Missed),
    Closed(OutputClosed),
}

/// The first message: whose run this is, and what the opening read left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunOpened {
    pub protocol_version: ProtocolVersion,
    pub job_id: JobId,
    pub id: String,
    pub name: String,
    /// The log, relative to `ManifestSummary::records_root`.
    pub path: String,
    /// Whether the run was still going when this opened. `false` is a run
    /// that has ended, and the history is all of it.
    pub live: bool,
    /// Older lines the opening read left out, because the window is bounded.
    pub skipped: u64,
}
