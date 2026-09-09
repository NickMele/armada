//! One detached checkout per base commit, shared by every Job measured against
//! it.
//!
//! **The opposite policy from [`crate::worktree`], for the opposite reason.**
//! That file refuses everything that already exists, because a Job's branch is
//! a line of history two Jobs must never share. Nothing writes here, so an
//! existing checkout at the spec's path is the answer rather than a collision —
//! and idempotence is what lets two Fleet turns ask for the same base without
//! either having to know about the other.
//!
//! The order is:
//!
//! 1. Answer an existing checkout as it stands, having read whether it is
//!    prepared.
//! 2. Clear a registration whose directory is gone.
//! 3. Create `.armada/bases`, a ref to check out, and the worktree.
//! 4. Detach it, and delete the ref.
//!
//! Step four is why step three has a ref at all. libgit2 will only add a
//! worktree at a branch — `git_worktree_add_options.ref` must be a valid,
//! non-checked-out branch — and the branch `main` is checked out in the
//! repository itself, so the base cannot simply be added at it. A ref made and
//! immediately abandoned is the shape that leaves a detached head and no branch
//! behind, which is what [`adapter_traits::BaseCheckout`] promises.

use std::fs;
use std::path::Path;

use adapter_traits::{BaseCheckout, BaseSpec};
use git2::{BranchType, ErrorCode, Repository, WorktreeAddOptions};

use crate::error::CreateWorktreeError;

/// The branch a base checkout is created at and detached from within the same
/// call.
///
/// **Under `armada/` so it is in Armada's own namespace even for the moment it
/// exists**, and named for the commit so two of them cannot collide. If the
/// process dies between the worktree and the detach, this is the ref that is
/// left — and being namespaced is what makes it safe to find and delete.
fn scaffold_branch(spec: &BaseSpec) -> String {
    format!("armada/base/{}", spec.commit())
}

/// What commit a ref is at.
///
/// **A branch name, resolved as a branch.** A caller could pass anything git
/// would revparse, and the base is always a local branch name — so the narrow
/// lookup is the one that says *this branch is not here* rather than *that was
/// not a revision*.
pub fn commit_at(repo_root: &str, r#ref: &str) -> Result<String, CreateWorktreeError> {
    let repo = open(repo_root)?;
    let branch = repo
        .find_branch(r#ref, BranchType::Local)
        .map_err(|cause| match cause.code() {
            ErrorCode::NotFound => CreateWorktreeError::RefNotFound {
                repo: repo_root.to_string(),
                r#ref: r#ref.to_string(),
            },
            _ => CreateWorktreeError::RepoUnreadable {
                repo: repo_root.to_string(),
                cause,
            },
        })?;
    let commit = branch.into_reference().peel_to_commit().map_err(|cause| {
        CreateWorktreeError::RepoUnreadable {
            repo: repo_root.to_string(),
            cause,
        }
    })?;
    Ok(commit.id().to_string())
}

/// The checkout at this commit, made if it is not there.
pub fn base_checkout(spec: &BaseSpec) -> Result<BaseCheckout, CreateWorktreeError> {
    let path = spec.path();
    if let Some(standing) = already_there(spec, &path) {
        return Ok(standing);
    }
    let repo = open(spec.repo_root())?;
    clear_a_stale_registration(&repo, spec)?;
    let parent = spec.parent();
    fs::create_dir_all(&parent).map_err(|cause| CreateWorktreeError::ParentNotCreated {
        path: parent.clone(),
        cause,
    })?;
    add_detached(&repo, spec, &path)?;
    // Freshly created is freshly unprepared, whatever a marker on disk would
    // say — and nothing wrote one, because the directory did not exist a moment
    // ago.
    Ok(BaseCheckout::at(path, spec.commit(), false))
}

/// Remove the checkout and the record git keeps of it.
///
/// **The directory first and the record second.** git will not prune a record
/// whose directory is still there, so the other order needs two passes; this
/// one leaves at worst a prunable record, which is exactly what the next
/// [`base_checkout`] at that commit clears on its way past.
pub fn drop_base_checkout(spec: &BaseSpec) -> Result<(), CreateWorktreeError> {
    let path = spec.path();
    match fs::remove_dir_all(&path) {
        Ok(()) => {}
        // Nothing there is the answer this asks for, not a failure: the reclaim
        // that calls this walks a directory it does not lock.
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {}
        Err(cause) => {
            return Err(CreateWorktreeError::BaseNotRemoved {
                path: path.clone(),
                cause,
            })
        }
    }
    let repo = open(spec.repo_root())?;
    if let Ok(registered) = repo.find_worktree(&spec.registration_name()) {
        // Prunable is false where the directory somehow survived the remove
        // above, and a record left behind is not worth failing a sweep over —
        // the create path clears it. `None` for `crate::worktree`'s reason:
        // the defaults never touch a working tree.
        if registered.is_prunable(None).unwrap_or(false) {
            let _ = registered.prune(None);
        }
    }
    Ok(())
}

/// The checkout as it already stands, or `None` where there is nothing at the
/// path.
///
/// **A directory with anything in it counts.** An empty one is what a
/// half-finished create leaves and it is not a checkout — the create path takes
/// it from there. Whether git still holds a record for it is not asked here,
/// because the answer would not change what a caller does: it is served either
/// way.
fn already_there(spec: &BaseSpec, path: &str) -> Option<BaseCheckout> {
    let entries = fs::read_dir(path).ok()?.count();
    if entries == 0 {
        return None;
    }
    let prepared = Path::new(&spec.ready_marker()).exists();
    Some(BaseCheckout::at(path, spec.commit(), prepared))
}

fn open(repo_root: &str) -> Result<Repository, CreateWorktreeError> {
    Repository::open(repo_root).map_err(|cause| CreateWorktreeError::RepoUnreadable {
        repo: repo_root.to_string(),
        cause,
    })
}

/// Clear a record git kept for a checkout whose directory has gone.
///
/// The narrow prune [`crate::worktree::clear_a_stale_registration`] makes, and
/// narrow in the same two ways: only this commit's own registration name, and
/// never the working tree. A live one is not refused here the way a Job's is —
/// [`already_there`] has already answered for any directory that exists, so
/// reaching this with a live registration means the record points somewhere
/// else, and git's own refusal on the add below is the honest report of that.
fn clear_a_stale_registration(
    repo: &Repository,
    spec: &BaseSpec,
) -> Result<(), CreateWorktreeError> {
    let name = spec.registration_name();
    let Ok(registered) = repo.find_worktree(&name) else {
        return Ok(());
    };
    if registered.is_prunable(None).unwrap_or(false) {
        registered.prune(None).map_err(|cause| {
            CreateWorktreeError::StaleRegistrationNotCleared {
                job_id: name,
                registered_at: registered.path().display().to_string(),
                cause,
            }
        })?;
    }
    Ok(())
}

/// Add the worktree at a scaffold ref, then detach it and take the ref away.
///
/// **A failure after the worktree exists leaves the scaffold branch.** The
/// checkout is at the right commit either way, so the alternative — unwinding a
/// working checkout because a ref would not delete — would cost more than the
/// ref does. The next call finds the directory and never reaches here.
fn add_detached(repo: &Repository, spec: &BaseSpec, path: &str) -> Result<(), CreateWorktreeError> {
    let oid = git2::Oid::from_str(spec.commit()).map_err(|cause| {
        CreateWorktreeError::RepoUnreadable {
            repo: spec.repo_root().to_string(),
            cause,
        }
    })?;
    let commit = repo
        .find_commit(oid)
        .map_err(|cause| CreateWorktreeError::RepoUnreadable {
            repo: spec.repo_root().to_string(),
            cause,
        })?;
    let scaffold = scaffold_branch(spec);
    // `force` is true where it is false for a Job's worktree, and the two are
    // not the same question: a Job's branch collision is two Jobs sharing a
    // history, and this ref is created and destroyed inside this function, so
    // one left by an interrupted call is this function's own litter to
    // overwrite.
    let branch = repo
        .branch(&scaffold, &commit, true)
        .map_err(|cause| not_created(path, &scaffold, cause))?;
    let reference = branch.into_reference();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    repo.worktree(&spec.registration_name(), Path::new(path), Some(&options))
        .map_err(|cause| not_created(path, &scaffold, cause))?;

    let checkout = Repository::open(path).map_err(|cause| not_created(path, &scaffold, cause))?;
    checkout
        .set_head_detached(oid)
        .map_err(|cause| not_created(path, &scaffold, cause))?;
    // The ref is no longer checked out anywhere, so this is an ordinary delete
    // rather than a forced one. A failure is not fatal: what the caller asked
    // for is a tree at a commit, and it has one.
    if let Ok(mut planted) = repo.find_branch(&scaffold, BranchType::Local) {
        let _ = planted.delete();
    }
    Ok(())
}

fn not_created(path: &str, branch: &str, cause: git2::Error) -> CreateWorktreeError {
    CreateWorktreeError::WorktreeNotCreated {
        path: path.to_string(),
        branch: branch.to_string(),
        cause,
    }
}
