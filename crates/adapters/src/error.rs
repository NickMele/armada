//! What this crate refuses, by name.
//!
//! Typed leaf enums with structured fields — the values that failed, not a
//! sentence about them. A cause from git2 or from the filesystem is *carried*
//! and returned from [`Error::source`], never formatted into a message, so the
//! chain stays traversable up to the one place it is flattened.
//!
//! **Why a Job needs to be failed with more than "the worktree failed".**
//! Creation happens at approval, before any Drone exists, and a failure there
//! fails the Job — so the error is the *only* thing a person has: no
//! transcript, no log from a Drone, no partial work. Each variant below is a
//! different thing to go and do, which is the test for whether it deserves to
//! be one.
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

/// Why a Job's work product could not be read.
///
/// **A different shape of failure from creating a worktree.** Creation fails
/// before a Drone exists; this fails at the gate, with a Drone running and work
/// on disk. Every variant means *the diff is unknown*, which is why none of
/// them can be answered with an empty list — a `diff_nonempty` check decided on
/// a reading that never happened is the vacuous pass the gate exists to refuse.
#[derive(Debug)]
pub enum ReadWorkProductError {
    /// The worktree's path is not a repository, or will not open.
    WorktreeUnreadable {
        worktree: String,
        cause: git2::Error,
    },
    /// The worktree opened and the repository it belongs to did not. Distinct
    /// because a checkout can outlive the repository it was cut from.
    RepositoryUnreadable {
        worktree: String,
        cause: git2::Error,
    },
    /// The branch has no commit to read, so there is nothing to diff from.
    BranchUnreadable {
        worktree: String,
        branch: String,
        cause: git2::Error,
    },
    /// The commit the branch was cut from is not in the object database.
    BaseUnreadable {
        worktree: String,
        base: String,
        cause: git2::Error,
    },
    /// The diff itself failed.
    DiffFailed {
        worktree: String,
        cause: git2::Error,
    },
    /// A path the diff named would not open, **and not because it is gone.** A
    /// deletion is a change like any other and reads as one; this is a file git
    /// can see and the filesystem will not hand over, which must not arrive at
    /// a footprint as an absence — two readings that both swallowed it would
    /// compare equal and report a step that did nothing.
    ContentUnreadable {
        worktree: String,
        path: String,
        cause: std::io::Error,
    },
}

impl fmt::Display for ReadWorkProductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadWorkProductError::WorktreeUnreadable { worktree, cause } => {
                write!(f, "the worktree at {worktree} would not open: {cause}")
            }
            ReadWorkProductError::RepositoryUnreadable { worktree, cause } => write!(
                f,
                "the repository behind the worktree at {worktree} would not open: {cause}"
            ),
            ReadWorkProductError::BranchUnreadable {
                worktree,
                branch,
                cause,
            } => write!(
                f,
                "`{branch}` in the worktree at {worktree} has no commit to read: {cause}"
            ),
            ReadWorkProductError::BaseUnreadable {
                worktree,
                base,
                cause,
            } => write!(
                f,
                "the commit {base} the worktree at {worktree} was cut from is not \
                 readable: {cause}"
            ),
            ReadWorkProductError::DiffFailed { worktree, cause } => {
                write!(f, "the diff of the worktree at {worktree} failed: {cause}")
            }
            ReadWorkProductError::ContentUnreadable {
                worktree,
                path,
                cause,
            } => write!(
                f,
                "{path} in the worktree at {worktree} changed and would not be \
                 read: {cause}"
            ),
        }
    }
}

impl Error for ReadWorkProductError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ReadWorkProductError::WorktreeUnreadable { cause, .. }
            | ReadWorkProductError::RepositoryUnreadable { cause, .. }
            | ReadWorkProductError::BranchUnreadable { cause, .. }
            | ReadWorkProductError::BaseUnreadable { cause, .. }
            | ReadWorkProductError::DiffFailed { cause, .. } => Some(cause),
            ReadWorkProductError::ContentUnreadable { cause, .. } => Some(cause),
        }
    }
}

/// Why a finished Job's work was not committed.
///
/// **A different shape again.** Creating a worktree fails before a Drone
/// exists; this fails after every Check has passed, with the work sitting in
/// the worktree. Nothing here loses that work — the worktree is left exactly as
/// the Drone left it — so each variant is something to fix and then commit by
/// hand, not something to redispatch.
#[derive(Debug)]
pub enum CommitWorkError {
    /// The worktree's path is not a repository, or will not open.
    WorktreeUnreadable {
        worktree: String,
        cause: git2::Error,
    },
    /// The work would not stage. A file git can see and cannot read.
    NotStaged {
        worktree: String,
        cause: git2::Error,
    },
    /// Everything staged and git refused the commit itself. The index is
    /// written, so the work is staged and a person's `git commit` will take it.
    NotCommitted {
        worktree: String,
        branch: String,
        cause: git2::Error,
    },
}

impl fmt::Display for CommitWorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitWorkError::WorktreeUnreadable { worktree, cause } => {
                write!(f, "the worktree at {worktree} would not open: {cause}")
            }
            CommitWorkError::NotStaged { worktree, cause } => write!(
                f,
                "the work in the worktree at {worktree} would not stage: {cause}. \
                 Nothing was committed and nothing was lost"
            ),
            CommitWorkError::NotCommitted {
                worktree,
                branch,
                cause,
            } => write!(
                f,
                "`{branch}` would not take a commit of the work at {worktree}: \
                 {cause}. The work is staged — `git -C {worktree} commit` takes it"
            ),
        }
    }
}

impl Error for CommitWorkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CommitWorkError::WorktreeUnreadable { cause, .. }
            | CommitWorkError::NotStaged { cause, .. }
            | CommitWorkError::NotCommitted { cause, .. } => Some(cause),
        }
    }
}
