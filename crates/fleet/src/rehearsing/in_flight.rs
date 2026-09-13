//! Which owners have a run out, how to stop it, and where its output goes.
//!
//! **In memory and never written down**, for `crate::showing_again::Pressing`'s
//! reason: a run is true only as long as the process running it lives.
//!
//! **One per owner, and an owner is a Job or the main checkout** — see
//! [`Owner`](super::owner::Owner). Two runs in one tree fight over one build
//! directory, so a second run on the same owner is refused; a Job's run and
//! the checkout's are in different trees and neither waits for the other.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::watch;

use super::owner::Owner;
use super::record::{Record, Underway};
use super::verifying::Verifies;

type Stopping = (Arc<watch::Sender<bool>>, watch::Receiver<Option<Record>>);

/// `std`'s lock: it is never held across an `.await`.
#[derive(Clone, Default)]
pub(crate) struct Rehearsals {
    out: Arc<Mutex<BTreeMap<Owner, InFlight>>>,
    /// The checkout's latest Verify. Here rather than on `Fleet`: it is this
    /// one-run-at-a-time state, one sequence up — `super::verifying`.
    verifies: Verifies,
}

struct InFlight {
    underway: Underway,
    stop: Arc<watch::Sender<bool>>,
    done: watch::Receiver<Option<Record>>,
    /// The run's own channel, held here so a viewer can subscribe to it. The
    /// run holds the other end; both going is what tells a viewer it ended.
    feed: api::RunFeed,
}

impl Rehearsals {
    pub(super) fn verifies(&self) -> &Verifies {
        &self.verifies
    }

    pub(super) fn in_flight(&self, owner: &Owner) -> Option<Underway> {
        self.held().get(owner).map(|out| out.underway.clone())
    }

    /// Take this owner for a run, or `None` where one is already out.
    pub(super) fn take(
        &self,
        owner: &Owner,
        underway: &Underway,
        stop: watch::Sender<bool>,
        done: watch::Receiver<Option<Record>>,
        feed: api::RunFeed,
    ) -> Option<Held> {
        let mut out = self.held();
        if out.contains_key(owner) {
            return None;
        }
        out.insert(
            owner.clone(),
            InFlight {
                underway: underway.clone(),
                stop: Arc::new(stop),
                done,
                feed,
            },
        );
        Some(Held {
            rehearsals: self.clone(),
            owner: owner.clone(),
        })
    }

    /// How to stop the run `id`, and hear that it ended — only while it is the
    /// one out on this owner.
    pub(super) fn stopping(&self, owner: &Owner, id: &str) -> Option<Stopping> {
        self.held()
            .get(owner)
            .filter(|out| out.underway.id == id)
            .map(|out| (Arc::clone(&out.stop), out.done.clone()))
    }

    /// A subscription to the run `id`'s output, with its name — only while it
    /// is the one out on this owner.
    pub(super) fn watching(&self, owner: &Owner, id: &str) -> Option<(String, api::RunWatch)> {
        self.held()
            .get(owner)
            .filter(|out| out.underway.id == id)
            .map(|out| (out.underway.name.clone(), out.feed.watch()))
    }

    fn held(&self) -> MutexGuard<'_, BTreeMap<Owner, InFlight>> {
        self.out
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// One owner held for a run, given back **however the run ends** — a `Drop`,
/// so a run that panics still leaves the owner runnable.
pub(crate) struct Held {
    rehearsals: Rehearsals,
    owner: Owner,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.rehearsals.held().remove(&self.owner);
    }
}
