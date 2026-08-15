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
//!    drop, so a dropped handle leaves a `<defunct>` entry until Armada exits —
//!    and a fifteen-minute detached run accumulates them.
//!
//! Rule 3 is why this module owns the child rather than handing it out: a
//! caller cannot forget to wait for something it never held.

use armada_core::ctx::{Run, RunOutput, RunRequest, SpawnError, SpawnErrorKind, StdioMode};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::posix;

/// How long a group gets between SIGTERM and SIGKILL.
///
/// Long enough for a compose stack or a dev server to flush and exit, short
/// enough that `armada manifest clean` on a wedged tree is not something you wait out.
pub const GRACE: Duration = Duration::from_secs(5);

/// The most output Armada keeps from one child.
///
/// `run_retention` is a count of runs, not a size, so nothing else bounds a
/// single one: a `commands:` entry writing gigabytes to stdout under
/// `stdio: pipe` fills the disk with Armada faithfully copying every byte, and
/// the disk-full failure then lands on the state store (PLAN.md §3.1). Head and
/// tail are retained with the middle elided, so a truncated stream never reads
/// as a complete one.
pub const CAPTURE_CAP: usize = 10 * 1024 * 1024;

/// How often the wait loop wakes to check a deadline.
const POLL: Duration = Duration::from_millis(10);

/// How often the wait loop calls the caller's tick — the lease heartbeat.
///
/// Matches [`armada_core::lease::RENEW_INTERVAL_MS`]: twelve of these fit
/// inside the cold threshold, which is the margin that makes a missed renewal
/// a real signal rather than a jitter artefact.
const TICK: Duration = Duration::from_millis(armada_core::lease::RENEW_INTERVAL_MS);

/// The production [`Run`].
///
/// Zero-sized: everything it needs arrives in the [`RunRequest`], which is what
/// makes the fake a drop-in and the argv assertable.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealRun;

impl Run for RealRun {
    fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
        let mut group = ProcessGroup::spawn(request)?;
        Ok(group.wait(request.timeout, &mut || {}))
    }

    fn call_with_tick(
        &self,
        request: &RunRequest,
        tick: &mut dyn FnMut(),
    ) -> Result<RunOutput, SpawnError> {
        let mut group = ProcessGroup::spawn(request)?;
        Ok(group.wait(request.timeout, tick))
    }
}

/// A child and everything Armada needs to reach its whole tree.
pub struct ProcessGroup {
    child: Child,
    pgid: i32,
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    readers: Option<Readers>,
    timed_out: bool,
}

/// The two draining threads, once started.
struct Readers {
    stdout: Option<std::thread::JoinHandle<String>>,
    stderr: Option<std::thread::JoinHandle<String>>,
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

        // A document on stdin needs a pipe to write it down, whatever the
        // output streams do.
        let stdin = match request.stdin {
            Some(_) => Stdio::piped(),
            None => Stdio::null(),
        };
        match &request.stdio {
            StdioMode::Capture => {
                command
                    .stdin(stdin)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
            }
            // **Only stdout is Armada's.** The other two stay the terminal's so
            // that a secret provider can prompt on one and complain on the
            // other — see [`StdioMode::CaptureStdout`], which is this variant's
            // only caller and states why each stream goes where it does.
            StdioMode::CaptureStdout => {
                command
                    .stdin(match request.stdin {
                        Some(_) => Stdio::piped(),
                        None => Stdio::inherit(),
                    })
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit());
            }
            StdioMode::Inherit => {
                command
                    .stdin(match request.stdin {
                        Some(_) => Stdio::piped(),
                        None => Stdio::inherit(),
                    })
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());
            }
            // **A real file descriptor, so the child outlives Armada.** A
            // service started under `Capture` would hold the read end of a pipe
            // Armada is about to drop, and the first write after that is
            // `EPIPE` — which kills the service moments after `up` reported it
            // healthy.
            StdioMode::Log(path) => {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| SpawnError {
                        program: program.clone(),
                        kind: SpawnErrorKind::Other,
                        message: format!("cannot open {}: {e}", path.display()),
                    })?;
                let stderr = file.try_clone().map_err(|e| SpawnError {
                    program: program.clone(),
                    kind: SpawnErrorKind::Other,
                    message: format!("cannot open {}: {e}", path.display()),
                })?;
                command
                    .stdin(stdin)
                    .stdout(Stdio::from(file))
                    .stderr(Stdio::from(stderr));
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

        // **Written and then closed, before anything waits.** compose reads the
        // whole document before it does any work, so a handle left open is a
        // child that never sees EOF and never starts the stack. A document
        // larger than the pipe buffer would deadlock a caller that wrote it
        // after `wait`, which is why it happens here.
        if let Some(text) = &request.stdin {
            if let Some(mut pipe) = child.stdin.take() {
                use std::io::Write;
                let _ = pipe.write_all(text.as_bytes());
                let _ = pipe.flush();
            }
        }

        // With `setsid` the child *is* its own group leader, so its pid is the
        // pgid. Without it the child joins Armada's group, and killing that would
        // kill Armada — so an un-detached request records no group to kill.
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
            readers: None,
            timed_out: false,
        })
    }

    /// Has this child finished? **Never blocks.**
    ///
    /// This is what lets `armada manifest check` run several checks at once. The scheduler
    /// is a reducer over one run (`ARCHITECTURE.md` §1.2) and the shell executes
    /// what it proposes, so the shell holds N children and asks each of them
    /// this question on every turn of its loop.
    ///
    /// **The event loop never blocking is an invariant rather than a
    /// preference** (PLAN.md §4.3): the lease heartbeat is renewed from that
    /// loop and from no background timer, precisely so that a wedged loop is a
    /// loop that stopped renewing and the existing cold-heartbeat path reclaims
    /// it. Every blocking call added to it weakens the reclaim guarantee, which
    /// is why the blocking [`ProcessGroup::wait`] stays for the one-shot callers
    /// — git, docker, a `setup:` step — and this exists for the run.
    ///
    /// **The one residual, stated rather than papered over.** Once the child has
    /// exited this joins the reader threads, and a reader reaches EOF when the
    /// last holder of the pipe closes it. A *grandchild* that outlived the group
    /// still holds it — which for Armada's own children means one that called
    /// `setsid` for itself and left the tracked group, the case `docs/traps.md`
    /// records as detected rather than prevented. The kill path closes it for
    /// every other shape, because `killpg` reaches grandchildren.
    pub fn poll(&mut self) -> Option<RunOutput> {
        self.start_readers();
        let status = match self.child.try_wait() {
            Ok(Some(status)) => Some(status),
            Ok(None) => return None,
            // The only way `try_wait` errors is a handle Armada no longer owns,
            // which it cannot recover from and must not spin on.
            Err(_) => None,
        };
        Some(self.finish(status))
    }

    /// Signal the group: SIGTERM, or SIGKILL when `escalate`.
    ///
    /// **An unconditional escalation, not a retry** (`docs/traps.md`): measured,
    /// a leader running `trap '' TERM` leaves 3 of 3 alive after
    /// `killpg(SIGTERM)`, because children inherit an *ignored* disposition
    /// across `fork` and `exec` — so one uncooperative leader immunises its
    /// whole group, and a second SIGTERM is ignored exactly like the first.
    ///
    /// Unlike [`ProcessGroup::stop`] this does not wait out a grace period. The
    /// grace belongs to the caller's own loop, which has other children to
    /// attend to.
    pub fn signal(&mut self, escalate: bool) {
        let signal = if escalate {
            libc::SIGKILL
        } else {
            libc::SIGTERM
        };
        if self.pgid != 0 {
            let _ = posix::killpg(self.pgid, signal);
        } else if escalate {
            let _ = self.child.kill();
        }
        self.timed_out = true;
    }

    fn start_readers(&mut self) {
        if self.readers.is_none() {
            self.readers = Some(Readers {
                stdout: self.stdout.take().map(spawn_reader),
                stderr: self.stderr.take().map(spawn_reader),
            });
        }
    }

    fn finish(&mut self, status: Option<std::process::ExitStatus>) -> RunOutput {
        let (stdout, stderr) = match self.readers.take() {
            Some(readers) => (
                readers.stdout.map(join).unwrap_or_default(),
                readers.stderr.map(join).unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        };
        RunOutput {
            code: status.as_ref().and_then(std::process::ExitStatus::code),
            signal: status.as_ref().and_then(signal_of),
            stdout,
            stderr,
            timed_out: self.timed_out,
        }
    }

    /// The group to kill. `0` when the child was not detached.
    pub fn pgid(&self) -> i32 {
        self.pgid
    }

    /// The direct child's pid.
    pub fn pid(&self) -> i32 {
        self.child.id() as i32
    }

    /// Wait for the whole tree, killing the group if Armada's own deadline
    /// elapses.
    ///
    /// The reader threads are what make a deadline meaningful: reading the
    /// pipes inline would block Armada in `read` while the clock ran out, and
    /// draining them only after `wait` deadlocks the moment a child fills a
    /// pipe buffer.
    pub fn wait(&mut self, timeout: Option<Duration>, tick: &mut dyn FnMut()) -> RunOutput {
        self.start_readers();

        let deadline = timeout.map(|t| Instant::now() + t);
        let mut next_tick = Instant::now() + TICK;

        let status = loop {
            match self.child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {}
                // The only way `try_wait` errors is a handle Armada no longer
                // owns, which it cannot recover from and must not spin on.
                Err(_) => break None,
            }
            if let Some(deadline) = deadline {
                if Instant::now() >= deadline {
                    self.timed_out = true;
                    if self.pgid != 0 {
                        // Reaped through the handle rather than by group, so
                        // the `wait` below still finds the status: `Child`
                        // caches what `try_wait` collected, where a bare
                        // `waitpid` would have consumed it and left this
                        // answering `ECHILD`.
                        let pgid = self.pgid;
                        let child = &mut self.child;
                        posix::stop_group_reaping(pgid, GRACE, &mut || {
                            let _ = child.try_wait();
                        });
                    } else {
                        let _ = self.child.kill();
                    }
                    // Rule 3: reap the direct child even here. A timeout is
                    // exactly when a dropped handle would go unnoticed.
                    break self.child.wait().ok();
                }
            }
            if Instant::now() >= next_tick {
                tick();
                next_tick = Instant::now() + TICK;
            }
            std::thread::sleep(POLL);
        };

        self.finish(status)
    }

    /// Stop the whole tree: TERM, grace, KILL, **reaping as it goes**.
    ///
    /// The reap is interleaved rather than done at the end, because `gone` is
    /// read off a probe that a corpse of this process's own would answer — and
    /// the two platforms answer it differently while the corpse is unreaped
    /// (`docs/traps.md`). Reaping through the `Child` keeps the exit status
    /// where the handle can still see it.
    pub fn stop(&mut self) -> posix::StopReport {
        let report = if self.pgid != 0 {
            let pgid = self.pgid;
            let child = &mut self.child;
            posix::stop_group_reaping(pgid, GRACE, &mut || {
                let _ = child.try_wait();
            })
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
                "\n… {elided} bytes elided: output exceeded Armada's {CAPTURE_CAP}-byte cap …\n"
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

    /// Poll until the child is done, or give up. Bounded, because a hang here
    /// must fail the test rather than the suite.
    fn poll_until_done(group: &mut ProcessGroup, within: Duration) -> Option<RunOutput> {
        let deadline = Instant::now() + within;
        loop {
            if let Some(output) = group.poll() {
                return Some(output);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// **The invariant the whole run depends on**: the shell's event loop never
    /// blocks, so that a wedged loop is a loop that stopped renewing and the
    /// cold-heartbeat path reclaims it (PLAN.md §4.3).
    #[test]
    fn polling_a_running_child_answers_at_once_rather_than_waiting_for_it() {
        let mut group = ProcessGroup::spawn(&request(&["sleep", "30"])).unwrap();

        let before = Instant::now();
        let answer = group.poll();
        let elapsed = before.elapsed();

        assert!(answer.is_none(), "a running child reported a verdict");
        assert!(
            elapsed < Duration::from_millis(500),
            "poll blocked for {elapsed:?}"
        );

        group.signal(true);
        assert!(poll_until_done(&mut group, Duration::from_secs(10)).is_some());
    }

    #[test]
    fn polling_a_finished_child_answers_with_its_code_and_its_output() {
        let mut group =
            ProcessGroup::spawn(&request(&["sh", "-c", "echo out; echo err >&2; exit 3"])).unwrap();
        let output = poll_until_done(&mut group, Duration::from_secs(10)).expect("it finished");
        assert_eq!(output.code, Some(3));
        assert_eq!(output.stdout.trim(), "out");
        assert_eq!(output.stderr.trim(), "err");
    }

    /// **An unconditional escalation, not a retry.** Measured in `traps.md`: a
    /// leader running `trap '' TERM` leaves its whole group alive, because
    /// children inherit an *ignored* disposition across `fork` and `exec` — so
    /// a second SIGTERM is ignored exactly like the first, and only SIGKILL
    /// ends it.
    ///
    /// A cooperative `sleep` passes the first half of this while proving
    /// nothing, which is why the uncooperative case is the one asserted on.
    #[test]
    fn a_group_that_ignores_sigterm_survives_it_and_dies_on_the_escalation() {
        let mut group =
            ProcessGroup::spawn(&request(&["sh", "-c", "trap '' TERM; sleep 30"])).unwrap();
        // Let the trap be installed before signalling it.
        std::thread::sleep(Duration::from_millis(200));

        group.signal(false);
        assert!(
            poll_until_done(&mut group, Duration::from_millis(500)).is_none(),
            "SIGTERM ended a group that ignores SIGTERM"
        );

        group.signal(true);
        assert!(
            poll_until_done(&mut group, Duration::from_secs(10)).is_some(),
            "SIGKILL did not end the group"
        );
    }

    /// A polled child is reaped like any other. Measured: Rust's `Child` does
    /// not reap on drop, so a fifteen-minute run polling N children would
    /// accumulate a `<defunct>` entry for each one.
    #[test]
    fn a_polled_child_leaves_no_zombie() {
        let mut group = ProcessGroup::spawn(&request(&["true"])).unwrap();
        let pid = group.pid();
        assert!(poll_until_done(&mut group, Duration::from_secs(10)).is_some());

        let stat = std::process::Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        assert!(!stat.starts_with('Z'), "left a zombie: {stat:?}");
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
        env.insert("ARMADA_TEST_VALUE".to_string(), "layered".to_string());
        let out = RealRun
            .call(&request(&["/bin/sh", "-c", "echo $ARMADA_TEST_VALUE:$PATH"]).env(env))
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
