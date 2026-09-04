//! The two upgrades, and what goes down each.
//!
//! **Three sockets and not one.** `/events` is global and carries every Job's
//! state, because Bridge holds exactly one connection; `/jobs/:id/observe` is
//! one Job's turns and `/jobs/:id/log` is what Fleet did to it, both opened
//! deliberately on one Job. A transcript row on the global stream would evict
//! the state changes the Board is drawn from, and an eviction there is a full
//! resync of every Job rather than a lost row.
//!
//! **The third is not a fourth voice on the second.** `observe_job` exists
//! only while a Drone is writing — it answers `nothing_writing` and closes on
//! exactly the Job whose log this carries — and folding notes into `Saw` would
//! be a variant an old Bridge's `switch` has no arm for, which
//! `docs/practices/protocol.md` makes a major bump.
//!
//! All three are extractors in the same `Router`. There is no second port.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{JobId, Missed, Resync, StreamMessage, WireError, PROTOCOL_VERSION};

use crate::answers::{problem, refused};
use crate::daemon::Daemon;
use crate::routes::Served;
use crate::stream::Next;

/// A listener built with no reader for a Job's own log.
///
/// **A fault and not an empty answer.** A stream that opened and stayed silent
/// would read as a Job nothing is happening to, which is the exact reading this
/// whole route exists to stop being wrong.
pub(crate) const NO_JOURNAL: &str = "api.no_journal_reader";

/// One Job's turns. **Per-Job, and not on `/events`** — that stream is one
/// drop-oldest channel carrying every Job, and a transcript row on it would
/// evict the state changes a Board is drawn from.
///
/// The daemon is asked **before** the upgrade, so a Job that does not exist is
/// a 404 the caller reads at the moment they asked rather than a socket that
/// opens and says nothing. What comes back already holds the subscription and
/// the history, in that order.
pub(crate) async fn observe_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_job(JobId::carried(job_id)).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| crate::observing::relay(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

/// One Job's own log: what Fleet did to it, as the Job's log recorded it.
///
/// **Per-Job and its own socket**, for the reasons at the top of this module.
/// The Job is asked for **before** the upgrade, so an id that names nothing is
/// a 404 the caller reads at the moment they asked — the same order
/// [`observe_job`] takes, and once per connection rather than once per pass.
pub(crate) async fn job_log<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let job_id = JobId::carried(job_id);
    let Some(journal) = served.journal() else {
        return problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            &WireError::raised(
                NO_JOURNAL,
                "this Fleet was built with no reader for a Job's own log",
                served.run_id().clone(),
            )
            .about_job(job_id),
        );
    };
    if let Err(refusal) = served.daemon().get_job(job_id.clone()).await {
        return refused(refusal);
    }
    upgrade.on_upgrade(move |socket| crate::journal::relay(socket, job_id, journal))
}

/// The event stream. **Global, and a client subscribes to nothing** — one
/// socket carries every Job, because Bridge holds exactly one connection.
///
/// Nothing is read from the socket. The stream is one-directional by design:
/// there is no subscribe message to read, and a connection that carried state
/// would be a connection that is expensive to drop and remake.
pub(crate) async fn events<D: Daemon>(
    State(served): State<Served<D>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade.on_upgrade(move |socket| watch(socket, served))
}

async fn watch<D: Daemon>(mut socket: WebSocket, served: Served<D>) {
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
async fn resync<D: Daemon>(socket: &mut WebSocket, served: &Served<D>) -> bool {
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
