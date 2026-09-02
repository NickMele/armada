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
//! # A step ending is not the Job ending
//!
//! One channel belongs to one Drone, and a Job that advances exits one Drone
//! and spawns the next milliseconds later. A viewer dropped at the exit watches
//! the whole of the next step behind a panel that reads as a step which has not
//! started, which is what #324 recorded. So a [`Watch`] outlives the Drone it
//! opened on: it waits [`HANDOVER`] for the Job's next Drone and picks it up.
//! Only a Job where none arrives is [`Silence::DroneEnded`].
//!
//! `docs/concepts/observe.md` is the design.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use ipc::{
    Closed, JobId, Missed, Opened, Shown, Silence, TranscriptRow, TurnMessage, PROTOCOL_VERSION,
};
use tokio::sync::{broadcast, watch};

/// How many rows a Job's channel holds for a viewer that is not keeping up.
///
/// The same number the sink behind Fleet's tee uses, because the producer is
/// the same loop: a bound on the pathological case rather than a working size.
pub const WATCHING: usize = 1024;

/// How long a viewer waits for a Job's next Drone before it is told the Job
/// went quiet.
///
/// **It covers a hand-over and nothing longer.** The exit and the next spawn
/// were 5ms apart on the move record #324 was found on, so fifty times that is
/// generous for the case this exists for. A gap a gate or a Judge call runs in
/// is not covered on purpose: that one is Bridge's to close, on the event that
/// says a Drone was spawned, because a wait long enough for it would leave a
/// Job that has genuinely finished looking like one still working.
const HANDOVER: Duration = Duration::from_millis(250);

/// One Job's place in the watch set, which outlives any one of its Drones.
///
/// The value is whichever Drone is writing now — **`Weak`, so an entry cannot
/// outlive the Drone it belongs to.** Sending is what carries the next Drone's
/// channel to whoever is already watching.
type Slot = watch::Sender<Weak<broadcast::Sender<TranscriptRow>>>;

/// Every Job somebody could be watching.
///
/// Cheap to clone; every clone reaches the same set. Fleet holds one, and
/// nothing else needs to — a viewer reaches it through [`crate::Daemon`].
pub struct Turns {
    /// The strong end of each entry is the [`Feed`] the dispatch holds, and a
    /// Job with neither a Drone nor a viewer leaves a dead entry that the next
    /// call removes.
    open: Arc<Mutex<HashMap<String, Slot>>>,
}

impl Turns {
    pub fn new() -> Turns {
        Turns {
            open: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// The writer's end, for a Drone about to be spawned.
    ///
    /// **Dropping it ends this Drone's channel, not the watching.** A viewer of
    /// a Job that spawns another Drone is carried onto it; one of a Job that
    /// does not is told, and the entry is swept by the next call rather than by
    /// a `Drop` that would need this map's lock.
    pub fn feeding(&self, job: &JobId) -> Feed {
        let rows = Arc::new(broadcast::channel(WATCHING).0);
        let mut open = self
            .open
            .lock()
            .expect("the watch set is not held across a panic");
        // A slot with a viewer on it is kept even with no Drone writing: it is
        // the thing that will carry the next one across.
        open.retain(|_, slot| slot.borrow().strong_count() > 0 || slot.receiver_count() > 0);
        match open.get(job.as_str()) {
            // `send_replace` and not `send`: that one is a no-op where nothing
            // is watching, which would leave the slot naming the Drone that
            // just exited and answer the next viewer of a working Job with
            // nothing writing.
            Some(slot) => {
                slot.send_replace(Arc::downgrade(&rows));
            }
            None => {
                open.insert(job.as_str().to_string(), Slot::new(Arc::downgrade(&rows)));
            }
        }
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
        let slot = open.get(job.as_str())?;
        let rows = slot.borrow().upgrade()?;
        Some(Watch {
            inbound: rows.subscribe(),
            spawned: slot.subscribe(),
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
    /// The Job's next Drone, where one is spawned. **What tells a step ending
    /// from the Job ending** — see [`Watch::next`].
    spawned: watch::Receiver<Weak<broadcast::Sender<TranscriptRow>>>,
}

/// What a subscription has for the socket to send next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seen {
    Row(TranscriptRow),
    /// The bound dropped this many rows before this viewer read them.
    Missed(u64),
}

impl Watch {
    /// The next row, or `None` once the Job has stopped writing them.
    ///
    /// **A Drone exiting is not the end.** A Job that advances spawns its next
    /// step's Drone into a new channel milliseconds later, and this picks that
    /// one up rather than ending — see the module comment.
    pub async fn next(&mut self) -> Option<Seen> {
        loop {
            match self.inbound.recv().await {
                Ok(row) => return Some(Seen::Row(row)),
                Err(broadcast::error::RecvError::Lagged(dropped)) => {
                    return Some(Seen::Missed(dropped))
                }
                Err(broadcast::error::RecvError::Closed) => self.inbound = self.handover().await?,
            }
        }
    }

    /// The Job's next Drone's channel, or `None` where none was spawned inside
    /// [`HANDOVER`].
    ///
    /// Rows the new Drone wrote before this subscribed are lost, which is this
    /// channel's stated bargain: the durable record is the file, and a Drone's
    /// first row is a process start and a model call after the spawn rather
    /// than in the microseconds this takes.
    async fn handover(&mut self) -> Option<broadcast::Receiver<TranscriptRow>> {
        let arrived = tokio::time::timeout(HANDOVER, async {
            loop {
                // `Err` is the Job's whole entry going away, which is the same
                // answer as no Drone arriving.
                self.spawned.changed().await.ok()?;
                let rows = self.spawned.borrow_and_update().upgrade();
                if let Some(rows) = rows {
                    return Some(rows.subscribe());
                }
            }
        });
        arrived.await.ok().flatten()
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
