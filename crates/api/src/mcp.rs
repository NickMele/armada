//! The Drone endpoint: one path, the Drone's tools, on the listener that was
//! already there.
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
use std::net::SocketAddr;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
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

/// Where a Drone's call came from: the address the listener saw on the other
/// end of the connection it arrived on.
///
/// **Minted by the transport and by nothing else.** There is no constructor
/// taking a Job id and no field a caller could put one in, which is what makes
/// "a Drone cannot choose which Job its evidence lands against" a property of
/// the type rather than of a check somebody remembered to write. What the
/// daemon does with it — matching the port pair against the processes it
/// spawned — is `fleet::peer`'s, and this crate holds none of that.
///
/// `None` for the port is a request that arrived with no connection information
/// on it, which is a test's router rather than a served one. It attributes to
/// nothing, and a Drone tool call is refused rather than guessed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller(Option<SocketAddr>);

impl Caller {
    /// The peer of an accepted connection.
    pub fn at(peer: SocketAddr) -> Caller {
        Caller(Some(peer))
    }

    /// A call that arrived with no peer address. See the type.
    pub fn unplaceable() -> Caller {
        Caller(None)
    }

    /// The port the peer opened the connection from. **Half of the pair** —
    /// the other half is the port Fleet is listening on, and the daemon knows
    /// that for itself.
    pub fn port(&self) -> Option<u16> {
        self.0.map(|peer| peer.port())
    }
}

pub fn mounted<D: Daemon>() -> Router<Served<D>> {
    Router::new().route(MCP_PATH, post(called::<D>).get(no_stream).delete(no_stream))
}

/// The peer of the connection this request arrived on.
///
/// **Read out of the extensions rather than extracted**, so a router served
/// without `into_make_service_with_connect_info` answers
/// [`Caller::unplaceable`] instead of rejecting the request. A rejection would
/// be a 500 where the honest answer is that nothing said who called.
fn who_called(parts: &axum::http::request::Parts) -> Caller {
    match parts.extensions.get::<ConnectInfo<SocketAddr>>() {
        Some(ConnectInfo(peer)) => Caller::at(*peer),
        None => Caller::unplaceable(),
    }
}

/// One JSON-RPC message in, at most one out.
///
/// A notification is acknowledged with 202 and no body, because JSON-RPC
/// forbids answering one and an empty 200 would be a response with no id that
/// a client has to guess about.
async fn called<D: Daemon>(
    State(served): State<Served<D>>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let caller = who_called(&parts);
    let body = match axum::body::to_bytes(body, MOST_A_CALL_MAY_BE).await {
        Ok(body) => body,
        // A body larger than any tool call, or one that stopped arriving. The
        // same shape every other unreadable call gets: a tool error the Drone
        // reads, never a status code it can only retry.
        Err(_) => Bytes::new(),
    };
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
            match served.daemon().submit_evidence(caller, submission).await {
                Ok(receipt) => Answered::Recorded { id, receipt },
                Err(why) => Answered::Refused { id, why },
            }
        }
        Incoming::Declare { id, declaration } => {
            match served.daemon().declare_scope(caller, declaration).await {
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
        Incoming::RunChecks { id } => match served.daemon().run_checks(caller).await {
            Ok(report) => Answered::Checked { id, report },
            Err(why) => Answered::Refused { id, why },
        },
        // **The one arm whose success is another Job.** It goes through the
        // daemon like the rest and is refused the same way — a tool error the
        // Drone reads — because a Drone told "no" by a status code has nothing
        // to correct.
        Incoming::Dispatch { id, dispatch } => {
            match served.daemon().dispatch_job(caller, dispatch).await {
                Ok(receipt) => Answered::Recorded { id, receipt },
                Err(why) => Answered::Refused { id, why },
            }
        }
        // **Answered immediately, and the answer is not here.** The receipt
        // says Fleet took the question; what a person chose arrives in the
        // Drone's session as a turn, however much later. Holding this open
        // would put a person's thinking time on an HTTP connection and would
        // swallow every redirect sent to unstick the Drone while it waited —
        // an injected turn is consumed when the current tool call returns.
        //
        // So it adds nothing to the unbounded-sink risk either: one reply on
        // the Drone's own connection, with no queue behind it.
        Incoming::Ask { id, asking } => match served.daemon().ask_question(caller, asking).await {
            Ok(receipt) => Answered::Recorded { id, receipt },
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

/// The most a tool call may weigh.
///
/// Three prose fields and a path list; a megabyte is far more than any of them
/// and far less than a stream. It exists because reading the body by hand —
/// which is what taking the connection's peer costs — means saying what the
/// extractor said for itself.
const MOST_A_CALL_MAY_BE: usize = 1024 * 1024;

/// There is no server-initiated stream and no session to end.
async fn no_stream() -> Response {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}
