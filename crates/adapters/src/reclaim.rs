//! Giving one Job's worktree and branch back, in the order git needs.
//!
//! # It is handed a Job, never a pattern
//!
//! [`reclaim`] takes a [`WorktreeSpec`], whose only constructor is one
//! repository root and one job id. No parameter here could carry `armada/*`, so
//! this cannot be asked to delete a namespace — only the branch a particular
//! Job derives. Deleting by glob is how nine unrelated branches were destroyed
//! by hand, and care at the call site does not fix a shape that accepts one.
//!
//! # Remove, prune, then the branch
//!
//! Removing the checkout and pruning its record are one libgit2 call. The
//! branch goes last: git refuses to delete a branch that is checked out, and
//! the record under `.git/worktrees/<name>` keeps it checked out after the
//! directory is gone — which is why an `rm -rf` fails at the branch rather than
//! at the directory.
//!
//! Each half is answered on its own, so neither hides the other's fault.

use adapter_traits::WorktreeSpec;
use git2::{BranchType, ErrorCode, Repository, WorktreeLockStatus, WorktreePruneOptions};

/// What one Job's reclaim did, half by half.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reclaimed {
    pub worktree: WorktreeGone,
    pub branch: BranchGone,
}

impl Reclaimed {
    /// Whether anything went wrong. The caller decides what to do about it;
    /// this only spares it two matches.
    pub fn faulted(&self) -> bool {
        matches!(
            self.worktree,
            WorktreeGone::NotRemoved { .. } | WorktreeGone::Locked { .. }
        ) || matches!(self.branch, BranchGone::NotDeleted { .. })
    }
}

/// What became of the checkout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorktreeGone {
    /// git held a record and removed the checkout with it.
    Removed {
        path: String,
    },
    /// git held a record for a directory that was no longer there. The record
    /// is what would have refused the next worktree at that path.
    RecordCleared {
        path: String,
    },
    /// A directory git had no record of, removed as a plain directory.
    DirectoryRemoved {
        path: String,
    },
    /// Nothing at the path, and no record.
    Absent {
        path: String,
    },
    /// Somebody locked it. **Left alone** — a lock is a person saying not yet,
    /// and the reason they gave is carried out.
    Locked {
        path: String,
        reason: String,
    },
    NotRemoved {
        path: String,
        why: String,
    },
}

/// What became of the branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchGone {
    /// Deleted, and the commit it pointed at. **The tip is the whole point of
    /// this variant**: a deleted branch is recoverable from its SHA and from
    /// nothing else, so it is reported rather than logged.
    Deleted {
        branch: String,
        tip: String,
    },
    Absent {
        branch: String,
    },
    NotDeleted {
        branch: String,
        why: String,
    },
}

/// The repository itself could not be opened, so neither half was attempted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoUnreadable {
    pub repo: String,
    pub why: String,
}

/// Remove one Job's worktree, then delete the branch that Job derived.
pub fn reclaim(spec: &WorktreeSpec) -> Result<Reclaimed, RepoUnreadable> {
    let repo = Repository::open(spec.repo_root()).map_err(|cause| RepoUnreadable {
        repo: spec.repo_root().to_string(),
        why: cause.message().to_string(),
    })?;

    let worktree = remove_the_worktree(&repo, spec);
    // Only after the record is gone. A branch still checked out by a
    // registration git knows about cannot be deleted, and the message git gives
    // for that names the branch rather than the record.
    let branch = delete_the_branch(&repo, &spec.branch(), &worktree);
    Ok(Reclaimed { worktree, branch })
}

fn remove_the_worktree(repo: &Repository, spec: &WorktreeSpec) -> WorktreeGone {
    let path = spec.worktree_path();
    let on_disk = std::path::Path::new(&path).exists();

    let Ok(registered) = repo.find_worktree(spec.registration_name()) else {
        return match on_disk {
            // git never made this, or its record is already gone. Removing the
            // directory is all that is left, and it is the Job's own directory
            // by derivation.
            true => match std::fs::remove_dir_all(&path) {
                Ok(()) => WorktreeGone::DirectoryRemoved { path },
                Err(cause) => WorktreeGone::NotRemoved {
                    path,
                    why: cause.to_string(),
                },
            },
            false => WorktreeGone::Absent { path },
        };
    };

    if let Ok(WorktreeLockStatus::Locked(said)) = registered.is_locked() {
        return WorktreeGone::Locked {
            path,
            // An empty lock message is ordinary — `git worktree lock` takes the
            // reason as an option. Named, so the line still says what to do.
            reason: said.unwrap_or_else(|| String::from("no reason was given")),
        };
    }

    // `valid` because a worktree that is perfectly healthy is exactly the one
    // being reclaimed, and `working_tree` because the checkout goes with the
    // record. `locked` is left off: the case above answers it by leaving.
    let mut options = WorktreePruneOptions::new();
    options.valid(true).working_tree(true);
    match registered.prune(Some(&mut options)) {
        Ok(()) if on_disk => WorktreeGone::Removed { path },
        Ok(()) => WorktreeGone::RecordCleared { path },
        Err(cause) => WorktreeGone::NotRemoved {
            path,
            why: cause.message().to_string(),
        },
    }
}

fn delete_the_branch(repo: &Repository, branch: &str, worktree: &WorktreeGone) -> BranchGone {
    let mut found = match repo.find_branch(branch, BranchType::Local) {
        Ok(found) => found,
        Err(cause) if cause.code() == ErrorCode::NotFound => {
            return BranchGone::Absent {
                branch: branch.to_string(),
            }
        }
        Err(cause) => {
            return BranchGone::NotDeleted {
                branch: branch.to_string(),
                why: cause.message().to_string(),
            }
        }
    };

    // A worktree still standing still has this branch checked out, and git will
    // say so in its own words. Said here instead, because the thing to fix is
    // the line above this one.
    if let WorktreeGone::Locked { path, .. } | WorktreeGone::NotRemoved { path, .. } = worktree {
        return BranchGone::NotDeleted {
            branch: branch.to_string(),
            why: format!("{path} still has it checked out"),
        };
    }

    // Read before the delete, because after it there is nothing left to ask.
    let tip = found
        .get()
        .target()
        .map(|oid| oid.to_string())
        .unwrap_or_else(|| String::from("an unresolved ref"));
    match found.delete() {
        Ok(()) => BranchGone::Deleted {
            branch: branch.to_string(),
            tip,
        },
        Err(cause) => BranchGone::NotDeleted {
            branch: branch.to_string(),
            why: cause.message().to_string(),
        },
    }
}
