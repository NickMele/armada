//! Detaching a runner so it outlives whatever queued it.
//!
//! **`process_group(0)`, not `setsid()`.** Python's `ensure_runner` uses
//! `start_new_session=True` because the caller that queues a turn is killed
//! at a 600s timeout, and the runner has to survive that. `process_group(0)`
//! (stable since 1.64, no `unsafe`) puts the child in a new process group of
//! its own, so a `killpg` aimed at the caller's group never reaches it —
//! which is the property that actually matters here.
//!
//! **A named, not claimed, gap.** This is not a full `setsid()`: no new
//! session, no controlling-terminal detach. Every path that matters —
//! agents, CI — has no controlling terminal to begin with, so the
//! difference is not expected to bite, but it is named rather than assumed
//! away.

use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::dir::StateDir;
use super::lock::{LockError, TurnLock};

/// Spawn `program` detached from the caller's process group, appending its
/// stdout and stderr to `log_path`. Returns as soon as the child is
/// spawned — this never waits on it.
pub fn spawn_detached(program: &Path, args: &[&str], log_path: &Path) -> io::Result<Child> {
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let log_for_stderr = log.try_clone()?;

    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_for_stderr))
        .process_group(0)
        .spawn()
}

/// Start a runner unless the turn is already held by a live one.
///
/// Two callers racing here is harmless: the loser's own `spawn_detached`
/// still runs, but the runner it starts loses [`TurnLock::try_acquire`] to
/// whichever runner acquired first, and exits.
///
/// **The argv shape here is this stage's own choice — `cli.rs` does not yet
/// understand `--runner`.** `["land", "--runner", <common git dir>]`
/// matches `scripts/land`'s own `ensure_runner` (`["--runner",
/// os.path.dirname(state)]`); Stage 4 is what makes a real `armada` binary
/// read it.
pub fn ensure_runner(
    program: &Path,
    state: &StateDir,
    common_git_dir: &Path,
) -> Result<(), EnsureRunnerError> {
    if TurnLock::held(state).map_err(EnsureRunnerError::Lock)? {
        return Ok(());
    }
    let common = common_git_dir
        .to_str()
        .ok_or_else(|| EnsureRunnerError::PathNotUtf8 {
            path: common_git_dir.to_path_buf(),
        })?;
    let log_path = state.path().join("runner.log");
    spawn_detached(program, &["land", "--runner", common], &log_path)
        .map_err(|cause| EnsureRunnerError::SpawnFailed { cause })?;
    Ok(())
}

/// Why a runner could not be ensured.
#[derive(Debug)]
pub enum EnsureRunnerError {
    Lock(LockError),
    PathNotUtf8 { path: PathBuf },
    SpawnFailed { cause: io::Error },
}

impl std::fmt::Display for EnsureRunnerError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnsureRunnerError::Lock(why) => write!(out, "{why}"),
            EnsureRunnerError::PathNotUtf8 { path } => {
                write!(out, "{} is not valid UTF-8", path.display())
            }
            EnsureRunnerError::SpawnFailed { cause } => {
                write!(out, "the runner could not be spawned: {cause}")
            }
        }
    }
}

impl std::error::Error for EnsureRunnerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EnsureRunnerError::Lock(why) => Some(why),
            EnsureRunnerError::PathNotUtf8 { .. } => None,
            EnsureRunnerError::SpawnFailed { cause } => Some(cause),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::tests::TempDir;

    use super::{ensure_runner, spawn_detached};

    /// Proves the plumbing — argv construction and log redirection — runs a
    /// real child and captures its output. Surviving a `killpg` aimed at the
    /// caller's group is `process_group(0)`'s actual point and is an
    /// integration-test concern for a later stage, not asserted here.
    #[test]
    fn spawn_detached_runs_the_child_and_captures_its_stdout() {
        let dir = TempDir::new();
        let log = dir.path().join("runner.log");

        let mut child = spawn_detached(Path::new("/bin/echo"), &["hello-from-the-runner"], &log)
            .expect("spawn_detached spawns");
        let status = child.wait().expect("the child runs to completion");
        assert!(status.success());

        let logged = std::fs::read_to_string(&log).expect("the log file exists");
        assert!(
            logged.contains("hello-from-the-runner"),
            "the child's stdout landed in the log: {logged:?}"
        );
    }

    #[test]
    fn ensure_runner_does_nothing_while_the_turn_is_held() {
        let dir = TempDir::new();
        let state = crate::land::StateDir::for_testing(dir.path().join("armada-land"));
        let held = crate::land::lock::TurnLock::try_acquire(&state)
            .expect("try_acquire")
            .expect("nothing held it yet");

        // A path nothing on this machine could actually run — proving this
        // call short-circuited rather than merely happening to succeed.
        ensure_runner(Path::new("/does-not-exist/land-runner"), &state, dir.path())
            .expect("ensure_runner does not attempt to spawn while the turn is held");

        drop(held);
    }
}
