//! Cloning a repository a person named by URL into a folder they picked.
//!
//! **Git's job, run on a person's behalf.** Armada owns nothing about the
//! result but where it landed, so the only outcome a caller acts on is whether
//! it did — and when it did not, git's own sentence.

use alloc::string::String;
use core::time::Duration;

/// Why a clone did not land. **One concrete type rather than an associated
/// one**, for [`NotDelivered`](crate::NotDelivered)'s reason: nothing matches on
/// more than which of three things to tell a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotCloned {
    /// Git ran and refused: a URL it cannot reach, a destination it will not
    /// write into. What it said, verbatim.
    Refused { said: String },
    /// Still running when the bound ran out, and stopped. Nothing is left at
    /// the destination that the clone put there.
    TookTooLong { waited: Duration },
    /// Git could not be started at all.
    NotRun { said: String },
}

impl NotCloned {
    /// A sentence for a person, built beside the value.
    pub fn said(&self) -> String {
        match self {
            NotCloned::Refused { said } => alloc::format!("git refused the clone: {said}"),
            NotCloned::TookTooLong { waited } => alloc::format!(
                "the clone was still running after {} seconds, and was stopped",
                waited.as_secs()
            ),
            NotCloned::NotRun { said } => alloc::format!("git could not be started: {said}"),
        }
    }
}
