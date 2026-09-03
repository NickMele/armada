//! What giving one Job's worktree back did, half by half.
//!
//! # Two halves, because half of it happening is a real outcome
//!
//! The checkout goes and then the branch does, and either can go without the
//! other: a locked worktree keeps its branch checked out, and a branch holding
//! commits the base cannot reach is kept on purpose while its checkout goes. A
//! single `ok` would have to pick one of those to lie about.
//!
//! # Why neither half is a closed set
//!
//! `adapters` tells six worktree outcomes and five branch outcomes apart, and
//! every closed set on this seam is a `core-model` registry key spelled through
//! `as_wire` — `enums`'s whole rule. There is no registry for what version
//! control did to a directory, and minting one would put eleven spellings into
//! the domain model to describe an act it has no opinion about.
//!
//! So each half carries **the fact and the reason**: a bool for whether the
//! thing is gone, and a sentence for why it is not where it is not. Bridge
//! frames it; Fleet does not write the frame.
//!
//! **A kept branch is not a fault and a locked worktree is.** They are told
//! apart by [`ReclaimedBranch::unmerged_commits`], set only where deleting the
//! branch would have destroyed work nobody has taken.

use serde::{Deserialize, Serialize};

use crate::ids::JobId;

/// One Job's worktree and branch, given back while Fleet is running.
///
/// The answer to `reclaim_worktree`. It carries the id because a caller may
/// have sent several and the answers do not arrive in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeReclaimed {
    pub job_id: JobId,
    pub worktree: ReclaimedWorktree,
    pub branch: ReclaimedBranch,
}

/// What became of the checkout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimedWorktree {
    /// Where it was. Named even when it was already gone, because the path is
    /// what a person checks by hand.
    pub path: String,
    /// Whether the checkout is gone from disk. **True where there was nothing
    /// there to begin with**: a Job whose worktree was already swept is a Job
    /// whose disk is back, and answering `false` would send a person looking
    /// for a directory that does not exist.
    pub removed: bool,
    /// Why it is still there, where it is. A person's lock message, or what
    /// git said. Absent where it went.
    pub why: Option<String>,
}

/// What became of the branch the Job derived.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimedBranch {
    pub branch: String,
    /// Whether the branch was deleted. True where it was already absent, for
    /// [`ReclaimedWorktree::removed`]'s reason.
    pub deleted: bool,
    /// The commit it pointed at. **A deleted branch is recoverable from its
    /// SHA and from nothing else**, which is why it is answered rather than
    /// logged. Absent where there was no branch, or no commit under it.
    pub tip: Option<String>,
    /// Why it is still standing, where it is.
    pub why: Option<String>,
    /// The branch it was compared against, where it was kept for holding
    /// commits that one cannot reach.
    pub base: Option<String>,
    /// How many of its commits the base cannot reach. **Set only on the safe
    /// keep** — this is what tells a branch deliberately left alone from a
    /// branch that would not delete.
    pub unmerged_commits: Option<u32>,
}
