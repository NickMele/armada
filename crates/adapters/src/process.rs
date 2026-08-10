//! The process-group spawn/kill wrapper, and the real [`Run`] seam.
//!
//! Three rules, all of them measured (`docs/traps.md`), and each one is a
//! silent failure if you get it wrong:
//!
//! 1. **Spawn into a new session with `setsid`**, so one `killpg` reaches
//!    grandchildren. Verified: a group of three goes to zero.
//! 2. **SIGTERM, grace, then SIGKILL — unconditionally.** A leader running
//!    `trap '' TERM` leaves 3 of 3 alive after `killpg(SIGTERM)`, because
//!    children inherit an ignored disposition across `fork` and `exec`.
//! 3. **Every spawned `Child` is waited on.** Rust's `Child` does not reap on
//!    drop, so a dropped handle leaves a `<defunct>` entry until char exits —
//!    and a fifteen-minute detached run accumulates them.
//!
//! Rule 3 is why this module owns the child rather than handing it out: a
//! caller cannot forget to wait for something it never held.

use charkit_core::ctx::{Run, RunOutput, RunRequest, SpawnError, SpawnErrorKind, StdioMode};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::posix;

/// How long a group gets between SIGTERM and SIGKILL.
///
/// Long enough for a compose stack or a dev server to flush and exit, short
/// enough that `char clean` on a wedged tree is not something you wait out.
pub const GRACE: Duration = Duration::from_secs(5);

/// The most output char keeps from one child.
///
/// `run_retention` is a count of runs, not a size, so nothing else bounds a
/// single one: a `commands:` entry writing gigabytes to stdout under
/// `stdio: pipe` fills the disk with char faithfully copying every byte, and
/// the disk-full failure then lands on the state store (PLAN.md §3.1). Head and
/// tail are retained with the middle elided, so a truncated stream never reads
/// as a complete one.
pub const CAPTURE_CAP: usize = 10 * 1024 * 1024;

/// How often the wait loop wakes to check a deadline.
const POLL: Duration = Duration::from_millis(10);

/// The production [`Run`].
///
/// Zero-sized: everything it needs arrives in the [`RunRequest`], which is what
/// makes the fake a drop-in and the argv assertable.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealRun;

impl Run for RealRun {
    fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
        let mut group = ProcessGroup::spawn(request)?;
        Ok(group.wait(request.timeout))
    }
}

/// A child and everything char needs to reach its whole tree.
pub struct ProcessGroup {
    child: Child,
    pgid: i32,
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
}

impl ProcessGroup {
    /// Spawn, in its own session when the request asks for one.
    pub fn spawn(request: &RunRequest) -> Result<Self, SpawnError> {
        let (program, args) = request.argv.split_first().ok_or_else(|| SpawnError {
            program: String::new(),
            kind: SpawnErrorKind::Other,
            message: "empty argv".to_string(),
        })?;

        let mut command = Command::new(program);
        command.args(args).current_dir(&request.cwd);
        for (key, value) in &request.env {
            command.env(key, value);
        }

        match request.stdio {
            StdioMode::Capture => {
                command
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
            }
            StdioMode::Inherit => {
                command
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());
            }
        }

        if request.new_session {
            posix::new_session(&mut command);
        }

        let mut child = command.spawn().map_err(|e| SpawnError {
            program: program.clone(),
            kind: match e.kind() {
                std::io::ErrorKind::NotFound => SpawnErrorKind::NotFound,
                std::io::ErrorKind::PermissionDenied => SpawnErrorKind::PermissionDenied,
                _ => SpawnErrorKind::Other,
            },
            message: e.to_string(),
        })?;

        // With `setsid` the child *is* its own group leader, so its pid is the
        // pgid. Without it the child joins char's group, and killing that would
        // kill char — so an un-detached request records no group to kill.
        let pgid = if request.new_session {
            child.id() as i32
        } else {
            0
        };

        Ok(ProcessGroup {
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            child,
            pgid,
        })
    }

    /// The group to kill. `0` when the child was not detached.
    pub fn pgid(&self) -> i32 {
        self.pgid
    }

    /// The direct child's pid.
    pub fn pid(&self) -> i32 {
        self.child.id() as i32
    }

    /// Wait for the whole tree, killing the group if char's own deadline
    /// elapses.
    ///
    /// The reader threads are what make a deadline meaningful: reading the
    /// pipes inline would block char in `read` while the clock ran out, and
    /// draining them only after `wait` deadlocks the moment a child fills a
    /// pipe buffer.
    pub fn wait(&mut self, timeout: Option<Duration>) -> RunOutput {
        let out_reader = self.stdout.take().map(spawn_reader);
        let err_reader = self.stderr.take().map(spawn_reader);

        let deadline = timeout.map(|t| Instant::now() + t);
        let mut timed_out = false;

        let status = loop {
            match self.child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {}
                // The only way `try_wait` errors is a handle char no longer
                // owns, which it cannot recover from and must not spin on.
                Err(_) => break None,
            }
            if let Some(deadline) = deadline {
                if Instant::now() >= deadline {
                    timed_out = true;
                    if self.pgid != 0 {
                        posix::stop_group(self.pgid, GRACE);
                    } else {
                        let _ = self.child.kill();
                    }
                    // Rule 3: reap the direct child even here. A timeout is
                    // exactly when a dropped handle would go unnoticed.
                    break self.child.wait().ok();
                }
            }
            std::thread::sleep(POLL);
        };

        let stdout = out_reader.map(join).unwrap_or_default();
        let stderr = err_reader.map(join).unwrap_or_default();

        RunOutput {
            code: status.as_ref().and_then(std::process::ExitStatus::code),
            signal: status.as_ref().and_then(signal_of),
            stdout,
            stderr,
            timed_out,
        }
    }

    /// Stop the whole tree: TERM, grace, KILL — then reap.
    pub fn stop(&mut self) -> posix::StopReport {
        let report = if self.pgid != 0 {
            posix::stop_group(self.pgid, GRACE)
        } else {
            let _ = self.child.kill();
            posix::StopReport {
                existed: true,
                escalated: true,
                gone: true,
            }
        };
        let _ = self.child.wait();
        report
    }
}

impl Drop for ProcessGroup {
    /// The zombie guard, and it is a backstop rather than the mechanism.
    ///
    /// Measured: a `Child` dropped without `wait()` leaves a process in state
    /// `Z` — Rust does not reap on drop, and the docs say so. Every path above
    /// waits; this catches the one a future edit forgets.
    fn drop(&mut self) {
        let _ = self.child.try_wait();
    }
}

#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

fn spawn_reader<R: Read + Send + 'static>(mut source: R) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut head = Vec::new();
        let mut tail: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 8192];
        let mut elided = 0usize;

        loop {
            match source.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let chunk = &buffer[..n];
                    if head.len() < CAPTURE_CAP / 2 {
                        let room = CAPTURE_CAP / 2 - head.len();
                        head.extend_from_slice(&chunk[..room.min(n)]);
                        if n > room {
                            tail.extend_from_slice(&chunk[room..]);
                        }
                    } else {
                        tail.extend_from_slice(chunk);
                    }
                    if tail.len() > CAPTURE_CAP / 2 {
                        let drop = tail.len() - CAPTURE_CAP / 2;
                        tail.drain(..drop);
                        elided += drop;
                    }
                }
            }
        }

        let mut text = String::from_utf8_lossy(&head).into_owned();
        if elided > 0 {
            text.push_str(&format!(
                "\n… {elided} bytes elided: output exceeded char's {CAPTURE_CAP}-byte cap …\n"
            ));
        }
        text.push_str(&String::from_utf8_lossy(&tail));
        text
    })
}

fn join(handle: std::thread::JoinHandle<String>) -> String {
    handle.join().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn request(argv: &[&str]) -> RunRequest {
        RunRequest::new(
            argv.iter().map(|s| s.to_string()).collect(),
            PathBuf::from("/"),
        )
    }

    #[test]
    fn a_child_runs_and_its_output_comes_back() {
        let out = RealRun
            .call(&request(&["/bin/sh", "-c", "echo hi; echo bad >&2"]))
            .unwrap();
        assert_eq!(out.code, Some(0));
        assert_eq!(out.stdout.trim(), "hi");
        assert_eq!(out.stderr.trim(), "bad");
        assert!(!out.timed_out);
    }

    #[test]
    fn an_exit_code_comes_back_verbatim() {
        let out = RealRun
            .call(&request(&["/bin/sh", "-c", "exit 42"]))
            .unwrap();
        assert_eq!(out.code, Some(42));
    }

    #[test]
    fn a_program_that_is_not_there_is_a_typed_spawn_failure() {
        let err = RealRun
            .call(&request(&["/nonexistent/definitely-not-a-program"]))
            .unwrap_err();
        assert_eq!(err.kind, SpawnErrorKind::NotFound);
        assert_eq!(err.program, "/nonexistent/definitely-not-a-program");
    }

    #[test]
    fn the_environment_is_layered_over_the_inherited_one() {
        let mut env = BTreeMap::new();
        env.insert("CHAR_TEST_VALUE".to_string(), "layered".to_string());
        let out = RealRun
            .call(&request(&["/bin/sh", "-c", "echo $CHAR_TEST_VALUE:$PATH"]).env(env))
            .unwrap();
        assert!(out.stdout.starts_with("layered:"));
        // `$PATH` survived, so the layer was additive rather than a replacement.
        assert!(out.stdout.trim().len() > "layered:".len());
    }

    #[test]
    fn a_deadline_kills_the_group_and_says_so() {
        let out = RealRun
            .call(&request(&["/bin/sh", "-c", "sleep 30"]).timeout(Duration::from_millis(200)))
            .unwrap();
        assert!(out.timed_out);
    }

    #[test]
    fn the_working_directory_is_always_explicit() {
        let out = RealRun.call(&request(&["/bin/sh", "-c", "pwd"])).unwrap();
        assert_eq!(out.stdout.trim(), "/");
    }
}
