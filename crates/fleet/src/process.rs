//! Whether a pid is held, and whether it is held by the *same* process.
//!
//! **The question v1 never had to answer.** v1's `group_alive` proved a pid was
//! held, which was enough because the only reader of v1's pidfile was the
//! daemon reading its own — one process, one file, one boot. Bridge is a
//! second, independent reader asking a stronger question: **is the process at
//! this pid the Fleet that wrote this file.** A live pid does not answer it,
//! because pids are recycled and a recycled pid says "yes, something is here"
//! to every check that only asks for liveness. So [`holder_of`] returns *who*
//! holds a pid, expressed as the one fact about a process that is fixed for its
//! whole life and is not reused when its pid is: **when it started.** Comparing
//! that against what was recorded is the identity check.
//!
//! **An absolute start time, and no boot id.** The harvest names v1's shape for
//! the same problem on a Drone — boot id plus process start time — and the boot
//! id is there because v1 read a *relative* start time, jiffies since boot out
//! of `/proc/<pid>/stat`, and two boots both have a jiffy 4,096. An absolute
//! wall-clock start time needs no such disambiguation: a process starting after
//! a reboot cannot carry a start time from before it, so a boot id would only
//! ever restate what the start time already says.

use std::error::Error;
use std::fmt;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// When a process started, absolute, as the operating system reports it.
///
/// Opaque and compared only for equality. Nothing here parses it into a time:
/// the value is an identity token, and the moment it becomes a `Timestamp` the
/// next reader is tempted to do arithmetic on a string whose format belongs to
/// somebody else's tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StartedAt(String);

impl StartedAt {
    /// Carry a reading something else took.
    pub fn carried(value: impl Into<String>) -> StartedAt {
        StartedAt(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StartedAt {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

/// What is at a pid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holder {
    /// Nothing. The pid is not assigned to a live process.
    Vacant,
    /// A process, which started at this time.
    ///
    /// **Second resolution.** Two processes that take the same pid and start
    /// inside the same second are indistinguishable to this check. That window
    /// needs the original process to die, the pid counter to wrap all the way
    /// round, and the replacement to land — inside one second — and it is the
    /// one case the runtime file cannot defend.
    Held(StartedAt),
}

/// The largest pid the platform can express. Anything above it names no
/// process and is answered without asking, because `ps` treats it as a
/// malformed argument rather than as an absent process.
const PID_CEILING: u32 = i32::MAX as u32;

/// Who holds `pid`, if anybody.
///
/// **Pid zero is never held.** It names the caller's own process group to
/// `kill(2)` and nothing at all here, and v1 carried a test by that name for
/// the same reason: a zero-valued pid is what a half-written file reads as.
///
/// **It asks `ps`.** The two obvious alternatives read a kernel structure
/// directly and do it differently on each platform — `/proc/<pid>/stat` on
/// Linux, `kinfo_proc` through `sysctl` on macOS — needing `unsafe` or a
/// platform crate, and putting a platform fork in a crate with no other reason
/// for one. `ps -o lstart= -p <pid>` is one spelling on both, and the deciding
/// argument is that it is a spelling **Bridge can run too**: the runtime file
/// is a contract between a Rust process and a Node one, so an identity check
/// only Rust can perform is one the reader that needs it cannot make. Second
/// resolution is the cost, stated where it bites at [`Holder::Held`].
pub fn holder_of(pid: u32) -> Result<Holder, ProbeFailed> {
    if pid == 0 || pid > PID_CEILING {
        return Ok(Holder::Vacant);
    }

    let run = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .map_err(|cause| ProbeFailed {
            pid,
            doing: "asking ps when a process started",
            cause: Box::new(cause),
        })?;

    let complaint = String::from_utf8_lossy(&run.stderr).trim().to_string();
    let reading = String::from_utf8_lossy(&run.stdout).trim().to_string();

    // The three answers ps gives, and they are told apart by stderr rather than
    // by the exit code alone: an absent pid and a refused argument both exit
    // non-zero, and only one of them means "nothing is there".
    if !complaint.is_empty() {
        return Err(ProbeFailed {
            pid,
            doing: "asking ps when a process started",
            cause: Box::new(Complaint(complaint)),
        });
    }
    if reading.is_empty() {
        return Ok(Holder::Vacant);
    }
    Ok(Holder::Held(StartedAt(reading)))
}

/// The probe could not be taken. **Not the same as "nothing is there"** — this
/// is the check failing, and a caller that folds it into `Vacant` has decided
/// on no evidence that Fleet is not running.
#[derive(Debug)]
pub struct ProbeFailed {
    pub pid: u32,
    /// A fixed string chosen here, never anything read from outside.
    pub doing: &'static str,
    pub cause: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for ProbeFailed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "{} {}", self.doing, self.pid)
    }
}

impl Error for ProbeFailed {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

/// What the tool said when it refused. The leaf of the chain, and genuinely a
/// line of text — there is no typed error underneath somebody else's stderr.
#[derive(Debug)]
struct Complaint(String);

impl fmt::Display for Complaint {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

impl Error for Complaint {}
