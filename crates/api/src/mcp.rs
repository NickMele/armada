//! The Drone endpoint: one path, three tools, on the listener that was already
//! there.
//!
//! # Why this is not in the route table
//!
//! [`SERVED`](crate::SERVED) is the Fleet/Bridge inventory, checked against
//! `crates/ipc/operations.toml` by the gate. **This is a different seam** — the
//! peer is a Drone that Fleet itself spawned, the vocabulary is MCP's, and the
//! version negotiated is the MCP revision. A row here would claim Bridge can
//! call it, and the rule comparing the two files would need an exception.
//!
//! It shares the listener because a Drone reaches Fleet at the same loopback
//! port Bridge does, and a second port is a second thing to publish and hold.
//!
//! # What it deliberately does not serve
//!
//! No server-initiated stream, no session, no batching. `GET` and `DELETE`
//! answer 405, which is what the streamable HTTP transport says a server
//! without either answers — and it means **this endpoint adds nothing to the
//! unbounded-sink risk on the event socket**: every message is a reply to a
//! request, on the Drone's own connection, with no queue behind it.
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use ipc::mcp::{self, Answered, Incoming};

use crate::daemon::Daemon;
use crate::routes::Served;

/// Where the Evidence tool is served.
///
/// **The composition root builds a Drone's MCP configuration from this
/// constant.** The route table being hand-written is an accepted cost paid
/// against a gate rule that reads `SERVED`; this path is in no such table, so
/// the thing standing between a typo and a Drone that can never report is that
/// the address written into `mcp.json` and the address routed here are one
/// value.
pub const MCP_PATH: &str = "/mcp";

pub fn mounted<D: Daemon>() -> Router<Served<D>> {
    Router::new().route(MCP_PATH, post(called::<D>).get(no_stream).delete(no_stream))
}

/// One JSON-RPC message in, at most one out.
///
/// A notification is acknowledged with 202 and no body, because JSON-RPC
/// forbids answering one and an empty 200 would be a response with no id that
/// a client has to guess about.
async fn called<D: Daemon>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let answered = match mcp::read(&body) {
        Incoming::Nothing => return StatusCode::ACCEPTED.into_response(),
        Incoming::Handshake { id, revision } => Answered::Handshake { id, revision },
        Incoming::Ping { id } => Answered::Ping { id },
        Incoming::Tools { id } => Answered::Tools { id },
        Incoming::NoSuchMethod { id, named } => Answered::NoSuchMethod { id, named },
        Incoming::Unreadable { why } => Answered::Unreadable { why },
        // Refused before the daemon is reached, and refused the same way the
        // daemon refuses: as a tool error the Drone reads, never a 4xx it can
        // only retry.
        Incoming::NotASubmission { id, why } => Answered::Refused {
            id,
            why: ipc::mcp::NotRecorded {
                because: why.to_string(),
            },
        },
        Incoming::Submit { id, submission } => {
            match served.daemon().submit_evidence(submission).await {
                Ok(receipt) => Answered::Recorded { id, receipt },
                Err(why) => Answered::Refused { id, why },
            }
        }
        Incoming::Declare { id, declaration } => {
            match served.daemon().declare_scope(declaration).await {
                Ok(receipt) => Answered::Recorded { id, receipt },
                Err(why) => Answered::Refused { id, why },
            }
        }
        // **The one call that is held open while work happens.** Every other
        // arm here answers from a value the daemon already has; this one runs
        // the step's Checks and comes back with what they printed, which is
        // minutes rather than milliseconds. It adds nothing to the
        // unbounded-sink risk this module's comment names — it is still one
        // reply on the Drone's own connection — and what bounds the cost is
        // `Daemon::run_checks`'s, not the transport's.
        Incoming::RunChecks { id } => match served.daemon().run_checks().await {
            Ok(report) => Answered::Checked { id, report },
            Err(why) => Answered::Refused { id, why },
        },
    };
    match mcp::answer(answered) {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        // Unreachable for plain data, and answered rather than panicked: a
        // panic here drops the connection a Job's only report travels on.
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// There is no server-initiated stream and no session to end.
async fn no_stream() -> Response {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}
