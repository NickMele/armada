//! The two seams every fixture plants: what time it is, and what an id is.
//!
//! **`crate::clock` and `crate::mint` are the only two things in this workspace
//! that enter the process**, and both are held by Fleet so that everything
//! below them takes its instant and its id as an argument. That rule is only
//! affordable because a test can plant these — a fixture that read the real
//! clock would assert against the second it happened to run in.
//!
//! Here rather than in `tests::daemon`, which is a Fleet over a temporary
//! directory and is a different subject from the two seams it hands one.

use std::sync::atomic::{AtomicU64, Ordering};

use core_model::{Timestamp, Ulid};

use crate::clock::Clock;
use crate::mint::Mint;

/// A clock that answers a different second each time it is asked.
///
/// Different, so the pair of timestamps on a step row says how long the step
/// took; fixed in shape, so a test can write the answer down.
pub struct Ticking(AtomicU64);

impl Ticking {
    pub fn from_nine() -> Ticking {
        Ticking(AtomicU64::new(0))
    }
}

impl Clock for Ticking {
    fn now(&self) -> Timestamp {
        let tick = self.0.fetch_add(1, Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T09:{:02}:{:02}.000Z",
            tick / 60,
            tick % 60
        ))
    }
}

/// A clock that ticks a second per reading, and jumps when a test says so.
///
/// **The one fixture clock a case can move**, which is what makes a threshold
/// measured in minutes testable: sitting through it would be a test that is
/// slow *and* timing-dependent, and the number under test would stop being the
/// one that ships. `crate::tests::silence` pushes the liveness ladder with it
/// and `crate::tests::adopting` pushes the same ladder against a Drone Fleet
/// cannot hear — here rather than in either of them, because a second copy
/// would be two clocks that could come to disagree about what a second is.
pub struct Held {
    ticks: AtomicU64,
    pushed: AtomicU64,
}

impl Held {
    pub fn started() -> Held {
        Held {
            ticks: AtomicU64::new(0),
            pushed: AtomicU64::new(0),
        }
    }

    /// Move the clock on. **Never backwards**, which `converging::elapsed`
    /// reads as zero and which no machine should produce.
    pub fn on(&self, seconds: u64) {
        self.pushed.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T{:02}:{:02}:{:02}.000Z",
            (at / 3_600) % 24,
            (at / 60) % 60,
            at % 60
        ))
    }
}

/// Ids a test can write down. Twenty-six characters, and every one of them
/// legal in a directory name and in a branch name, because `WorktreeSpec`
/// refuses anything else.
pub struct Counted(AtomicU64);

impl Counted {
    pub fn from_one() -> Counted {
        Counted(AtomicU64::new(1))
    }

    /// A real mint is a ULID and cannot repeat; this one restarts at one on
    /// every assembly, so two Fleets over one store would hand out one id twice.
    pub fn from_next(next: u64) -> Counted {
        Counted(AtomicU64::new(next))
    }
}

impl Mint for Counted {
    fn ulid(&self) -> Ulid {
        Ulid::carried(format!(
            "01TEST{:020}",
            self.0.fetch_add(1, Ordering::SeqCst)
        ))
    }
}
