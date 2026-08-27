//! Trying a Job again after a hard failure, over the router that ships.
//!
//! The Job under test reaches `escalated` the way the real one did: it was
//! `running` when Fleet died, and the next Fleet's boot read found a row with
//! no process behind it. Nothing here fakes a status onto a record.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use core_model::{Actor, JobId, JobStatus, Target};
use http_body_util::BodyExt;
use ipc::{Redispatched, RunId, WireError};
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::tests::daemon::{
    a_fleet, a_fleet_minting_from, a_proposal, diff_evidence, worktree_directory,
};
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

    the_record_survives(&home, failed.as_str(), both.dispatched.id.as_str());
}

/// The failure's worktree and branch are still there, and the replacement's are
/// somewhere else.
///
/// **This is why the id has to change.** `create_worktree` refuses an existing
/// branch and nothing anywhere removes one, so a replacement under the same id
/// could only run by destroying the record of why the first one stopped.
fn the_record_survives(home: &TempDir, failed: &str, dispatched: &str) {
    let path = home.path().to_string_lossy().to_string();
    let failure =
        adapter_traits::WorktreeSpec::for_job(&path, failed).expect("a legal spec for the failure");
    let replacement = adapter_traits::WorktreeSpec::for_job(&path, dispatched)
        .expect("a legal spec for the replacement");
    assert!(
        std::path::Path::new(&failure.worktree_path()).exists(),
        "the stopped Job's worktree is evidence and is not swept by a redispatch"
    );
    assert_ne!(
        replacement.worktree_path(),
        failure.worktree_path(),
        "the replacement works somewhere else"
    );
    assert_ne!(
        replacement.branch(),
        failure.branch(),
        "the failure's branch is the record of why it stopped, and is not reused"
    );
}

/// **A Check said no, which is the primary case.** The Job is terminal already,
/// so it is left exactly where it is and the replacement is the whole act.
#[tokio::test]
async fn a_job_a_check_failed_is_replaced_and_left_where_it_stopped() {
    let home = TempDir::new();
    // Nothing changed, so `diff_nonempty` fails and the ruling ends the Job.
    let fleet = a_fleet(&home, FakeWorkProduct::untouched());
    let job = fleet
        .propose(a_proposal("change nothing"))
        .await
        .expect("a Job at the gate");
    let failed = job.id().clone();
    worktree_directory(&home, &failed);
    fleet.approve(&failed).await.expect("released to run");
    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("evidence taken");
    fleet.turn().await.expect("a ruling");
    assert_eq!(
        fleet.load(&failed).await.expect("the Job").status(),
        JobStatus::CompletedFailed,
        "the state the redispatch is asked from, reached the way it is reached"
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", failed.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let both: Redispatched = ipc::decode("a redispatch", &body).expect("a Redispatched");

    assert_eq!(
        both.replaced.status.domain(),
        JobStatus::CompletedFailed,
        "no terminal has an outbound edge, so there is no move to make"
    );
    assert_eq!(
        both.dispatched.status.domain(),
        JobStatus::AwaitingApproval,
        "a replacement enters where its original entered — a failure earns no trust"
    );
    assert_eq!(
        both.dispatched
            .redispatched_from
            .as_ref()
            .map(|id| id.as_str()),
        Some(failed.as_str())
    );
    the_record_survives(&home, failed.as_str(), both.dispatched.id.as_str());
}

/// **A Job stopped by hand, wanted back.** The same act as a failure's, and the
/// killed Job stays killed.
#[tokio::test]
async fn a_killed_job_is_replaced_and_stays_killed() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("stopped on purpose"))
        .await
        .expect("a Job at the gate");
    let killed = job.id().clone();
    worktree_directory(&home, &killed);
    fleet.approve(&killed).await.expect("released to run");
    fleet.kill_job(&killed).await.expect("ended by hand");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", killed.as_str())).await;
    assert_eq!(status, StatusCode::OK);
    let both: Redispatched = ipc::decode("a redispatch", &body).expect("a Redispatched");

    assert_eq!(both.replaced.status.domain(), JobStatus::Killed);
    assert_eq!(both.dispatched.status.domain(), JobStatus::AwaitingApproval);
    assert_eq!(
        both.dispatched
            .redispatched_from
            .as_ref()
            .map(|id| id.as_str()),
        Some(killed.as_str())
    );
    the_record_survives(&home, killed.as_str(), both.dispatched.id.as_str());
}

/// **A rejected Job never ran**, so the refusal says that rather than naming
/// the state: there is nothing to carry forward, and what is being asked for is
/// a new Job.
#[tokio::test]
async fn a_rejected_job_is_refused_because_it_never_ran() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("denied at the gate"))
        .await
        .expect("a Job at the gate");
    // No `reject_job` command exists yet, so the Job takes the edge the
    // registry has — `awaiting_approval -> rejected` — rather than a status
    // written onto the record.
    let rejected = fleet
        .move_job(&job, Target::Rejected, Actor::Human)
        .await
        .expect("a legal edge");
    assert_eq!(rejected.status(), JobStatus::Rejected);
    let job_id = job.id().clone();

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(&app, &format!("/jobs/{}/redispatch", job_id.as_str())).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let refused: WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert_eq!(refused.code, "fleet.not_redispatchable");
    assert!(
        refused.message.contains("never ran")
            && refused.message.contains("Evidence")
            && refused.message.contains("propose_job"),
        "the refusal says nothing was produced to carry forward, and where the \
         act actually lives: {}",
        refused.message
    );
    assert_eq!(
        refused.job_id.as_ref().map(|id| id.as_str()),
        Some(job_id.as_str())
    );
}

/// A Job at the approval gate has not stopped, and the refusal says where it
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
