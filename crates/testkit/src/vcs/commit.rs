//! The fake's commits: what it answers, and what it remembers being asked.
//!
//! **Its own file for size**, beside `tests.rs` and for that file's reason:
//! `vcs.rs` fakes three traits and was near the 900 lines the gate refuses at.

use adapter_traits::{CommitTime, Committed, Worktree};

use super::{FakeVcs, FakeVcsError};

/// One commit this fake said it made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeCommit {
    pub branch: String,
    pub message: String,
    pub at: CommitTime,
    /// The paths it was limited to, or `None` for everything in the worktree.
    pub paths: Option<Vec<String>>,
}

/// What the fake does when asked to commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Willing {
    /// The ordinary case: a commit is made and recorded.
    #[default]
    Yes,
    /// The worktree held nothing new. A `facts_note` Job's shape.
    NothingChanged,
    /// git refused. The work is still there and the caller has to say so.
    No(&'static str),
}

impl FakeVcs {
    /// Make every commit answer `NothingToCommit`, as a Job that wrote no file
    /// would.
    pub fn with_nothing_to_commit(self) -> FakeVcs {
        *self.commits.lock().expect("not poisoned") = Willing::NothingChanged;
        self
    }

    /// Make every commit fail as git refusing one would.
    pub fn refusing_to_commit(self, standing_in_for: &'static str) -> FakeVcs {
        *self.commits.lock().expect("not poisoned") = Willing::No(standing_in_for);
        self
    }

    /// Every commit this fake said it made, in order.
    pub fn committed(&self) -> Vec<FakeCommit> {
        self.committed.lock().expect("not poisoned").clone()
    }

    /// Both kinds of commit, told apart only by `paths`. Scripted for the
    /// reason the `commits` field is: there is no worktree here to look in.
    pub(super) fn commit(
        &self,
        worktree: &Worktree,
        paths: Option<Vec<String>>,
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, FakeVcsError> {
        match *self.commits.lock().expect("not poisoned") {
            Willing::NothingChanged => Ok(Committed::NothingToCommit),
            Willing::No(standing_in_for) => Err(FakeVcsError::NotCommitted { standing_in_for }),
            Willing::Yes => {
                let mut made = self.committed.lock().expect("not poisoned");
                made.push(FakeCommit {
                    branch: worktree.branch().to_string(),
                    message: message.to_string(),
                    at,
                    paths,
                });
                Ok(Committed::Made {
                    commit: format!("{:040x}", made.len()),
                })
            }
        }
    }
}
