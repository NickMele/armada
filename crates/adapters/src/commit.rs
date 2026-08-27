//! A finished Job's work, put on its branch.
//!
//! # The Drone did not do this, and could not have
//!
//! A Drone is denied `git`. So a Job that ran every step and passed every Check
//! left its work as an uncommitted modification in the worktree, on a branch
//! still pointing at the commit it started from — correct, verified, and
//! unmergeable. This is where that ends.
//!
//! # Nothing to commit is answered, not committed
//!
//! The staged tree is compared with the branch tip's, and an equal one comes
//! back as [`Committed::NothingToCommit`]. A Job whose work was a note wrote no
//! file, and an empty commit would put a record of nothing onto the branch a
//! person merges.
//!
//! # The identity is Armada's own, and it is not the operator's
//!
//! `repo.signature()` would read the machine's git config and attribute Fleet's
//! commit to whoever is at the keyboard, which is the machine pretending to be
//! a person. `.invalid` is reserved and resolves nowhere, so the address cannot
//! be somebody's by accident.

use adapter_traits::{CommitTime, Committed, Worktree};
use git2::{Commit, IndexAddOption, Repository, Signature, Time};

use crate::error::CommitWorkError;

const WHO: (&str, &str) = ("Armada Fleet", "fleet@armada.invalid");

pub(crate) fn commit_all(
    worktree: &Worktree,
    message: &str,
    at: CommitTime,
) -> Result<Committed, CommitWorkError> {
    let path = worktree.path();
    let repo = Repository::open(path).map_err(|cause| CommitWorkError::WorktreeUnreadable {
        worktree: path.to_string(),
        cause,
    })?;

    let tree_id = stage_everything(&repo, path)?;
    // The worktree's own HEAD, which is the Job's branch. `None` is a branch
    // with no commit under it, which the first commit is allowed to be.
    let parent = repo.head().and_then(|head| head.peel_to_commit()).ok();
    if parent.as_ref().is_some_and(|tip| tip.tree_id() == tree_id) {
        return Ok(Committed::NothingToCommit);
    }

    let tree = repo
        .find_tree(tree_id)
        .map_err(|cause| refused(worktree, cause))?;
    let when = Time::new(at.seconds(), 0);
    let who = Signature::new(WHO.0, WHO.1, &when).map_err(|cause| refused(worktree, cause))?;
    let parents: Vec<&Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &who, &who, message, &tree, &parents)
        .map(|commit| Committed::Made {
            commit: commit.to_string(),
        })
        .map_err(|cause| refused(worktree, cause))
}

/// Stage everything git can see, and answer with the tree it makes.
///
/// Two calls, because they cover different halves: `add_all` takes new and
/// modified files, and `update_all` takes the ones the Drone deleted. A commit
/// missing a deletion is a commit that does not build.
fn stage_everything(repo: &Repository, path: &str) -> Result<git2::Oid, CommitWorkError> {
    let refused = |cause| CommitWorkError::NotStaged {
        worktree: path.to_string(),
        cause,
    };
    let mut index = repo.index().map_err(refused)?;
    index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .map_err(refused)?;
    index.update_all(["*"], None).map_err(refused)?;
    index.write().map_err(refused)?;
    index.write_tree().map_err(refused)
}

fn refused(worktree: &Worktree, cause: git2::Error) -> CommitWorkError {
    CommitWorkError::NotCommitted {
        worktree: worktree.path().to_string(),
        branch: worktree.branch().to_string(),
        cause,
    }
}
