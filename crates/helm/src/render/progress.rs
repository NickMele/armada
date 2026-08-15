//! What a long run says while it is still going.
//!
//! **Progress goes to stderr. Always.** A redraw on stdout means `armada
//! manifest check | jq` receives frames of animation, and the one consumer the
//! envelope exists for is the one that breaks (PLAN.md §3.1.1). Anything that
//! redraws, animates or reports intermediate state is stderr's; stdout carries
//! the result and nothing else.
//!
//! **And only when stderr is a terminal.** The audience for a redraw is a
//! person watching one; a captured stderr is a log file, and a log file full of
//! half-erased frames is worse than a silent one. That is why the decision is
//! made from `stderr_is_tty` rather than from the flag that decides stdout's
//! colour — `armada manifest check | jq` is a piped stdout *and* a terminal
//! stderr, and it wants both.
//!
//! **It leaves nothing behind.** [`Progress::finish`] gives back the lines it
//! drew on, so the run's real output — the envelope on stdout, a failure on
//! stderr — arrives on a clean stream.
//!
//! This file is the contract. [`super::live`] is the only thing that implements
//! it for a person, and the run that decided its shape is described there.

use armada_core::error::Status;

/// A run reporting on itself as it goes.
///
/// **Every method has a do-nothing default**, so the silent case is
/// `impl Progress for Silent {}` and adding a hook later cannot break it. The
/// alternative — a `None` checked at four call sites — is four places to forget.
pub trait Progress {
    /// The checks this run will report on, **by name**, and the monotonic
    /// reading it starts from.
    ///
    /// **The ids rather than a count**, because the live table has a row per
    /// check from the first frame. A run that could only say `0/5` is the
    /// report that ended the thing this replaced.
    fn begin(&mut self, _checks: &[&str], _now_mono: u64) {}
    /// A check was spawned.
    fn started(&mut self, _id: &str) {}
    /// A check reached a verdict, with the one line explaining it if there is
    /// one — the same text the final table's `DETAIL` column will carry.
    fn finished(&mut self, _id: &str, _status: Status, _detail: Option<&str>) {}
    /// A turn of the scheduler's loop went by, at this monotonic reading.
    /// Redraw.
    ///
    /// **The clock is handed in rather than read.** It is injected
    /// (`ARCHITECTURE.md` §1.1), and a renderer that read the real one would be
    /// the one part of a run a test could not hold still.
    fn tick(&mut self, _now_mono: u64) {}
    /// The run is over. Leave the stream as it was found.
    fn finish(&mut self) {}
}

/// Reports nothing at all: a pipe, a `--json` run, and every test that is not
/// about progress.
pub struct Silent;

impl Progress for Silent {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The silent reporter is the whole of the non-terminal path, and it writes
    /// nothing anywhere — there is no stream for it to get wrong.
    ///
    /// **Which is what the do-nothing defaults buy.** A pipe, a `--json` run
    /// and every test that is not about progress all take this path, so a hook
    /// added to the trait later cannot break any of them.
    #[test]
    fn the_silent_reporter_has_nowhere_to_write() {
        let mut silent = Silent;
        silent.begin(&["api:lint"], 0);
        silent.started("api:lint");
        silent.finished("api:lint", Status::Pass, Some("exited 0"));
        silent.tick(20);
        silent.finish();
    }
}
