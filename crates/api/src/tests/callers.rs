//! Who the listener answers, over the router, with no socket.
//!
//! **Both halves, from one place.** A page reaches an HTTP route and a
//! WebSocket upgrade the same way, so the refusal is asserted on both here
//! rather than beside each handler — and the happy path is asserted beside it,
//! because a Fleet that refuses Bridge is worse than the defect this closes.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, DOOR_PATH, MCP_PATH};

fn wired() -> Router {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    router(Served::by(daemon, run_id(), events))
}

/// A page's own address, as a browser would send it. Any value refuses; this
/// one is here so the assertion on the field has something to read.
const A_PAGE: &str = "http://evil.invalid";

struct Answered {
    status: StatusCode,
    body: String,
}

async fn answered(app: &Router, request: Request<Body>) -> Answered {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body that reads")
        .to_bytes()
        .to_vec();
    Answered {
        status,
        body: String::from_utf8_lossy(&body).to_string(),
    }
}

/// A plain request, with the `Origin` a browser puts on it or without.
fn asked(method: &str, path: &str, from_a_page: bool) -> Request<Body> {
    let request = Request::builder().method(method).uri(path);
    let request = if from_a_page {
        request.header("origin", A_PAGE)
    } else {
        request
    };
    request
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("a well-formed request")
}

/// The handshake a browser sends to open a socket. It is an ordinary HTTP
/// request until the upgrade, which is the whole reason one layer holds both.
fn upgrading(path: &str, from_a_page: bool) -> Request<Body> {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==");
    let request = if from_a_page {
        request.header("origin", A_PAGE)
    } else {
        request
    };
    request.body(Body::empty()).expect("a well-formed upgrade")
}

/// **The read half is the wider one.** A WebSocket upgrade is not subject to
/// the same-origin policy, so a page that opened `/events` read every Job's
/// state, every turn and every Helm conversation. The socket never opens now.
#[tokio::test]
async fn a_page_cannot_open_the_event_stream() {
    let refused = answered(&wired(), upgrading("/events", true)).await;

    assert_eq!(refused.status, StatusCode::FORBIDDEN);
    assert!(
        refused.body.contains("api.from_a_page"),
        "a refusal a developer can act on names its code: {}",
        refused.body
    );
    assert!(
        refused.body.contains(A_PAGE),
        "and names the page, which is what says which tab: {}",
        refused.body
    );
}

/// The write half. A body a browser classes as simple is delivered without the
/// server being asked first, and the response staying unreadable does not
/// un-happen the effect.
#[tokio::test]
async fn a_page_cannot_deliver_a_command() {
    let refused = answered(&wired(), asked("POST", "/jobs", true)).await;

    assert_eq!(refused.status, StatusCode::FORBIDDEN);
    assert!(refused.body.contains("api.from_a_page"), "{}", refused.body);
}

/// Every path on the listener, not only the ones in the inventory. The Drone's
/// endpoint and the agent's door are on this port too, and a layer that held
/// only the Bridge surface would leave both open.
#[tokio::test]
async fn a_page_is_refused_on_every_path_this_listener_carries() {
    let app = wired();
    let paths = [
        ("GET", "/health"),
        ("GET", "/jobs"),
        ("POST", "/jobs/01JOB/kill_job"),
        ("POST", MCP_PATH),
        ("POST", DOOR_PATH),
    ];

    for (method, path) in paths {
        let refused = answered(&app, asked(method, path, true)).await;
        assert_eq!(
            refused.status,
            StatusCode::FORBIDDEN,
            "{method} {path} answered a page"
        );
    }

    let watching = answered(&app, upgrading("/jobs/01JOB/observe", true)).await;
    assert_eq!(watching.status, StatusCode::FORBIDDEN, "one Job's turns");
}

/// **A Fleet that refuses Bridge is worse than the defect.** Nothing that
/// reaches Fleet on purpose sends an `Origin`: Bridge's main process sets a
/// content type and nothing else, the `armada` CLI writes its own request head
/// by hand, and the door builds a request rather than forwarding one.
#[tokio::test]
async fn a_caller_that_sends_no_origin_is_answered_as_before() {
    let app = wired();

    let health = answered(&app, asked("GET", "/health", false)).await;
    assert_eq!(health.status, StatusCode::OK, "{}", health.body);

    // A real upgrade over a real connection, because the layer sits in front
    // of one: `oneshot` has no connection to hand over, so it could only ever
    // prove the refusal did not happen.
    let mut socket = crate::tests::connected(app, "/events", 8192).await;
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the stream answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the stream is text: {frame:?}");
    };
    assert!(
        json.contains("resync"),
        "the event stream still opens with current state: {json}"
    );
}

/// The door re-dispatches every tool call into the same surface. It carries no
/// header from the call that reached it, so a session that is not a browser
/// stays one all the way through.
#[tokio::test]
async fn an_agent_still_reaches_the_surface_through_the_door() {
    let handshake = r#"{"jsonrpc":"2.0","id":0,"method":"initialize",
        "params":{"protocolVersion":"2025-06-18","capabilities":{},
        "clientInfo":{"name":"a-client","version":"1"}}}"#;
    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(handshake))
        .expect("a well-formed request");

    let opened = answered(&wired(), request).await;

    assert_eq!(opened.status, StatusCode::OK, "{}", opened.body);
    assert!(
        !opened.body.contains("api.from_a_page"),
        "a relay is not a browser: {}",
        opened.body
    );
}
