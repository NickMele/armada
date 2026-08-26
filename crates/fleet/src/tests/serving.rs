//! The five operations, over the real router, answered by a real Fleet.
//!
//! `api`'s own suite proves the same routes against a fake daemon. This one
//! proves the claim that fake was standing in for: **the operations answer from
//! Fleet**, over the router that ships, with no socket — a `Router` is a
//! `Service`, and a `Service` does not need a port to be called.
//!
//! Nothing here reaches past the transport. What comes back is read with
//! `ipc::decode`, which is both the only sanctioned way to turn bytes into a
//! typed value outside `store` and the assertion that the redaction at the
//! boundary produced something a Bridge could read.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobList, JobSummary, RunId};
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
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
    (status, body)
}

const A_PROPOSAL: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "01WF",
    "owner_manifest_id": "01FIXTUREMANIFEST",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false,
    "model": "a-model",
    "acceptance_criteria": [{"text": "the symptom is gone", "source": "check"}]
}"#;

/// Propose, list, approve — through HTTP, against Fleet.
#[tokio::test]
async fn the_operations_answer_from_a_real_fleet() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED, "the Job exists, at the gate");
    let proposed: JobSummary = ipc::decode("a proposed Job", &body).expect("a JobSummary");
    assert_eq!(proposed.status.as_wire(), "awaiting_approval");
    assert_eq!(proposed.title, "fix the off-by-one in the log reader");

    let (status, body) = call(&app, "GET", "/jobs", "").await;
    assert_eq!(status, StatusCode::OK);
    let listed: JobList = ipc::decode("the Job list", &body).expect("a JobList");
    assert_eq!(listed.jobs.len(), 1);
    assert!(
        listed.unreadable.is_empty(),
        "nothing on disk refused to fold"
    );

    // The Drone needs a directory that is really there — see the note in
    // `tests::daemon`.
    worktree_directory(&home, &proposed.id.to_domain());

    let (status, body) = call(
        &app,
        "POST",
        &format!("/jobs/{}/approve_dispatch", proposed.id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let approved: JobSummary = ipc::decode("an approved Job", &body).expect("a JobSummary");
    assert_eq!(
        approved.status.as_wire(),
        "running",
        "approval released it and the slot was free"
    );
    assert_eq!(
        approved.current_step_id.as_ref().map(|id| id.as_str()),
        Some("implement")
    );
}

/// A Job that is not there is a 404, and the code is decided from the typed
/// error rather than from a message.
#[tokio::test]
async fn a_job_that_is_not_there_is_refused_as_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(&app, "POST", "/jobs/01NOSUCHJOB/approve_dispatch", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Approving a Job that is already running is the machine refusing, which is a
/// 409 and not a fault.
#[tokio::test]
async fn a_move_the_machine_refuses_is_a_conflict() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet.propose(a_proposal("approved twice")).await.unwrap();
    worktree_directory(&home, job.id());
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let approve = format!("/jobs/{}/approve_dispatch", job.id().as_str());
    let (first, _) = call(&app, "POST", &approve, "").await;
    assert_eq!(first, StatusCode::OK);
    let (again, _) = call(&app, "POST", &approve, "").await;
    assert_eq!(
        again,
        StatusCode::CONFLICT,
        "`running -> queued` is not an edge, and 409 is what that is"
    );
}
