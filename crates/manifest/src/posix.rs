//! The POSIX calls that have no safe equivalent.
//!
//! `unsafe` is denied crate-wide (see `lib.rs`) and allowed here, in this
//! module alone, for the calls the design permits: `libc::signal` for SIGPIPE
//! **and for the interrupt handler below**, `setsid` inside `pre_exec`,
//! `libc::killpg`, `libc::waitpid` for [`reap_group`], and `clock_gettime` for
//! the monotonic heartbeat column (`ARCHITECTURE.md` §3). The interrupt handler
//! reuses `libc::signal`; it is the same call, not a sixth one.
//!
//! `waitpid` is the newest of the five and it is here for the same reason
//! `killpg` is: [`group_alive`] cannot answer honestly about a group that still
//! holds this process's own unreaped child, and reaping is the only thing that
//! makes the two platforms agree (`docs/traps.md`).
//!
//! Every one is measured rather than assumed, and the measurements disagree
//! with the obvious reading in three places — see `docs/traps.md` and the
//! doc comments below.

use std::io;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Duration;

/// Set by the handler, read by the run loop. Nothing else may write it.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Which signal arrived, so the exit can be `128 + N` for the right N.
static CAUGHT: AtomicI32 = AtomicI32::new(0);

/// The whole handler. **Setting an atomic is the only thing done here** —
/// everything else, including killing children and writing the envelope,
/// happens on the run loop where it is allowed to block, allocate and fail.
///
/// A second signal is not swallowed. Once the flag is set, the disposition is
/// restored to the default and the signal re-raised, so a second Ctrl-C kills
/// the process outright. A tool that traps SIGINT and then wedges is worse
/// than one that never trapped it, and the operator's second press is an
/// instruction rather than a repeat.
extern "C" fn on_interrupt(signal: libc::c_int) {
    CAUGHT.store(signal, Ordering::SeqCst);
    if INTERRUPTED.swap(true, Ordering::SeqCst) {
        // SAFETY: both calls are async-signal-safe and this is the escape
        // hatch — restore the default and let the signal do what it would
        // have done had it never been trapped.
        #[allow(unsafe_code)]
        unsafe {
            libc::signal(signal, libc::SIG_DFL);
            libc::raise(signal);
        }
    }
}

/// Trap `SIGINT` and `SIGTERM` so a run ends rather than dies.
///
/// **Without this the children survive.** They are `setsid`'d into their own
/// sessions ([`spawn`]), so a signal delivered to Armada's process group never
/// reaches them: Ctrl-C during `check` returns the shell and leaves `cargo
/// test` running. Measured, and recorded in `PHASES.md` §9.3.
///
/// Call it once, at the top of `main`, before any thread exists.
pub fn catch_interrupts() {
    // SAFETY: `signal` with a real signal number and an `extern "C"` handler is
    // defined. The handler touches one atomic and, on the second delivery,
    // two async-signal-safe libc calls. This runs before any thread is spawned.
    #[allow(unsafe_code)]
    unsafe {
        let handler = on_interrupt as *const () as libc::sighandler_t;
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
    }
}

/// Whether an interrupt has been seen. Polled by the run loop.
pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

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
    // handler, which Armada has no use for. This runs before any thread is
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
    // errno like any other syscall wrapper. A pgid Armada never recorded is
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
/// empty, and that is how Armada *confirms* a kill rather than assuming one.
///
/// **An unreaped child of Armada's own is still a member of its group, and the
/// two platforms disagree about it.** Measured: against a group whose only
/// remaining member is a zombie, `killpg(pgid, 0)` *succeeds* on Linux and
/// fails on darwin with **`EPERM`, not `ESRCH`** — `ESRCH` arrives on both only
/// after the `waitpid` (`docs/traps.md`). So a caller that parented the group
/// **must reap before reading this as "empty"** — [`stop_group`] and
/// [`crate::process::ProcessGroup::stop`] both do, and [`reap_group`] is what
/// they call.
///
/// **This function is deliberately not the one that reaps.** It answers exactly
/// one question — *does this group still have members* — and a zombie is a
/// member. Folding the reap in here would make a probe with an obvious name do
/// something with none of the obvious name's constraints, on every caller's
/// behalf including the ones holding a `Child` whose status it would eat.
///
/// An **orphan** needs none of this: its parent is gone, so init reaps it the
/// moment it dies and both platforms agree. That is *a* case Armada handles, and
/// an earlier note here said it was the only one — it is not, because
/// `armada-fleet`'s `drone::start` abandons a child this process is still the
/// parent of, and `drone::stop` then kills it. Note the
/// consequence of testing `rc == 0`: a genuine `EPERM` — a group Armada may not
/// signal — also reads as "not alive". Armada started every group it probes, so
/// it has permission by construction; a future caller that probes a group it
/// did not start does not, and would need to branch on the errno.
pub fn group_alive(pgid: i32) -> bool {
    #[allow(unsafe_code)]
    // SAFETY: as `killpg` above; signal 0 delivers nothing at all.
    let rc = unsafe { libc::killpg(pgid, 0) };
    rc == 0
}

/// Reap **this process's own** already-dead children in `pgid`, and say how
/// many there were.
///
/// This is the missing half of [`group_alive`], and the two questions it keeps
/// apart are the ones `docs/traps.md` records: *does this group still have
/// members* and *can anything in it still run*. A zombie answers yes to the
/// first and no to the second, and only a `waitpid` turns one into the other.
/// Until it happens the two platforms give opposite answers to a probe about
/// the same corpse — `killpg(pgid, 0)` succeeds on Linux and fails with `EPERM`
/// on darwin — so any caller reading "empty" off the probe must reap first.
///
/// **`waitpid(-pgid)` and never `waitpid(-1)`.** The negative form waits for
/// any child in *that one process group*, which is exactly the set this
/// function claims. `-1` would reap whichever child of this process exited
/// first — another Job's Drone, a `check` step's compiler — and hand its exit
/// status to nobody. `pgid <= 0` is refused for the same reason: `waitpid(0)`
/// means *this process's own group*, and `waitpid(-1)` is what a negated `1`
/// would collapse to.
///
/// **`WNOHANG`, so it never blocks.** A live member returns `0` and is left
/// alone: this reaps corpses, it does not wait for anything to become one. A
/// pid that was never this process's child returns `ECHILD` and is likewise
/// left alone — which is why calling it on an orphaned group Armada merely
/// inherited is free and does nothing.
///
/// **It cannot steal a status a [`crate::process::ProcessGroup`] still wants**,
/// because that type reaps through its own `Child` handle — see
/// [`stop_group_reaping`]. It is the caller that deliberately *abandoned* its
/// child, as `armada-fleet`'s `drone::start` does, that has no handle left and
/// needs this.
pub fn reap_group(pgid: i32) -> usize {
    if pgid <= 0 {
        return 0;
    }
    let mut reaped = 0;
    loop {
        let mut status: libc::c_int = 0;
        // SAFETY: `waitpid` takes a pid and a pointer to a `c_int` this frame
        // owns and borrows exclusively for the call. `WNOHANG` makes it return
        // immediately in every case, so it cannot block, and the pgid is
        // guarded positive above so the negation cannot land on `0` or `-1`.
        #[allow(unsafe_code)]
        let rc = unsafe { libc::waitpid(-pgid, &mut status, libc::WNOHANG) };
        // `0` is "children remain, none has exited"; a negative is `ECHILD`,
        // meaning none of them were ours to begin with. Both are done.
        if rc <= 0 {
            return reaped;
        }
        reaped += 1;
    }
}

/// What stopping a group actually did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopReport {
    /// The group existed when Armada started.
    pub existed: bool,
    /// SIGTERM alone was not enough and Armada escalated.
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
///
/// **Every reading of the group is taken after a [`reap_group`]**, because
/// `killpg(pgid, 0)` against a group whose last member is this process's own
/// unreaped corpse succeeds on Linux and fails on darwin, and neither answer is
/// about the kill. Without the reap this returns `gone: false` on Linux for a
/// group that died on the first SIGTERM — measured, in CI, and recorded in
/// `docs/traps.md`. Use [`stop_group_reaping`] instead when the caller still
/// holds the `Child`.
pub fn stop_group(pgid: i32, grace: Duration) -> StopReport {
    stop_group_reaping(pgid, grace, &mut || {
        reap_group(pgid);
    })
}

/// [`stop_group`], with the reaping done by whoever owns the children.
///
/// **`gone` is a claim about what can still run, and only a reap makes it
/// one.** Every probe below is preceded by `reap`, so the group is read after
/// this process's own dead have stopped counting as members of it.
///
/// The default reaper is [`reap_group`], which waits on the group directly.
/// That is right for a caller that abandoned its child — `armada-fleet`'s
/// `drone::start` drops the handle on purpose, because a Drone outlives the
/// invocation that started it — and wrong for one that still holds a
/// `std::process::Child`, whose exit status a bare `waitpid` would consume
/// before the handle could read it, leaving the handle to answer `ECHILD`
/// forever. So [`crate::process::ProcessGroup`] passes its own `try_wait`
/// here, which reaps *and* records the status in the place that will be asked
/// for it.
pub fn stop_group_reaping(pgid: i32, grace: Duration, reap: &mut dyn FnMut()) -> StopReport {
    reap();
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
        reap();
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
    // bounded moment rather than reporting a live group Armada has in fact
    // killed.
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    while std::time::Instant::now() < deadline {
        reap();
        if !group_alive(pgid) {
            return StopReport {
                existed: true,
                escalated: true,
                gone: true,
            };
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // **`Survived` still means something after all this.** A group is reported
    // as surviving only when something in it is neither reapable nor killable:
    // a member wedged in uninterruptible sleep, or a corpse whose parent is
    // some other process that will not wait on it. Neither is reachable by
    // anything Armada can send, and both are worth saying out loud.
    reap();
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
/// excludes suspend. Armada wants the clock that does *not* advance while the
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
    // exist on their target, so the call cannot fail for a reason Armada could
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

/// Exit the way the signal would have, once the run has been ended cleanly.
///
/// **`ARCHITECTURE.md` §1.6 names this mistake by name.** A signal *"exits
/// `128+N` and has no error class at all — an implementer following [the exit
/// rule] literally would map them into a class"*. Catching SIGINT to end the
/// run properly is exactly the thing that tempts you to return `aborted`'s
/// exit `5`, and every shell and CI system in the world reads `130` for
/// Ctrl-C. The envelope still says `aborted`, because that describes the run;
/// the exit code describes the signal.
///
/// Restoring the default and re-raising rather than calling `exit(130)` is
/// deliberate: it leaves the parent shell seeing a real `WIFSIGNALED`, so
/// job control and `^C` reporting behave as they would have.
///
/// Call it after the envelope has been written, and only then.
pub fn die_by_signal() -> ! {
    let signal = CAUGHT.load(Ordering::SeqCst);
    // SAFETY: both calls are async-signal-safe, the signal number came from a
    // handler this module installed, and nothing runs after `raise`.
    #[allow(unsafe_code)]
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
    // Unreachable in practice: the default disposition for SIGINT and SIGTERM
    // terminates. Kept because the compiler cannot know that.
    std::process::exit(128 + signal);
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

    /// A detached child whose handle has been **dropped on purpose**, which is
    /// the shape `armada-fleet`'s `drone::start` leaves behind: still this
    /// process's child, with nothing left that could `wait` on it.
    fn abandoned_sleeper() -> i32 {
        let request = armada_core::ctx::RunRequest::new(
            vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "sleep 60".to_string(),
            ],
            std::path::PathBuf::from("/"),
        );
        let group = crate::process::ProcessGroup::spawn(&request).expect("/bin/sh exists");
        let pgid = group.pgid();
        drop(group);
        pgid
    }

    /// **The contract, asserted on both platforms**: a group whose only member
    /// is this process's own corpse is reaped, and *then* reads as empty.
    ///
    /// This is the mechanism behind the Linux-only failure of
    /// `armada-fleet`'s `drone::stop` and it is asserted here rather than
    /// there, because the platform that shows the bug is not the platform this
    /// is written on. The count is the part that is platform-independent:
    /// darwin already read the unreaped group as empty (`EPERM`) and Linux read
    /// it as alive, but on both there was exactly one corpse to collect and
    /// neither one collected it.
    #[test]
    fn a_group_holding_only_our_own_corpse_is_reaped_and_then_reads_empty() {
        let pgid = abandoned_sleeper();
        let _ = killpg(pgid, libc::SIGKILL);

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut reaped = 0;
        while reaped == 0 && std::time::Instant::now() < deadline {
            reaped = reap_group(pgid);
            std::thread::sleep(Duration::from_millis(20));
        }

        assert_eq!(
            reaped, 1,
            "the abandoned Drone's corpse was never collected"
        );
        assert!(
            !group_alive(pgid),
            "the group still had a member after its only corpse was reaped"
        );
        // And it is idempotent: there is nothing left that was ever ours.
        assert_eq!(reap_group(pgid), 0);
    }

    /// **It reaps corpses; it does not wait for anything to become one.** A
    /// group with something running in it comes back untouched, which is what
    /// keeps `Survived` a real answer rather than one the reap talks its way
    /// out of.
    #[test]
    fn reaping_a_running_group_collects_nothing_and_leaves_it_alive() {
        let pgid = abandoned_sleeper();

        assert_eq!(reap_group(pgid), 0, "a running child was reaped");
        assert!(
            group_alive(pgid),
            "the reap ended a group it should not have"
        );

        let report = stop_group(pgid, Duration::from_millis(300));
        assert!(report.existed);
        assert!(report.gone, "the group outlived its stop: {report:?}");
    }

    /// **A second `stop_group` finds nothing, on either platform.** Before the
    /// reap this was the divergence: darwin answered `EPERM` and reported
    /// nothing to stop, Linux answered `0` and reported a group that had
    /// survived SIGKILL. One kill, two verdicts, and only one of them was even
    /// accidentally right (`docs/traps.md`).
    #[test]
    fn stopping_a_group_twice_reports_nothing_to_stop_the_second_time() {
        let pgid = abandoned_sleeper();

        let first = stop_group(pgid, Duration::from_millis(300));
        assert!(first.existed && first.gone, "{first:?}");

        let second = stop_group(pgid, Duration::from_millis(300));
        assert!(
            !second.existed && second.gone,
            "the corpse was still counted as a member: {second:?}"
        );
    }

    /// **`waitpid(0)` is this process's own group and `waitpid(-1)` is every
    /// child it has.** Either would reap somebody else's child — another Job's
    /// Drone, another test's leader — and hand the status to nobody, so a
    /// non-positive pgid is refused rather than negated.
    #[test]
    fn a_non_positive_pgid_is_never_waited_on() {
        assert_eq!(reap_group(0), 0);
        assert_eq!(reap_group(-1), 0);
        assert_eq!(reap_group(i32::MIN), 0);
    }
}
