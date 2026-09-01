//! Running one Check in one worktree, bounded.
//! **There is no shell.** A Manifest's `run` is split into a program and its
//! arguments and executed directly. Handing the string to `sh -c` would make
//! the two failures this module exists to separate indistinguishable: a shell
//! reports *command not found* as exit `127`, and `127` is also a code a real
//! program returns. The Check would then fail — correctly, this time — for a
//! reason nobody could read, and a step expecting `127` would pass on a command
//! that was never installed. Spawning the program directly makes "not found" an
//! operating system error before any code exists. The cost is that a `run`
//! string cannot pipe, redirect or chain, which is consistent with what a Check
//! is: a command and an exit code, with nothing reading its output. A
//! repository that needs a pipeline writes a script and names the script.
//!
//! **A hanging Check is a failure, and its children go with it.** The child is
//! put in a process group of its own and the whole group is ended when the
//! budget expires. Killing only the process Fleet started leaves the test
//! runner it spawned holding the worktree and the CPU, which is v1's shape of
//! this failure: the Job ends, the machine does not notice, and the next Job is
//! slower for reasons nobody connects.
//!
//! **Nothing here reads the output.** It is captured so a person can read it
//! and returned to the caller; no branch in this file looks at a byte of it.
//! Deciding which lines were the failure is a Judge's question answered by
//! reading the diff, and a runner that grepped stdout would answer it badly.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use verification::{Exit, NeverRan};

/// How much of a Check's output is kept.
///
/// **The tail, not the head.** A test runner prints its failures last, and a
/// runaway command prints forever — so keeping the beginning would keep the
/// part nobody needs and grow without bound doing it.
const CAPTURE_LIMIT: usize = 64 * 1024;

/// What a Check printed, for a person to read.
///
/// Lossy on purpose, and it says so: `truncated` is what stops a reader
/// treating a cut-off log as a complete one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    /// Whether either stream was longer than the capture limit.
    pub truncated: bool,
}

/// One Check, run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    /// How it ended. **The fact the gate reads**, and the only part of this
    /// value anything decides on.
    pub exit: Exit,
    /// What it printed. Read by a person, never by a rule.
    pub output: Output,
}

impl Attempt {
    fn never(why: NeverRan) -> Attempt {
        Attempt {
            exit: Exit::NeverRan(why),
            output: Output::default(),
        }
    }
}

/// Run a Manifest Check's command in a Job's worktree, bounded by `budget`.
///
/// Every way this can go wrong produces an [`Exit`] that the gate cannot
/// compare into a pass. There is no error return, because there is no failure
/// here that means *the Check has not been decided* — a Check that could not be
/// started is a Check that failed.
pub async fn run(command: &str, worktree: &Path, budget: Duration) -> Attempt {
    let Some((program, args)) = split(command) else {
        return Attempt::never(NeverRan::NothingToRun);
    };

    let mut spawning = Command::new(&program);
    spawning
        .args(&args)
        .current_dir(worktree)
        // A Check that waits on input waits forever, and the budget would be
        // the only thing that ended it. Null is what makes it fail fast.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // Zero means "a new group, led by the child". The group id is then the
    // child's pid, which is what the timeout path signals.
    spawning.process_group(0);

    let mut child = match spawning.spawn() {
        Ok(child) => child,
        Err(error) => return Attempt::never(not_started(error, program, worktree)),
    };

    let group = child.id();
    let (Some(mut stdout), Some(mut stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return Attempt::never(NeverRan::NotSpawned {
            program,
            kind: std::io::ErrorKind::BrokenPipe,
        });
    };

    let mut out = Vec::new();
    let mut err = Vec::new();
    let finished = {
        let reading = async {
            let (_, _, status) = tokio::try_join!(
                stdout.read_to_end(&mut out),
                stderr.read_to_end(&mut err),
                child.wait(),
            )?;
            Ok::<std::process::ExitStatus, std::io::Error>(status)
        };
        tokio::time::timeout(budget, reading).await
    };

    match finished {
        Ok(Ok(status)) => Attempt {
            exit: ended(&status),
            output: captured(&out, &err),
        },
        Ok(Err(error)) => Attempt::never(NeverRan::NotSpawned {
            program,
            kind: error.kind(),
        }),
        Err(_) => {
            // The group first and the child second. A group signalled after
            // its leader has been reaped can land on a recycled group id, and
            // the ordering is what makes that unreachable rather than rare.
            end_the_group(group);
            let _ = child.kill().await;
            Attempt {
                exit: Exit::TimedOut { after: budget },
                output: captured(&out, &err),
            }
        }
    }
}

/// Why the spawn failed, with the one ambiguity resolved.
///
/// The operating system reports both a missing program and a missing working
/// directory as *not found*, and the two need opposite responses — install the
/// tool, or find out what removed a running Job's checkout. The worktree is
/// probed only on that path, so the ordinary spawn costs no extra call.
fn not_started(error: std::io::Error, program: String, worktree: &Path) -> NeverRan {
    match error.kind() {
        std::io::ErrorKind::NotFound if !worktree.is_dir() => NeverRan::WorktreeGone {
            worktree: worktree.display().to_string(),
        },
        std::io::ErrorKind::NotFound => NeverRan::NoSuchCommand { program },
        kind => NeverRan::NotSpawned { program, kind },
    }
}

/// A code, or the signal that meant there was none.
fn ended(status: &std::process::ExitStatus) -> Exit {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => Exit::Code(code),
        // Unreachable on Unix without a signal, and an absent code with no
        // signal is still not a code — so it is reported as a signal of zero
        // rather than folded into `Exit::Code`.
        None => Exit::Signalled {
            signal: status.signal().unwrap_or(0),
        },
    }
}

/// Send `SIGKILL` to the child's whole process group.
///
/// `SIGKILL` rather than `SIGTERM`: the budget has already expired, so a
/// graceful shutdown window would be a second budget nobody configured.
#[allow(unsafe_code)]
fn end_the_group(group: Option<u32>) {
    let Some(group) = group else { return };
    // SAFETY: `killpg` is a plain system call taking two integers. The group
    // id is the child's own pid, made a group leader by `process_group(0)`
    // above, so the signal reaches the Check's processes and nothing else.
    unsafe {
        libc::killpg(group as libc::pid_t, libc::SIGKILL);
    }
}

/// The tail of each stream, as text.
fn captured(stdout: &[u8], stderr: &[u8]) -> Output {
    let truncated = stdout.len() > CAPTURE_LIMIT || stderr.len() > CAPTURE_LIMIT;
    Output {
        stdout: tail(stdout),
        stderr: tail(stderr),
        truncated,
    }
}

fn tail(bytes: &[u8]) -> String {
    let from = bytes.len().saturating_sub(CAPTURE_LIMIT);
    String::from_utf8_lossy(&bytes[from..]).into_owned()
}

/// Split a `run` string into a program and its arguments.
///
/// Single and double quotes group a word and are not kept. Nothing else is
/// interpreted — no variable expansion, no escapes, no globbing — because each
/// of those is a shell feature and this is not a shell. `None` where the string
/// holds no program at all.
fn split(command: &str) -> Option<(String, Vec<String>)> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;

    for c in command.chars() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), c) => word.push(c),
            (None, '\'') | (None, '"') => {
                quote = Some(c);
                // An empty quoted string is an argument, and it would otherwise
                // vanish along with the quotes that made it.
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, c) => word.push(c),
        }
    }
    if started || !word.is_empty() {
        words.push(word);
    }

    let mut words = words.into_iter();
    let program = words.next()?;
    Some((program, words.collect()))
}
