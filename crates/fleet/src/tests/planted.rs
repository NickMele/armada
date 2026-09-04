//! What a case plants under a Fleet: what time it is, what an id is, and when
//! a real child starts and stops.
//!
//! # The two seams
//!
//! **`crate::clock` and `crate::mint` are the only two things in this workspace
//! that enter the process**, and both are held by Fleet so that everything
//! below them takes its instant and its id as an argument. That rule is only
//! affordable because a test can plant these — a fixture that read the real
//! clock would assert against the second it happened to run in.
//!
//! # And the third thing, which is a process
//!
//! A Drone is a real detached child in most of this suite, and a case that
//! needs one *started* or one *finished* has to be able to say so. `#443` is
//! what it costs when it cannot: eleven cases across seven files got their
//! Drone's lifetime from a shell script and the scheduler, and on a machine at
//! load 80 the scheduler answered differently. Neither shape below sleeps and
//! neither polls — each of them is a point the operating system will not move.
//!
//! Here rather than in `tests::daemon`, which is a Fleet over a temporary
//! directory and is a different subject from the seams it hands one.

use std::sync::atomic::{AtomicU64, Ordering};

use adapter_traits::{CallDetail, DroneEvent};
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

/// A Drone that says one thing and then leaves — **once it has been told**.
///
/// **The read is the whole fixture.** `crate::drone::start` writes the opening
/// brief into the Drone's stdin and checks the write, so a program that has
/// already exited when the write lands answers `EPIPE` and the spawn fails as
/// `DroneNotStarted::DiedBeforeItWasTold`. That is a real Drone failing to
/// start, faithfully reported — and it is not what any case wanting an empty
/// slot is about. `echo BUSY` on its own raced that write and lost it: five
/// cases panicked at their own `approve`, on a machine at load 80, having said
/// nothing about the act under test.
///
/// **It reads before it speaks, rather than after.** Printing first and reading
/// second would also keep a reader on the pipe, but it leaves the event racing
/// the dispatch a case takes its baseline from. Blocking first orders the two
/// the only way that cannot come out backwards, and it is the shape
/// `crate::tests::transcript::SAYS_THREE` has always had — which is why that
/// file never flaked.
///
/// One tool call, because that is the cheapest thing a Drone that is turning
/// emits and every case that took this shape scripted the same one.
pub fn a_drone_that_leaves() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "IFS= read -r _; echo BUSY"]).reading(
        "BUSY",
        vec![DroneEvent::Called {
            tool: String::from("Read"),
            call: String::from("a-call"),
            detail: CallDetail::of("a file"),
        }],
    )
}

/// End the Drone in this Fleet's one slot, for certain, and say which pid it
/// was.
///
/// **What dropping the Fleet does not give you.** Dropping it closes the pipe
/// into a `/bin/cat` Drone, and a `cat` that has lost its stdin does exit —
/// eventually. *Eventually* is the scheduler's word: on a loaded machine the
/// child has not been run since the write end closed, and on one that did run
/// it the child is a **zombie** until Tokio's `SIGCHLD` reaper gets a turn,
/// because nothing waits on a `Child` that was dropped with its Fleet.
///
/// `crate::holder_of` asks `ps -o lstart=`, and `ps` reports a zombie with the
/// start time it was born with — measured, not assumed. So `crate::reattaching`
/// finds the recorded pid held at the recorded instant and **adopts** the
/// Drone, which is `crate::adopting` behaving exactly as `#409` specified. The
/// six cases that failed were asserting the answer reconciliation gave before
/// there was anything to adopt.
///
/// `DroneSession::terminate` is the one act on a Drone with no *eventually* in
/// it: it ends the process group, signals the pid, and `wait`s. **The `await`
/// returning is the guarantee** — the child has been collected and the pid is
/// the operating system's again — so there is no probe here to confirm it and
/// no window for one to be taken in. A case calls this before it throws its
/// Fleet away, and what the next Fleet reads is then fixed.
pub async fn the_drone_is_gone(fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>) -> u32 {
    let slot = fleet.the_only_slot().await;
    let held = slot.lock().await;
    let working = held
        .as_ref()
        .expect("a Drone is in the slot, or there is nothing for this to make gone");
    let pid = working.session().pid();
    working
        .session()
        .terminate()
        .await
        .expect("the Drone ends and is collected");
    pid
}
