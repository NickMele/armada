//! The POSIX calls that have no safe equivalent.
//!
//! `unsafe` is denied crate-wide (see `lib.rs`) and allowed here, in this
//! module alone, for the four calls the design permits: `libc::signal` for
//! SIGPIPE, `setsid` inside `pre_exec`, `libc::killpg`, and `clock_gettime`
//! for the monotonic heartbeat column (`ARCHITECTURE.md` §3).
//!
//! Every one is measured rather than assumed, and the measurements disagree
//! with the obvious reading in three places — see `docs/traps.md` and the
//! doc comments below.

use std::io;
use std::process::Command;
use std::time::Duration;

/// Restore the default disposition for `SIGPIPE`.
///
/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` at startup, so a write to a
/// closed pipe returns `EPIPE`, `println!` unwraps it, and the process
/// **panics with exit 101 and a backtrace** — measured, and worse than doing
/// nothing (`docs/traps.md`). Restoring the default gives the ordinary Unix
/// behaviour: silent death, exit 141, which is the code
/// `ARCHITECTURE.md` §1.6 carries in its signal carve-out.
///
/// Call it at the top of `main`, before anything writes.
pub fn restore_sigpipe() {
    // SAFETY: `signal` with `SIG_DFL` on a real signal number is defined and
    // cannot fail in a way that matters here; the return value is the previous
    // handler, which char has no use for. This runs before any thread is
    // spawned, so there is no concurrent handler to race with.
    #[allow(unsafe_code)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

/// Arrange for the child to run in a **new session** via `setsid`, so its whole
/// tree can be reached with one `killpg`.
///
/// **Not `Command::process_group(0)`, and the two are mutually exclusive.**
/// `process_group(0)` gives a new process *group* in the same session, which
/// does not detach from the controlling terminal; and measured, setting both on
/// one `Command` fails with `Operation not permitted (os error 1)`, because
/// `setsid` refuses when the caller is already a process-group leader — which
/// `process_group(0)` has just made it. Pick `setsid` alone.
pub fn new_session(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: `pre_exec` runs between fork and exec, where only
    // async-signal-safe calls are legal. `setsid` is one, it allocates nothing,
    // and it touches no state shared with the parent. The failure case is
    // benign: `setsid` fails only when the caller is already a session leader,
    // which for a freshly forked child it is not.
    #[allow(unsafe_code)]
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

/// Signal an entire process group.
///
/// **Measured: `killpg` against a `setsid`'d group does reach grandchildren.**
/// A group of three — `sh -c 'sleep 300 & sleep 300'` — went to zero on one
/// `killpg(SIGTERM)`. That is the project's central cleanup claim, and nothing
/// had confirmed it in Rust before phase 2's suite did it again.
///
/// It is also not sufficient on its own; see [`stop_group`].
pub fn killpg(pgid: i32, signal: i32) -> io::Result<()> {
    // SAFETY: `killpg` is an extern fn and therefore unsafe to call, but it
    // takes two integers, dereferences nothing, and reports failure through
    // errno like any other syscall wrapper. A pgid char never recorded is
    // guarded above this call by `boot_id` and `pid_started_at`, not here.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::killpg(pgid, signal) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Whether any process remains in the group.
///
/// Signal 0 performs the permission and existence checks without delivering
/// anything, so any failure reads as "not alive" — `ESRCH` when the group is
/// empty, and that is how char *confirms* a kill rather than assuming one.
///
/// **An unreaped child of char's own is still a member of its group, and the
/// two platforms disagree about it.** Measured: against a group whose only
/// remaining member is a zombie, `killpg(pgid, 0)` *succeeds* on Linux and
/// fails on darwin with **`EPERM`, not `ESRCH`** — `ESRCH` arrives on both only
/// after the `waitpid` (`docs/traps.md`). So a caller that parented the group
/// must reap before reading this as "empty", which
/// [`crate::process::ProcessGroup::stop`] does, and which is why its report
/// describes the kill rather than the state after it.
///
/// The case char actually reclaims is an **orphan**, whose parent is gone, so
/// init reaps it the moment it dies and both platforms agree. Note the
/// consequence of testing `rc == 0`: a genuine `EPERM` — a group char may not
/// signal — also reads as "not alive". char started every group it probes, so
/// it has permission by construction; a future caller that probes a group it
/// did not start does not, and would need to branch on the errno.
pub fn group_alive(pgid: i32) -> bool {
    #[allow(unsafe_code)]
    // SAFETY: as `killpg` above; signal 0 delivers nothing at all.
    let rc = unsafe { libc::killpg(pgid, 0) };
    rc == 0
}

/// What stopping a group actually did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopReport {
    /// The group existed when char started.
    pub existed: bool,
    /// SIGTERM alone was not enough and char escalated.
    pub escalated: bool,
    /// The group is empty now.
    pub gone: bool,
}

/// **SIGTERM, wait a grace period, then SIGKILL** — an unconditional
/// escalation, not a retry.
///
/// Measured, and this is why the escalation is not conditional on anything: a
/// group leader running `trap '' TERM` leaves **3 of 3 alive** after
/// `killpg(SIGTERM)`, because children inherit an *ignored* disposition across
/// `fork` and `exec`. One uncooperative leader immunises its whole group, and a
/// process that ignores SIGTERM ignores the second one too — so retrying TERM
/// is pure delay.
///
/// **What this cannot reach, stated because no signal fixes it:** a service
/// that calls `setsid` itself — ordinary daemonising — leaves the tracked group
/// entirely, so its pgid is not the one recorded and no `killpg` finds it. That
/// case is *detected* by the port still being bound afterwards, not prevented.
pub fn stop_group(pgid: i32, grace: Duration) -> StopReport {
    if !group_alive(pgid) {
        return StopReport {
            existed: false,
            escalated: false,
            gone: true,
        };
    }

    let _ = killpg(pgid, libc::SIGTERM);

    let deadline = std::time::Instant::now() + grace;
    while std::time::Instant::now() < deadline {
        if !group_alive(pgid) {
            return StopReport {
                existed: true,
                escalated: false,
                gone: true,
            };
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let _ = killpg(pgid, libc::SIGKILL);
    // SIGKILL cannot be caught, but the kernel still has to reap; give it a
    // bounded moment rather than reporting a live group char has in fact
    // killed.
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    while std::time::Instant::now() < deadline {
        if !group_alive(pgid) {
            return StopReport {
                existed: true,
                escalated: true,
                gone: true,
            };
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    StopReport {
        existed: true,
        escalated: true,
        gone: !group_alive(pgid),
    }
}

/// Milliseconds on a **suspend-excluding** monotonic clock.
///
/// `CLOCK_MONOTONIC` means opposite things on the two platforms this project
/// supports: measured on darwin it counted **4.4 days of sleep**, while Linux's
/// excludes suspend. char wants the clock that does *not* advance while the
/// machine is suspended, because the lease holder was not running either and
/// its heartbeat should not age. Rust's `Instant` already picks correctly on
/// both — `CLOCK_UPTIME_RAW` on darwin, `CLOCK_MONOTONIC` on Linux — so the
/// rule is **`Instant` semantics**, and this exists only because an `Instant`
/// cannot be stored in a database column.
pub fn mono_ms() -> u64 {
    #[cfg(target_os = "macos")]
    const CLOCK: libc::clockid_t = libc::CLOCK_UPTIME_RAW;
    #[cfg(not(target_os = "macos"))]
    const CLOCK: libc::clockid_t = libc::CLOCK_MONOTONIC;

    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `clock_gettime` writes into the `timespec` we own and borrow
    // exclusively for the call. Both clock ids are compile-time constants that
    // exist on their target, so the call cannot fail for a reason char could
    // act on.
    #[allow(unsafe_code)]
    unsafe {
        libc::clock_gettime(CLOCK, &mut ts);
    }
    (ts.tv_sec as u64) * 1_000 + (ts.tv_nsec as u64) / 1_000_000
}

/// This process's id, for the `pid` column on a lease.
pub fn pid() -> i32 {
    std::process::id() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_monotonic_clock_moves_forward_and_never_backward() {
        let first = mono_ms();
        std::thread::sleep(Duration::from_millis(5));
        let second = mono_ms();
        assert!(second >= first, "{second} < {first}");
        assert!(
            second - first < 5_000,
            "a 5 ms sleep measured as {}",
            second - first
        );
    }

    #[test]
    fn stopping_a_group_that_does_not_exist_reports_it_rather_than_erroring() {
        // A pgid that cannot be live: the kernel never issues 0x7FFFFFFF.
        let report = stop_group(i32::MAX, Duration::from_millis(10));
        assert!(!report.existed);
        assert!(report.gone);
    }

    #[test]
    fn our_own_group_is_alive() {
        // Whatever group this test runs in, something is in it: us.
        assert!(group_alive(0) || group_alive(pid()));
    }
}
