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
