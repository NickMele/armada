//! A test broken on main, claimed by the Job drafted to fix it. #999.
//!
//! **One claim per repository, Check and test**, so a second Drone reporting
//! the same failure drafts nothing. The claim belongs to the fix: it ends when
//! that Job ends, and forgetting the fix removes it.

use alloc::string::String;

use crate::job::ids::{JobId, ManifestId};

/// What broke: the Check a failing test ran under, the test's name as the
/// Drone copied it from the output, and what running it on main came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breakage {
    pub check: String,
    pub test: String,
    pub failure: String,
}

/// A breakage and the Job fixing it.
///
/// **`reported_by` is an id and not a link.** The reporting Job may be
/// forgotten while the fix is still being worked, and the claim outlives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakageClaim {
    pub fix: JobId,
    pub repository: ManifestId,
    pub breakage: Breakage,
    pub reported_by: JobId,
}

/// A Job whose Check failed on a claimed test, pointed at the Job fixing it.
/// #1001.
///
/// **`fix` is an id and not a link**, for `reported_by`'s reason: the pointer
/// belongs to the waiting Job, and it is given back when the fix settles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixWaiter {
    pub waiting: JobId,
    pub fix: JobId,
    pub repository: ManifestId,
    pub check: String,
    pub test: String,
}
