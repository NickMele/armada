//! The agent door under one Fleet serving two repositories: a session is
//! answered about the one it stands in, and refused one Fleet does not serve.
//! `#987`.

use std::future::Future;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::RunId;
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::helm::{Carry, Carrying, Heard, Hosting};
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::http::call;
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

const FIRST: &str = "01FIXTUREMANIFEST";
const SECOND: &str = "01SECONDMANIFEST";

/// A Helm session's pid and the port its relay calls from: `propose_job` is
/// offered to that session alone.
const SESSION: u32 = 4242;
const RELAY: u16 = 52000;

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

fn committed(at: &Path, text: &str) {
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(at)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?} failed");
    };
    std::fs::create_dir_all(at).expect("a checkout");
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(at.join("armada.yml"), text).expect("the Manifest");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
}

/// A Fleet serving the fixture's repository and a second beside it, behind the router.
fn two_repositories(home: &TempDir) -> Router {
    let first_text = format!("version: 1\nid: {FIRST}\n");
    committed(home.path(), &first_text);
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest =
        config::Manifest::parse(Path::new("armada.yml"), &first_text).expect("the first loads");
    let workflows = fittings.starting().workflows.clone();
    let peers = Placing::nothing();
    peers.holding(SESSION + 1, RELAY, fittings.host.port);
    peers.started(SESSION, SESSION + 1);
    fittings.peers = peers;
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_helm_on(Arc::new(Running)));
    let root = home.path().join("second");
    let second_text = format!("version: 1\nid: {SECOND}\n");
    committed(&root, &second_text);
    let manifest =
        config::Manifest::parse(&root.join("armada.yml"), &second_text).expect("the second loads");
    fleet
        .repositories()
        .add(Located {
            root: root.to_string_lossy().to_string(),
            records_root: home
                .path()
                .join("second-records")
                .to_string_lossy()
                .to_string(),
            set_up: Some(SetUp::of(manifest, workflows)),
        })
        .expect("the second is served");
    let events = fleet.events();
    api::router(api::Served::sharing(fleet, RunId::carried("01RUN"), events))
}

async fn proposed(app: &Router, manifest_id: &str, title: &str) -> ipc::JobSummary {
    let proposal = format!(
        r#"{{"title": "{title}", "workflow_id": "fixture-workflow",
            "owner_manifest_id": "{manifest_id}", "origin": "manual",
            "urgency": "normal", "atomic": false}}"#
    );
    let (status, body) = call(app, "POST", "/jobs", &proposal).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    ipc::decode("a proposed Job", &body).expect("a Job")
}

async fn standing_in(app: &Router, manifest_id: &str, body: &str) -> String {
    through(app, manifest_id, body, None).await
}

/// The same call from the Helm session's relay.
async fn helm_standing_in(app: &Router, manifest_id: &str, body: &str) -> String {
    through(app, manifest_id, body, Some(RELAY)).await
}

async fn through(app: &Router, manifest_id: &str, body: &str, port: Option<u16>) -> String {
    let mut request = Request::builder()
        .method("POST")
        .uri(api::door_within(manifest_id))
        .header("content-type", "application/json");
    if let Some(port) = port {
        let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
        request = request.extension(axum::extract::ConnectInfo(peer));
    }
    let request = request
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    String::from_utf8_lossy(&body).to_string()
}

fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

const HANDSHAKE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;

#[tokio::test]
async fn a_session_standing_in_the_second_repository_is_answered_about_the_second() {
    let home = TempDir::new();
    let app = two_repositories(&home);
    let first = proposed(&app, FIRST, "the first repository's Job").await;
    proposed(&app, SECOND, "the second repository's Job").await;

    let handshake = standing_in(&app, SECOND, HANDSHAKE).await;
    assert!(handshake.contains(SECOND), "{handshake}");

    let board = standing_in(&app, SECOND, &calling("list_job_board", "{}")).await;
    assert!(board.contains("the second repository's Job"), "{board}");
    assert!(!board.contains("the first repository's Job"), "{board}");

    // Both repositories hold a Job 1; the session's is the one it means.
    let one = standing_in(&app, SECOND, &calling("get_job", r#"{"job_id":"1"}"#)).await;
    assert!(one.contains("the second repository's Job"), "{one}");

    let theirs = standing_in(
        &app,
        SECOND,
        &calling(
            "get_job",
            &format!(r#"{{"job_id":"{}"}}"#, first.id.as_str()),
        ),
    )
    .await;
    assert!(theirs.contains("\"isError\":true"), "{theirs}");
    assert!(theirs.contains("belongs to another repository"), "{theirs}");
    assert!(
        theirs.contains("fleet.job_in_another_repository"),
        "the ElsewhereOwned refusal, by its own code: {theirs}"
    );

    // Bridge's own read of that Job is unscoped.
    let (status, _) = call(&app, "GET", &format!("/jobs/{}", first.id.as_str()), "").await;
    assert_eq!(status, StatusCode::OK);
    // The Manifest reads answer about the second too; Bridge's unnamed read is the first's.
    let drift = standing_in(&app, SECOND, &calling("get_manifest_drift", "{}")).await;
    assert!(drift.contains("second"), "{drift}");
    let (_, unnamed) = call(&app, "GET", "/manifest/drift", "").await;
    assert!(
        !String::from_utf8_lossy(&unnamed).contains("second"),
        "the first's checkout"
    );
}

#[tokio::test]
async fn a_session_naming_a_manifest_this_fleet_does_not_serve_is_refused_plainly() {
    let home = TempDir::new();
    let app = two_repositories(&home);
    proposed(&app, FIRST, "the first repository's Job").await;

    let handshake = standing_in(&app, "01NOTSERVEDMANIFEST", HANDSHAKE).await;
    assert!(handshake.contains("01NOTSERVEDMANIFEST"), "{handshake}");
    assert!(
        handshake.contains(FIRST) && handshake.contains(SECOND),
        "names what is served: {handshake}"
    );

    let listed = standing_in(&app, "01NOTSERVEDMANIFEST", &calling("list_jobs", "{}")).await;
    assert!(listed.contains("\"isError\":true"), "{listed}");
    assert!(!listed.contains("the first repository's Job"), "{listed}");
}

/// A proposal through the door is owned by the session's repository, and one
/// naming another is refused.
#[tokio::test]
async fn a_proposal_through_the_door_is_owned_by_the_repository_the_session_stands_in() {
    let home = TempDir::new();
    let app = two_repositories(&home);
    let unowned = r#"{"title": "drafted in the second", "workflow_id": "fixture-workflow",
        "origin": "manual", "urgency": "normal", "atomic": false}"#;
    let drafted = helm_standing_in(
        &app,
        SECOND,
        &calling("propose_job", &format!(r#"{{"body":{unowned}}}"#)),
    )
    .await;
    assert!(drafted.contains("\"isError\":false"), "{drafted}");
    let (_, owned) = call(&app, "GET", &format!("/jobs?manifest_id={SECOND}"), "").await;
    assert!(
        String::from_utf8_lossy(&owned).contains("drafted in the second"),
        "the second owns it"
    );

    let elsewhere = unowned.replace(
        "\"origin\"",
        &format!("\"owner_manifest_id\": \"{FIRST}\", \"origin\""),
    );
    let refused = helm_standing_in(
        &app,
        SECOND,
        &calling("propose_job", &format!(r#"{{"body":{elsewhere}}}"#)),
    )
    .await;
    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(refused.contains(&format!("names `{FIRST}`")), "{refused}");
}

#[tokio::test]
async fn a_call_naming_the_other_repository_is_refused_and_events_are_counted_for_the_scope() {
    let home = TempDir::new();
    let app = two_repositories(&home);
    proposed(&app, FIRST, "the first repository's Job").await;
    proposed(&app, SECOND, "the second repository's Job").await;

    let reading = standing_in(
        &app,
        SECOND,
        &calling(
            "get_manifest_reading",
            &format!(r#"{{"manifest_id":"{FIRST}"}}"#),
        ),
    )
    .await;
    assert!(reading.contains("\"isError\":true"), "{reading}");
    assert!(reading.contains(&format!("named `{FIRST}`")), "{reading}");
    let manifest = standing_in(
        &app,
        SECOND,
        &calling("get_manifest", &format!(r#"{{"manifest_id":"{FIRST}"}}"#)),
    )
    .await;
    assert!(manifest.contains("\"isError\":true"), "{manifest}");

    let scoped = standing_in(
        &app,
        SECOND,
        &calling("get_events_since", r#"{"since":"0"}"#),
    )
    .await;
    assert!(scoped.contains(r#"job.created\",\"count\":1"#), "{scoped}");
    let (_, every) = call(&app, "GET", "/events/since?since=0", "").await;
    let every = String::from_utf8_lossy(&every);
    assert!(every.contains(r#""job.created","count":2"#), "{every}");
}
