//! Which conversation a message belongs to, and the one reply at a time each
//! conversation writes. `#939`.
//!
//! **A conversation is looked up by a [`ConversationKey`], and a key is a
//! repository today.** A session per topic is a second constructor on the key;
//! the table, the thread and the host are keyed by whatever it holds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::asking::{Asks, HelmAskHold};
use super::hosting::Hosting;
use super::thread::Thread;

/// Which conversation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationKey(String);

impl ConversationKey {
    /// A repository's conversation. **The one constructor**, so a key per topic
    /// is a second one here and nothing else.
    pub fn of_repository(manifest: &core_model::ManifestId) -> ConversationKey {
        ConversationKey(manifest.as_str().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Where its thread is kept. A character a file name should not carry is
    /// written as `_`, so no key can name a path outside the directory.
    pub(crate) fn thread_in(&self, records_root: &str) -> PathBuf {
        let name: String = self
            .0
            .chars()
            .map(
                |c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    true => c,
                    false => '_',
                },
            )
            .collect();
        Path::new(records_root)
            .join("helm")
            .join(format!("{name}.jsonl"))
    }
}

/// Every conversation Fleet hosts, and the host that carries their messages.
pub struct Conversations {
    host: Arc<dyn Hosting>,
    open: Mutex<BTreeMap<ConversationKey, Arc<Conversation>>>,
    /// Every call held open for a person, across every conversation. **Here
    /// rather than on a [`Conversation`]** because the dock is one list: what
    /// is waiting on a person is waiting on them wherever it came from.
    asks: Asks,
}

impl Conversations {
    pub fn hosted_by(host: Arc<dyn Hosting>, hold: HelmAskHold) -> Conversations {
        Conversations {
            host,
            open: Mutex::new(BTreeMap::new()),
            asks: Asks::holding_for(hold),
        }
    }

    pub(crate) fn host(&self) -> &dyn Hosting {
        self.host.as_ref()
    }

    pub(crate) fn asks(&self) -> &Asks {
        &self.asks
    }

    /// The hold this Fleet was assembled with, so a Fleet rebuilt around a
    /// different host keeps the bound the composition root resolved.
    pub fn hold(&self) -> HelmAskHold {
        self.asks.held_for()
    }

    /// The conversation `key` names, its thread read from under `records_root`
    /// the first time it is asked for.
    pub(crate) fn open(&self, key: &ConversationKey, records_root: &str) -> Arc<Conversation> {
        let mut open = self
            .open
            .lock()
            .expect("the conversations are not held across a panic");
        let conversation = open.entry(key.clone()).or_insert_with(|| {
            Arc::new(Conversation {
                turn: tokio::sync::Mutex::new(()),
                waiting: AtomicUsize::new(0),
                thread: Thread::opened(key.thread_in(records_root)),
            })
        });
        Arc::clone(conversation)
    }
}

/// One conversation.
pub(crate) struct Conversation {
    /// Held while a reply is written. **One at a time**: a second message
    /// resumes the session the first reply ran in, so it waits for that one.
    pub(crate) turn: tokio::sync::Mutex<()>,
    /// Messages taken and not yet answered.
    waiting: AtomicUsize,
    pub(crate) thread: Thread,
}

impl Conversation {
    pub(crate) fn taken(&self) {
        self.waiting.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn answered(&self) {
        self.waiting.fetch_sub(1, Ordering::SeqCst);
    }

    pub(crate) fn replying(&self) -> bool {
        self.waiting.load(Ordering::SeqCst) > 0
    }
}
