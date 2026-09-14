//! What `ask_person_to_approve` answers with. `#1041`.
//!
//! **Nothing moves, so nothing here is a finding.** The route's whole job is
//! to confirm the Job named exists — `Resolved` already refused the call
//! otherwise — and hand back enough for Helm to say what it did. Which Job to
//! draw a card for, and whether that Job is still `awaiting_approval`, is
//! `HelmThread`'s own read of the Board at render time, not a fact this answer
//! freezes.

use serde::{Deserialize, Serialize};

use crate::ids::JobId;

/// The Job a Helm session asked the person to approve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskedApproval {
    pub job_id: JobId,
    /// What a person reads instead of the id — the same handle every other
    /// answer about this Job carries.
    pub handle: String,
}
