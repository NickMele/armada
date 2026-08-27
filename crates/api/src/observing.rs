//! Watching one Job's turns: a channel per Job, and the socket that relays it.
//!
//! # A channel per Job, not [`crate::Broadcaster`]
//!
//! That stream is one drop-oldest channel carrying every Job, so a transcript
//! row on it would evict the state changes a Board is drawn from — an eviction
//! there is a full resync of every Job, paid for as long as anybody watches.
//!
//! # History and live are one mechanism
//!
//! [`Observed`] carries both, built by the daemon in the order the event stream
//! already documents: **subscribe, then read the history.** The other order
//! loses a row arriving between the two; this one repeats it, and a repeat is
//! detectable from a row's call id while a gap is not detectable at all.
//!
//! # Nothing here can slow the loop that advances the Job
//!
//! [`Feed::offer`] is a broadcast send: synchronous, non-blocking, drop-oldest.
//! A slow viewer loses the oldest rows and is told how many.
//!
//! `docs/concepts/observe.md` is the design.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use axum::extract::ws::{Message, WebSocket};
use ipc::{
    Closed, JobId, Missed, Opened, Shown, Silence, TranscriptRow, TurnMessage, PROTOCOL_VERSION,
};
use tokio::sync::broadcast;

/// How many rows a Job's channel holds for a viewer that is not keeping up.
///
/// The same number the sink behind Fleet's tee uses, because the producer is
/// the same loop: a bound on the pathological case rather than a working size.
pub const WATCHING: usize = 1024;

/// Every Job somebody could be watching.
///
/// Cheap to clone; every clone reaches the same set. Fleet holds one, and
/// nothing else needs to — a viewer reaches it through [`crate::Daemon`].
pub struct Turns {
    /// **`Weak`, so an entry cannot outlive the Drone it belongs to.** The
    /// strong end is the [`Feed`] the dispatch holds, and a Job whose Drone is
    /// gone leaves a dead entry that the next call removes.
    open: Arc<Mutex<HashMap<String, Weak<broadcast::Sender<TranscriptRow>>>>>,
}

impl Turns {
    pub fn new() -> Turns {
        Turns {
            open: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// The writer's end, for a Drone about to be spawned.
    ///
    /// **Dropping it ends the watching.** The channel closes, every viewer is
    /// told, and the entry is swept by the next call rather than by a `Drop`
    /// that would need this map's lock.
    pub fn feeding(&self, job: &JobId) -> Feed {
        let rows = Arc::new(broadcast::channel(WATCHING).0);
        let mut open = self
            .open
            .lock()
            .expect("the watch set is not held across a panic");
        open.retain(|_, feed| feed.strong_count() > 0);
        open.insert(job.as_str().to_string(), Arc::downgrade(&rows));
        Feed { rows }
    }

    /// Listen. `None` where nothing is writing this Job, which is ordinary: it
    /// was never dispatched, it is finished, or its Drone went with the Fleet
    /// that spawned it.
    ///
    /// **Open this before reading the history.** See the module comment.
    pub fn watching(&self, job: &JobId) -> Option<Watch> {
        let open = self
            .open
            .lock()
            .expect("the watch set is not held across a panic");
        let rows = open.get(job.as_str())?.upgrade()?;
        Some(Watch {
            inbound: rows.subscribe(),
        })
    }
}

impl Default for Turns {
    fn default() -> Turns {
        Turns::new()
    }
}

impl Clone for Turns {
    fn clone(&self) -> Turns {
        Turns {
            open: Arc::clone(&self.open),
        }
    }
}

/// One Drone's rows going out. Held by whatever spawned the Drone.
pub struct Feed {
    rows: Arc<broadcast::Sender<TranscriptRow>>,
}

impl Feed {
    /// Offer a row. **Never blocks and never fails.**
    ///
    /// With nobody watching the row is dropped and nothing is told, which is
    /// how publishing already behaves here: the durable record is the file, and
    /// this channel is a view of it rather than a copy.
    pub fn offer(&self, row: TranscriptRow) {
        let _ = self.rows.send(row);
    }
}

/// One viewer's end of a Job's channel.
pub struct Watch {
    inbound: broadcast::Receiver<TranscriptRow>,
}

/// What a subscription has for the socket to send next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seen {
    Row(TranscriptRow),
    /// The bound dropped this many rows before this viewer read them.
    Missed(u64),
}

impl Watch {
    /// The next row, or `None` once the Drone that was writing has gone.
    pub async fn next(&mut self) -> Option<Seen> {
        match self.inbound.recv().await {
            Ok(row) => Some(Seen::Row(row)),
            Err(broadcast::error::RecvError::Lagged(dropped)) => Some(Seen::Missed(dropped)),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }
}

/// What one viewer is answered with: the subscription, opened first, and the
/// history read after it.
///
/// **Assembled by the daemon**, because only the daemon can do both halves in
/// that order. A caller holding this cannot get the order wrong.
pub struct Observed {
    pub job_id: JobId,
    /// `None` where nothing is writing this Job. The history is still served.
    pub live: Option<Watch>,
    /// The turns already taken, oldest first, across every Drone the Job has
    /// had.
    pub history: Vec<TranscriptRow>,
    /// Older rows the history left out, because the backfill is bounded.
    pub skipped: u64,
}

/// Serve one viewer: what has already happened, then what happens next.
///
/// The socket is never read from. There is no subscribe message — the Job is in
/// the path — and dropping the connection is the whole of unsubscribing.
pub(crate) async fn relay(mut socket: WebSocket, observed: Observed) {
    let Observed {
        job_id,
        mut live,
        history,
        skipped,
    } = observed;
    let opened = Opened {
        protocol_version: PROTOCOL_VERSION,
        job_id,
        live: live.is_some(),
        skipped,
    };
    if !send(&mut socket, &TurnMessage::Opened(opened)).await {
        return;
    }
    for row in history {
        // A withheld row is skipped rather than refused: the file holds kinds a
        // viewer is not shown, and reading one is ordinary.
        let Some(shown) = Shown::of(row) else {
            continue;
        };
        if !send(&mut socket, &TurnMessage::Row(shown)).await {
            return;
        }
    }
    let Some(live) = live.take() else {
        say_closed(&mut socket, Silence::NothingWriting).await;
        return;
    };
    if carrying(&mut socket, live).await {
        say_closed(&mut socket, Silence::DroneEnded).await;
    }
}

/// Relay until the viewer goes away or the Drone does. `true` where it was the
/// Drone, which is the only ending worth a message.
async fn carrying(socket: &mut WebSocket, mut live: Watch) -> bool {
    while let Some(seen) = live.next().await {
        let delivered = match seen {
            Seen::Row(row) => match Shown::of(row) {
                Some(shown) => send(socket, &TurnMessage::Row(shown)).await,
                None => true,
            },
            // Stated rather than left. A viewer that believed its history was
            // whole would read a gap as a Drone that went quiet, which is the
            // one thing this view exists to tell apart.
            Seen::Missed(dropped) => send(socket, &TurnMessage::Missed(Missed { dropped })).await,
        };
        if !delivered {
            return false;
        }
    }
    true
}

async fn say_closed(socket: &mut WebSocket, because: Silence) {
    send(socket, &TurnMessage::Closed(Closed { because })).await;
}

async fn send(socket: &mut WebSocket, message: &TurnMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued: a slow viewer slows this task and nothing else. The
    // bound upstream is what gives, and it says so.
    socket.send(Message::Text(text)).await.is_ok()
}
