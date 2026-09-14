//! The machine's places for Checks: one `checks-at-once` shared by every Job's
//! gate, every Drone's own run, fix drafts and proofs after a merge, across
//! every repository this Fleet serves. #1063.
//!
//! **Whoever is waiting on the answer goes first.** A Drone's mid-step run takes
//! the next free place; a gate and a fix draft follow in the order they asked;
//! a proof after a merge, which nobody waits on, comes last. No ask is passed
//! over more than [`OVERTAKEN_AT_MOST`] times, so a stream of Drones' runs slows
//! a gate and never holds it back for good.
//!
//! A heavy run outside `crate::checking::ran` holds one through [`Room::place`].
//! A join of identical runs (#338) would sit in front of the ask.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::Notify;

use crate::headroom::{Bytes, Headroom, Machine, Reading, Spare};
use crate::ordering::Past;

/// How many later asks may take a place ahead of one already waiting, before
/// it goes next whoever else asks. Four Drones' Checks, each cut short at its
/// first failure (#1062), is under one turn of every place at the shipped limit
/// on a machine of ten cores.
pub const OVERTAKEN_AT_MOST: u32 = 4;

/// How many Checks run at once on this machine, across every Job and every
/// repository. **The `settings.checks-at-once` row**, shipped by the
/// composition root and replaced by one a person saves — `crate::limits`.
///
/// **No `Default`**, for [`Concurrency`](crate::Concurrency)'s reason: the
/// shipped number is a decision, and `armada::serve` says what decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChecksAtOnce(usize);

impl ChecksAtOnce {
    /// At least one: a gate that could start no Check would never rule.
    pub const fn of(checks: usize) -> ChecksAtOnce {
        ChecksAtOnce(if checks == 0 { 1 } else { checks })
    }

    /// Half the cores, from one to eight. A build or a test suite already
    /// spreads over every core, so a Check past half buys contention rather
    /// than speed; the other half is the Drones'. Eight is the most a save holds.
    pub const fn for_cores(cores: usize) -> ChecksAtOnce {
        let half = cores / 2;
        ChecksAtOnce::of(if half > 8 { 8 } else { half })
    }

    pub const fn get(&self) -> usize {
        self.0
    }
}

/// Who is asking for a place, which decides where it waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asking {
    /// A Drone's own mid-step run: the Drone is waiting on it, and it stops at
    /// its first failure.
    DronesRun,
    /// A step's gate.
    Gate,
    /// One test against main before a fix is drafted. **Beside the gates**: the
    /// Drone that asked hears on a later turn and works on, and main's checkout
    /// may need a whole build first, so neither of the Drones' run's reasons holds.
    FixDraft,
    /// A proof after a merge. Nobody waits on it, and it only writes a record.
    Proof,
}

impl Asking {
    const fn rank(self) -> u8 {
        match self {
            Asking::DronesRun => 0,
            Asking::Gate | Asking::FixDraft => 1,
            Asking::Proof => 2,
        }
    }
}

/// The machine's places, shared by every [`Room`] one Fleet hands out.
#[derive(Clone)]
pub struct Places(Arc<Shared>);

struct Shared {
    state: Mutex<State>,
    /// Told when a place frees or is taken, an ask leaves, or the limit moves.
    changed: Notify,
}

struct State {
    at_once: ChecksAtOnce,
    held: usize,
    /// Counts places given back, so a waiter that found the machine short reads
    /// it again only once something has finished.
    given_back: u64,
    asked: u64,
    waiting: Vec<Waiter>,
}

struct Waiter {
    seq: u64,
    rank: u8,
    overtaken: u32,
}

impl State {
    /// The ask whose turn it is: one passed over too often, then by rank, then
    /// by when it asked.
    fn next(&self) -> Option<u64> {
        self.waiting
            .iter()
            .min_by_key(|one| match one.overtaken >= OVERTAKEN_AT_MOST {
                true => (0, 0, one.seq),
                false => (1, one.rank, one.seq),
            })
            .map(|one| one.seq)
    }

    fn take(&mut self, seq: u64) {
        self.waiting.retain(|one| one.seq != seq);
        for earlier in self.waiting.iter_mut().filter(|one| one.seq < seq) {
            earlier.overtaken += 1;
        }
        self.held += 1;
    }
}

impl Places {
    pub fn of(at_once: ChecksAtOnce) -> Places {
        Places(Arc::new(Shared {
            state: Mutex::new(State {
                at_once,
                held: 0,
                given_back: 0,
                asked: 0,
                waiting: Vec::new(),
            }),
            changed: Notify::new(),
        }))
    }

    /// The limit in force.
    pub fn at_once(&self) -> ChecksAtOnce {
        self.state().at_once
    }

    /// Put a saved limit in force. It counts from the next place given out.
    pub(crate) fn limit(&self, at_once: ChecksAtOnce) {
        self.state().at_once = at_once;
        self.0.changed.notify_waiters();
    }

    /// How many places are held right now.
    pub fn held(&self) -> usize {
        self.state().held
    }

    fn ask(&self, asking: Asking) -> Ask {
        let mut state = self.state();
        let seq = state.asked;
        state.asked += 1;
        state.waiting.push(Waiter {
            seq,
            rank: asking.rank(),
            overtaken: 0,
        });
        Ask {
            places: self.clone(),
            seq,
            short_at: None,
        }
    }

    /// Never held across an `.await`, and nothing panics while holding it.
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One place asked for. **The turn is the `Ask`'s**, so a caller can stop
/// waiting and wait again without losing its position; dropping it leaves the line.
pub(crate) struct Ask {
    places: Places,
    seq: u64,
    /// How many places had been given back when the machine last read short.
    short_at: Option<u64>,
}

enum Turn {
    Wait(usize),
    Read(u64),
    Taken,
}

impl Ask {
    fn turn(&self) -> Turn {
        let mut state = self.places.state();
        let held = state.held;
        if state.next() != Some(self.seq) || !may_start(held, state.at_once, false) {
            return Turn::Wait(held);
        }
        if may_start(held, state.at_once, true) {
            state.take(self.seq);
            return Turn::Taken;
        }
        match self.short_at == Some(state.given_back) {
            true => Turn::Wait(held),
            false => Turn::Read(state.given_back),
        }
    }

    fn took_after_reading(&mut self, short: bool, given_back: u64) -> bool {
        let mut state = self.places.state();
        if state.next() == Some(self.seq) && may_start(state.held, state.at_once, short) {
            state.take(self.seq);
            return true;
        }
        if short {
            self.short_at = Some(given_back);
        }
        false
    }

    async fn granted(
        &mut self,
        machine: &Arc<dyn Machine>,
        headroom: Headroom,
        mut waiting: impl FnMut(usize),
    ) -> Place {
        // Its own handle, so the wake can be held while the turn is taken.
        let places = self.places.clone();
        loop {
            let changed = places.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let given_back = match self.turn() {
                Turn::Taken => return self.taken(),
                Turn::Wait(held) => {
                    waiting(held);
                    changed.await;
                    continue;
                }
                Turn::Read(given_back) => given_back,
            };
            let machine = Arc::clone(machine);
            let reading = tokio::task::spawn_blocking(move || machine.read())
                .await
                .ok()
                .flatten();
            let short = reading.is_some_and(|now| headroom.short_of(&now).is_some());
            if self.took_after_reading(short, given_back) {
                return self.taken();
            }
        }
    }

    /// Taken, so whoever is next may have room too.
    fn taken(&self) -> Place {
        self.places.0.changed.notify_waiters();
        Place {
            places: self.places.clone(),
        }
    }
}

impl Drop for Ask {
    fn drop(&mut self) {
        let left = {
            let mut state = self.places.state();
            let before = state.waiting.len();
            state.waiting.retain(|one| one.seq != self.seq);
            state.waiting.len() != before
        };
        if left {
            self.places.0.changed.notify_waiters();
        }
    }
}

/// A place held on the machine. Dropping it is the only way it is given back.
#[must_use = "a place is given back the moment it is dropped"]
pub struct Place {
    places: Places,
}

impl Drop for Place {
    fn drop(&mut self) {
        {
            let mut state = self.places.state();
            state.held = state.held.saturating_sub(1);
            state.given_back += 1;
        }
        self.places.0.changed.notify_waiters();
    }
}

/// What a run asks before each command it starts: a place on the machine, and
/// the memory and disk for it. #284, #1063.
///
/// **The first on the machine always starts**, so a short machine slows Checks
/// and never stops them. The headroom is taken when the run begins.
#[derive(Clone)]
pub struct Room {
    places: Places,
    asking: Asking,
    machine: Arc<dyn Machine>,
    headroom: Headroom,
    /// How long each Check took before, which decides which starts first.
    past: Past,
}

impl Room {
    /// A gate's room with places of its own, sharing the machine with nothing.
    pub fn of(at_once: ChecksAtOnce, machine: Arc<dyn Machine>, headroom: Headroom) -> Room {
        Room::sharing(&Places::of(at_once), Asking::Gate, machine, headroom)
    }

    /// A room in `places`, asking as `asking`.
    pub(crate) fn sharing(
        places: &Places,
        asking: Asking,
        machine: Arc<dyn Machine>,
        headroom: Headroom,
    ) -> Room {
        Room {
            places: places.clone(),
            asking,
            machine,
            headroom,
            past: Past::default(),
        }
    }

    /// The same room, starting the Checks this repository has timed fastest
    /// first. #1062.
    pub(crate) fn knowing(self, past: Past) -> Room {
        Room { past, ..self }
    }

    pub(crate) fn past(&self) -> &Past {
        &self.past
    }

    /// Bounded by `at_once` and nothing else: the machine is never read. For a
    /// caller with no machine to ask, such as a test or the acceptance bench.
    pub fn ignoring_the_machine(at_once: ChecksAtOnce) -> Room {
        Room::of(
            at_once,
            Arc::new(Unread),
            Headroom::of(Spare::percent(0), Bytes::gibibytes(0)),
        )
    }

    /// Join the line for a place.
    pub(crate) fn ask(&self) -> Ask {
        self.places.ask(self.asking)
    }

    /// The place `ask` waits for. `waiting` is told how many places are held
    /// each time it waits. **Safe to drop and call again.**
    pub(crate) async fn granted(&self, ask: &mut Ask, waiting: impl FnMut(usize)) -> Place {
        ask.granted(&self.machine, self.headroom, waiting).await
    }

    /// Wait in line and hold a place, for a heavy run of any kind.
    pub async fn place(&self) -> Place {
        let mut ask = self.ask();
        self.granted(&mut ask, |_| {}).await
    }
}

/// Whether another command may start beside `held` places, given whether the
/// machine is short: the pure half of [`Room::granted`].
pub(crate) fn may_start(held: usize, at_once: ChecksAtOnce, short: bool) -> bool {
    held == 0 || (held < at_once.get() && !short)
}

/// A machine that never answers, so it never holds a Check back.
struct Unread;

impl Machine for Unread {
    fn read(&self) -> Option<Reading> {
        None
    }

    fn disk_free_at(&self, _path: &Path) -> Option<Bytes> {
        None
    }
}
