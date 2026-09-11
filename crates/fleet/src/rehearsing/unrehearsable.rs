//! Why a run was not started, stopped or undone, and how each is spelled on
//! the wire. **Each is checked before anything runs or is written.**

use std::fmt;

use api::Refusal;
use ipc::WireError;

/// A run that cannot start, a run that cannot stop, a stop of nothing.
const CANNOT_RUN_HERE: &str = "fleet.cannot_run_here";
/// An Undo that would take somebody's work, or has nothing to put back.
const CANNOT_UNDO: &str = "fleet.cannot_undo";
/// A name nothing this Job can run declares.
const NOT_DECLARED_HERE: &str = "fleet.not_declared_here";
/// The worktree's own `armada.yml`, asked for, would not read.
const WORKTREE_MANIFEST_UNREADABLE: &str = "fleet.worktree_manifest_unreadable";
/// A narrowed run with nothing to narrow to.
const NOTHING_TO_NARROW_TO: &str = "fleet.nothing_to_narrow_to";
/// An id no run of this Job has.
const NO_SUCH_RUN: &str = "fleet.no_such_run";
/// The worktree or the run's own directory would not read or write.
const RUN_FAULT: &str = "fleet.run_fault";

#[derive(Debug)]
pub enum Unrehearsable {
    NoWorktree,
    AlreadyRunning { name: String },
    NotDeclared { name: String, declared: Vec<String> },
    IsAServer { name: String },
    WorktreeManifest { why: String },
    DoesNotNarrow { name: String },
    NothingToNarrowTo { name: String },
    Unreadable { why: String },
    NoSuchRun { id: String },
    NotRunning { id: String },
    DroneWorking,
    RunInFlight,
    AlreadyUndone { id: String },
    NothingToUndo { id: String, why: String },
    Moved { paths: Vec<String> },
    NotKept { why: String },
}

impl Unrehearsable {
    /// The code and the refusal. **A conflict with the tree's state is a 409;
    /// a name or a narrowing that cannot work is a 422**; a disk that would
    /// not read or write is Fleet's fault.
    pub(super) fn spelled(&self) -> (&'static str, fn(WireError) -> Refusal) {
        use Unrehearsable::*;
        match self {
            NoWorktree | AlreadyRunning { .. } | NotRunning { .. } => {
                (CANNOT_RUN_HERE, Refusal::IllegalMove)
            }
            DroneWorking
            | RunInFlight
            | AlreadyUndone { .. }
            | NothingToUndo { .. }
            | Moved { .. } => (CANNOT_UNDO, Refusal::IllegalMove),
            NotDeclared { .. } | IsAServer { .. } => (NOT_DECLARED_HERE, Refusal::Unacceptable),
            WorktreeManifest { .. } => (WORKTREE_MANIFEST_UNREADABLE, Refusal::Unacceptable),
            DoesNotNarrow { .. } | NothingToNarrowTo { .. } => {
                (NOTHING_TO_NARROW_TO, Refusal::Unacceptable)
            }
            NoSuchRun { .. } => (NO_SUCH_RUN, Refusal::Unacceptable),
            Unreadable { .. } | NotKept { .. } => (RUN_FAULT, Refusal::Fault),
        }
    }
}

impl fmt::Display for Unrehearsable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Unrehearsable::*;
        match self {
            NoWorktree => out.write_str(
                "this Job's worktree is no longer on disk, so there is nowhere to run anything. \
                 A clean or a reclaim took it",
            ),
            AlreadyRunning { name } => write!(
                out,
                "`{name}` is already running in this Job's worktree. Stop it or wait for it — \
                 two runs in one tree fight over one build directory"
            ),
            NotDeclared { name, declared } => write!(
                out,
                "`{name}` is not a Check or Command this Job can run — it declares {}",
                declared
                    .iter()
                    .map(|one| format!("`{one}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            IsAServer { name } => write!(
                out,
                "`{name}` is a server — it declares `serve` and stays running — so it is \
                 started with start_server and held for the Job, not run here"
            ),
            WorktreeManifest { why } => write!(
                out,
                "the worktree's own armada.yml could not be read, so its version cannot run: {why}"
            ),
            DoesNotNarrow { name } => write!(
                out,
                "`{name}` declares no `narrow`, so it runs on the whole tree or not at all"
            ),
            NothingToNarrowTo { name } => write!(
                out,
                "nothing this Job changed feeds `{name}`'s narrowing, so a narrowed run would \
                 measure nothing. Run it on the whole tree"
            ),
            Unreadable { why } => write!(
                out,
                "the worktree's change could not be read, so the narrowed command could not be \
                 worked out: {why}"
            ),
            NoSuchRun { id } => write!(out, "no run of this Job is called `{id}`"),
            NotRunning { id } => write!(
                out,
                "run `{id}` has already finished, so there is nothing to stop"
            ),
            DroneWorking => out.write_str(
                "a Drone is working in this Job's worktree, and Undo would take its edits with \
                 the run's. Undo once the Job stops",
            ),
            RunInFlight => {
                out.write_str("a run is still going in this Job's worktree. Undo once it ends")
            }
            AlreadyUndone { id } => write!(out, "run `{id}` has already been undone"),
            NothingToUndo { id, why } => write!(out, "run `{id}` has nothing to undo: {why}"),
            Moved { paths } => write!(
                out,
                "{} changed again since the run, so putting the run's snapshot back would take \
                 that later work",
                paths.join(", ")
            ),
            NotKept { why } => write!(out, "the run's record could not be kept: {why}"),
        }
    }
}

impl std::error::Error for Unrehearsable {}
