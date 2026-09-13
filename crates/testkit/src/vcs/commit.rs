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
    /// What it committed: everything, some paths as the working directory
    /// holds them, or one path's content given directly.
    pub scope: CommitScope,
}

/// What one commit covered. **Three variants because the real trait has three
/// methods that write one**, and a test asserting on `scope` is asserting on
/// which of them was called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitScope {
    /// `commit_all`: everything the worktree holds.
    All,
    /// `commit_paths`: these paths, as the working directory holds them.
    Paths(Vec<String>),
    /// `commit_content`: this one path, given this exact content — the
    /// working directory is never consulted.
    Content { path: String, content: String },
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

    /// Put commits on the branch that the base has not got, as a Drone running
    /// `git commit` itself leaves it. Pair with
    /// [`with_nothing_to_commit`](FakeVcs::with_nothing_to_commit) for the clean
    /// worktree that follows.
    pub fn with_commits_on_the_branch(self, commits: usize) -> FakeVcs {
        *self.ahead.lock().expect("not poisoned") = commits;
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

    /// Every kind of commit, told apart only by `scope`. Scripted for the
    /// reason the `commits` field is: there is no worktree here to look in.
    pub(super) fn commit(
        &self,
        worktree: &Worktree,
        scope: CommitScope,
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
                    scope,
                });
                Ok(Committed::Made {
                    commit: format!("{:040x}", made.len()),
                })
            }
        }
    }
}
