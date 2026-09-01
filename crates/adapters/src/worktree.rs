//! One worktree per Job, on its own new branch, created before a Drone exists.
//!
//! **Every refusal happens before anything is written.** v1's bug was that the
//! branch collision was discovered by `git worktree add` failing halfway, which
//! reported git's own sentence — the operation, not the reason — and left the
//! caller to guess. So the order below is the specification.
//!
//! 1. Open the repository.
//! 2. Refuse if the branch is already there. **Refused, never reused**: adding
//!    a worktree onto an existing branch checks it out, and two Jobs then
//!    commit to one line of history.
//! 3. Clear a stale registration for *this Job's name*, or refuse if the
//!    worktree it names is still live.
//! 4. Refuse if the path holds something git did not put there.
//! 5. Create `.armada/worktrees`, the branch, and the worktree.
//!
//! Step three is the one a person will actually hit, and
//! [`clear_a_stale_registration`] holds why and how narrowly it is done.
//!
//! **What is not here: no removal, no sweep, no retention.** [`Vcs`] has no
//! method for it, so this file could not offer one anyway — a worktree survives
//! every terminal state because nothing Fleet hands a Drone can delete one.
//! Removal is a person's act, at `armada clean`, through
//! [`reclaim`](crate::reclaim()).

use std::fs;
use std::path::Path;

use adapter_traits::{CommitTime, Committed, Vcs, Worktree, WorktreeSpec};
use git2::{BranchType, ErrorCode, Repository, WorktreeAddOptions};

use crate::error::{CommitWorkError, CreateWorktreeError};

/// Version control against a real repository on this machine.
///
/// Holds nothing. Every operation is scoped by the [`WorktreeSpec`] it is
/// handed, which is what names the repository — so one of these can serve every
/// Job on the machine and cannot be pointed at the wrong repository by having
/// been constructed against one earlier.
#[derive(Debug, Default, Clone, Copy)]
pub struct GitVcs;

impl GitVcs {
    pub fn new() -> GitVcs {
        GitVcs
    }
}

impl Vcs for GitVcs {
    type Error = CreateWorktreeError;
    type CommitError = CommitWorkError;

    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, Self::Error> {
        let repo = Repository::open(spec.repo_root()).map_err(|cause| {
            CreateWorktreeError::RepoUnreadable {
                repo: spec.repo_root().to_string(),
                cause,
            }
        })?;

        let branch_name = spec.branch();
        refuse_an_existing_branch(&repo, spec, &branch_name)?;
        clear_a_stale_registration(&repo, spec)?;

        let path = spec.worktree_path();
        refuse_an_occupied_path(Path::new(&path))?;

        let parent = spec.worktree_parent();
        fs::create_dir_all(&parent).map_err(|cause| CreateWorktreeError::ParentNotCreated {
            path: parent.clone(),
            cause,
        })?;

        add(&repo, spec, &branch_name, &path)
    }

    fn commit_all(
        &self,
        worktree: &Worktree,
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError> {
        crate::commit::commit_all(worktree, message, at)
    }
}

/// **The pre-flight v1 learned to write.**
///
/// Probing costs one ref lookup and turns git's *"could not create the
/// worktree"* into a sentence naming the branch and both ways out.
fn refuse_an_existing_branch(
    repo: &Repository,
    spec: &WorktreeSpec,
    branch: &str,
) -> Result<(), CreateWorktreeError> {
    match repo.find_branch(branch, BranchType::Local) {
        Ok(_) => Err(CreateWorktreeError::BranchExists {
            repo: spec.repo_root().to_string(),
            branch: branch.to_string(),
        }),
        // A branch that is not found is the ordinary case, and it is the only
        // error treated as "no". Anything else is the repository failing to
        // answer, which is not a licence to carry on and create one.
        Err(cause) if cause.code() == ErrorCode::NotFound => Ok(()),
        Err(cause) => Err(CreateWorktreeError::RepoUnreadable {
            repo: spec.repo_root().to_string(),
            cause,
        }),
    }
}

/// Clear the administrative record left by a worktree whose directory is gone.
///
/// Prunable, in libgit2's default sense, means exactly: not locked, not still
/// checked out, and no longer valid — the directory it points at is not there.
/// A live worktree is therefore never pruned; it is refused.
///
/// **The record outlives the directory.** git files one under
/// `.git/worktrees/<name>`, M1 has no cleanup on purpose, and `rm -rf` on a
/// checkout leaves the record behind because nothing about a directory tells
/// git it is gone. The next worktree at that path is then refused with *"is a
/// missing but already registered working tree"*.
///
/// The pruning is narrow in two ways that matter. **Only this Job's name** —
/// walking every registration and pruning what looks dead is a sweeper, and a
/// sweeper is the thing M1 must not have. **Never the working tree** —
/// [`git2::WorktreePruneOptions`] defaults leave the `working_tree` flag off,
/// and the defaults are passed as `None` rather than built, so there is no
/// field in this file that could be flipped to make a prune delete somebody's
/// files.
fn clear_a_stale_registration(
    repo: &Repository,
    spec: &WorktreeSpec,
) -> Result<(), CreateWorktreeError> {
    let name = spec.registration_name();
    let Ok(registered) = repo.find_worktree(name) else {
        return Ok(());
    };
    let registered_at = registered.path().display().to_string();

    // `None` is the whole safety argument: the defaults prune the record and
    // leave any working tree alone, and there is no options value here that
    // could say otherwise.
    match registered.is_prunable(None) {
        Ok(true) => registered.prune(None).map_err(|cause| {
            CreateWorktreeError::StaleRegistrationNotCleared {
                job_id: spec.job_id().to_string(),
                registered_at,
                cause,
            }
        }),
        Ok(false) => Err(CreateWorktreeError::WorktreeAlreadyLive {
            job_id: spec.job_id().to_string(),
            registered_at,
        }),
        Err(cause) => Err(CreateWorktreeError::StaleRegistrationNotCleared {
            job_id: spec.job_id().to_string(),
            registered_at,
            cause,
        }),
    }
}

/// Refuse a path holding something git has no record of.
///
/// An empty directory is fine — git will check out into it, and a directory
/// left by a partly-finished create is the ordinary shape of that.
fn refuse_an_occupied_path(path: &Path) -> Result<(), CreateWorktreeError> {
    let Ok(entries) = fs::read_dir(path) else {
        return Ok(());
    };
    let count = entries.count();
    if count == 0 {
        return Ok(());
    }
    Err(CreateWorktreeError::PathOccupied {
        path: path.display().to_string(),
        entries: count,
    })
}

/// Create the branch, then the worktree checked out on it.
///
/// The two are separate calls because libgit2 takes a *reference* to check out
/// rather than a branch name to create, which is why a half-finished create is
/// representable at all. It is not rolled back: see
/// [`CreateWorktreeError::WorktreeNotCreated`].
fn add(
    repo: &Repository,
    spec: &WorktreeSpec,
    branch_name: &str,
    path: &str,
) -> Result<Worktree, CreateWorktreeError> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(cause) if cause.code() == ErrorCode::UnbornBranch => {
            return Err(CreateWorktreeError::NoCommitToBranchFrom {
                repo: spec.repo_root().to_string(),
            })
        }
        Err(cause) => {
            return Err(CreateWorktreeError::RepoUnreadable {
                repo: spec.repo_root().to_string(),
                cause,
            })
        }
    };
    let from = head
        .peel_to_commit()
        .map_err(|cause| CreateWorktreeError::RepoUnreadable {
            repo: spec.repo_root().to_string(),
            cause,
        })?;

    // `force` is false. The branch was already probed above, and passing true
    // here would silently reopen the case that probe exists to refuse.
    let branch = repo.branch(branch_name, &from, false).map_err(|cause| {
        CreateWorktreeError::WorktreeNotCreated {
            path: path.to_string(),
            branch: branch_name.to_string(),
            cause,
        }
    })?;
    let reference = branch.into_reference();

    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));

    repo.worktree(spec.registration_name(), Path::new(path), Some(&options))
        .map_err(|cause| CreateWorktreeError::WorktreeNotCreated {
            path: path.to_string(),
            branch: branch_name.to_string(),
            cause,
        })?;

    Ok(Worktree::at(path, branch_name))
}
