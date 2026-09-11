//! Which servers Fleet holds, and the last instance of each that ended.
//!
//! **In memory and never written down**, for `crate::rehearsing::Rehearsals`'
//! reason: a server is true only as long as the process holding it lives.
//! **One per holder per name**, taken under one lock — `docs/concepts/fleet.md`,
//! *Servers* — so a person and a Drone asking at once share one instance.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use core_model::JobId;
use ipc::ServerState;
use tokio::sync::watch;

/// Whose worktree and span a server runs in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Holder {
    Job(JobId),
    MainCheckout,
}

type Key = (Holder, String);

/// How to stop one instance, and hear where it stands.
pub(crate) type Stopping = (Arc<watch::Sender<bool>>, watch::Receiver<ServerState>);

/// `std`'s lock: it is never held across an `.await`.
#[derive(Clone, Default)]
pub(crate) struct Servers(Arc<Mutex<Book>>);

#[derive(Default)]
struct Book {
    live: BTreeMap<Key, Live>,
    ended: BTreeMap<Key, Ended>,
}

/// One instance that is starting or serving.
pub(crate) struct Live {
    /// Where it stands. The task holds the sender and moves it on each phase.
    pub(crate) now: watch::Receiver<ServerState>,
    pub(crate) stop: Arc<watch::Sender<bool>>,
    /// Its output, held here so a viewer can subscribe. The task holds the
    /// other end; both going is what tells a viewer it ended.
    pub(crate) feed: api::RunFeed,
    pub(crate) dir: PathBuf,
}

struct Ended {
    state: ServerState,
    dir: PathBuf,
}

impl Servers {
    /// The instance up for this holder and name, if one is.
    pub(crate) fn running(
        &self,
        holder: &Holder,
        name: &str,
    ) -> Option<watch::Receiver<ServerState>> {
        self.book()
            .live
            .get(&(holder.clone(), name.to_string()))
            .map(|live| live.now.clone())
    }

    /// This holder's instance of `name`: the live one, or the last that ended.
    pub(crate) fn instance(&self, holder: &Holder, name: &str) -> Option<ServerState> {
        let key = (holder.clone(), name.to_string());
        let book = self.book();
        match book.live.get(&key) {
            Some(live) => Some(live.now.borrow().clone()),
            None => book.ended.get(&key).map(|ended| ended.state.clone()),
        }
    }

    /// Hold `live` for this holder and name — **or hand back the one already
    /// held**, which is the whole of "one instance per Job per server".
    pub(crate) fn take(
        &self,
        holder: &Holder,
        name: &str,
        id: &str,
        live: Live,
    ) -> Result<Held, watch::Receiver<ServerState>> {
        let key = (holder.clone(), name.to_string());
        let mut book = self.book();
        if let Some(up) = book.live.get(&key) {
            return Err(up.now.clone());
        }
        book.ended.remove(&key);
        let dir = live.dir.clone();
        book.live.insert(key.clone(), live);
        Ok(Held {
            servers: self.clone(),
            key,
            id: id.to_string(),
            dir,
            done: false,
        })
    }

    /// How to stop the instance `id` — only while it is up.
    pub(crate) fn stopping(&self, id: &str) -> Option<Stopping> {
        self.book()
            .live
            .values()
            .find(|live| live.now.borrow().id == id)
            .map(|live| (Arc::clone(&live.stop), live.now.clone()))
    }

    /// Whether `id` is an instance that has ended.
    pub(crate) fn has_ended(&self, id: &str) -> bool {
        self.book().ended.values().any(|ended| ended.state.id == id)
    }

    /// `id`'s state, a subscription to its output while it is up, and its
    /// directory. **The subscription is taken under the lock**, before the
    /// caller reads the log, for `observe_run`'s reason.
    pub(crate) fn watching(
        &self,
        id: &str,
    ) -> Option<(ServerState, Option<api::RunWatch>, PathBuf)> {
        let book = self.book();
        if let Some(live) = book.live.values().find(|live| live.now.borrow().id == id) {
            return Some((
                live.now.borrow().clone(),
                Some(live.feed.watch()),
                live.dir.clone(),
            ));
        }
        book.ended
            .values()
            .find(|ended| ended.state.id == id)
            .map(|ended| (ended.state.clone(), None, ended.dir.clone()))
    }

    /// Every instance held, and the last of each that ended, newest first.
    pub(crate) fn every(&self) -> Vec<ServerState> {
        let book = self.book();
        let mut every: Vec<ServerState> = book
            .live
            .values()
            .map(|live| live.now.borrow().clone())
            .chain(book.ended.values().map(|ended| ended.state.clone()))
            .collect();
        // An id is a ULID, so its order is the order they started in.
        every.sort_by(|a, b| b.id.cmp(&a.id));
        every
    }

    /// How to stop every instance `holder` has up — every one Fleet has, where
    /// `holder` is `None`.
    pub(crate) fn held_by(&self, holder: Option<&Holder>) -> Vec<Stopping> {
        self.book()
            .live
            .iter()
            .filter(|((whose, _), _)| holder.is_none_or(|holder| holder == whose))
            .map(|(_, live)| (Arc::clone(&live.stop), live.now.clone()))
            .collect()
    }

    /// Let go of every ended instance `holder` left. A Job that ended has
    /// nothing left to show a server of.
    pub(crate) fn forget(&self, holder: &Holder) {
        self.book().ended.retain(|(whose, _), _| whose != holder);
    }

    /// Every instance up, by id — what a sweep of old directories keeps.
    pub(crate) fn live_ids(&self) -> Vec<String> {
        self.book()
            .live
            .values()
            .map(|live| live.now.borrow().id.clone())
            .collect()
    }

    fn book(&self) -> MutexGuard<'_, Book> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// One holder's name held for an instance, given back **however the task
/// ends** — a `Drop`, so a task that panics leaves the name startable.
pub(crate) struct Held {
    servers: Servers,
    key: Key,
    id: String,
    dir: PathBuf,
    done: bool,
}

impl Held {
    /// The instance ended: kept as the last one, so a server that fell over
    /// can still be read, and the name is free to start again.
    pub(crate) fn ended(mut self, state: ServerState) {
        let mut book = self.servers.book();
        if book
            .live
            .get(&self.key)
            .is_some_and(|live| live.now.borrow().id == self.id)
        {
            book.live.remove(&self.key);
        }
        book.ended.insert(
            self.key.clone(),
            Ended {
                state,
                dir: self.dir.clone(),
            },
        );
        drop(book);
        self.done = true;
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        let mut book = self.servers.book();
        if book
            .live
            .get(&self.key)
            .is_some_and(|live| live.now.borrow().id == self.id)
        {
            book.live.remove(&self.key);
        }
    }
}
