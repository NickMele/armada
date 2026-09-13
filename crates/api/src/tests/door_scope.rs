//! The agent door scoped to the Manifest its session stands in: every list,
//! every Job, and every argument naming a Manifest. `#987`.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::door::Reachable;
use tower::ServiceExt;

use crate::tests::fake::{owned_by, running, FakeDaemon};
use crate::tests::shapes::{run_id, A_PROPOSAL, THE_MANIFEST};
use crate::{door_within, router, Broadcaster, Served};

fn wired(daemon: FakeDaemon) -> Router {
    router(Served::by(daemon, run_id(), Broadcaster::new()))
}

/// A Job the session's Manifest owns, and one another Manifest owns.
fn holding_two_owners() -> FakeDaemon {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB");
    owned_by(&daemon, "01OTHERJOB", "2-another-repository", "01OTHER");
    daemon
}

async fn read(response: axum::response::Response) -> (StatusCode, String) {
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

async fn standing_in(app: &Router, manifest_id: &str, body: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri(door_within(manifest_id))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    read(app.clone().oneshot(request).await.expect("an answer"))
        .await
        .1
}

/// The port a planted Helm session calls from: `propose_job` is offered to
/// nobody else.
const HELM: u16 = 52000;

fn acting(_: &Reachable) -> bool {
    true
}

fn helm_holding_two_owners() -> Router {
    let daemon = holding_two_owners();
    *daemon.helm_on.lock().expect("not poisoned") = Some((HELM, acting));
    router(Served::sharing(
        Arc::new(daemon),
        run_id(),
        Broadcaster::new(),
    ))
}

async fn helm_standing_in(app: &Router, manifest_id: &str, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{HELM}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(door_within(manifest_id))
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    read(app.clone().oneshot(request).await.expect("an answer"))
        .await
        .1
}

/// A read Bridge makes, beside the door rather than through it.
async fn bridge_get(app: &Router, uri: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("a well-formed request");
    read(app.clone().oneshot(request).await.expect("an answer")).await
}

fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

/// **The door names its scope on every Job list**, so a session is listed only
/// the Jobs the Manifest it stands in owns, and Bridge's unnamed read is all.
#[tokio::test]
async fn every_job_list_through_the_door_is_the_sessions_manifests_alone() {
    let app = wired(holding_two_owners());
    for tool in [
        "list_jobs",
        "get_activity_feed",
        "list_job_board",
        "list_reviews",
    ] {
        let body = standing_in(&app, THE_MANIFEST, &calling(tool, "{}")).await;
        assert!(!body.contains("2-another-repository"), "{tool}: {body}");
    }
    let listed = standing_in(&app, THE_MANIFEST, &calling("list_jobs", "{}")).await;
    assert!(listed.contains("1-a-job"), "{listed}");
    let (_, every) = bridge_get(&app, "/jobs").await;
    assert!(every.contains("2-another-repository"), "{every}");
}

/// The fake's one Drone works a Job the session's Manifest does not own.
#[tokio::test]
async fn the_drone_list_through_the_door_is_the_sessions_manifests_alone() {
    let app = wired(holding_two_owners());
    let listed = standing_in(&app, THE_MANIFEST, &calling("list_drones", "{}")).await;
    assert!(!listed.contains("01JOB1"), "{listed}");
    let (_, every) = bridge_get(&app, "/drones").await;
    assert!(every.contains("01JOB1"), "{every}");
}

/// A Job another Manifest owns is refused through the door, and still answers
/// on the route Bridge reads.
#[tokio::test]
async fn a_job_another_manifest_owns_is_refused_through_the_door_only() {
    let app = wired(holding_two_owners());
    let body = standing_in(
        &app,
        THE_MANIFEST,
        &calling("get_job", r#"{"job_id":"01OTHERJOB"}"#),
    )
    .await;
    assert!(body.contains("\"isError\":true"), "{body}");
    let (status, _) = bridge_get(&app, "/jobs/01OTHERJOB").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_session_naming_a_manifest_this_fleet_does_not_serve_is_refused_plainly() {
    let app = wired(holding_two_owners());
    let handshake = standing_in(
        &app,
        "01NOTSERVED",
        r#"{"jsonrpc":"2.0","id":6,"method":"initialize","params":{}}"#,
    )
    .await;
    assert!(
        handshake.contains("serves no Manifest `01NOTSERVED`"),
        "{handshake}"
    );
    let called = standing_in(&app, "01NOTSERVED", &calling("list_jobs", "{}")).await;
    assert!(called.contains("\"isError\":true"), "{called}");
    assert!(!called.contains("1-a-job"), "{called}");
}

/// **Refused, never a second `manifest_id=` beside the scope's.**
#[tokio::test]
async fn a_call_naming_another_manifest_is_refused_and_its_own_is_answered() {
    let app = wired(holding_two_owners());
    let other = standing_in(
        &app,
        THE_MANIFEST,
        &calling("get_manifest_reading", r#"{"manifest_id":"01OTHER"}"#),
    )
    .await;
    assert!(other.contains("\"isError\":true"), "{other}");
    assert!(other.contains("named `01OTHER`"), "{other}");
    let own = standing_in(
        &app,
        THE_MANIFEST,
        &calling(
            "get_manifest_reading",
            &format!(r#"{{"manifest_id":"{THE_MANIFEST}"}}"#),
        ),
    )
    .await;
    assert!(!own.contains("named `"), "{own}");
    let segment = standing_in(
        &app,
        THE_MANIFEST,
        &calling("get_manifest", r#"{"manifest_id":"01OTHER"}"#),
    )
    .await;
    assert!(segment.contains("named `01OTHER`"), "{segment}");
}

#[tokio::test]
async fn a_proposal_naming_another_owner_is_refused_through_the_door() {
    let app = helm_holding_two_owners();
    let elsewhere = A_PROPOSAL.replace(THE_MANIFEST, "01OTHER");
    let refused = helm_standing_in(
        &app,
        THE_MANIFEST,
        &calling("propose_job", &format!(r#"{{"body":{elsewhere}}}"#)),
    )
    .await;
    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(refused.contains("names `01OTHER`"), "{refused}");
}
