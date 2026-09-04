//! The process group a Drone was detached into: reading it, and ending it.
//!
//! **The mirror of `crate::detach`.** That file makes the group — one
//! `libc::setsid()` between fork and exec, so a Drone leads a session and a
//! process group whose id is its own pid. This one is the only thing that
//! speaks to that group afterwards, and it exists as a file of its own so the
//! exemption the gate grants is the two calls below rather than everything
//! `crate::session` does.
//!
//! **Both calls take a [`NonZeroU32`], and that is the safety argument at the
//! call site.** Zero is the *caller's* own group to a group-directed call —
//! Fleet, and every process Fleet has not detached — so the number that would
//! turn one Drone's ending into a fleet-wide kill cannot be spelled. The pid
//! being non-zero is not enough on its own: it also has to still be this
//! Drone's, and **the two callers prove that two different ways.**
//!
//! [`DroneSession::group`](crate::session::DroneSession) proves it by holding
//! an uncollected child on the pid: while the child has not been waited on the
//! operating system cannot hand the number to anybody else. `crate::adopting`
//! cannot — an adopted Drone is not this process's child and there is nothing
//! to hold — so it proves it by asking [`crate::holder_of`] when the process at
//! that pid started and comparing the answer against what was recorded at the
//! spawn. That is a weaker claim by exactly the width of `ps`'s second
//! resolution, which is stated where it bites at
//! [`Holder::Held`](crate::Holder).

use std::num::NonZeroU32;

/// Whether the process has exited, **without collecting it**.
///
/// `WNOWAIT` is what makes this a reading rather than a reap: the child is left
/// exactly as it was found, so the `try_wait` that follows still collects it
/// and tokio's own bookkeeping never learns this happened. That is the whole
/// reason it exists — the group can be signalled while the pid is still the
/// Drone's, instead of a few instructions after the reap handed it back.
///
/// `waitid` and not `waitpid`, because `WNOWAIT` is only legal on the first:
/// `waitpid(pid, WNOHANG | WNOWAIT)` answers `EINVAL` here, measured.
///
/// **`false` on every error**, including the `ECHILD` of a child something else
/// has already collected. A reading that failed is not a Drone that stopped,
/// and the only thing a `true` can do is add a signal.
#[allow(unsafe_code)]
pub(crate) fn run_is_over(pid: NonZeroU32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid.get()) else {
        return false;
    };
    // SAFETY: `waitid` writes one `siginfo_t` through the pointer given, which
    // is a live local zeroed to start with — `si_pid` stays zero unless the
    // call filled it in, which is how "nothing changed state" is told from
    // "this pid exited". Nothing is read after the call but that field.
    unsafe {
        let mut reading: libc::siginfo_t = std::mem::zeroed();
        let asked = libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut reading,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        );
        asked == 0 && reading.si_pid == pid
    }
}

/// `SIGKILL` to a whole process group.
///
/// `SIGKILL` rather than `SIGTERM`, which is `checks-runner`'s reasoning in the
/// other place this workspace signals a group: the decision to end has already
/// been taken, and a graceful window would be a second one nobody configured.
///
/// **A group that is already gone is not an error.** `killpg` answers `ESRCH`
/// for a Drone that exited between the reading above and this line, which is
/// the ordinary case rather than a fault — so nothing is returned and nothing
/// is logged. What happened to the Drone itself is on the `wait` its caller
/// does next.
#[allow(unsafe_code)]
pub(crate) fn end_the_group(group: NonZeroU32) {
    // A pid the platform cannot express is not a group, and `try_from` says so
    // rather than a cast quietly making one up.
    let Ok(group) = libc::pid_t::try_from(group.get()) else {
        return;
    };
    // SAFETY: `killpg` is a plain system call over two integers, and this one
    // is the pid `libc::setsid()` in `crate::detach` made a group leader — so
    // the signal reaches that Drone and what it spawned, and stops there. Both
    // callers hold an uncollected child on the pid, which is what stops it
    // naming anything else.
    unsafe {
        libc::killpg(group, libc::SIGKILL);
    }
}
