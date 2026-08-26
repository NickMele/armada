//! What this crate refuses, by name.
//!
//! Typed leaf enums with structured fields — the values that failed, not a
//! sentence about them. A cause from git2 or from the filesystem is *carried*
//! and returned from [`Error::source`], never formatted into a message, so the
//! chain stays traversable up to the one place it is flattened.
//!
//! # Why a Job needs to be failed with more than "the worktree failed"
//!
//! Creation happens at approval, before any Drone exists, and a failure there
//! fails the Job. So the error is the *only* thing a person has: there is no
//! transcript, no log from a Drone, no partial work. Each variant below is a
//! different thing to go and do, which is the test for whether it deserves to
//! be its own variant:
//!
//! | Variant | What the person does |
//! | --- | --- |
//! | [`CreateWorktreeError::BranchExists`] | Renames the Job, or deletes the branch |
//! | [`CreateWorktreeError::WorktreeAlreadyLive`] | Looks at the Job already using it |
//! | [`CreateWorktreeError::PathOccupied`] | Removes the leftover directory |
//! | [`CreateWorktreeError::NoCommitToBranchFrom`] | Makes a first commit |
//! | [`CreateWorktreeError::RepoUnreadable`] | Fixes the path, or the repository |
//! | [`CreateWorktreeError::ParentNotCreated`] | Fixes the disk or the permissions |
//! | [`CreateWorktreeError::StaleRegistrationNotCleared`] | Reads the cause; git refused |
//! | [`CreateWorktreeError::WorktreeNotCreated`] | Reads the cause, then deletes the branch |
//!
//! A collision and a disk error are two variants and two sentences, and neither
//! is reachable from the other by reading prose.

use std::error::Error;
use std::fmt;

/// Why a Job's worktree was not created.
///
/// **Every variant is fatal to the Job**, and none of them leaves a Drone
/// running, because there is no Drone yet when this is raised.
#[derive(Debug)]
pub enum CreateWorktreeError {
    /// The repository would not open. The path is not a repository, is not
    /// there, or cannot be read.
    RepoUnreadable { repo: String, cause: git2::Error },
    /// The repository has no commit, so there is nothing for a branch to point
    /// at. A repository initialised and never committed to — distinguished
    /// because git's own message for it names an unborn branch rather than the
    /// missing commit.
    NoCommitToBranchFrom { repo: String },
    /// **A branch of this name is already there, and it is refused rather than
    /// reused.**
    ///
    /// Adding a worktree onto an existing branch checks that branch out, which
    /// interleaves two Jobs' commits on one line of history. v1 hit this twice
    /// on a real machine and both times the failure arrived as git's own
    /// sentence, which names the operation and not the reason.
    ///
    /// The remedies are both in the message on purpose: v1's fix was not the
    /// refusal alone but the refusal *saying what to do about it*.
    BranchExists { repo: String, branch: String },
    /// A worktree is registered under this Job's name and its directory is
    /// still there. Not a stale record — something is using it.
    WorktreeAlreadyLive {
        job_id: String,
        registered_at: String,
    },
    /// git holds an administrative record for this name that could not be
    /// cleared. The record outlives the directory, so this is what a hand
    /// deletion leaves behind when clearing it then fails.
    StaleRegistrationNotCleared {
        job_id: String,
        registered_at: String,
        cause: git2::Error,
    },
    /// Something is at the worktree path that git did not put there. Not a
    /// worktree and not a registration — an ordinary non-empty directory.
    PathOccupied { path: String, entries: usize },
    /// `.armada/worktrees` could not be made. A disk error, and it reads like
    /// one: the `io::Error` is the cause rather than a phrase inside a message.
    ParentNotCreated { path: String, cause: std::io::Error },
    /// The branch was created and git then refused the worktree.
    ///
    /// **The branch is left in place.** Nothing in M1 removes a branch, and a
    /// rollback here would be the first piece of the cleanup this milestone
    /// deliberately does not have — so the half-finished state is left as the
    /// evidence it is, and the message says how to clear it before a retry.
    WorktreeNotCreated {
        path: String,
        branch: String,
        cause: git2::Error,
    },
}

impl CreateWorktreeError {
    /// Whether this is a name already taken rather than a machine that would
    /// not cooperate.
    ///
    /// The distinction the step asks a person to be able to make, available to
    /// a caller without matching every variant — a Job failed on a collision is
    /// fixed by renaming or deleting, and a Job failed on a disk is fixed by
    /// fixing the disk and redispatching unchanged.
    pub fn is_a_collision(&self) -> bool {
        matches!(
            self,
            CreateWorktreeError::BranchExists { .. }
                | CreateWorktreeError::WorktreeAlreadyLive { .. }
                | CreateWorktreeError::PathOccupied { .. }
        )
    }
}

impl fmt::Display for CreateWorktreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateWorktreeError::RepoUnreadable { repo, cause } => {
                write!(f, "{repo} could not be opened as a repository: {cause}")
            }
            CreateWorktreeError::NoCommitToBranchFrom { repo } => write!(
                f,
                "{repo} has no commit yet, so a branch has nothing to point at"
            ),
            CreateWorktreeError::BranchExists { repo, branch } => write!(
                f,
                "{repo} already has a branch `{branch}`, and it is refused rather \
                 than reused — reusing it would put two Jobs' commits on one \
                 branch. Redispatch the Job under a new id, or `git branch -D \
                 {branch}` first"
            ),
            CreateWorktreeError::WorktreeAlreadyLive {
                job_id,
                registered_at,
            } => write!(
                f,
                "a worktree for job {job_id} is already checked out at \
                 {registered_at}"
            ),
            CreateWorktreeError::StaleRegistrationNotCleared {
                job_id,
                registered_at,
                cause,
            } => write!(
                f,
                "job {job_id} has a leftover worktree record pointing at \
                 {registered_at}, and clearing it failed: {cause}"
            ),
            CreateWorktreeError::PathOccupied { path, entries } => write!(
                f,
                "{path} already exists and holds {entries} entries git did not \
                 put there — remove it, then redispatch"
            ),
            CreateWorktreeError::ParentNotCreated { path, cause } => {
                write!(f, "{path} could not be created: {cause}")
            }
            CreateWorktreeError::WorktreeNotCreated {
                path,
                branch,
                cause,
            } => write!(
                f,
                "the worktree at {path} was refused: {cause}. The branch \
                 `{branch}` was created and is left in place — `git branch -D \
                 {branch}` before retrying"
            ),
        }
    }
}

impl Error for CreateWorktreeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CreateWorktreeError::RepoUnreadable { cause, .. }
            | CreateWorktreeError::StaleRegistrationNotCleared { cause, .. }
            | CreateWorktreeError::WorktreeNotCreated { cause, .. } => Some(cause),
            CreateWorktreeError::ParentNotCreated { cause, .. } => Some(cause),
            CreateWorktreeError::NoCommitToBranchFrom { .. }
            | CreateWorktreeError::BranchExists { .. }
            | CreateWorktreeError::WorktreeAlreadyLive { .. }
            | CreateWorktreeError::PathOccupied { .. } => None,
        }
    }
}
