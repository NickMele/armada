//! What a redispatch leaves readable, over the router that ships.
//!
//! Beside `redispatch`, which is about the act: this is about the two ends of
//! the record it wrote, which is what closes the dead end in `#1439`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use core_model::JobStatus;
use http_body_util::BodyExt;
use ipc::{Redispatched, RunId};
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::redispatch::call;
use crate::tests::tmp::TempDir;

/// **The dead end, closed.** A person who lands on the Job that was replaced
/// reads which Job took the work on, and the replacement names where it came
/// from — the two ends of one record, `#1439`.
#[tokio::test]
async fn a_replaced_jobs_detail_names_the_job_that_took_the_work_on() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("stopped, then asked again"))
        .await
        .expect("a Job at the gate");
    let killed = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &killed).await.expect("released to run");
    fleet.kill_job(&killed).await.expect("ended by hand");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = call(&app, &format!("/jobs/{}/redispatch", killed.as_str())).await;
    let both: Redispatched = ipc::decode("a redispatch", &body).expect("a Redispatched");

    let replaced = read_job(&app, killed.as_str()).await;
    let replacement = read_job(&app, both.dispatched.id.as_str()).await;

    let named = replaced
        .replaced_by
        .expect("the dead end names its successor");
    assert_eq!(named.job_id.as_str(), both.dispatched.id.as_str());
    assert_eq!(
        named.handle, both.dispatched.handle,
        "the one handle a person reads, composed by core-model and not twice"
    );
    assert_eq!(
        replacement
            .job
            .redispatched_from
            .as_ref()
            .map(|id| id.as_str()),
        Some(killed.as_str()),
        "the replacement names where it came from"
    );
    // The other half of that id, and the whole of `#1474`: what the header
    // draws is a handle a person has read before, not the ULID beside it.
    let from = replacement
        .replaces
        .expect("the replacement names what it replaced");
    assert_eq!(from.job_id.as_str(), killed.as_str());
    assert_eq!(
        from.handle, replaced.job.handle,
        "the predecessor's own handle, composed by core-model and not twice"
    );
    assert!(
        replacement.replaced_by.is_none(),
        "nothing has replaced the replacement, so it is not a dead end"
    );
    assert!(
        replaced.replaces.is_none(),
        "the first dispatch replaced nothing"
    );
}

/// A Job stopped and left alone reads as stopped, with nothing added.
#[tokio::test]
async fn a_killed_job_nobody_redispatched_says_nothing_extra() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("stopped and left"))
        .await
        .expect("a Job at the gate");
    let killed = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &killed).await.expect("released to run");
    fleet.kill_job(&killed).await.expect("ended by hand");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let whole = read_job(&app, killed.as_str()).await;

    assert_eq!(whole.job.status.domain(), JobStatus::Killed);
    assert!(whole.replaced_by.is_none());
    assert!(whole.replaces.is_none());
    let encoded = ipc::encode(&whole).expect("it encodes");
    assert!(
        !encoded.contains("replaced_by") && !encoded.contains("\"replaces\""),
        "absent, never present-and-null"
    );
}

/// One Job in full, over the router that ships.
async fn read_job(app: &Router, job_id: &str) -> ipc::JobDetail {
    let request = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{job_id}"))
        .body(Body::empty())
        .expect("a well-formed request");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body that reads")
        .to_bytes()
        .to_vec();
    ipc::decode("one job", &body).expect("a JobDetail")
}
