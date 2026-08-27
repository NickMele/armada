//! What [`Vcs::commit_all`](crate::Vcs::commit_all) takes and what it comes
//! back with.
//!
//! # Why a commit is Fleet's and not a Drone's
//!
//! A Drone is denied `git` and stays denied, so nothing a Drone does can put
//! its own work on a branch. Work that passed every Check and stayed
//! uncommitted is work nobody can merge — which is the whole reason this pair
//! of types exists.
//!
//! # Two outcomes, because "nothing changed" is not a failure
//!
//! A Job whose work was a note legitimately writes no file, and an empty commit
//! records nothing while still landing on the branch a person merges.
//! [`Committed`] separates the two, so a caller cannot read one as the other.

use alloc::string::String;

/// When a commit is stamped: seconds since the epoch, UTC.
///
/// **Handed in, never read.** An implementation that asked the machine for the
/// time would be the one place below Fleet that reads a clock, and a commit
/// nobody can predict the instant of is a commit no test can assert on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommitTime(i64);

impl CommitTime {
    pub const fn seconds_since_epoch(seconds: i64) -> CommitTime {
        CommitTime(seconds)
    }

    pub fn seconds(&self) -> i64 {
        self.0
    }
}

/// What committing a Job's work came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Committed {
    /// The commit now carrying the work, by its id. The branch points at it.
    Made { commit: String },
    /// The worktree held nothing the branch did not already have. **Not a
    /// failure and not an empty commit** — named, so a caller reading this as
    /// "it worked" and a caller reading it as "it broke" are both wrong out
    /// loud rather than quietly.
    NothingToCommit,
}
