//! The turn lock: one runner at a time may act on the merge line.
//!
//! **`flock`, by way of `fd-lock`.** No other crate here already wrapped it,
//! and `unsafe_code = "forbid"` rules out a hand-rolled `libc::flock` call.
//! The kernel releases the lock the moment its holder's file descriptor
//! closes — a killed process does that unconditionally, so nothing here
//! reads a pid back to decide whether a lock is stale. There is no
//! staleness question left to answer, and the code that used to answer it
//! — `fleet::process::holder_of`, `StartedAt` — is gone with it.

//! **A `'static` guard, by [`Box::leak`], stands in for a self-referential
//! struct.** [`RwLockWriteGuard`] borrows the [`RwLock`] it locked, and this
//! type hands that guard to a caller who holds it far longer than any stack
//! frame in this module. Leaking one small `RwLock<File>` per acquisition
//! is bounded by how many turns a runner — a short-lived process by design
//! — takes, and is a safe, ordinary use of `Box::leak`, not an oversight.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use fd_lock::{RwLock, RwLockWriteGuard};

use super::dir::StateDir;

/// The lock file's name, directly under the state directory — matching
/// `scripts/land`'s own `runner.lock`.
const FILE_NAME: &str = "runner.lock";

/// A held turn. Held for the life of the value — dropping it releases the
/// `flock` and removes the file.
#[derive(Debug)]
pub struct TurnLock {
    // Order matters for `Drop`: fields drop in declaration order after the
    // explicit `drop` body runs, and the guard (which unlocks) must still be
    // alive when that body removes the file, so it is declared after `path`.
    path: PathBuf,
    // Never read; held only so its `Drop` unlocks when `TurnLock` does.
    #[allow(dead_code)]
    guard: RwLockWriteGuard<'static, File>,
}

impl TurnLock {
    /// Acquire the turn, or find it genuinely held.
    ///
    /// `Ok(None)` means a live process holds it — `flock` says so directly,
    /// with no reading of who that process is. `Ok(Some(_))` means this call
    /// now holds it.
    pub fn try_acquire(state: &StateDir) -> Result<Option<TurnLock>, LockError> {
        let path = lock_path(state);
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .map_err(|cause| LockError::Unwritable {
                path: path.clone(),
                cause,
            })?;

        // Leaked deliberately — see the module doc. `lock` is never read
        // back through this binding again; it exists only so `try_write`
        // has somewhere `'static` to borrow from.
        let lock: &'static mut RwLock<File> = Box::leak(Box::new(RwLock::new(file)));
        match lock.try_write() {
            Ok(mut guard) => {
                // Best-effort and purely informational — nothing here reads
                // this back to decide anything, unlike the pid the previous
                // version staked correctness on.
                let _ = write!(guard, "{}", std::process::id());
                Ok(Some(TurnLock { path, guard }))
            }
            Err(cause) if cause.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(cause) => Err(LockError::Unwritable { path, cause }),
        }
    }

    /// Whether the turn is currently held, without acquiring it — for
    /// `runner_alive`-style polling.
    ///
    /// Opens its own file descriptor and tries the lock itself; a successful
    /// try releases again immediately (the guard is dropped at the end of
    /// the match arm), so this never actually holds anything.
    pub fn held(state: &StateDir) -> Result<bool, LockError> {
        let path = lock_path(state);
        let file = match OpenOptions::new().write(true).open(&path) {
            Ok(file) => file,
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(cause) => return Err(LockError::Unwritable { path, cause }),
        };
        let mut lock = RwLock::new(file);
        let result = match lock.try_write() {
            Ok(_) => Ok(false),
            Err(cause) if cause.kind() == io::ErrorKind::WouldBlock => Ok(true),
            Err(cause) => Err(LockError::Unwritable { path, cause }),
        };
        result
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TurnLock {
    fn drop(&mut self) {
        // The file goes first, while `self.guard` is still alive and the
        // lock still held, so nothing else can observe a path that exists
        // but is unlocked. `self.guard`'s own `Drop` runs immediately after
        // this function returns and is what actually calls `flock(Unlock)`.
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_path(state: &StateDir) -> PathBuf {
    state.path().join(FILE_NAME)
}

/// Why the turn lock could not be acquired or checked.
#[derive(Debug)]
pub enum LockError {
    Unwritable { path: PathBuf, cause: io::Error },
}

impl std::fmt::Display for LockError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Unwritable { path, cause } => {
                write!(out, "{} could not be locked: {cause}", path.display())
            }
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockError::Unwritable { cause, .. } => Some(cause),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TempDir;

    use super::TurnLock;

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

    /// The property this file now gets from the kernel rather than from
    /// reading a pid back: a lock held by a process that is killed (never
    /// runs its own `Drop`) is released the instant its file descriptors
    /// close, with no staleness check anywhere here deciding whether to
    /// believe it.
    #[test]
    fn a_lock_whose_holder_was_killed_is_released_by_the_kernel_not_by_this_code() {
        let dir = TempDir::new();
        let state = crate::land::StateDir::for_testing(dir.path().join("armada-land"));
        let state_path = state.path().to_path_buf();

        // A child process that acquires the lock and then waits to be
        // killed — standing in for a runner that never reaches its own
        // `Drop`.
        let mut child = std::process::Command::new(std::env::current_exe().expect("current_exe"))
            .env("ARMADA_LAND_LOCK_TEST_HOLD", &state_path)
            .arg("--exact")
            .arg("land::lock::tests::held_by_a_child_until_killed")
            .arg("--ignored")
            .arg("--nocapture")
            .spawn()
            .expect("the holder process spawns");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !TurnLock::held(&state).unwrap_or(false) {
            assert!(
                std::time::Instant::now() < deadline,
                "the child never acquired the lock"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        child.kill().expect("the child can be killed");
        child.wait().expect("the child's exit is observable");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while TurnLock::held(&state).unwrap_or(true) {
            assert!(
                std::time::Instant::now() < deadline,
                "the kernel never released a killed holder's lock"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    /// Not a scenario in its own right — the child process the test above
    /// spawns, standing in for a runner killed mid-turn. `#[ignore]` keeps
    /// an ordinary run from executing it directly; it runs only because the
    /// parent test names it with `--exact --ignored`.
    #[test]
    #[ignore]
    fn held_by_a_child_until_killed() {
        let Ok(state_path) = std::env::var("ARMADA_LAND_LOCK_TEST_HOLD") else {
            return;
        };
        let state = crate::land::StateDir::for_testing(state_path.into());
        let _lock = TurnLock::try_acquire(&state)
            .expect("try_acquire")
            .expect("nothing held it yet");
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
