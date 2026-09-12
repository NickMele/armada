//! The agent's door, over the router, with no agent and no socket.
//!
//! What these prove is the one thing `ipc`'s own cases cannot: that a tool
//! call reaches the same handler a Bridge request reaches, and comes back with
//! what that handler answered.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::{running, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{offered, router, Broadcaster, Served, DOOR_PATH, SERVED};

fn wired(daemon: FakeDaemon) -> Router {
    let events = Broadcaster::new();
    router(Served::by(daemon, run_id(), events))
}

/// A Fleet holding one running Job, which every case below reaches by handle,
/// by number or by id.
fn holding_one() -> FakeDaemon {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB");
    daemon
}

async fn call(app: &Router, body: &str) -> (StatusCode, String) {
    method(app, "POST", body).await
}

async fn method(app: &Router, method: &str, body: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

/// **The tool set is the inventory's, not this crate's.** A hand-kept list is
/// how a row gains `agent_access = "Yes"` and stays unreachable.
#[test]
fn every_tool_offered_is_an_operation_the_route_table_serves() {
    let unserved: Vec<&str> = offered()
        .iter()
        .filter(|shape| shape.path.is_empty())
        .map(|shape| shape.operation)
        .collect();
    assert!(
        unserved.is_empty(),
        "these operations are reachable by an agent and served at no route: {unserved:?}. \
         The gate rule in `xtask` names the same set"
    );
    assert!(
        offered()
            .iter()
            .all(|shape| SERVED.iter().any(|route| route.path == shape.path)),
        "a tool names a path the route table does not"
    );
}

#[tokio::test]
async fn the_tool_list_is_every_reachable_operation_and_names_its_route() {
    let app = wired(holding_one());
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for expected in ["list_jobs", "get_job", "get_events_since", "start_run"] {
        assert!(
            body.contains(&format!("\"name\":\"{expected}\"")),
            "the list is missing {expected}"
        );
    }
    assert!(
        !body.contains("\"name\":\"observe_job\""),
        "a Bridge-only operation reached the agent's tool set"
    );
    assert!(
        body.contains("GET /jobs/:job_id"),
        "a tool must name its route"
    );
}

/// The point of the whole door: a tool call and a Bridge request reach one
/// handler and come back with one answer.
#[tokio::test]
async fn a_tool_call_is_answered_by_the_route_that_serves_the_operation() {
    let app = wired(holding_one());
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"get_job","arguments":{"job_id":"1-a-job"}}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("1-a-job"), "{body}");
    assert!(body.contains("\"isError\":false"), "{body}");
}

/// **A handle, not only a ULID.** The `:job_id` extractor resolves all three
/// forms, and an agent typing what a person types must reach the same Job.
#[tokio::test]
async fn a_number_reaches_the_job_a_person_would_have_meant() {
    let app = wired(holding_one());
    let (_, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"get_job","arguments":{"job_id":"1"}}}"#,
    )
    .await;
    assert!(body.contains("1-a-job"), "{body}");
}

/// A 404 from the surface is a tool error the model reads, never a transport
/// failure it can only retry.
#[tokio::test]
async fn a_refusal_from_the_surface_reaches_the_agent_as_something_to_read() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"get_job","arguments":{"job_id":"99"}}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"isError\":true"), "{body}");
    assert!(body.contains("no_such_job"), "{body}");
}

/// A command goes through as a POST and its body decodes.
///
/// **The refusal is the proof.** `fake.no_drone_to_redirect` is the daemon
/// answering about a Job it holds, which it could only reach with a decoded
/// `Redirection` — an undecodable body is a different refusal one layer out.
#[tokio::test]
async fn a_command_reaches_the_post_route_with_its_body() {
    let app = wired(holding_one());
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{"name":"redirect_drone","arguments":{"job_id":"1",
            "body":{"instruction":"try the other suite"}}}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("no_drone_to_redirect"), "{body}");
    assert!(
        !body.contains("undecodable"),
        "the body did not decode: {body}"
    );
}

/// The handshake says which Manifest the session is inside and what stands in
/// for the stream it does not get.
#[tokio::test]
async fn the_handshake_states_the_scope_and_the_polling_obligation() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let (_, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":6,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
    )
    .await;
    assert!(body.contains("armada-fleet"), "{body}");
    assert!(body.contains("01MF"), "the scope is named: {body}");
    assert!(body.contains("get_events_since"), "{body}");
}

/// No server-initiated stream and no session to end, which is what keeps this
/// door out of the unbounded-sink risk on the event socket.
#[tokio::test]
async fn a_get_or_a_delete_is_refused_the_way_the_drones_endpoint_refuses_one() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    for verb in ["GET", "DELETE"] {
        let (status, _) = method(&app, verb, "").await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{verb}");
    }
}

/// The Drone's endpoint keeps its own tools, and this door does not widen it.
#[tokio::test]
async fn the_drones_endpoint_is_untouched_by_the_door_beside_it() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let request = Request::builder()
        .method("POST")
        .uri(crate::MCP_PATH)
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#.to_string(),
        ))
        .expect("a well-formed request");
    let response = app.oneshot(request).await.expect("the router answers");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    let body = String::from_utf8_lossy(&body).to_string();
    assert!(
        !body.contains("\"name\":\"get_job\""),
        "the Drone's endpoint gained a fleet-control tool: {body}"
    );
}
