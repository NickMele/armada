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

use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use verification::{Exit, NeverRan};

/// How much of a Check's output is kept.
///
/// **The tail, not the head.** A test runner prints its failures last, and a
/// runaway command prints forever — so keeping the beginning would keep the
/// part nobody needs and grow without bound doing it.
const CAPTURE_LIMIT: usize = 64 * 1024;

/// How much of a Check's output is written to the live file before it stops.
///
/// **A bound on the disk, not on what the gate keeps.** The tail the record
/// keeps is [`CAPTURE_LIMIT`] and is unaffected. This stops a runaway command
/// filling a disk with a file whose only reader is a person watching it, and
/// the file says where it stopped rather than simply ending.
const LIVE_LIMIT: u64 = 8 * 1024 * 1024;

/// What the live file says where it stops being written.
const LIVE_CUT: &[u8] =
    b"\n--- the rest was not written here; the Check log recorded at the ruling keeps its tail ---\n";

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
    run_writing(command, worktree, budget, None).await
}

/// [`run`], and every chunk of output appended to `live` as it is read, so a
/// person watching a Check that takes minutes can read what it has printed so
/// far. **Nothing decides on that file**: the [`Attempt`] handed back is
/// captured exactly as [`run`] captures it.
///
/// **Both streams go into one file, in the order they arrived.** That is what
/// a terminal shows, and it is the only order there is while the Check is
/// still running. The recorded Check log keeps the two apart behind markers,
/// because by then each has been captured whole; this file cannot, and does
/// not pretend to.
///
/// **A file that will not open costs the Check nothing.** It runs, and is
/// ruled on, exactly as it would have with `None`: what is lost is a person's
/// view of it, and refusing to run a Check because a log would not open would
/// lose the verdict as well.
pub async fn run_writing(
    command: &str,
    worktree: &Path,
    budget: Duration,
    live: Option<&Path>,
) -> Attempt {
    let writing = live.map_or(Writing::Nowhere, Writing::Fresh);
    run_until(command, worktree, budget, writing, std::future::pending()).await
}

/// Where a run's output is written as it arrives.
#[derive(Clone, Copy, Debug)]
pub enum Writing<'p> {
    Nowhere,
    /// Truncated first: one run, one file.
    Fresh(&'p Path),
    /// Added to the end, so a Check's prerequisites and the Check itself read
    /// as one log in the order they ran.
    Appending(&'p Path),
}

/// [`run_writing`], ended early the moment `stop` completes.
///
/// **A stop ends the whole group**, for the budget's reason: the test runner a
/// command started would otherwise outlive the person pressing Stop. What
/// printed before it is kept, and the exit is the signal that ended it.
pub async fn run_until<S: std::future::Future<Output = ()>>(
    command: &str,
    worktree: &Path,
    budget: Duration,
    writing: Writing<'_>,
    stop: S,
) -> Attempt {
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
    let live = Mutex::new(match writing {
        Writing::Nowhere => None,
        Writing::Fresh(path) => Live::create(path),
        Writing::Appending(path) => Live::append(path),
    });
    let finished = {
        let reading = async {
            let (_, _, status) = tokio::try_join!(
                pumped(&mut stdout, &mut out, &live),
                pumped(&mut stderr, &mut err, &live),
                child.wait(),
            )?;
            Ok::<std::process::ExitStatus, std::io::Error>(status)
        };
        tokio::select! {
            ran = tokio::time::timeout(budget, reading) => Some(ran),
            () = stop => None,
        }
    };

    match finished {
        Some(Ok(Ok(status))) => Attempt {
            exit: ended(&status),
            output: captured(&out, &err),
        },
        Some(Ok(Err(error))) => Attempt::never(NeverRan::NotSpawned {
            program,
            kind: error.kind(),
        }),
        // Stopped from outside. The group first, for the ordering below.
        None => {
            end_the_group(group);
            let _ = child.kill().await;
            Attempt {
                exit: Exit::Signalled {
                    signal: libc::SIGKILL,
                },
                output: captured(&out, &err),
            }
        }
        Some(Err(_)) => {
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

/// Read one stream to its end, keeping every byte and writing each chunk on.
///
/// **The same bytes `read_to_end` would have kept**, in the same buffer, so
/// what [`captured`] makes of them does not depend on whether anybody was
/// watching. The chunk is written to the live file after it is kept, and a
/// write that fails is dropped rather than returned: the stream is the Check's
/// and the file is a person's.
async fn pumped<R: AsyncRead + Unpin>(
    from: &mut R,
    into: &mut Vec<u8>,
    live: &Mutex<Option<Live>>,
) -> std::io::Result<()> {
    let mut chunk = [0u8; 8 * 1024];
    loop {
        let read = from.read(&mut chunk).await?;
        if read == 0 {
            return Ok(());
        }
        into.extend_from_slice(&chunk[..read]);
        // Never held across an `.await`: the lock covers one `write` call on a
        // file, and the other stream waits for that and nothing longer.
        if let Ok(mut held) = live.lock() {
            if let Some(file) = held.as_mut() {
                file.wrote(&chunk[..read]);
            }
        }
    }
}

/// The file a Check's output is written to while it runs, and how much of it
/// has been.
struct Live {
    file: std::fs::File,
    written: u64,
    cut: bool,
}

impl Live {
    /// Truncated rather than appended to: a gate run again on the same attempt
    /// writes the same name, and a file holding two runs would read as one.
    fn create(path: &Path) -> Option<Live> {
        std::fs::File::create(path).ok().map(|file| Live {
            file,
            written: 0,
            cut: false,
        })
    }

    /// Added to rather than truncated, counting what is already there against
    /// the limit so a run of several commands is bounded as one file.
    fn append(path: &Path) -> Option<Live> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()?;
        let written = file.metadata().map(|held| held.len()).unwrap_or_default();
        Some(Live {
            file,
            written,
            cut: false,
        })
    }

    /// Unbuffered on purpose: a `BufWriter` here would hold back exactly the
    /// last few lines a person watching is waiting for.
    fn wrote(&mut self, chunk: &[u8]) {
        if self.cut {
            return;
        }
        if self.written + chunk.len() as u64 > LIVE_LIMIT {
            self.cut = true;
            let _ = self.file.write_all(LIVE_CUT);
            return;
        }
        if self.file.write_all(chunk).is_ok() {
            self.written += chunk.len() as u64;
        }
    }
}

/// Why the spawn failed, with the one ambiguity resolved.
///
/// The operating system reports both a missing program and a missing working
/// directory as *not found*, and the two need opposite responses — install the
/// tool, or find out what removed a running Job's checkout. The worktree is
/// probed only on that path, so the ordinary spawn costs no extra call.
pub(crate) fn not_started(error: std::io::Error, program: String, worktree: &Path) -> NeverRan {
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
pub(crate) fn end_the_group(group: Option<u32>) {
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
pub(crate) fn split(command: &str) -> Option<(String, Vec<String>)> {
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
