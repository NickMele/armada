//! The agent door in front of a real Fleet, with a Helm session placed by its
//! connection: what it is listed, what it is refused by name, and a Job it
//! drafts landing at the approval gate. `#941`.
//!
//! **The placement is planted, and the rule is not.** Which process holds a
//! socket is `crate::tests::peer`'s, against the kernel; here `fleet::helm::may`
//! and the door meet over the real router.

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::RunId;
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::helm::{Carry, Carrying, Heard, Hosting};
use crate::tests::daemon::fittings;
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

/// The pid a stand-in host says its session runs as, and the port that
/// session's relay calls from.
const SESSION: u32 = 4242;
const RELAY: u16 = 52000;
/// A port a person's own agent session calls from.
const SOMEONE: u16 = 52001;

/// A host whose one session is running, and never answers.
struct Running;

impl Hosting for Running {
    fn carry<'a>(&'a self, _carry: Carry, _heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(std::future::pending()) as Pin<Box<dyn Future<Output = _> + Send>>
    }

    fn running(&self) -> Vec<u32> {
        vec![SESSION]
    }
}

const A_PROPOSAL: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "fixture-workflow",
    "owner_manifest_id": "01FIXTUREMANIFEST",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false,
    "model": "a-model",
    "acceptance_criteria": [{"text": "the symptom is gone", "source": "check"}]
}"#;

/// A Fleet hosting one Helm session whose relay holds [`RELAY`], behind the router.
fn served(home: &TempDir) -> Router {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    let peers = Placing::nothing();
    peers.holding(SESSION + 1, RELAY, fittings.host.port);
    peers.started(SESSION, SESSION + 1);
    peers.holding(9999, SOMEONE, fittings.host.port);
    fittings.peers = peers;
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_helm_on(Arc::new(Running)));
    let events = fleet.events();
    api::router(api::Served::sharing(fleet, RunId::carried("01RUN"), events))
}

async fn from(app: &Router, port: u16, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(api::DOOR_PATH)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    String::from_utf8_lossy(&body).to_string()
}

fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

#[tokio::test]
async fn a_helm_session_is_listed_its_acts_and_refused_an_undo_by_name() {
    let home = TempDir::new();
    let app = served(&home);

    let listed = from(&app, RELAY, LISTING).await;
    assert!(listed.contains("\"name\":\"redirect_drone\""), "{listed}");
    assert!(listed.contains("\"name\":\"propose_job\""), "{listed}");
    assert!(!listed.contains("\"name\":\"undo_run\""), "{listed}");

    let undo = from(
        &app,
        RELAY,
        &calling("undo_run", r#"{"job_id":"1","run":"a-run"}"#),
    )
    .await;
    assert!(
        undo.contains("`undo_run` is not Helm's to call: that act is left to the person"),
        "{undo}"
    );
}

#[tokio::test]
async fn a_job_a_helm_session_drafts_reaches_the_approval_gate() {
    let home = TempDir::new();
    let app = served(&home);
    let drafted = from(
        &app,
        RELAY,
        &calling("propose_job", &format!(r#"{{"body":{A_PROPOSAL}}}"#)),
    )
    .await;
    assert!(drafted.contains("\"isError\":false"), "{drafted}");
    assert!(drafted.contains("awaiting_approval"), "{drafted}");
}

/// **Only the session is narrowed.** A person's own agent session keeps what
/// it had, and cannot draft by calling from beside a Helm session.
#[tokio::test]
async fn any_other_caller_is_answered_as_it_was() {
    let home = TempDir::new();
    let app = served(&home);
    let listed = from(&app, SOMEONE, LISTING).await;
    assert!(listed.contains("\"name\":\"undo_run\""), "{listed}");
    assert!(!listed.contains("\"name\":\"propose_job\""), "{listed}");
}
