//! The two upgrades, and what goes down each.
//!
//! **Two sockets and not one.** `/events` is global and carries every Job's
//! state, because Bridge holds exactly one connection; `/jobs/:id/observe` is
//! one Job's turns and is opened deliberately. A transcript row on the global
//! stream would evict the state changes the Board is drawn from, and an
//! eviction there is a full resync of every Job rather than a lost row.
//!
//! Both are extractors in the same `Router`. There is no second port.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use ipc::{JobId, Missed, Resync, StreamMessage, PROTOCOL_VERSION};

use crate::answers::refused;
use crate::daemon::Queries;
use crate::routes::Served;
use crate::stream::Next;

/// One Job's turns. **Per-Job, and not on `/events`** — that stream is one
/// drop-oldest channel carrying every Job, and a transcript row on it would
/// evict the state changes a Board is drawn from.
///
/// The daemon is asked **before** the upgrade, so a Job that does not exist is
/// a 404 the caller reads at the moment they asked rather than a socket that
/// opens and says nothing. What comes back already holds the subscription and
/// the history, in that order.
pub(crate) async fn observe_job<D: Queries>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_job(JobId::carried(job_id)).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| crate::observing::relay(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

/// The event stream. **Global, and a client subscribes to nothing** — one
/// socket carries every Job, because Bridge holds exactly one connection.
///
/// Nothing is read from the socket. The stream is one-directional by design:
/// there is no subscribe message to read, and a connection that carried state
/// would be a connection that is expensive to drop and remake.
pub(crate) async fn events<D: Queries>(
    State(served): State<Served<D>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| watch(socket, served))
}

async fn watch<D: Queries>(mut socket: WebSocket, served: Served<D>) {
    // Subscribe first, then snapshot. The other order drops whatever lands in
    // between; this order can only repeat, and a repeat is detectable.
    let mut subscription = served.events().subscribe();
    if !resync(&mut socket, &served).await {
        return;
    }
    while let Some(next) = subscription.next().await {
        let delivered = match next {
            Next::Send(delivered) => send(&mut socket, &StreamMessage::Event(delivered)).await,
            // The count alone cannot repair what the client holds, so the drop
            // is always followed by current state.
            Next::Missed(dropped) => {
                send(&mut socket, &StreamMessage::Missed(Missed { dropped })).await
                    && resync(&mut socket, &served).await
            }
        };
        if !delivered {
            return;
        }
    }
}

/// Current state, whole. `false` where the socket or the daemon is gone, and
/// the caller stops.
///
/// A daemon that cannot answer closes the socket rather than sending a partial
/// snapshot: there is no error message on this stream, and a client that
/// reconnects gets a whole answer or none.
async fn resync<D: Queries>(socket: &mut WebSocket, served: &Served<D>) -> bool {
    let cursor = served.events().cursor();
    let Ok(jobs) = served.daemon().list_jobs().await else {
        return false;
    };
    send(
        socket,
        &StreamMessage::Resync(Resync {
            protocol_version: PROTOCOL_VERSION,
            cursor,
            jobs,
        }),
    )
    .await
}

async fn send(socket: &mut WebSocket, message: &StreamMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued: this is what makes a slow client slow *this* task
    // rather than Fleet's memory, so the bound upstream is the thing that gives.
    socket.send(Message::Text(text)).await.is_ok()
}
