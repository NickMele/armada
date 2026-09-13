//! A conversation's thread: what was said, kept in a file under the
//! repository's records and offered to whoever is watching. `#939`.
//!
//! **The file is the thread; the session is not.** The agent CLI keeps its own
//! transcript, in its own format and on its own retention, and a Bridge opened
//! after a restart reads the thread from here.
//!
//! **Appended as it is said, synchronously.** A reply is a handful of lines, and
//! a queue in front of them could leave a thread short when Fleet restarts
//! straight after a reply.

use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use api::{HelmFeed, HelmWatch};
use ipc::{HelmClosed, HelmMessage, HelmSilence};

/// How many messages a viewer is sent when it opens. What is left out is
/// counted.
pub const THREAD: usize = 512;

pub(crate) struct Thread {
    feed: HelmFeed,
    held: Mutex<Held>,
    file: PathBuf,
}

#[derive(Default)]
struct Held {
    messages: VecDeque<HelmMessage>,
    skipped: u64,
}

impl Held {
    fn push(&mut self, message: HelmMessage) {
        if self.messages.len() == THREAD {
            self.messages.pop_front();
            self.skipped += 1;
        }
        self.messages.push_back(message);
    }
}

impl Thread {
    /// The thread `file` holds. **A line that will not decode is counted**, for
    /// the transcript backfill's reason: it was a message.
    pub(crate) fn opened(file: PathBuf) -> Thread {
        let mut held = Held::default();
        if let Ok(text) = std::fs::read_to_string(&file) {
            for line in text.lines().filter(|line| !line.trim().is_empty()) {
                match ipc::decode::<HelmMessage>("a Helm thread line", line.as_bytes()) {
                    Ok(message) => held.push(message),
                    Err(_) => held.skipped += 1,
                }
            }
        }
        Thread {
            feed: HelmFeed::new(),
            held: Mutex::new(held),
            file,
        }
    }

    /// Say something into the thread: held, offered, written.
    ///
    /// **A file that will not take the line loses it on a restart and nowhere
    /// else** — viewers already have it, and the session remembers regardless.
    pub(crate) fn say(&self, message: HelmMessage) {
        let mut held = self.held();
        if let Ok(line) = ipc::encode(&message) {
            let _ = self.appended(&line);
        }
        held.push(message.clone());
        self.feed.offer(message);
    }

    /// The subscription, then the thread, under one lock so nothing said
    /// between the two is lost or sent twice.
    pub(crate) fn watched(&self) -> (HelmWatch, Vec<HelmMessage>, u64) {
        let held = self.held();
        let watch = self.feed.watch();
        (watch, held.messages.iter().cloned().collect(), held.skipped)
    }

    /// Forget the thread, and tell every viewer it is gone.
    pub(crate) fn clear(&self) {
        let mut held = self.held();
        *held = Held::default();
        let _ = std::fs::remove_file(&self.file);
        self.feed.offer(HelmMessage::Closed(HelmClosed {
            because: HelmSilence::StartedFresh,
        }));
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.held
            .lock()
            .expect("the thread is not held across a panic")
    }

    fn appended(&self, line: &str) -> std::io::Result<()> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)?;
        writeln!(out, "{line}")
    }
}
