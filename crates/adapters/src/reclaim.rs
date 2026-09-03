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
//! # A branch the base cannot reach is kept
//!
//! Fleet commits a finished Job's work, so the branch *is* the work. Only
//! [`UnmergedWork::Delete`] removes commits nobody has taken, and [`standing`]
//! asks that question without acting. `crate::base` says which branch is base.
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
use git2::{BranchType, ErrorCode, Oid, Repository, WorktreeLockStatus, WorktreePruneOptions};

/// Whether a branch holding commits the base cannot reach may be deleted.
///
/// Two named states rather than a `bool`, because the call site is a delete and
/// `reclaim(&spec, true)` says nothing about what is true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnmergedWork {
    /// Left where it is. Work nobody has taken is not a clean's to remove.
    Keep,
    /// Deleted, and the commits with it — `armada clean --force`.
    Delete,
}

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
    /// **Left standing on purpose.** `commits` of its own are not on `base`,
    /// so deleting it would destroy work nobody has taken.
    Kept {
        branch: String,
        tip: String,
        base: String,
        commits: usize,
    },
    /// Left standing because nothing here could say whether it was merged.
    /// Unanswered is kept: the cost of guessing wrong is a lost commit.
    KeptUnanswered {
        branch: String,
        tip: String,
        why: String,
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

/// What the repository says about one Job's worktree and branch, asked
/// without touching either.
///
/// Two readings rather than one verdict: **whether a thing is safe to reclaim
/// is not git's question**, and answering it here would put half of the rule
/// in `adapters` and the other half — terminal, piloted, depended on — in
/// Fleet, where nobody could read the whole of it in one place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub worktree: WorktreeStanding,
    pub branch: BranchStanding,
}

impl Standing {
    /// Whether there is anything here to give back at all.
    ///
    /// A Job reclaimed on an earlier sweep answers `true`, which is what keeps
    /// a sweep from reporting the same Job every time it runs.
    pub fn empty_handed(&self) -> bool {
        self.worktree == WorktreeStanding::Absent && self.branch == BranchStanding::Absent
    }
}

/// What the checkout holds, as `git status --porcelain` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorktreeStanding {
    /// Nothing at the path — reclaimed already, or never made.
    Absent,
    /// Nothing written that is not committed. Ignored files do not count:
    /// `.gitignore` is the repository saying they are not part of it, and a
    /// Job's own `.armada/` deliverables live under one.
    Clean,
    /// **The case that looks like no work at all from outside.** A Drone or a
    /// person wrote these and committed none of them, so the branch is not
    /// ahead of anything and the tree still reads as merged.
    Dirty { files: Vec<String> },
    /// Somebody locked it, which is a person saying not yet.
    Locked { reason: String },
    /// The checkout is there and git would not say what is in it. **Not
    /// clean** — unanswered and clean must never read alike.
    Unreadable { why: String },
}

/// Where the branch stands against the base, without deleting anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchStanding {
    /// No such branch. Deleted already, or the Job never reached a dispatch.
    Absent,
    /// The base reaches every commit on it, so the branch holds no copy of
    /// anything.
    Merged { tip: String },
    /// `commits` of its own that `base` cannot reach.
    Ahead {
        tip: String,
        base: String,
        commits: usize,
    },
    /// Nothing here could say. Unanswered is held, for [`BranchGone`]'s reason.
    Unanswered { tip: String, why: String },
}

/// Read one Job's worktree and branch. **Nothing is removed.**
pub fn standing(
    spec: &WorktreeSpec,
    declared_base: Option<&str>,
) -> Result<Standing, RepoUnreadable> {
    let repo = Repository::open(spec.repo_root()).map_err(|cause| RepoUnreadable {
        repo: spec.repo_root().to_string(),
        why: cause.message().to_string(),
    })?;
    Ok(Standing {
        worktree: worktree_standing(&repo, spec),
        branch: branch_standing(&repo, &spec.branch(), declared_base),
    })
}

fn worktree_standing(repo: &Repository, spec: &WorktreeSpec) -> WorktreeStanding {
    let path = spec.worktree_path();
    if !std::path::Path::new(&path).exists() {
        return WorktreeStanding::Absent;
    }
    // Before the status, because a locked worktree is held whatever is in it
    // and the lock is the reason a person gave.
    if let Ok(registered) = repo.find_worktree(spec.registration_name()) {
        if let Ok(WorktreeLockStatus::Locked(said)) = registered.is_locked() {
            return WorktreeStanding::Locked {
                reason: said.unwrap_or_else(|| String::from("no reason was given")),
            };
        }
    }
    porcelain(&path)
}

/// `git status --porcelain`, run as a person would run it.
///
/// **The command line rather than libgit2**, which is the one place in this
/// file that reaches outside the library. `tests/repo.rs` already recorded why:
/// libgit2 reads a subdirectory holding a `.git` as a repository of its own
/// and drops it, where the command line reports `?? nested/`. That difference
/// is the difference between "clean" and a checkout holding a clone somebody
/// has not pushed — and this reading is a *guard*, so it takes the answer a
/// person would be shown. `crate::delivery` reaches for git the same way and
/// for the same kind of reason.
///
/// **Anything that is not a clean answer is [`WorktreeStanding::Unreadable`]**,
/// never `Clean`. git missing from `PATH` must not read as an empty tree.
fn porcelain(path: &str) -> WorktreeStanding {
    let run = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        // A daemon has no terminal for git to ask at. `crate::delivery` says
        // the same thing about the same variable.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output();
    let run = match run {
        Ok(run) => run,
        Err(cause) => {
            return WorktreeStanding::Unreadable {
                why: format!("git would not run: {cause}"),
            }
        }
    };
    if !run.status.success() {
        return WorktreeStanding::Unreadable {
            why: String::from_utf8_lossy(&run.stderr).trim().to_string(),
        };
    }
    // The three-character prefix is the two status letters and a space. The
    // path is what a person would act on, and the letters are what this reading
    // already decided by having any line at all.
    let files: Vec<String> = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(|line| line.get(3..).unwrap_or(line).to_string())
        .collect();
    match files.is_empty() {
        true => WorktreeStanding::Clean,
        false => WorktreeStanding::Dirty { files },
    }
}

fn branch_standing(repo: &Repository, branch: &str, declared_base: Option<&str>) -> BranchStanding {
    let Ok(found) = repo.find_branch(branch, BranchType::Local) else {
        return BranchStanding::Absent;
    };
    // A ref pointing at no commit has no commits to lose. `reclaim` does not
    // ask about one either, and the two must agree.
    let Some(target) = found.get().target() else {
        return BranchStanding::Merged {
            tip: String::from("an unresolved ref"),
        };
    };
    let tip = target.to_string();
    match standing_against_the_base(repo, declared_base, target) {
        Reach::Merged => BranchStanding::Merged { tip },
        Reach::Ahead { base, commits } => BranchStanding::Ahead { tip, base, commits },
        Reach::Unanswered { why } => BranchStanding::Unanswered { tip, why },
    }
}

/// Remove one Job's worktree, then delete the branch that Job derived.
pub fn reclaim(
    spec: &WorktreeSpec,
    declared_base: Option<&str>,
    unmerged: UnmergedWork,
) -> Result<Reclaimed, RepoUnreadable> {
    let repo = Repository::open(spec.repo_root()).map_err(|cause| RepoUnreadable {
        repo: spec.repo_root().to_string(),
        why: cause.message().to_string(),
    })?;

    let worktree = remove_the_worktree(&repo, spec);
    // Only after the record is gone. A branch still checked out by a
    // registration git knows about cannot be deleted, and the message git gives
    // for that names the branch rather than the record.
    let branch = delete_the_branch(&repo, &spec.branch(), declared_base, &worktree, unmerged);
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

fn delete_the_branch(
    repo: &Repository,
    branch: &str,
    declared_base: Option<&str>,
    worktree: &WorktreeGone,
    unmerged: UnmergedWork,
) -> BranchGone {
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
    let target = found.get().target();
    let tip = target
        .map(|oid| oid.to_string())
        .unwrap_or_else(|| String::from("an unresolved ref"));

    // A ref pointing at no commit has no commits to lose, so it is not asked
    // about. Everything else is, unless the caller said --force.
    if let (UnmergedWork::Keep, Some(oid)) = (unmerged, target) {
        match standing_against_the_base(repo, declared_base, oid) {
            Reach::Merged => {}
            Reach::Ahead { base, commits } => {
                return BranchGone::Kept {
                    branch: branch.to_string(),
                    tip,
                    base,
                    commits,
                }
            }
            Reach::Unanswered { why } => {
                return BranchGone::KeptUnanswered {
                    branch: branch.to_string(),
                    tip,
                    why,
                }
            }
        }
    }

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

/// Where a branch stands relative to the base it would be merged into.
///
/// **The one answer to "is this merged".** [`reclaim`] reads it to decide
/// whether a branch may be deleted and [`standing`] reads it to decide whether
/// a worktree is provably safe to take back — one derivation, so a sweep and a
/// delete cannot come to different conclusions about the same branch.
enum Reach {
    /// The base already reaches every commit on it.
    Merged,
    Ahead {
        base: String,
        commits: usize,
    },
    Unanswered {
        why: String,
    },
}

fn standing_against_the_base(repo: &Repository, declared_base: Option<&str>, tip: Oid) -> Reach {
    let looked_for = crate::base::candidates(repo, declared_base);
    let Some((base, base_tip)) = looked_for
        .iter()
        .find_map(|name| the_branch_tip(repo, name).map(|oid| (name.clone(), oid)))
    else {
        return Reach::Unanswered {
            why: format!(
                "none of {} is here, so nothing says what it would be merged into",
                looked_for.join(", ")
            ),
        };
    };
    match repo.graph_ahead_behind(tip, base_tip) {
        Ok((0, _)) => Reach::Merged,
        Ok((commits, _)) => Reach::Ahead { base, commits },
        Err(cause) => Reach::Unanswered {
            why: format!("{base} would not compare: {}", cause.message()),
        },
    }
}

fn the_branch_tip(repo: &Repository, name: &str) -> Option<Oid> {
    repo.find_branch(name, BranchType::Local)
        .ok()?
        .get()
        .target()
}
