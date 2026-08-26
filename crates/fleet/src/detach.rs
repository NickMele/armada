//! Spawning a child that outlives this process, and no other kind of spawn.
//!
//! # Why this is a type and not a helper function
//!
//! Every Drone is spawned into its own session. A supervisor signals a job's
//! whole process tree, so a Drone spawned as a plain child dies at every Fleet
//! restart — silently, mid-Job, burning tokens against a real repository, which
//! is the one thing "Fleet never auto-kills" forbids. A helper that a caller
//! remembers to use is exactly the convention failure this codebase is built to
//! remove, so there is no `Command` in this crate's public surface at all:
//! [`Detached`] is the only way to start a process from Fleet, and it applies
//! the call in its constructor. A caller cannot spawn an attached child because
//! the call does not exist, not because a review catches it.
//!
//! There is deliberately **no `process_group`** on this type, and no way to
//! reach the underlying `Command` and set one. That is not tidiness: setting a
//! process group and creating a session are mutually exclusive, and a spawn
//! that asks for both fails at `pre_exec` with `Operation not permitted` — the
//! caller having become a process-group leader is precisely why the session
//! call then refuses. v1 measured that. One flag, and it is not optional, so
//! the pair cannot be requested.
//!
//! # Why `libc` and not a shell
//!
//! macOS ships no `/usr/bin/setsid`, so this is a library call between fork and
//! exec rather than a wrapper program.
//!
//! # The one place `unsafe` is spoken in this workspace
//!
//! `pre_exec` is unsafe because the closure runs in the forked child, between
//! fork and exec, where only async-signal-safe calls are legal. The closure
//! below makes exactly one, and touches nothing else. `crates/fleet/Cargo.toml`
//! therefore sets `unsafe_code = "deny"` instead of inheriting the workspace's
//! `forbid`, so that this single site can carry an `allow` and every other site
//! in the crate still fails to compile. The deviation is one attribute wide and
//! is greppable.

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::Stdio;

use tokio::process::{Child, Command};

/// A child that will be spawned into a session of its own.
///
/// Cannot be built any other way, and cannot be turned back into a plain
/// command. The surface below is what Fleet needs to start a Drone and nothing
/// more — a new requirement is a new named method, never a raw argv escape
/// hatch, for the same reason `DroneSpawnConfig` has none.
pub struct Detached {
    command: Command,
}

impl Detached {
    /// A command that will detach when it is spawned.
    ///
    /// The session call is attached here rather than in [`Detached::spawn`], so
    /// there is no window in which a partially configured command exists
    /// without it.
    pub fn program(program: impl AsRef<OsStr>) -> Detached {
        let mut command = Command::new(program);
        detach(&mut command);
        Detached { command }
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Detached {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Detached
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    /// The worktree the child runs in.
    pub fn in_directory(mut self, directory: impl AsRef<Path>) -> Detached {
        self.command.current_dir(directory);
        self
    }

    pub fn with_env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Detached {
        self.command.env(key, value);
        self
    }

    /// Start from an empty environment rather than inheriting Fleet's.
    pub fn with_clean_env(mut self) -> Detached {
        self.command.env_clear();
        self
    }

    pub fn capturing_output(mut self) -> Detached {
        self.command.stdout(Stdio::piped()).stderr(Stdio::piped());
        self
    }

    /// Start it. The child is a session leader before its program is running,
    /// so nothing signalled at Fleet's process group reaches it.
    pub fn spawn(mut self) -> io::Result<Child> {
        self.command.spawn()
    }
}

/// The fork-to-exec step. `setsid` puts the child in a new session and a new
/// process group, both led by the child, which is what makes a group-directed
/// signal at Fleet stop at Fleet.
///
/// It fails only when the caller is already a process-group leader, which the
/// forked child never is. The error is returned rather than ignored: a child
/// that silently stayed attached is the failure mode this whole module exists
/// to remove, so it must not be able to look like a successful spawn.
#[allow(unsafe_code)]
fn detach(command: &mut Command) {
    // SAFETY: the closure runs in the forked child before exec, where only
    // async-signal-safe calls are permitted. `setsid` is one, it allocates
    // nothing, and it is the only call made here.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}
