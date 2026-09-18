//! `armada land` — the merge line. These stages are the state it keeps on
//! disk, the pure comparisons its gate makes, and the infrastructure a turn
//! runs on: the two reused worktrees, the turn lock, the detached runner.
//! The CLI verb and the merge orchestration itself are the last stage.
//!
//! State stays JSON, on disk, decoded and encoded only through
//! [`ipc::decode`]/[`ipc::encode`] — the same doorway-wearing-a-file pattern
//! as `crates/fleet/src/runtime.rs`, so nothing here parses untyped JSON
//! directly (gate rule five).
//!
//! [`dir::key`] hex-encodes a branch name's own UTF-8 bytes rather than
//! hashing it, unlike `scripts/land`'s sha256: longer, but bijective, and no
//! new dependency.

pub mod codec;
pub mod dir;
pub mod gate;
pub mod git;
pub mod lock;
pub mod outcome;
pub mod queue;
pub mod runner;
pub mod stamp;
pub mod worktree;

pub use codec::{ReadStateError, WriteStateError};
pub use dir::{key, StateDir, StateDirError};
pub use gate::{
    a_report, already_red_on_base, failing_lines, finding_counts, foundations_delta, not_installed,
    FoundationsComparison,
};
pub use git::{GitFailed, GitFailure};
pub use lock::{LockError, TurnLock};
pub use outcome::{
    merge_outcome, read_outcome, MergeOutcomeError, Outcome, OutcomePatch, OutcomeState, Place,
};
pub use queue::{nonce, queued, read_queue_entry, write_queue_entry, QueueEntry, QueuedError};
pub use runner::{ensure_runner, spawn_detached, EnsureRunnerError};
pub use stamp::{read_stamp, write_stamp, PreflightStamp};
pub use worktree::{
    drop_worktree, land_root, main_tree, reused, LandRootError, LandWorktree, LogError,
    MainTreeError, ReuseError,
};
