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
//! # Some paths, built from the tip rather than the index
//!
//! [`commit_paths`] takes the branch tip's tree and replaces the named paths in
//! it, so whatever else is staged stays staged and out of the commit.
//!
//! # The identity is Armada's own, and it is not the operator's
//!
//! `repo.signature()` would read the machine's git config and attribute Fleet's
//! commit to whoever is at the keyboard, which is the machine pretending to be
//! a person. `.invalid` is reserved and resolves nowhere, so the address cannot
//! be somebody's by accident.

use std::path::{Component, Path};

use adapter_traits::{CommitTime, Committed, Worktree};
use git2::build::TreeUpdateBuilder;
use git2::{Commit, FileMode, Index, IndexAddOption, Oid, Repository, Signature, Time, Tree};

use crate::error::CommitWorkError;

const WHO: (&str, &str) = ("Armada Fleet", "fleet@armada.invalid");

pub(crate) fn commit_all(
    worktree: &Worktree,
    message: &str,
    at: CommitTime,
) -> Result<Committed, CommitWorkError> {
    let repo = open(worktree)?;
    let tree_id = stage_everything(&repo, worktree.path())?;
    let parent = tip(&repo);
    if parent.as_ref().is_some_and(|tip| tip.tree_id() == tree_id) {
        return Ok(Committed::NothingToCommit);
    }
    made(&repo, worktree, tree_id, parent.as_ref(), message, at)
}

/// Commit these paths as the working directory holds them, and nothing else.
///
/// Each path is staged in the real index and the tree takes its entry from
/// there, which runs it through the filters `git add` would and leaves a
/// person's `git status` showing it clean afterwards.
pub(crate) fn commit_paths(
    worktree: &Worktree,
    paths: &[&str],
    message: &str,
    at: CommitTime,
) -> Result<Committed, CommitWorkError> {
    if let Some(outside) = paths.iter().find(|path| !inside(path)) {
        return Err(CommitWorkError::PathOutsideTheWorktree {
            worktree: worktree.path().to_string(),
            path: outside.to_string(),
        });
    }
    let repo = open(worktree)?;
    let staged = |cause| CommitWorkError::NotStaged {
        worktree: worktree.path().to_string(),
        cause,
    };
    let parent = tip(&repo);
    let base = match &parent {
        Some(tip) => tip.tree(),
        None => repo
            .treebuilder(None)
            .and_then(|empty| empty.write())
            .and_then(|id| repo.find_tree(id)),
    }
    .map_err(staged)?;

    let mut index = repo.index().map_err(staged)?;
    let mut update = TreeUpdateBuilder::new();
    for path in paths {
        take(&repo, &mut index, &base, &mut update, path).map_err(staged)?;
    }
    index.write().map_err(staged)?;
    let tree_id = update.create_updated(&repo, &base).map_err(staged)?;
    if tree_id == base.id() {
        return Ok(Committed::NothingToCommit);
    }
    made(&repo, worktree, tree_id, parent.as_ref(), message, at)
}

/// Stage everything git can see, and answer with the tree it makes.
///
/// Two calls, because they cover different halves: `add_all` takes new and
/// modified files, and `update_all` takes the ones the Drone deleted. A commit
/// missing a deletion is a commit that does not build.
fn stage_everything(repo: &Repository, path: &str) -> Result<Oid, CommitWorkError> {
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

/// One path into the index and into the tree: its entry as `git add` makes it,
/// or its removal where the working directory no longer has it.
fn take(
    repo: &Repository,
    index: &mut Index,
    base: &Tree<'_>,
    update: &mut TreeUpdateBuilder,
    path: &str,
) -> Result<(), git2::Error> {
    let relative = Path::new(path);
    let workdir = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("the worktree has no working directory"))?;
    if workdir.join(relative).symlink_metadata().is_ok() {
        index.add_path(relative)?;
        let entry = index
            .get_path(relative, 0)
            .ok_or_else(|| git2::Error::from_str("staged, and then not in the index"))?;
        update.upsert(path, entry.id, mode(entry.mode));
    } else {
        // Both, because what is gone may have been a file or a directory.
        index.remove_path(relative)?;
        index.remove_dir(relative, 0)?;
        if base.get_path(relative).is_ok() {
            update.remove(path);
        }
    }
    Ok(())
}

/// A tree's name for an index entry's mode. The index holds only these four.
fn mode(bits: u32) -> FileMode {
    match bits {
        0o100755 => FileMode::BlobExecutable,
        0o120000 => FileMode::Link,
        0o160000 => FileMode::Commit,
        _ => FileMode::Blob,
    }
}

/// Relative, and never climbing out. An absolute path or a `..` names a file
/// this commit has no business with, so it is refused before anything stages.
fn inside(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn open(worktree: &Worktree) -> Result<Repository, CommitWorkError> {
    Repository::open(worktree.path()).map_err(|cause| CommitWorkError::WorktreeUnreadable {
        worktree: worktree.path().to_string(),
        cause,
    })
}

/// The worktree's own HEAD, which is the Job's branch. `None` is a branch with
/// no commit under it, which the first commit is allowed to be.
fn tip(repo: &Repository) -> Option<Commit<'_>> {
    repo.head().and_then(|head| head.peel_to_commit()).ok()
}

fn made(
    repo: &Repository,
    worktree: &Worktree,
    tree_id: Oid,
    parent: Option<&Commit<'_>>,
    message: &str,
    at: CommitTime,
) -> Result<Committed, CommitWorkError> {
    let tree = repo
        .find_tree(tree_id)
        .map_err(|cause| refused(worktree, cause))?;
    let when = Time::new(at.seconds(), 0);
    let who = Signature::new(WHO.0, WHO.1, &when).map_err(|cause| refused(worktree, cause))?;
    let parents: Vec<&Commit<'_>> = parent.into_iter().collect();
    repo.commit(Some("HEAD"), &who, &who, message, &tree, &parents)
        .map(|commit| Committed::Made {
            commit: commit.to_string(),
        })
        .map_err(|cause| refused(worktree, cause))
}

fn refused(worktree: &Worktree, cause: git2::Error) -> CommitWorkError {
    CommitWorkError::NotCommitted {
        worktree: worktree.path().to_string(),
        branch: worktree.branch().to_string(),
        cause,
    }
}
