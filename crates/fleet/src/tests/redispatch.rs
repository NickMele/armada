//! Trying a Job again after a hard failure, over the router that ships.
//!
//! The Job under test reaches `escalated` the way the real one did: it was
//! `running` when Fleet died, and the next Fleet's boot read found a row with
//! no process behind it. Nothing here fakes a status onto a record.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use core_model::{JobId, JobStatus};
use http_body_util::BodyExt;
use ipc::{Redispatched, RunId, WireError};
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::tests::daemon::{a_fleet, a_fleet_minting_from, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

async fn call(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
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

/// A Job that was `running` when its Fleet died, found by the next one's boot
/// read. The state the complaint is about, reached the way it is really
/// reached.
async fn an_escalated_job(home: &TempDir) -> JobId {
    let job_id = {
        let fleet = a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"]));
        let job = fleet
            .propose(a_proposal("a Drone that could not spawn"))
            .await
            .expect("a Job at the gate");
        worktree_directory(home, job.id());
        fleet.approve(job.id()).await.expect("released to run");
        job.id().clone()
    };
    let restarted = a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"]));
    let reconciled = restarted.reconcile().await.expect("a boot read");
    assert_eq!(reconciled.interrupted, vec![job_id.clone()]);
    job_id
}

/// **The complaint, answered.** An escalated Job is replaced by a new one that
/// points back at it, and the failure is killed rather than reopened.
#[tokio::test]
async fn an_escalated_job_is_replaced_by_a_new_one_pointing_back_at_it() {
    let home = TempDir::new();
    let failed = an_escalated_job(&home).await;
    let fleet = a_fleet_minting_from(&home, FakeWorkProduct::changed(&["src/log.rs"]), 10);
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", failed.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let both: Redispatched = ipc::decode("a redispatch", &body).expect("a Redispatched");

    assert_eq!(both.replaced.id.as_str(), failed.as_str());
    assert_eq!(
        both.replaced.status.domain(),
        JobStatus::Killed,
        "the failure is ended, not reopened — `escalated -> running` is a redirect's edge"
    );
    assert_ne!(
        both.dispatched.id.as_str(),
        failed.as_str(),
        "a new id, because the branch `armada/<job_id>` still holds the failure"
    );
    assert_eq!(
        both.dispatched
            .redispatched_from
            .as_ref()
            .map(|id| id.as_str()),
        Some(failed.as_str()),
        "the lineage a repeat is counted along"
    );
    assert_eq!(
        both.dispatched.status.domain(),
        JobStatus::AwaitingApproval,
        "a replacement enters where its original entered"
    );
    assert_eq!(both.dispatched.title, both.replaced.title);

    // The evidence is untouched. Nothing in the workspace removes a worktree,
    // and a redispatch is not the exception.
    let spec =
        adapter_traits::WorktreeSpec::for_job(&home.path().to_string_lossy(), failed.as_str())
            .expect("a legal spec");
    assert!(
        std::path::Path::new(&spec.worktree_path()).exists(),
        "the failed Job's worktree is evidence and is not swept by a redispatch"
    );
}

/// A Job at the approval gate has not failed, and the refusal says where it
/// actually is rather than that the request was bad.
#[tokio::test]
async fn a_job_that_has_not_failed_is_refused_with_the_status_it_is_in() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("still waiting for approval"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", job_id.as_str())).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "the Job is somewhere a replacement would mean nothing"
    );
    let refused: WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert_eq!(refused.code, "fleet.not_redispatchable");
    assert!(
        refused.message.contains("awaiting_approval"),
        "the refusal names where the Job actually is: {}",
        refused.message
    );
    assert_eq!(
        refused.job_id.as_ref().map(|id| id.as_str()),
        Some(job_id.as_str())
    );
}

/// A Job Fleet does not hold is a 404, the same as every other route's.
#[tokio::test]
async fn a_job_that_is_not_there_is_refused_as_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(&app, "/jobs/01NOSUCHJOB/redispatch").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
