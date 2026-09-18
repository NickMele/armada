//! The turn lock: one runner at a time may act on the merge line.
//!
//! **`create_new`, not `flock`.** No `fs4`/`fs2`/`fslock` crate is in this
//! workspace's dependency graph, and `unsafe_code = "forbid"` rules out a
//! hand-rolled `libc::flock` call. `OpenOptions::create_new`
//! (`O_CREAT|O_EXCL`) gives the one property `flock` was chosen for: two
//! racing callers can never both believe they hold the turn, because only
//! the first to create the file can.
//!
//! **Staleness by `holder_of`, not by the kernel.** `flock` releases itself
//! when its holder dies; nothing here holds a kernel lock to release, so a
//! file left by a killed runner is reclaimed by asking
//! `fleet::process::holder_of` whether the pid it names is still the same
//! process — the same check `fleet::runtime` already trusts for its own
//! runtime file.

//! **Release is [`Drop`]; staleness is the backstop.** A clean exit removes
//! the file; a killed one leaves it, naming a pid the next caller finds
//! dead — reclaimed by retrying `create_new`, so the same atomicity that
//! protects the first acquisition protects the second.
//!
//! **Flagged, not pre-approved.** No new dependency is added, but this
//! substitutes `holder_of` for a kernel primitive nothing here reaches; see
//! this stage's own report for the alternative (a small `fd-lock` crate).

use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use fleet::process::{holder_of, Holder, ProbeFailed, StartedAt};

use super::dir::StateDir;

/// The lock file's name, directly under the state directory — matching
/// `scripts/land`'s own `runner.lock`.
const FILE_NAME: &str = "runner.lock";

/// One held turn, as written to the lock file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LockFile {
    pid: u32,
    started_at: StartedAt,
}

/// A held turn. Held for the life of the value — dropping it releases the
/// lock.
#[derive(Debug)]
pub struct TurnLock {
    path: PathBuf,
}

impl TurnLock {
    /// Acquire the turn, or find it genuinely held.
    ///
    /// `Ok(None)` means a live process holds it. `Ok(Some(_))` means this
    /// call now holds it, whether the file was fresh or was a stale one
    /// reclaimed first.
    pub fn try_acquire(state: &StateDir) -> Result<Option<TurnLock>, LockError> {
        let path = lock_path(state);
        let mine = this_process()?;
        let body = ipc::encode(&mine).map_err(LockError::Unencodable)?;

        match create(&path, body.as_bytes()) {
            Ok(()) => return Ok(Some(TurnLock { path })),
            Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => {}
            Err(cause) => return Err(LockError::Unwritable { path, cause }),
        }

        if !reclaim_if_stale(&path)? {
            return Ok(None);
        }
        match create(&path, body.as_bytes()) {
            Ok(()) => Ok(Some(TurnLock { path })),
            // Lost the retry to whoever reclaimed it first — the same
            // outcome as losing the first race.
            Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => Ok(None),
            Err(cause) => Err(LockError::Unwritable { path, cause }),
        }
    }

    /// Whether the turn is currently held, without acquiring it — for
    /// `runner_alive`-style polling.
    pub fn held(state: &StateDir) -> Result<bool, LockError> {
        let path = lock_path(state);
        match read_lock_file(&path) {
            Ok(None) => Ok(false),
            Ok(Some(found)) => is_live(&found),
            // Exists but will not decode: evidence the path is claimed, not
            // evidence it is free.
            Err(LockError::Undecodable { .. }) => Ok(true),
            Err(other) => Err(other),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TurnLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_path(state: &StateDir) -> PathBuf {
    state.path().join(FILE_NAME)
}

fn create(path: &Path, body: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(body)
}

/// This process's own pid and start time, the way `fleet::runtime::publish`
/// reads its own identity before writing the runtime file.
fn this_process() -> Result<LockFile, LockError> {
    let pid = std::process::id();
    match holder_of(pid).map_err(LockError::ProbeFailed)? {
        Holder::Held(started_at) => Ok(LockFile { pid, started_at }),
        Holder::Vacant => Err(LockError::OwnPidNotHeld { pid }),
    }
}

/// Reclaim `path` if the pid it names is dead or has been reused, so the
/// caller may retry `create_new`. `Ok(true)` means retry; `Ok(false)` means
/// a live process genuinely holds it.
fn reclaim_if_stale(path: &Path) -> Result<bool, LockError> {
    match read_lock_file(path) {
        // Released between the failed create and this read.
        Ok(None) => Ok(true),
        Ok(Some(found)) => {
            if is_live(&found)? {
                Ok(false)
            } else {
                remove_if_present(path)?;
                Ok(true)
            }
        }
        // Cannot prove it stale, so it is not touched.
        Err(LockError::Undecodable { .. }) => Ok(false),
        Err(other) => Err(other),
    }
}

fn is_live(found: &LockFile) -> Result<bool, LockError> {
    match holder_of(found.pid).map_err(LockError::ProbeFailed)? {
        Holder::Vacant => Ok(false),
        Holder::Held(started_at) => Ok(started_at == found.started_at),
    }
}

fn read_lock_file(path: &Path) -> Result<Option<LockFile>, LockError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => {
            return Err(LockError::Unreadable {
                path: path.to_path_buf(),
                cause,
            })
        }
    };
    ipc::decode("turn lock", &bytes)
        .map(Some)
        .map_err(|cause| LockError::Undecodable {
            path: path.to_path_buf(),
            cause,
        })
}

fn remove_if_present(path: &Path) -> Result<(), LockError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(cause) => Err(LockError::Unwritable {
            path: path.to_path_buf(),
            cause,
        }),
    }
}

/// Why the turn lock could not be acquired, checked, or written.
#[derive(Debug)]
pub enum LockError {
    Unwritable {
        path: PathBuf,
        cause: io::Error,
    },
    Unreadable {
        path: PathBuf,
        cause: io::Error,
    },
    Undecodable {
        path: PathBuf,
        cause: ipc::Undecodable,
    },
    Unencodable(ipc::Unencodable),
    ProbeFailed(ProbeFailed),
    /// The process asking is not the process it asks about. Unreachable in
    /// practice, and named the way `fleet::runtime::PublishError` names it,
    /// rather than unwrapped.
    OwnPidNotHeld {
        pid: u32,
    },
}

impl fmt::Display for LockError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockError::Unwritable { path, .. } => {
                write!(out, "{} could not be written", path.display())
            }
            LockError::Unreadable { path, .. } => {
                write!(out, "{} could not be read", path.display())
            }
            LockError::Undecodable { path, .. } => {
                write!(
                    out,
                    "{} is not a turn lock armada land wrote",
                    path.display()
                )
            }
            LockError::Unencodable(why) => write!(out, "{why}"),
            LockError::ProbeFailed(why) => write!(out, "{why}"),
            LockError::OwnPidNotHeld { pid } => {
                write!(out, "this process reports pid {pid}, which nothing holds")
            }
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockError::Unwritable { cause, .. } => Some(cause),
            LockError::Unreadable { cause, .. } => Some(cause),
            LockError::Undecodable { cause, .. } => Some(cause),
            LockError::Unencodable(why) => Some(why),
            LockError::ProbeFailed(why) => Some(why),
            LockError::OwnPidNotHeld { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TempDir;

    use super::{LockFile, TurnLock, FILE_NAME};

    #[test]
    fn try_acquire_succeeds_when_nothing_holds_it() {
        let dir = TempDir::new();
        let state = crate::land::StateDir::for_testing(dir.path().join("armada-land"));
        let lock = TurnLock::try_acquire(&state)
            .expect("try_acquire")
            .expect("nothing held it yet");
        drop(lock);
    }

    #[test]
    fn a_second_caller_finds_it_taken_while_the_first_is_alive() {
        let dir = TempDir::new();
        let state = crate::land::StateDir::for_testing(dir.path().join("armada-land"));
        let first = TurnLock::try_acquire(&state)
            .expect("try_acquire")
            .expect("acquired");

        let second = TurnLock::try_acquire(&state).expect("try_acquire");
        assert!(second.is_none(), "a live holder is never reclaimed");
        assert!(
            TurnLock::held(&state).expect("held"),
            "held() agrees while the first lock is alive"
        );
        drop(first);
    }

    #[test]
    fn dropping_the_lock_lets_the_next_caller_acquire_it() {
        let dir = TempDir::new();
        let state = crate::land::StateDir::for_testing(dir.path().join("armada-land"));
        let first = TurnLock::try_acquire(&state)
            .expect("try_acquire")
            .expect("acquired");
        drop(first);

        let second = TurnLock::try_acquire(&state).expect("try_acquire");
        assert!(
            second.is_some(),
            "the file is gone once its holder released it"
        );
    }

    /// The backstop this whole module exists for: a lock naming a pid that
    /// has since exited is reclaimed rather than treated as held forever.
    #[test]
    fn a_lock_naming_a_dead_pid_is_reclaimed() {
        let dir = TempDir::new();
        let state = crate::land::StateDir::for_testing(dir.path().join("armada-land"));

        let mut child = std::process::Command::new("/bin/echo")
            .arg("done")
            .spawn()
            .expect("echo runs");
        let pid = child.id();
        child.wait().expect("echo exits");
        assert!(
            matches!(
                fleet::process::holder_of(pid),
                Ok(fleet::process::Holder::Vacant)
            ),
            "the child must actually be gone, or reclaiming it proves nothing"
        );

        let stale = LockFile {
            pid,
            started_at: fleet::process::StartedAt::carried("Mon Jan  1 00:00:00 2001"),
        };
        let body = ipc::encode(&stale).expect("a lock file encodes");
        std::fs::write(state.path().join(FILE_NAME), body).expect("a hand-written stale lock");

        let reclaimed = TurnLock::try_acquire(&state)
            .expect("try_acquire")
            .expect("a lock naming a dead pid is reclaimed, not treated as held");
        drop(reclaimed);
    }
}
