//! The POSIX calls that have no safe equivalent.
//!
//! `unsafe` is denied crate-wide (see `lib.rs`) and allowed here, in this
//! module alone, for the four calls the design permits: `libc::signal` for
//! SIGPIPE, `setsid` inside `pre_exec`, `libc::killpg`, and `clock_gettime`
//! for the monotonic heartbeat column (`ARCHITECTURE.md` §3). Phase 1 needs
//! only the first; the other three arrive with the process wrapper in phase 2
//! and belong in this module when they do.

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
