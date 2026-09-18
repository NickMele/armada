//! Where the merge line keeps its state, and the name a branch is given
//! there.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The six subdirectories every state directory carries, in
/// `scripts/land`'s own `state_dir` order.
const SUBDIRS: [&str; 6] = [
    "queue",
    "outcomes",
    "stamps",
    "logs",
    "foundations",
    "checks",
];

/// `armada-land/`, under a repository's git common directory, with its six
/// subdirectories guaranteed to exist.
///
/// **The common directory, not the worktree's own `.git`.** A worktree has
/// its own `.git` file pointing back at the one this resolves to, so every
/// worktree of one clone shares a single state directory — which is the
/// point: the queue and the turn are for the repository, not the checkout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDir {
    at: PathBuf,
}

impl StateDir {
    /// Resolve `repo`'s state directory and ensure its subdirectories exist.
    pub fn resolve(repo: &Path) -> Result<StateDir, StateDirError> {
        let common = git_common_dir(repo)?;
        StateDir::ensure(common.join("armada-land"))
    }

    fn ensure(at: PathBuf) -> Result<StateDir, StateDirError> {
        for sub in SUBDIRS {
            let path = at.join(sub);
            std::fs::create_dir_all(&path)
                .map_err(|cause| StateDirError::SubdirectoryUnwritable { path, cause })?;
        }
        Ok(StateDir { at })
    }

    /// A state directory at an already-chosen path, skipping the git call.
    ///
    /// Test-only: a unit test that only exercises the read/write/merge layer
    /// below has no repository to resolve one from, and paying for a `git
    /// rev-parse` in every one of them would test the shell-out rather than
    /// the state it produces. [`resolve`](StateDir::resolve) is still what
    /// every non-test caller goes through.
    #[cfg(test)]
    pub(crate) fn for_testing(at: PathBuf) -> StateDir {
        StateDir::ensure(at).expect("a fresh temporary directory accepts its own subdirectories")
    }

    pub fn path(&self) -> &Path {
        &self.at
    }

    pub(crate) fn queue_dir(&self) -> PathBuf {
        self.at.join("queue")
    }

    fn outcomes_dir(&self) -> PathBuf {
        self.at.join("outcomes")
    }

    fn stamps_dir(&self) -> PathBuf {
        self.at.join("stamps")
    }

    pub fn queue_entry_path(&self, branch: &str) -> PathBuf {
        self.queue_dir().join(format!("{}.json", key(branch)))
    }

    pub fn outcome_path(&self, branch: &str) -> PathBuf {
        self.outcomes_dir().join(format!("{}.json", key(branch)))
    }

    pub fn stamp_path(&self, branch: &str) -> PathBuf {
        self.stamps_dir().join(format!("{}.json", key(branch)))
    }
}

/// The file name a branch is given under the state directory.
///
/// A branch name holds `/` and cannot be a file name as-is. Hex-encoding its
/// own UTF-8 bytes (`docs/practices/rust.md`-style: no new dependency) is
/// bijective, unlike `scripts/land`'s sha256 truncated to sixteen
/// characters — longer, but two branches never share a key.
pub fn key(branch: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(branch.len() * 2);
    for byte in branch.as_bytes() {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn git_common_dir(repo: &Path) -> Result<PathBuf, StateDirError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|cause| StateDirError::GitUnavailable {
            repo: repo.to_path_buf(),
            cause,
        })?;
    if !output.status.success() {
        return Err(StateDirError::NotARepository {
            repo: repo.to_path_buf(),
            why: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

/// Why a state directory could not be resolved.
#[derive(Debug)]
pub enum StateDirError {
    /// `git` itself could not be run — not installed, or `repo` does not
    /// exist.
    GitUnavailable {
        repo: PathBuf,
        cause: std::io::Error,
    },
    /// `repo` is not inside a git working tree.
    NotARepository { repo: PathBuf, why: String },
    /// One of the six subdirectories could not be created.
    SubdirectoryUnwritable {
        path: PathBuf,
        cause: std::io::Error,
    },
}

impl std::fmt::Display for StateDirError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateDirError::GitUnavailable { repo, .. } => {
                write!(out, "git could not be run against {}", repo.display())
            }
            StateDirError::NotARepository { repo, why } => {
                write!(out, "{} is not a git repository: {why}", repo.display())
            }
            StateDirError::SubdirectoryUnwritable { path, .. } => {
                write!(out, "{} could not be created", path.display())
            }
        }
    }
}

impl std::error::Error for StateDirError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StateDirError::GitUnavailable { cause, .. } => Some(cause),
            StateDirError::NotARepository { .. } => None,
            StateDirError::SubdirectoryUnwritable { cause, .. } => Some(cause),
        }
    }
}
