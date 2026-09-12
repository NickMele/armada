//! The agent's door: a second MCP path on the listener already bound.
//!
//! **It calls the HTTP surface rather than the daemon.** Every tool is one row
//! of `crates/ipc/operations.toml` reached at the route [`SERVED`] already
//! names, so there is no second implementation to drift and a route that moves
//! moves for both callers at once.
//!
//! It shares the listener for [`crate::mcp`]'s reason, and answers 405 to
//! `GET` and `DELETE` for it too: no server-initiated stream, no session to
//! end, and so nothing added to the unbounded-sink risk on the event socket.
//!
//! Who may open this door is `#698`. What it is scoped to is here.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use ipc::door::{self, Answer, Answered, Asked, Shape};
use tower::ServiceExt;

use crate::daemon::Queries;
use crate::routes::SERVED;
use crate::served::Served;

/// Where an agent reaches Fleet.
///
/// **Beside [`crate::mcp::MCP_PATH`], not instead of it.** That one is the
/// Drone's endpoint, whose caller is a process Fleet spawned and whose eight
/// tools are in no inventory; this one is the HTTP surface.
pub const DOOR_PATH: &str = "/agent/mcp";

/// The most a tool call's own message may weigh. [`crate::mcp`]'s constant and
/// its reason: a few fields, never a stream. Stated rather than left to the
/// extractor's default, so the two endpoints refuse at the same size.
const MOST_A_CALL_MAY_BE: usize = 1024 * 1024;

/// The query parameters a route reads, where it reads any.
///
/// **Two rows, and they are the route table's own fact.** A path segment is
/// visible in the path and a query is not, so this is the one thing about a
/// route the door cannot read off [`SERVED`].
const QUERIES: &[(&str, &[&str])] = &[
    ("search_files", &["q"]),
    ("get_events_since", &["since"]),
];

/// Every tool this door offers: the inventory's `agent_access` column, joined
/// to the route table.
///
/// **The set is the file's and the route is this crate's.** An operation the
/// inventory marks reachable and nothing serves gets an empty path and is
/// listed as not served — which the gate refuses, and which is answered rather
/// than silently dropped if it ever happens.
pub fn offered() -> Vec<Shape> {
    door::REACHABLE
        .iter()
        .map(|row| {
            let route = SERVED
                .iter()
                .find(|route| route.operation == row.operation);
            Shape {
                operation: row.operation,
                method: route.map(|route| route.method).unwrap_or("GET"),
                path: route.map(|route| route.path).unwrap_or(""),
                query: QUERIES
                    .iter()
                    .find(|(operation, _)| *operation == row.operation)
                    .map(|(_, names)| *names)
                    .unwrap_or(&[]),
            }
        })
        .collect()
}

/// The Manifest a session is answered inside.
///
/// **The mechanism, not the identity.** One scope is reachable today — the
/// Manifest this Fleet serves — and every call is checked against it, so when
/// `#698` gives a session a chosen Manifest the check is already the thing
/// that enforces it rather than something to remember to add.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope(String);

impl Scope {
    pub fn of(manifest_id: impl Into<String>) -> Scope {
        Scope(manifest_id.into())
    }

    pub fn named(&self) -> &str {
        &self.0
    }
}

/// What a door handler holds: the daemon, for the scope, and the surface every
/// tool call is made against.
///
/// **The surface is a second `Router` over the same state**, not the one this
/// is merged into — a router holding itself is not a value that exists. It is
/// the same table built twice, which costs a clone of a route list at start
/// and nothing afterwards.
pub struct Doorway<D> {
    served: Served<D>,
    surface: Router,
}

/// Cloned per request like every other state on this listener. **Written out
/// rather than derived**, because `#[derive(Clone)]` would demand `D: Clone`
/// and the daemon is held by `Arc` precisely so it is not.
impl<D> Clone for Doorway<D> {
    fn clone(&self) -> Doorway<D> {
        Doorway {
            served: self.served.clone(),
            surface: self.surface.clone(),
        }
    }
}

pub fn mounted<D: Queries>(served: Served<D>, surface: Router) -> Router {
    Router::new()
        .route(
            DOOR_PATH,
            post(called::<D>).get(no_stream).delete(no_stream),
        )
        .layer(axum::extract::DefaultBodyLimit::max(MOST_A_CALL_MAY_BE))
        .with_state(Doorway { served, surface })
}

/// One JSON-RPC message in, at most one out.
///
/// A notification is acknowledged with 202 and no body, for
/// [`crate::mcp`]'s reason: JSON-RPC forbids answering one.
async fn called<D: Queries>(State(doorway): State<Doorway<D>>, body: Bytes) -> Response {
    let shapes = offered();
    let answered = match door::read(&body, &shapes) {
        Asked::Nothing => return StatusCode::ACCEPTED.into_response(),
        Asked::Ping { id } => Answered::Ping { id },
        Asked::Tools { id } => Answered::Tools { id, shapes },
        Asked::NoSuchMethod { id, named } => Answered::NoSuchMethod { id, named },
        Asked::Unreadable { why } => Answered::Unreadable { why },
        // Refused as a tool error and never a status code, for the reason the
        // Drone's endpoint refuses that way: an agent reads the one and can
        // only retry the other.
        Asked::NotACall { id, why } => Answered::Refused { id, why },
        Asked::Handshake { id, revision } => match doorway.scope().await {
            Ok(scope) => Answered::Handshake {
                id,
                revision,
                scope: scope.named().to_string(),
            },
            Err(why) => Answered::Refused { id, why },
        },
        Asked::Call { id, call } => match doorway.scope().await {
            Err(why) => Answered::Refused { id, why },
            Ok(_) => Answered::Served {
                id,
                answer: doorway.through(&call).await,
            },
        },
    };
    match door::answer(answered) {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        // Unreachable for plain data, and answered rather than panicked: a
        // panic here drops the connection an agent's whole session runs on.
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

impl<D: Queries> Doorway<D> {
    /// The Manifest this session is answered inside.
    ///
    /// **Asked per call rather than held.** A session that outlived a
    /// Manifest reload would be answering inside one Fleet no longer serves,
    /// and the read is a field on a struct.
    async fn scope(&self) -> Result<Scope, String> {
        match self.served.daemon().scope().await {
            Ok(manifest_id) => Ok(Scope::of(manifest_id.as_str())),
            Err(refusal) => Err(format!(
                "this Fleet could not say which Manifest your session is inside, so nothing \
                 can be answered within one: {}",
                refusal.error().message
            )),
        }
    }

    /// One tool call, made against the surface.
    async fn through(&self, call: &door::Call) -> Answer {
        let request = Request::builder()
            .method(call.method)
            .uri(&call.path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(call.body.clone().unwrap_or_default()));
        let Ok(request) = request else {
            return unanswerable(call, "that call did not make a request");
        };
        // Infallible: the surface's error type is `Infallible`, so a failure
        // here is a request that was never made rather than a route that
        // refused — and a refusal comes back as a status like any other.
        let response = match self.surface.clone().oneshot(request).await {
            Ok(response) => response,
            Err(_) => return unanswerable(call, "the surface did not answer"),
        };
        let status = response.status().as_u16();
        let media_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json")
            .to_string();
        let body = axum::body::to_bytes(response.into_body(), MOST_A_RESPONSE_MAY_BE)
            .await
            .map(|bytes| bytes.to_vec())
            .unwrap_or_default();
        Answer {
            status,
            media_type,
            body,
            whole_at: call.path.clone(),
        }
    }
}

/// The most a surface answer may weigh before the door stops reading it.
///
/// **Far above the cap the answer is cut to**, so what an agent is told is
/// *cut at 65536 of 900000* rather than a body that silently ended. A diff
/// larger than this is one no session was going to hold either way.
const MOST_A_RESPONSE_MAY_BE: usize = 16 * 1024 * 1024;

/// A call the transport could not make or could not read back. **A 500 in the
/// answer, never a dropped connection**, so the agent reads a sentence.
fn unanswerable(call: &door::Call, why: &str) -> Answer {
    Answer {
        status: 500,
        media_type: "text/plain".to_string(),
        body: format!("{why}: {} {}", call.method, call.path).into_bytes(),
        whole_at: call.path.clone(),
    }
}

/// There is no server-initiated stream and no session to end.
async fn no_stream() -> Response {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}
