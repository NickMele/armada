//! A person deleting one Job's branch after its checkout is gone.
//!
//! **The branch half of [`reclaim`](super::reclaim) with
//! [`UnmergedWork::Delete`]**, the path `armada clean --force` takes, and no
//! second deletion. What it adds is the guard: the checkout must be gone and
//! the branch must still stand at the tip the person was shown.

use adapter_traits::WorktreeSpec;
use git2::{BranchType, Repository};

use super::UnmergedWork;
use super::{delete_the_branch, on_disk, remove_the_worktree, BranchGone, RepoUnreadable};

/// Why a delete was not attempted. Nothing was touched on any of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchRefused {
    /// The checkout is still there. Reclaiming it is the act that comes first.
    CheckoutOnDisk {
        path: String,
    },
    Absent {
        branch: String,
    },
    /// **The branch moved since the person looked**, so what they agreed to
    /// lose is not what is there.
    TipMoved {
        branch: String,
        asked: String,
        found: String,
    },
    Unreadable(RepoUnreadable),
}

/// Delete the branch one Job derived, only if it stands at `tip` and its
/// checkout is gone. `Deleted` carries the tip, for [`BranchGone`]'s reason.
pub fn delete_branch(spec: &WorktreeSpec, tip: &str) -> Result<BranchGone, BranchRefused> {
    let repo = Repository::open(spec.repo_root()).map_err(|cause| {
        BranchRefused::Unreadable(RepoUnreadable {
            repo: spec.repo_root().to_string(),
            why: cause.message().to_string(),
        })
    })?;
    if on_disk(spec) {
        return Err(BranchRefused::CheckoutOnDisk {
            path: spec.worktree_path(),
        });
    }
    let branch = spec.branch();
    let found = the_tip(&repo, &branch).ok_or_else(|| BranchRefused::Absent {
        branch: branch.clone(),
    })?;
    if found != tip {
        return Err(BranchRefused::TipMoved {
            branch,
            asked: tip.to_string(),
            found,
        });
    }
    // With nothing on disk this only clears a stale record, which would
    // otherwise keep the branch checked out in git's eyes.
    let worktree = remove_the_worktree(&repo, spec);
    Ok(delete_the_branch(
        &repo,
        &branch,
        None,
        &worktree,
        UnmergedWork::Delete,
    ))
}

fn the_tip(repo: &Repository, branch: &str) -> Option<String> {
    let found = repo.find_branch(branch, BranchType::Local).ok()?;
    Some(
        found
            .get()
            .target()
            .map(|oid| oid.to_string())
            .unwrap_or_else(|| String::from("an unresolved ref")),
    )
}
