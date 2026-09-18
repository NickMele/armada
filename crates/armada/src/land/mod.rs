//! `armada land` — the merge line. These stages are the state the line keeps
//! on disk and the pure comparisons its gate makes; the CLI verb, the two
//! worktrees, the turn lock and the runner are later stages of the same port.
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
pub mod outcome;
pub mod queue;
pub mod stamp;

pub use codec::{ReadStateError, WriteStateError};
pub use dir::{key, StateDir, StateDirError};
pub use gate::{
    a_report, already_red_on_base, failing_lines, finding_counts, foundations_delta, not_installed,
    FoundationsComparison,
};
pub use outcome::{
    merge_outcome, read_outcome, MergeOutcomeError, Outcome, OutcomePatch, OutcomeState, Place,
};
pub use queue::{nonce, queued, read_queue_entry, write_queue_entry, QueueEntry, QueuedError};
pub use stamp::{read_stamp, write_stamp, PreflightStamp};
