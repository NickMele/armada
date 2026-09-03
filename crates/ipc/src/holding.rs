//! What Fleet is holding disk for, and the test each one did not pass. The
//! wire form of `crates/fleet/src/holding.rs`, which is the definition.
//!
//! **Nothing here says how large a worktree is.** Bytes are not the decision —
//! which commits go, whether anything else has them, and which uncommitted
//! files exist nowhere but that checkout is — and a size beside those would be
//! the figure read first and meaning least.
//!
//! **A piloted worktree is not on this wire at all**, so there is no variant
//! below to render by mistake. `#367`, and Fleet drops it through
//! `Holding::offerable` rather than trusting a client with a flag.

use serde::{Deserialize, Serialize};

use crate::enums::JobStatus;
use crate::ids::JobId;

/// Every worktree Fleet is holding disk for, ordered by Job id.
///
/// **Complete, including the ones the sweep will take on its own.** A list
/// filtered to the held ones would be a list the sweep is about to change, and
/// a person who came looking for a worktree that is not on it would have no way
/// to tell "already gone" from "Fleet has it and will not say".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreesHeld {
    pub worktrees: Vec<WorktreeHeld>,
}

/// One Job's worktree, and every test it failed.
///
/// The Job is named as a person reads it — the title and where it got to —
/// because the id alone is a ULID and the decision is about work somebody
/// remembers doing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeHeld {
    pub job_id: JobId,
    pub job_title: String,
    pub status: JobStatus,
    /// The checkout on disk. **What a person goes and looks at**, and the one
    /// value here that is worth copying.
    pub path: String,
    /// The branch the Job derived. Named even where it is already gone: it is
    /// what the commits are recoverable from.
    pub branch: String,
    /// **Empty is the whole of the safety claim.** Fleet's own sweep takes
    /// exactly the empty ones, so a row with nothing here is a row nobody has
    /// to decide about.
    pub held: Vec<HeldReason>,
}

impl WorktreeHeld {
    /// Whether Fleet will give this one back without being asked.
    pub fn provably_safe(&self) -> bool {
        self.held.is_empty()
    }
}

/// One test a worktree did not pass.
///
/// **Tagged on `why`, and each arm carries what its own decision needs.** The
/// count of commits without the branch they are on, or a claim of uncommitted
/// work without the filenames, is a row that asks somebody to guess.
///
/// **Not a `wire_enum`, because there is no registry for it.** `enums`'s rule
/// is a `core-model` key spelled through `as_wire`, and `reclaimed` already
/// argued the exception for this kind of value: the set is a reading of git and
/// of Fleet's scheduler, not a state the Job machine can be in.
///
/// **Widening it is a major bump.** Bridge branches on `why` to pick which
/// facts to show, which is `docs/practices/protocol.md`'s row for a variant the
/// other side matches on — `FleetCapacity.held_by`'s caveat does not reach it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum HeldReason {
    /// The Job is still moving, so it may still need its worktree. **Nothing
    /// may reclaim this one** — `reclaim_worktree` refuses a status that is not
    /// terminal, and a surface that offered it would be offering a 409.
    NotTerminal { status: JobStatus },
    /// The branch holds commits the base cannot reach.
    ///
    /// **Reclaiming does not destroy them.** There is no force on this seam, so
    /// the checkout goes and the branch stays exactly where it is — which is
    /// what makes this the reason a person can act on most freely, and why the
    /// tip travels: it is what the work is reachable from afterwards.
    Unmerged {
        base: String,
        commits: u32,
        tip: String,
    },
    /// Nothing could say what the branch would be merged into, so nothing can
    /// say whether it holds a copy of anything. Kept, for the same reason an
    /// unmerged branch is kept: the cost of guessing wrong is a lost commit.
    BaseUnanswered { detail: String },
    /// Files written and committed nowhere.
    ///
    /// **The one reason where reclaiming destroys something.** Uncommitted work
    /// leaves a branch level with its base, so every merged-ness reading says
    /// the checkout is disposable — and no branch carries these, so removing
    /// the directory is the end of them. Named file by file for that reason.
    Uncommitted { files: Vec<String> },
    /// Somebody locked the checkout, which is a person saying not yet. The
    /// reclaim leaves a locked worktree alone and says so.
    Locked { reason: String },
    /// A Job that depends on this one has not finished, so it may still need
    /// what this one wrote.
    DependedOn { by: Vec<JobId> },
    /// git would not say what is in the checkout. **Unanswered and clean must
    /// never read alike**, because only one of them can be taken back.
    Unreadable { detail: String },
}
