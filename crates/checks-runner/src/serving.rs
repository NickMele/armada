//! A program held up while something else runs against it.
//!
//! **The same process rules as [`run`](crate::run), and that is why it is
//! here.** No shell, a process group of its own, and the whole group ended
//! rather than the leader alone — three properties `run`'s module comment
//! argues for at length, and re-deriving them one crate over would be two
//! spellings that agree until one of them is edited. What is different is the
//! lifetime: a Check is started and awaited, and this is started and *held*,
//! because the thing that reads it is a second command.
//!
//! **It reads nothing and decides nothing.** Whether the program came up is
//! answered by running the repository's own readiness command against it, which
//! is `fleet::showing`'s loop and not this module's — a runner that probed a
//! port would be Armada holding an opinion about how a thing serves itself.
//! Output is never piped: a server left printing into a pipe nobody drains
//! blocks on a full buffer. [`Served::spawn`] discards it, and
//! [`Served::spawn_logging`] hands the process a log file a person reads.
//!
//! **Ending is not optional and does not depend on being called.** [`Drop`]
//! signals the group, so a caller that returns early — a readiness probe that
//! ran out of budget, a spec that panicked — leaves no server holding a port
//! and no test runner holding the worktree. That is v1's shape of this failure:
//! the Job ends, the machine does not notice, and the next Job is slower for
//! reasons nobody connects.

use std::path::Path;
use std::process::Stdio;

use tokio::process::{Child, Command};
use verification::{Exit, NeverRan};

use crate::run::{end_the_group, ended, not_started, split};

/// A program that is up, and the capability to end it.
///
/// **No accessor for the child and none for the group.** What a caller may do
/// is hold one and drop it; anything more would be a handle to a process this
/// module promises to reap.
pub struct Served {
    child: Child,
    /// The group id, which is the child's own pid. `None` where the child was
    /// reaped before it could be read, in which case there is nothing to
    /// signal.
    group: Option<u32>,
}

impl Served {
    /// Start the command in the worktree and hold it.
    ///
    /// **`Err` is the spawn failing and never the program failing.** A serve
    /// command that starts and immediately exits — a port already taken, a
    /// build that is not there — is a successful spawn, and what tells a caller
    /// that is [`still_up`](Served::still_up). The two are different facts and
    /// the readiness probe is what turns the second into a sentence.
    pub fn spawn(command: &str, worktree: &Path) -> Result<Served, NeverRan> {
        let Some((program, args)) = split(command) else {
            return Err(NeverRan::NothingToRun);
        };
        let mut spawning = Command::new(&program);
        spawning
            .args(&args)
            .current_dir(worktree)
            // A server that waits on input waits forever, and nothing here has
            // a budget to end it with.
            .stdin(Stdio::null())
            // Discarded rather than piped: nothing reads it, and a pipe nobody
            // drains is a server that blocks on a full buffer partway through
            // the run it was started for.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        // Zero means "a new group, led by the child" — so a preview server that
        // forks a watcher has both ended, rather than the watcher surviving to
        // hold the port.
        spawning.process_group(0);
        let child = spawning
            .spawn()
            .map_err(|error| not_started(error, program, worktree))?;
        let group = child.id();
        Ok(Served { child, group })
    }

    /// [`spawn`](Served::spawn), for a server a person reads: both streams
    /// appended to `log` rather than discarded, and `env` added over Fleet's
    /// own. **The file is the pipe**, so nothing has to drain it and a server
    /// printing forever cannot block on a full buffer.
    pub fn spawn_logging(
        command: &str,
        worktree: &Path,
        env: &[(String, String)],
        log: &Path,
    ) -> Result<Served, NeverRan> {
        let Some((program, args)) = split(command) else {
            return Err(NeverRan::NothingToRun);
        };
        let opened = || {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
        };
        let (Ok(out), Ok(err)) = (opened(), opened()) else {
            return Err(NeverRan::NotSpawned {
                program,
                kind: std::io::ErrorKind::PermissionDenied,
            });
        };
        let mut spawning = Command::new(&program);
        spawning
            .args(&args)
            .current_dir(worktree)
            .envs(
                env.iter()
                    .map(|(name, value)| (name.as_str(), value.as_str())),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err))
            .kill_on_drop(true);
        spawning.process_group(0);
        let child = spawning
            .spawn()
            .map_err(|error| not_started(error, program, worktree))?;
        let group = child.id();
        Ok(Served { child, group })
    }

    /// How it ended, once it has. **Cancel-safe**, so a caller can race it
    /// against a stop and a readiness probe and ask again after either.
    pub async fn exited(&mut self) -> Exit {
        match self.child.wait().await {
            Ok(status) => ended(&status),
            Err(error) => Exit::NeverRan(NeverRan::NotSpawned {
                program: String::from("the server"),
                kind: error.kind(),
            }),
        }
    }

    /// Whether it is still running, asked without waiting.
    ///
    /// **A serve command that has already exited is the failure worth naming.**
    /// The readiness probe would otherwise spend its whole budget asking a port
    /// nothing is listening on, and report a timeout — which reads as a slow
    /// machine and is a command that died on its first line.
    pub fn still_up(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// End the group, and wait for the leader to be reaped.
    ///
    /// **The group first and the child second**, which is [`run`](crate::run)'s
    /// ordering and its reason: a group signalled after its leader has been
    /// reaped can land on a recycled group id, and doing it in this order makes
    /// that unreachable rather than rare.
    pub async fn end(mut self) {
        end_the_group(self.group);
        let _ = self.child.kill().await;
        // Nothing is left for `Drop` to signal, and signalling twice is what
        // the recycled-id argument above is about.
        self.group = None;
    }
}

impl Drop for Served {
    /// **The one that catches the paths nobody wrote.** `kill_on_drop` ends the
    /// leader and says nothing about the group it leads, so a serve command
    /// that forked would survive its own Job without this.
    fn drop(&mut self) {
        end_the_group(self.group);
    }
}
