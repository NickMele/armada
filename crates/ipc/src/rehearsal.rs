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
    /// `start_run`. Since protocol 10.10.
    #[serde(default)]
    pub servers: Vec<crate::servers::ServerEntry>,
    /// What the worktree's build directories started from. **Absent where the
    /// Job's Manifest declares no `setup.seed`.** Since protocol 13.46.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seeding: Option<crate::seeding::WorktreeSeeding>,
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

// ---------------------------------------------------------------------------
// The checkout's half — Journey 9, *Running one*
// ---------------------------------------------------------------------------
//
// **Its own shapes rather than the Job's with the id left out.** Every type
// above carries a `job_id` that is required and means something, and the main
// checkout is not a Job with a blank one: a sentinel there is an id that names
// no Job, and both sides would have to learn it. What the checkout's half
// drops is what only a Job has — a frozen Manifest, a worktree that can be
// gone, a Drone in the tree, a diff to narrow against — so these are smaller
// shapes, not copies.

/// `get_checkout_run_sheet`: what can be run in the main checkout, and the
/// facts the panel's header draws from.
///
/// **No `worktree_on_disk` and no `drone_working`.** The tree is the checkout
/// Fleet is running on, which is there for as long as Fleet is, and no Drone
/// ever works in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutRunSheet {
    /// The Commands `setup.requires` names, in its order.
    pub setup: Vec<RunEntry>,
    /// Every Check, in the order the Manifest declares them.
    pub checks: Vec<RunEntry>,
    /// Every Command `setup.requires` does not name.
    pub commands: Vec<RunEntry>,
    /// When the Manifest file was last changed by a commit. Absent where no
    /// commit in reach touched it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_edited_at: Option<Instant>,
    /// The run in flight in the checkout, if one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<CheckoutRunUnderway>,
    /// The Commands declaring `serve`, with the checkout's instance of each.
    /// Not in `commands`: a server is started with `start_server`.
    #[serde(default)]
    pub servers: Vec<crate::servers::ServerEntry>,
    /// The latest Verify in this checkout: underway, or ended and not yet
    /// replaced by the next. **Absent where none has run since Fleet started.**
    /// It is held in memory, as a run in flight is; each step's own record is
    /// on disk under `.armada/runs/` like any other run's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<CheckoutVerify>,
    /// Each workspace below the root with its own `armada.yml`, and the
    /// Commands that file declares — pressed with `StartCheckoutRun.workspace`.
    #[serde(default)]
    pub workspaces: Vec<WorkspaceCommands>,
    /// The seed `setup.seed` declares, and how warm it is. **Absent where the
    /// Manifest declares none.** Since protocol 13.46.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<crate::seeding::DeclaredSeed>,
}

/// One workspace's own Commands, as `CheckoutRunSheet::workspaces` lists them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCommands {
    /// Relative to the repository root, `/`-separated.
    pub dir: String,
    /// Every Command its `setup.requires` does not name, servers aside.
    pub commands: Vec<RunEntry>,
}

/// `start_checkout_run`'s body: a name, and the workspace whose file declares it.
///
/// Not [`StartRun`]: there is no frozen Manifest to choose against and no
/// Job's diff to narrow to, so neither of that type's two flags has an answer
/// here. A caller cannot ask for one because the field is not there to set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartCheckoutRun {
    pub name: String,
    /// A directory below the repository root whose own `armada.yml` declares
    /// `name`, run in that directory. Absent, empty or `.` is the root's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

/// A checkout run that has started and not finished. `start_checkout_run`'s
/// answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutRunUnderway {
    /// The run's own id. What `stop_checkout_run`, `undo_checkout_run` and
    /// `get_checkout_run_output` take.
    pub id: String,
    pub name: String,
    /// The command line that is running, as the Manifest declares it.
    pub command: String,
    /// Elapsed time is counted from here; nothing ticks on the wire.
    pub started_at: Instant,
    /// The workspace whose own file declared it. Absent is the root's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

/// One finished checkout run, as its directory under `.armada` records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutRunRecord {
    pub id: String,
    pub name: String,
    /// The workspace whose own file declared it. Absent is the root's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub command: String,
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
    /// Why the change could not be read, where it could not. `changed` is then
    /// empty and means nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_unreadable: Option<String>,
    /// **Whether Undo is there to offer**, and not where the tree before the
    /// run is kept: this tree holds a person's own uncommitted work, so a
    /// surface that offered Undo with no snapshot behind it would be offering
    /// to discard it. `false` is a run there is nothing to put back from.
    pub undoable: bool,
    /// When a person undid it. A run is undone once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undone_at: Option<Instant>,
    /// The log, relative to `ManifestSummary::records_root`.
    pub log: String,
}

/// `get_checkout_run_diff`: what one checkout run changed, as a patch.
///
/// **Never against `HEAD`.** This tree holds a person's own uncommitted work,
/// and a patch against `HEAD` would show all of it as the run's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutRunDiff {
    /// The run this is the change of.
    pub id: String,
    /// What the patch is measured against, said rather than assumed.
    pub against: DiffAgainst,
    pub reading: RunDiffReading,
}

/// What a run's patch is measured against. **One value**, stated on the wire
/// so a reader never has to know that this route does not fall back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffAgainst {
    /// The tree just before the run against the tree just after it, both kept
    /// by the run — so nothing edited either side of the run is in the patch.
    RunSnapshot,
}

/// The patch, or why there is none to read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RunDiffReading {
    Read {
        /// The same list `CheckoutRunRecord::changed` holds, read again beside
        /// the patch so the two cannot disagree. Empty: the run changed nothing.
        files: Vec<ChangedFile>,
        /// Git's unified diff. **Absent where there is nothing in it.**
        #[serde(default, skip_serializing_if = "Option::is_none")]
        patch: Option<String>,
    },
    /// **The snapshot is gone, and no patch stands in for it.** Never taken,
    /// or no longer in the repository.
    Gone { why: String },
}

/// `list_checkout_runs`: the checkout's earlier runs, newest first — and the
/// directories that would not read, said rather than dropped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutRunList {
    pub runs: Vec<CheckoutRunRecord>,
    pub unreadable: Vec<UnreadableRun>,
}

/// One message on a checkout run's socket, `observe_checkout_run`.
/// [`RunMessage`]'s four, one subject over.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum CheckoutRunMessage {
    Opened(CheckoutRunOpened),
    Lines(OutputLines),
    Missed(Missed),
    Closed(OutputClosed),
}

/// The first message: which run, and what the opening read left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutRunOpened {
    pub protocol_version: ProtocolVersion,
    pub id: String,
    pub name: String,
    /// The log, relative to `ManifestSummary::records_root`.
    pub path: String,
    /// Whether the run was still going when this opened.
    pub live: bool,
    /// Older lines the opening read left out, because the window is bounded.
    pub skipped: u64,
}

// ---------------------------------------------------------------------------
// Verify — Journey 9, *Verify*: setup and every Check once, in the checkout
// ---------------------------------------------------------------------------
//
// **A sequence of ordinary checkout runs, not a record of its own.** Each step
// is a run like one a person starts by hand, with its own log, snapshot, diff
// and Undo — so nothing here restates a run's shape. What this adds is the
// order, which step is out, and why a step did not run.

/// `start_checkout_verify`'s body, which may be empty: an empty body is the
/// root's Manifest, as every press before workspaces sent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartCheckoutVerify {
    /// A directory below the repository root whose own `armada.yml` Verify
    /// runs, in that directory. Absent, empty or `.` is the root's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

/// `start_checkout_verify`'s answer, and `CheckoutRunSheet::verify`.
///
/// **A rehearsal of the whole file, never a verdict on it.** No step writes
/// Evidence or a Check row, nothing reaches Doctor, and no field here is a
/// pass or a fail: a step that ran carries its record, whose exit code is a
/// fact read beside the code it expects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutVerify {
    pub id: String,
    pub started_at: Instant,
    /// When the last step ended or the rest were not run. **Absent while it is
    /// underway.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// The workspace whose own `armada.yml` this ran, relative to the
    /// repository root. **Absent is the root's Manifest.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    /// Setup in `setup.requires` order, then every Check in the order the
    /// Manifest writes them — what Verify runs, and nothing else it declares.
    pub steps: Vec<VerifyStep>,
}

/// One step of a Verify, and where it has got to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyStep {
    pub group: VerifyGroup,
    /// The Command or Check's own name in the Manifest.
    pub name: String,
    /// Its `run` line, exactly as declared.
    pub run: String,
    #[serde(flatten)]
    pub state: VerifyStepState,
}

/// Which of the file's two runnable groups a step is from. **Commands that
/// `setup.requires` does not name, and servers, are in neither**: Verify runs
/// what makes a tree workable and what gates code, and nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyGroup {
    Setup,
    Checks,
}

/// Where one step has got to. **No state is a verdict**: `ran` is a run that
/// ended however it ended, and `not_run` says why in a sentence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VerifyStepState {
    /// Not reached yet.
    Waiting,
    /// Out now, as the checkout's one run — followed on `observe_checkout_run`.
    Running { run_id: String },
    /// Ended, with the record `list_checkout_runs` also lists.
    Ran { record: CheckoutRunRecord },
    /// Never started, and why: a stop, a step before it that cut the rest
    /// short, or a run Fleet could not start.
    NotRun { why: String },
}
