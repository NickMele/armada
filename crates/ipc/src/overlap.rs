//! Another Job claiming to write where this one says it will.
//!
//! # It is a fact on the card, never a verdict
//!
//! `docs/concepts/fleet.md` — **"Surfaced, never serialised."** Nothing here
//! carries a decision: there is no `blocked` flag, no severity, and no field a
//! Bridge could read to grey out the approve button. A person is told and
//! approves anyway, which `docs/concepts/job-board.md` calls the ordinary case.
//!
//! # It says two Jobs *claimed* the same place, and no more than that
//!
//! Both sides are declarations. A Drone writing somewhere it never named is
//! not here and could not be — its worktree is a whole-repo checkout, which is
//! the same fact that makes a lease over the declaration pointless. The check
//! that reads a real diff is the per-step drift check, and it compares one step
//! against its own plan rather than against another Job.
//!
//! # Absent, not empty
//!
//! [`JobDetail::write_scope_overlaps`](crate::JobDetail) is skipped when there
//! is nothing to say, on this crate's own rule: a client that receives an empty
//! array cannot tell "nobody overlaps" from "this Fleet has not worked it out".
//! Here the two really are different — a Job whose scope nothing has declared
//! yet has no comparison to make, and saying "no overlap" about it would be a
//! claim nothing supports.

use serde::{Deserialize, Serialize};

use crate::enums::JobStatus;
use crate::ids::{JobId, StepId};

/// One other Job that claims to be writing where this one does.
///
/// **One entry per Job, not per path.** A person decides about the other Job,
/// so the Job is the row and the paths are its detail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeOverlap {
    pub job_id: JobId,
    /// What the other Job is called. Carried rather than fetched: a card that
    /// names an id is a card a person has to go and look something up from,
    /// and `docs/concepts/fleet.md` says the overlap **names** the Job.
    pub title: String,
    /// Where the other Job is. A person weighs `running` differently from
    /// `awaiting_review`, and the remedy differs too — the second has stopped
    /// writing.
    pub status: JobStatus,
    /// Never empty. An entry exists because a path is shared.
    pub paths: Vec<SharedPath>,
}

/// One place both Jobs claim, and who claimed it on each side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedPath {
    /// The narrower of the two claims. Where one Job says `crates/` and the
    /// other says `crates/fleet/src`, this is `crates/fleet/src`.
    pub path: String,
    /// The step of **this** Job whose Drone declared it. Absent where the claim
    /// is this Job's own `write_targets`, which the requester wrote before
    /// anything ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub this_step: Option<StepId>,
    /// The same, on the other Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_step: Option<StepId>,
}
