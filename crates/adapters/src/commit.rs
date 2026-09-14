//! A finished Job's work, put on its branch.
//!
//! **Fleet commits, because a Drone cannot** — it is denied `git`, see
//! `crate::git_guard`, and would otherwise leave verified work uncommitted.
//!
//! **A merge part-way through is finished here**: `MERGE_HEAD` becomes the
//! second parent once no conflicted file still holds a marker. `#1131`.
//!
//! **Nothing to commit is answered, not committed** — except a merge, whose
//! second parent is what makes the branch hold its base.
//!
//! **The identity is Armada's own.** `repo.signature()` would attribute Fleet's
//! commit to whoever is at the keyboard; `.invalid` resolves nowhere.

use std::path::{Component, Path};

use adapter_traits::{CommitTime, Committed, Worktree};
use git2::build::TreeUpdateBuilder;
use git2::{Commit, FileMode, Index, IndexAddOption, Oid, Repository, Signature, Time, Tree};

use crate::error::CommitWorkError;
use crate::merging_in::{holds_a_marker, merge_head};

pub(crate) const WHO: (&str, &str) = ("Armada Fleet", "fleet@armada.invalid");

pub(crate) fn commit_all(
    worktree: &Worktree,
    message: &str,
    at: CommitTime,
) -> Result<Committed, CommitWorkError> {
    let repo = open(worktree)?;
    let merging = merge_head(&repo)
        .map(|oid| repo.find_commit(oid))
        .transpose()
        .map_err(|cause| refused(worktree, cause))?;
    match merging {
        None => refuse_unmerged(&repo, worktree)?,
        Some(_) => refuse_marked(&repo, worktree)?,
    }
    let tree_id = stage_everything(&repo, worktree.path())?;
    let parent = tip(&repo);
    if merging.is_none() && parent.as_ref().is_some_and(|tip| tip.tree_id() == tree_id) {
        return Ok(Committed::NothingToCommit);
    }
    let parents: Vec<&Commit<'_>> = parent.iter().chain(merging.iter()).collect();
    let committed = made(&repo, worktree, tree_id, &parents, message, at)?;
    // Only once the commit stands. Cleared first, a commit that then failed
    // would leave the next one with no second parent and the base unmerged.
    if merging.is_some() {
        repo.cleanup_state()
            .map_err(|cause| refused(worktree, cause))?;
    }
    Ok(committed)
}

/// Commit these paths as the working directory holds them, and nothing else.
///
/// **The tree is the branch tip's with these paths replaced, never the
/// index's**, so whatever else is staged stays staged and out of the commit.
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
    refuse_unmerged(&repo, worktree)?;
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
    let parents: Vec<&Commit<'_>> = parent.iter().collect();
    made(&repo, worktree, tree_id, &parents, message, at)
}

/// Refuse a worktree whose index still holds unmerged paths — a conflict a
/// stash pop left behind and nobody resolved. `add_all` and `git add` alike
/// would stage the marker text as ordinary file content and call it resolved;
/// this runs first so that never happens. `#1097`.
fn refuse_unmerged(repo: &Repository, worktree: &Worktree) -> Result<(), CommitWorkError> {
    let paths = conflicted(repo, worktree)?;
    match paths.is_empty() {
        true => Ok(()),
        false => Err(CommitWorkError::UnmergedPaths {
            worktree: worktree.path().to_string(),
            paths,
        }),
    }
}

/// Refuse a merge while any conflicted file still holds a marker. **The marker
/// is the gate, not the Drone's word** that it cleared them all.
fn refuse_marked(repo: &Repository, worktree: &Worktree) -> Result<(), CommitWorkError> {
    let root = Path::new(worktree.path());
    let paths: Vec<String> = conflicted(repo, worktree)?
        .into_iter()
        .filter(|path| std::fs::read(root.join(path)).is_ok_and(|bytes| holds_a_marker(&bytes)))
        .collect();
    match paths.is_empty() {
        true => Ok(()),
        false => Err(CommitWorkError::UnmergedPaths {
            worktree: worktree.path().to_string(),
            paths,
        }),
    }
}

/// The paths the index holds as conflicted, sorted and once each.
fn conflicted(repo: &Repository, worktree: &Worktree) -> Result<Vec<String>, CommitWorkError> {
    let index = repo.index().map_err(|cause| CommitWorkError::NotStaged {
        worktree: worktree.path().to_string(),
        cause,
    })?;
    if !index.has_conflicts() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<String> = index
        .conflicts()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|conflict| conflict.our.or(conflict.their).or(conflict.ancestor))
        .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
        .collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
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
    parents: &[&Commit<'_>],
    message: &str,
    at: CommitTime,
) -> Result<Committed, CommitWorkError> {
    let tree = repo
        .find_tree(tree_id)
        .map_err(|cause| refused(worktree, cause))?;
    let when = Time::new(at.seconds(), 0);
    let who = Signature::new(WHO.0, WHO.1, &when).map_err(|cause| refused(worktree, cause))?;
    repo.commit(Some("HEAD"), &who, &who, message, &tree, parents)
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
