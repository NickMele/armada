//! The route table, by hand, and the handlers under it.
//!
//! # Hand-written, and that is the accepted cost
//!
//! A typo in a path here is a runtime 404, not a compile error. That trade was
//! made deliberately against carrying a codegen toolchain, and it is why
//! [`SERVED`] exists: every route is declared once as data beside the operation
//! name `crates/ipc/operations.toml` keys it under, so a test can walk the
//! table and prove each row is actually routed. **A route that exists in the
//! inventory and nowhere in the router is exactly the failure this shape is
//! paying for.**
//!
//! # One listener
//!
//! The WebSocket upgrade is an extractor in this same `Router`. There is no
//! second port and no assembly step: queries and commands answer over HTTP
//! because they are request-response, and only the unsolicited push needs the
//! socket. Who initiates is the whole rule.

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use ipc::{JobId, Missed, ProposeJob, Resync, RunId, StreamMessage, WireError, PROTOCOL_VERSION};
use serde::Serialize;
use std::sync::Arc;

use crate::daemon::{Daemon, Refusal};
use crate::stream::{Broadcaster, Next};

/// One operation, and where it is served.
///
/// `operation` is the key in `crates/ipc/operations.toml`, spelled exactly as
/// that file spells it, so comparing the two needs a set lookup and no mapping.
pub struct Route {
    pub operation: &'static str,
    pub method: &'static str,
    pub path: &'static str,
}

/// The operations M1 serves — a deliberate subset of the inventory, not all of it.
///
/// The rest of the inventory, the `/v0` lifeboat and version-skew handling are
/// the Ship milestone's. Nothing here stubs them: a route that answers with a
/// placeholder is worse than one that 404s, because a client cannot tell the
/// difference between not built and not working.
///
/// A path is checkable against its inventory key by eye. `list_jobs` and
/// `get_job` are the collection and the member and name no act; every other row
/// spells its key in the last segment, except `redispatch`, which drops `_job`
/// because no `redispatch_drone` exists to tell it apart from. The lifeboat's
/// `POST /v0/jobs/:id/kill` is the same act as `kill_job` under a frozen prefix
/// that shares nothing with this table, by design.
pub const SERVED: &[Route] = &[
    Route {
        operation: "list_jobs",
        method: "GET",
        path: "/jobs",
    },
    Route {
        operation: "get_job",
        method: "GET",
        path: "/jobs/:job_id",
    },
    Route {
        operation: "list_workflows",
        method: "GET",
        path: "/workflows",
    },
    Route {
        operation: "list_manifests",
        method: "GET",
        path: "/manifests",
    },
    Route {
        operation: "list_models",
        method: "GET",
        path: "/models",
    },
    Route {
        operation: "propose_job",
        method: "POST",
        path: "/jobs",
    },
    Route {
        operation: "approve_dispatch",
        method: "POST",
        path: "/jobs/:job_id/approve_dispatch",
    },
    Route {
        operation: "kill_drone",
        method: "POST",
        path: "/jobs/:job_id/kill_drone",
    },
    Route {
        operation: "kill_job",
        method: "POST",
        path: "/jobs/:job_id/kill_job",
    },
    Route {
        operation: "redispatch_job",
        method: "POST",
        path: "/jobs/:job_id/redispatch",
    },
    // A socket rather than a body: it opens with what has already happened and
    // then continues, which no request-response shape carries. The path drops
    // `_job` for the reason `redispatch` does — the segment before it already
    // names the Job.
    Route {
        operation: "observe_job",
        method: "GET",
        path: "/jobs/:job_id/observe",
    },
    // Every event kind is served on the one socket, and every one is named:
    // `SERVED` is what a rule compares to the inventory, so a kind published
    // and not listed here is a kind no rule can see.
    Route {
        operation: "job.created",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.state_changed",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.step_advanced",
        method: "GET",
        path: "/events",
    },
];

/// A request body that would not parse.
///
/// The code is declared beside the thing that raises it, which is what makes
/// the set closed by collection rather than by a registry somebody has to keep.
const UNDECODABLE_REQUEST: &str = "api.undecodable_request";

/// A response that would not serialise. Unreachable for plain data, and
/// answered rather than panicked: a panic here drops a socket mid-Job.
const UNENCODABLE_RESPONSE: &str = "api.unencodable_response";

/// Everything a handler needs. Cloned per request, so nothing here may be
/// expensive to clone.
pub struct Served<D> {
    daemon: Arc<D>,
    /// **This process's** run id, minted at start and never Fleet's-by-
    /// assumption. Every error the transport raises carries it.
    run_id: RunId,
    events: Broadcaster,
}

impl<D> Served<D> {
    /// The daemon this listener answers from, handed over.
    ///
    /// The transport is the only holder. For a caller that also has to *drive*
    /// the daemon — anything calling a turn on an interval — see
    /// [`Served::sharing`].
    pub fn by(daemon: D, run_id: RunId, events: Broadcaster) -> Served<D> {
        Served::sharing(Arc::new(daemon), run_id, events)
    }

    /// The same daemon the caller keeps a reference to.
    ///
    /// **This is what lets a Job advance.** Serving is one of two things a
    /// process does with a daemon and driving it is the other, so a
    /// constructor that consumed it left nothing in the process able to call
    /// `turn` — a Job dispatched on approval and then never settled. The state
    /// was already an `Arc` for cloning per request; this only stops that
    /// `Arc` being made where nobody else can reach it.
    ///
    /// No `Daemon` implementation for `Arc<D>` is needed for this and none is
    /// stated: the handlers reach the daemon through the state's own `Arc`, so
    /// `D` stays the concrete daemon and one indirection stays one.
    pub fn sharing(daemon: Arc<D>, run_id: RunId, events: Broadcaster) -> Served<D> {
        Served {
            daemon,
            run_id,
            events,
        }
    }

    /// The stream this listener publishes from, for whoever holds the daemon.
    pub fn events(&self) -> Broadcaster {
        self.events.clone()
    }

    /// The daemon, for a handler in another module of this crate.
    pub(crate) fn daemon(&self) -> &D {
        &self.daemon
    }
}

impl<D> Clone for Served<D> {
    fn clone(&self) -> Served<D> {
        Served {
            daemon: Arc::clone(&self.daemon),
            run_id: self.run_id.clone(),
            events: self.events.clone(),
        }
    }
}

/// The one listener. HTTP for queries and commands, an upgrade for events.
pub fn router<D: Daemon>(served: Served<D>) -> Router {
    Router::new()
        .route("/jobs", get(list_jobs::<D>).post(propose_job::<D>))
        .route("/workflows", get(list_workflows::<D>))
        .route("/manifests", get(list_manifests::<D>))
        .route("/models", get(list_models::<D>))
        .route("/jobs/:job_id", get(get_job::<D>))
        .route(
            "/jobs/:job_id/approve_dispatch",
            post(approve_dispatch::<D>),
        )
        .route("/jobs/:job_id/kill_drone", post(kill_drone::<D>))
        .route("/jobs/:job_id/kill_job", post(kill_job::<D>))
        .route("/jobs/:job_id/redispatch", post(redispatch_job::<D>))
        .route("/jobs/:job_id/observe", get(observe_job::<D>))
        .route("/events", get(events::<D>))
        // The Evidence endpoint, on the same listener and deliberately not in
        // `SERVED`: it is the Fleet/Drone seam rather than the Fleet/Bridge
        // one, and the inventory this table is checked against is Bridge's.
        .merge(crate::mcp::mounted::<D>())
        .with_state(served)
}

// ------------------------------------------------------------------ queries

async fn list_jobs<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_jobs().await {
        Ok(jobs) => answer(StatusCode::OK, &jobs, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// One Job in full. **The Board row plus what the list redacts** — the steps
/// and where each got to, the criteria, the branch, the brief.
async fn get_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.get_job(JobId::carried(job_id)).await {
        Ok(detail) => answer(StatusCode::OK, &detail, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The workflows a proposal may name. **The set Fleet will accept**, which is
/// why it is served rather than left to a caller to know.
async fn list_workflows<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_workflows().await {
        Ok(workflows) => answer(StatusCode::OK, &workflows, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

async fn list_manifests<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_manifests().await {
        Ok(manifests) => answer(StatusCode::OK, &manifests, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

async fn list_models<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_models().await {
        Ok(models) => answer(StatusCode::OK, &models, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

// ----------------------------------------------------------------- commands

async fn propose_job<D: Daemon>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let proposal: ProposeJob = match ipc::decode("proposed Job", &body) {
        Ok(proposal) => proposal,
        // 400 is the transport's own refusal and never the daemon's — the
        // bytes did not become a request, so nothing downstream was asked.
        Err(why) => {
            return problem(
                StatusCode::BAD_REQUEST,
                &WireError::raised(UNDECODABLE_REQUEST, why.to_string(), served.run_id.clone())
                    .caused_by(vec![why.to_string()]),
            )
        }
    };
    match served.daemon.propose_job(proposal).await {
        // 201: the Job now exists, at the approval gate. It is not running, and
        // nothing here approves it.
        Ok(job) => answer(StatusCode::CREATED, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

async fn approve_dispatch<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.approve_dispatch(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The process, not the unit of work. What comes back is the Job the Drone was
/// on, which is still there.
async fn kill_drone<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.kill_drone(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The unit of work, not the process. A separate operation from `kill_drone`
/// because two of the edges into `killed` leave a status no
/// Drone has ever existed under, and neither one can be spelled as killing a
/// Drone.
async fn kill_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.kill_job(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Mint a replacement for a stopped Job. **Two Jobs come back**, and
/// the answer is 200 rather than 201 because the act a caller asked for is the
/// recovery, not the creation — the new Job's id is in the body.
async fn redispatch_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.redispatch_job(JobId::carried(job_id)).await {
        Ok(both) => answer(StatusCode::OK, &both, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

// ------------------------------------------------------------- the upgrade

/// The event stream. **Global, and a client subscribes to nothing** — one
/// socket carries every Job, because Bridge holds exactly one connection.
///
/// Nothing is read from the socket. The stream is one-directional by design:
/// there is no subscribe message to read, and a connection that carried state
/// would be a connection that is expensive to drop and remake.
/// One Job's turns. **Per-Job, and not on `/events`** — that stream is one
/// drop-oldest channel carrying every Job, and a transcript row on it would
/// evict the state changes a Board is drawn from.
///
/// The daemon is asked **before** the upgrade, so a Job that does not exist is
/// a 404 the caller reads at the moment they asked rather than a socket that
/// opens and says nothing. What comes back already holds the subscription and
/// the history, in that order.
async fn observe_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon.observe_job(JobId::carried(job_id)).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| crate::observing::relay(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

async fn events<D: Daemon>(State(served): State<Served<D>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| watch(socket, served))
}

async fn watch<D: Daemon>(mut socket: WebSocket, served: Served<D>) {
    // Subscribe first, then snapshot. The other order drops whatever lands in
    // between; this order can only repeat, and a repeat is detectable.
    let mut subscription = served.events.subscribe();
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
    let cursor = served.events.cursor();
    let Ok(jobs) = served.daemon.list_jobs().await else {
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

// ------------------------------------------------------------------ answers

fn answer<T: Serialize>(status: StatusCode, value: &T, run_id: &RunId) -> Response {
    match ipc::encode(value) {
        Ok(body) => (status, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(why) => problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            &WireError::raised(UNENCODABLE_RESPONSE, why.to_string(), run_id.clone())
                .caused_by(vec![why.to_string()]),
        ),
    }
}

fn refused(refusal: Refusal) -> Response {
    let status =
        StatusCode::from_u16(refusal.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    problem(status, refusal.error())
}

fn problem(status: StatusCode, error: &WireError) -> Response {
    match ipc::encode(error) {
        Ok(body) => (status, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        // The error would not serialise, which leaves nothing true to say with
        // a body. The status is still the answer.
        Err(_) => status.into_response(),
    }
}
