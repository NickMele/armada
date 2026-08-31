//! How many Jobs Fleet works at once, and where each one's Drone is held.
//!
//! # The bound is a number somebody configured, not one Fleet computed
//!
//! Nothing in this workspace reads memory, CPU or quota — `#44` is where that
//! arrives — so [`Concurrency`] is handed in by the composition root, exactly
//! as the Check budget and the liveness thresholds are. `settings.toml`'s
//! `concurrency-cap` row is the specification it satisfies, and that row stopped
//! being informational when this type started refusing admissions.
//!
//! **It bounds Drones, never approvals.** `docs/concepts/fleet.md` is
//! unambiguous that every Job-level dispatch is approved explicitly, one by one,
//! and nothing here is reached until a person has done that: `admit_next` picks
//! from `queued`, and a Job reaches `queued` only by [`Fleet::approve`] or by
//! being created sub-dispatched inside a parent somebody already approved.
//!
//! [`Fleet::approve`]: crate::Fleet::approve
//!
//! # Two locks, and the order between them is the whole of the concurrency
//!
//! The roster — this type, behind one mutex — and each Job's own slot, behind
//! its own. **The roster is always taken first**, and no path takes a slot and
//! then reaches for the roster.
//!
//! | Held for | What it blocks |
//! |---|---|
//! | The roster, briefly | Another Job's slot being *found*. Milliseconds |
//! | The roster, across an admission | One dispatch at a time — a worktree, a rebase, a spawn |
//! | One Job's slot | Everything about **that** Job: its watchers, its gate, its Drone's tool calls |
//!
//! A single lock over the whole set would have been simpler and would have been
//! the working slot again under another name: the gate holds a slot across a
//! Check, so one Job running `cargo nextest` would have held every other Job's
//! Drone out of `submit_evidence` for the length of it. The slot a Check holds
//! is the slot of the Job being checked, and nothing else waits on it.

use std::collections::BTreeMap;
use std::sync::Arc;

use core_model::JobId;
use tokio::sync::Mutex;

use crate::working::Working;

/// How many Jobs Fleet may be working at one time.
///
/// **No `Default`**, for the reason [`Liveness`](crate::Liveness) has none: the
/// number is a decision somebody made and wrote down, and a type that supplies
/// one lets a caller not make it.
///
/// Zero is not expressible. A Fleet that may work no Jobs is a Fleet that
/// dispatches nothing and reports nothing about why, so [`Concurrency::of`]
/// raises a zero to one rather than admitting a value with no working state
/// under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Concurrency(usize);

impl Concurrency {
    pub const fn of(jobs: usize) -> Concurrency {
        Concurrency(if jobs == 0 { 1 } else { jobs })
    }

    pub const fn jobs(&self) -> usize {
        self.0
    }
}

/// One Job's working slot: the same `Option<Working>` the single slot was,
/// with its own lock.
///
/// **`Some` is still the whole of what "this Job is being worked" means.** What
/// changed is that there is one of these per Job rather than one for Fleet.
pub(crate) type Slot = Arc<Mutex<Option<Working>>>;

/// Which Jobs are being worked, and how many may be.
///
/// **It holds no `Working` itself.** Every value in the map is a lock somebody
/// else may be inside, which is what lets one Job's gate run a Check without
/// another Job's Drone waiting on it.
pub(crate) struct Slots {
    cap: Concurrency,
    held: BTreeMap<JobId, Slot>,
}

impl Slots {
    pub(crate) fn bounded_by(cap: Concurrency) -> Slots {
        Slots {
            cap,
            held: BTreeMap::new(),
        }
    }

    /// Forget every slot whose Drone has gone.
    ///
    /// A slot is emptied by the code holding it — `end_the_drone`, `reap`,
    /// `stood_down` — and that code cannot remove the entry, because removing
    /// it means taking the roster while holding a slot and the order forbids
    /// it. So the entry is swept here instead, before anything counts.
    ///
    /// **`try_lock`, never `lock`.** A slot somebody is inside is a Job being
    /// worked whatever it holds, and a sweep that waited for it would be the
    /// roster blocking on a Check.
    fn sweep(&mut self) {
        self.held.retain(|_, slot| match slot.try_lock() {
            Ok(held) => held.is_some(),
            Err(_) => true,
        });
    }

    /// How many Jobs are being worked.
    pub(crate) fn count(&mut self) -> usize {
        self.sweep();
        self.held.len()
    }

    /// Whether another Job may be admitted. **The predicate `admit_next` opens
    /// with, and the one `queued_reason` answers `waiting_on_resources` from.**
    /// One answer, not two — a Board saying a Job is blocked while Fleet is
    /// starting it is what a second predicate here would produce.
    pub(crate) fn room(&mut self) -> bool {
        self.count() < self.cap.jobs()
    }

    /// Every Job being worked, oldest id first.
    pub(crate) fn working_on(&mut self) -> Vec<JobId> {
        self.sweep();
        self.held.keys().cloned().collect()
    }

    /// Every Job's slot, for a turn to walk.
    ///
    /// The handles are cloned out and the roster lock is dropped before any of
    /// them is taken, which is what keeps the order — roster, then slot — with
    /// no path holding both.
    pub(crate) fn each(&mut self) -> Vec<(JobId, Slot)> {
        self.sweep();
        self.held
            .iter()
            .map(|(job, slot)| (job.clone(), Arc::clone(slot)))
            .collect()
    }

    /// This Job's slot, where it has one. `None` is a Job with no Drone.
    pub(crate) fn slot_of(&self, job: &JobId) -> Option<Slot> {
        self.held.get(job).map(Arc::clone)
    }

    /// This Job's slot, made if it has none.
    ///
    /// **Admission's, and nothing else calls it.** A slot that exists holding
    /// nothing counts against the bound while the dispatch that will fill it
    /// runs, which is right: the Job has been taken out of the queue.
    pub(crate) fn opened_for(&mut self, job: &JobId) -> Slot {
        Arc::clone(
            self.held
                .entry(job.clone())
                .or_insert_with(|| Arc::new(Mutex::new(None))),
        )
    }

    /// Forget this Job's slot outright, whatever it holds.
    ///
    /// For the one caller that knows the dispatch failed and must not leave the
    /// bound spent on a Job with no Drone.
    pub(crate) fn closed(&mut self, job: &JobId) {
        self.held.remove(job);
    }
}
