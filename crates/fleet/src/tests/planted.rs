//! What a fixture plants: what time it is, what an id is, and when a Drone's
//! process begins and ends.
//!
//! **`crate::clock` and `crate::mint` are the only two things in this workspace
//! that enter the process**, and both are held by Fleet so that everything
//! below them takes its instant and its id as an argument. That rule is only
//! affordable because a test can plant these — a fixture that read the real
//! clock would assert against the second it happened to run in.
//!
//! # The third thing, which is not a seam
//!
//! A Drone is a real child, and the two ends of its life are the only part of
//! these fixtures the operating system decides. `#443`: three cases assumed a
//! child would be gone by the time the next line ran and one assumed a child
//! would still be there, and both assumptions held on an idle machine and
//! failed at load average 17. **Neither is a wait to be lengthened** — a sleep
//! tuned on a quiet machine is the same bug with a bigger number — so the two
//! helpers below make the ends of a Drone's life something a test states rather
//! than something it hopes for.
//!
//! Here rather than in `tests::daemon`, which is a Fleet over a temporary
//! directory and is a different subject from what it is handed.

use std::sync::atomic::{AtomicU64, Ordering};

use core_model::{Timestamp, Ulid};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::mint::Mint;
use crate::session::LiveSession;

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

// ------------------------------------------- when a Drone's process ends

/// End the Drone this Fleet is holding, and do not come back until the
/// operating system has collected it.
///
/// **The fixture a restart case needs, and it is a reap rather than a wait.**
/// Dropping a Fleet does not end its Drone — `crate::detach` calls
/// `libc::setsid()` on every spawn precisely so that it does not — it only
/// drops the write end of the pipe the child is reading. What follows is three
/// things the machine schedules: the child noticing end-of-file, the child
/// exiting, and tokio's orphan queue collecting it. Until the last of those the
/// pid is a zombie, `ps` still reports when it started, and
/// [`crate::holder_of`] therefore answers `Held` — which since `#409` is a
/// Drone the next Fleet **adopts** rather than one it reports vanished.
///
/// [`LiveSession::terminate`] is the whole of the guarantee: it signals the
/// group, signals the pid, and *waits*, so by the time this returns the pid
/// names nothing. Nothing here polls and nothing here sleeps.
///
/// **The process stays real.** Two of the cases that need this are about what
/// survives a Fleet restart, so the Drone is a child that was genuinely spawned
/// and genuinely died; what the fixture removes is the uncertainty about
/// *when*, not the process.
pub async fn the_drone_it_holds_is_gone(fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>) {
    let slot = fleet.the_only_slot().await;
    let held = slot.lock().await;
    let working = held
        .as_ref()
        .expect("a Fleet that dispatched a Job is holding its Drone");
    working
        .session()
        .terminate()
        .await
        .expect("a Drone this Fleet spawned is one it can end");
}

/// A Drone that says one line and leaves — **after it has been told, never
/// before.**
///
/// `IFS= read -r _` is the whole of it, and `crate::tests::group` spells the
/// same line for the same reason. [`crate::drone::start`] writes the first turn
/// into a pipe the child is holding open; a child that had already exited has
/// closed the read end, so the write answers `EPIPE` and the spawn fails with
/// [`DiedBeforeItWasTold`](crate::drone::DroneNotStarted::DiedBeforeItWasTold).
/// **`echo BUSY` on its own is therefore not a Drone that leaves but a race
/// against the spawn**, and a busy machine loses it: measured on 2026-09-04 at
/// load average 40, a full run of this crate's suite lost it two to four times
/// in seven hundred tests.
///
/// Reading first costs the case nothing, because a Drone that leaves is only
/// ever wanted for the slot it empties and the step it strands — and it still
/// leaves, one turn later, without anything having waited.
pub fn a_drone_that_leaves(saying: &str) -> FakeHarness {
    let says_it_then_waits_to_be_told = format!("echo {saying}; IFS= read -r _");
    FakeHarness::running("/bin/sh", &["-c", says_it_then_waits_to_be_told.as_str()])
}
