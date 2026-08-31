//! Another Job claiming to write where this one says it will.
//!
//! **A fact, never a verdict** — no `blocked` flag, no severity, nothing a
//! Bridge could grey the approve button from. No field for the remedy either:
//! the offered `depends_on` edge is a command, and it is `#231`.
//!
//! Both sides are declarations, so the Drone that writes where it never said is
//! not here and could not be.
//!
//! **Absent is not empty**, and on this field the difference is the whole
//! point: absent is a Job nothing compared, empty is a comparison that found
//! nobody. `docs/concepts/fleet.md`, Write-scope overlap, holds the rest.

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
