//! The event stream: bounded, lossy, and honest about it.
//!
//! # It bounds the risk the architecture named as its largest
//!
//! axum's WebSocket sink is unbounded from the application side, so several
//! Drones producing at Drone speed into a minimised Bridge grow Fleet's memory
//! with nothing pushing back. Everything published here goes through a
//! [`tokio::sync::broadcast`] of fixed capacity, which is bounded and
//! **drop-oldest** by construction: a subscriber that falls behind loses the
//! oldest events and is told how many, and the sink below it is fed one message
//! at a time by a task that awaits each send.
//!
//! Bounded-and-lossy is only safe if the client knows it happened, which is why
//! a drop is a message and not a silence: **you missed N events, here is
//! current state**. A reconnecting Bridge that believed its history was complete
//! would render a Board that is quietly wrong, and quietly wrong is worse than
//! visibly stale — nothing on the screen tells the person to distrust it.
//!
//! Errors are not exempt from the bound. An error is an ordinary event and may
//! be dropped like any other; what a drop costs is the speed of noticing, never
//! the fact, because the durable record is written before anything is broadcast.
//!
//! # A resync may repeat, and never omits
//!
//! The subscription is opened *before* the snapshot is read, so no event can
//! fall between the two. The cost is the other direction: a snapshot may
//! already reflect an event that then arrives, and a client can see a
//! transition it has already applied. That trade is deliberate — a duplicate
//! `job.state_changed` is detectable from its `from` status, and a missing one
//! is not detectable at all.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ipc::{Cursor, Delivered, Event};
use tokio::sync::broadcast;

/// How many events the stream holds for a subscriber that is not keeping up.
///
/// **The number is not settled** — `docs/practices/protocol.md` carries it as
/// an open question, along with whether one capacity serves every event kind.
/// This one is chosen to absorb the burst a few Drones produce while a renderer
/// is busy, and to be small enough that a Bridge which has stopped draining
/// hears about it in seconds rather than minutes.
pub const BACKLOG: usize = 256;

/// Where events are published, and what a socket subscribes to.
///
/// Cheap to clone: every clone publishes into the same channel. Fleet holds one
/// and `api` holds one, which is the in-process channel the topology draws
/// between them.
pub struct Broadcaster {
    next: Arc<AtomicU64>,
    outbound: broadcast::Sender<Delivered>,
}

impl Broadcaster {
    /// A stream with the default backlog.
    pub fn new() -> Broadcaster {
        Broadcaster::with_backlog(BACKLOG)
    }

    /// A stream with a stated backlog. Taken as an argument so a test can drive
    /// the drop path deliberately rather than by producing 256 events.
    pub fn with_backlog(backlog: usize) -> Broadcaster {
        let (outbound, _) = broadcast::channel(backlog.max(1));
        Broadcaster {
            next: Arc::new(AtomicU64::new(0)),
            outbound,
        }
    }

    /// Publish an event and return the position it was given.
    ///
    /// Never blocks and never fails. With nobody listening the event is
    /// dropped, which is correct: the durable record is the store's, and this
    /// channel is a notification of it rather than a copy of it.
    pub fn publish(&self, event: Event) -> Cursor {
        let cursor = Cursor::at(self.next.fetch_add(1, Ordering::SeqCst));
        let _ = self.outbound.send(Delivered { cursor, event });
        cursor
    }

    /// The position the next published event will take. What a snapshot is
    /// current as of.
    pub fn cursor(&self) -> Cursor {
        Cursor::at(self.next.load(Ordering::SeqCst))
    }

    /// How many clients are listening right now.
    ///
    /// **What lets a producer decline to produce.** Everything published here
    /// is cheap to make except one thing — the footprint of a Drone's worktree,
    /// which is a repository read — and a Fleet nobody has open should not be
    /// paying for a list no socket will carry. It is a count and not a
    /// subscription: a producer asks whether anyone is there, and cannot learn
    /// who or what they want.
    ///
    /// Racy by nature. A client may connect or drop between this answer and
    /// whatever is done about it, and the cost of being wrong either way is one
    /// reading taken or one interval's delay before the next.
    pub fn watching(&self) -> usize {
        self.outbound.receiver_count()
    }

    /// Listen. **Open this before reading the snapshot it accompanies** — the
    /// other order can lose an event in between.
    pub fn subscribe(&self) -> Subscription {
        Subscription {
            inbound: self.outbound.subscribe(),
        }
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Broadcaster::new()
    }
}

impl Clone for Broadcaster {
    fn clone(&self) -> Broadcaster {
        Broadcaster {
            next: Arc::clone(&self.next),
            outbound: self.outbound.clone(),
        }
    }
}

/// One listener's view of the stream.
pub struct Subscription {
    inbound: broadcast::Receiver<Delivered>,
}

/// What a subscription has for the socket to send next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// An event, at its position.
    Send(Delivered),
    /// The bound dropped this many events before this subscriber read them.
    /// The socket owes the client a fresh resync, not just the count.
    Missed(u64),
}

impl Subscription {
    /// The next thing to send, or `None` once the broadcaster is gone.
    pub async fn next(&mut self) -> Option<Next> {
        match self.inbound.recv().await {
            Ok(delivered) => Some(Next::Send(delivered)),
            Err(broadcast::error::RecvError::Lagged(dropped)) => Some(Next::Missed(dropped)),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }
}
