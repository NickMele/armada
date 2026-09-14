//! A test broken on main, and the Job fixing it. #999.
//!
//! **One entry reads from either side.** The fix Job's detail names what it
//! claims, and the detail of the Job whose Drone reported it names who is
//! fixing what it found. Nothing here holds anything back: the reporter's own
//! gate still fails until the fix lands.

use serde::{Deserialize, Serialize};

use crate::ids::JobId;

/// One claimed fix for one test.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimedBreakage {
    /// The Check the test failed under.
    pub check: String,
    /// The test, by the name the reporting Drone copied from the output.
    pub test: String,
    /// What the output said about the failure.
    pub failure: String,
    /// The Job fixing it.
    pub fix: JobId,
    /// What the fix Job is called. Carried, as `ScopeOverlap::title` is.
    pub fix_title: String,
    /// The Job whose Drone reported it.
    pub reported_by: JobId,
    /// What that Job is called. **Absent where it has been forgotten** — the
    /// claim outlives its reporter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_by_title: Option<String>,
    /// The Jobs pointed at this fix because their Checks failed on the test.
    /// **Since 13.42**, and empty is none — a detail from an older Fleet reads
    /// the same way. #1001.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting: Vec<WaitingOnFix>,
}

/// One Job pointed at a fix, waiting for it to land.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingOnFix {
    pub job_id: JobId,
    /// What the Job is called. Absent where it could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}
