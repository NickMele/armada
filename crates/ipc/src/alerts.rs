//! What is waiting on a person, in two buckets rather than one.
//!
//! **Derived, never stored.** Fleet keeps no Alert record; the list is read off
//! the Jobs it already holds, which is why nothing publishes `alert.raised`.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId};

/// One Job that is not moving without somebody.
///
/// **A row, not a Job.** Anything past naming it and where it stopped is
/// `get_job`, which is one call away.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    pub job_id: JobId,
    /// What a person calls this Job, so an answer names it the way the Board
    /// does without a second read.
    pub handle: String,
    /// The status it is sitting at, as the wire spells it.
    pub status: String,
    /// The stored reason for the transition that brought it here. **Absent is a
    /// real answer**: a status whose `reason_storage` is `None` stores nothing,
    /// and a sentence invented here would read as the record's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// When it stopped here, where the record holds an instant for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Instant>,
}

/// Everything waiting on a person, split by what the waiting costs.
///
/// **`blocked` is work stopped mid-flight** — a Drone, a worktree and a port
/// span are held while it sits. **`waiting` is work resting at a gate**,
/// holding nothing a Job that never started would not hold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertList {
    /// Oldest first, because the thing that has waited longest is the thing
    /// nobody has looked at. Not an urgency ranking.
    pub blocked: Vec<Alert>,
    /// Oldest first, for `blocked`'s reason.
    pub waiting: Vec<Alert>,
}
