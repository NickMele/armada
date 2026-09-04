//! One Job in full, over the router, answered by a real Fleet.
//!
//! What is asserted here and not in `api`'s suite is the half a fake cannot
//! have: the `job_steps` rows, the branch the worktree was made on, and the
//! brief the Job was given. `api`'s fake holds `JobSummary` and nothing under
//! it, which is what makes it a fake.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobDetail, RunId};
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::planted::the_drone_it_holds_is_gone;
use crate::tests::tmp::TempDir;

pub(super) async fn get(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
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

/// A running Job's detail carries what the Board row leaves behind, and the
/// branch is read from the worktree rather than recomputed from the id.
#[tokio::test]
async fn a_dispatched_job_carries_its_branch_and_its_steps() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("read the detail back"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");

    assert_eq!(detail.job.id.as_str(), job_id.as_str());
    assert_eq!(detail.job.status.as_wire(), "running");
    assert_eq!(
        detail.branch.as_deref(),
        Some(format!("armada/{}", job_id.as_str()).as_str()),
        "what dispatch actually made, recorded when it made it"
    );
    assert_eq!(detail.steps.len(), 2, "the frozen workflow's two steps");
    assert_eq!(detail.steps[0].step_id.as_str(), "implement");
    assert_eq!(detail.steps[0].ordinal, 0);
    assert_eq!(detail.steps[0].state.as_wire(), "running");
    assert_eq!(
        detail.steps[1].state.as_wire(),
        "not_started",
        "written at creation, and not reached"
    );
    assert!(
        detail.steps[0].last_verdict.is_none(),
        "no gate has ruled on it yet"
    );
    assert_eq!(
        detail.facts.as_deref(),
        Some("the reader is off by one"),
        "the brief, which the Board row redacts and the detail is mostly for"
    );
    assert_eq!(detail.acceptance_criteria.len(), 1);
    assert_eq!(detail.acceptance_criteria[0].criterion_id.as_str(), "c1");
    assert!(!detail.created_at.as_str().is_empty());
}

/// **A Job that has no worktree does not claim a branch.** The key is missing
/// from the body, not present and null: a client cannot tell "not yet" from
/// "Fleet forgot" apart from a null.
#[tokio::test]
async fn a_job_at_the_gate_carries_no_branch_key_at_all() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("never approved"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8(body.clone()).expect("the body is text");
    assert!(
        !text.contains("\"branch\""),
        "absent, never present-and-null: {text}"
    );
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    assert!(detail.branch.is_none());
    assert_eq!(detail.steps.len(), 2, "the rows are written at creation");
    assert_eq!(detail.steps[0].state.as_wire(), "not_started");
}

/// The branch survives the process that recorded it. A Job read back by a
/// second Fleet is where the complaint's Job is: escalated, after a restart.
#[tokio::test]
async fn the_branch_survives_a_fleet_restart() {
    let home = TempDir::new();
    let job_id = {
        let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
        let job = fleet
            .propose(a_proposal("interrupted mid-flight"))
            .await
            .expect("a Job at the gate");
        worktree_directory(&home, job.id());
        fleet.approve(job.id()).await.expect("released to run");
        // **Ended here rather than left to the drop.** A Drone outlives the
        // Fleet that spawned it by design, and one still in the process table
        // is one the second Fleet adopts — which would leave this Job
        // `running`. See `crate::tests::planted`.
        the_drone_it_holds_is_gone(&fleet).await;
        job.id().clone()
    };
    let restarted = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    restarted.reconcile().await.expect("a boot read");
    let events = restarted.events();
    let app = api::router(api::Served::by(restarted, RunId::carried("01RUN"), events));

    let (status, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    assert_eq!(detail.job.status.as_wire(), "escalated");
    assert_eq!(
        detail.branch.as_deref(),
        Some(format!("armada/{}", job_id.as_str()).as_str()),
        "a column, not a fold — the log carries no worktree"
    );
}

/// A Job Fleet does not hold is a 404, like every other route's.
#[tokio::test]
async fn a_job_that_is_not_there_is_refused_as_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = get(&app, "/jobs/01NOSUCHJOB").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
