//! The six operations, over the router, with no socket.
//!
//! `tower::ServiceExt::oneshot` calls the router directly — it is a `Service`,
//! and a `Service` does not need a port to be called. What this proves is the
//! step's own claim: the daemon core is drivable with zero network, because
//! nothing between a request and the daemon requires one.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobList, JobSummary, StreamMessage, WireError};
use tower::ServiceExt;

use crate::tests::fake::{at, run_id, running, FakeDaemon, A_PROPOSAL};
use crate::{router, Broadcaster, Next, Served, Subscription, SERVED};

fn wired(daemon: FakeDaemon, events: Broadcaster) -> Router {
    router(Served::by(daemon, run_id(), events))
}

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

/// The route table is hand-written, so nothing but a call proves a path is
/// really there. This walks [`SERVED`] and refuses a row the router does not
/// answer — the runtime-500 class of mistake the accepted cost buys.
#[tokio::test]
async fn every_operation_the_table_names_is_routed() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    for route in SERVED {
        let uri = route.path.replace(":job_id", "01JOB0");
        let (status, _) = call(&app, route.method, &uri, A_PROPOSAL).await;
        assert_ne!(
            status,
            StatusCode::NOT_FOUND,
            "{} is in the table and not in the router: {} {}",
            route.operation,
            route.method,
            route.path
        );
        assert_ne!(
            status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{} is routed at {} but not for {}",
            route.operation,
            route.path,
            route.method
        );
    }
}

/// One Job, through every operation M1 serves, watched on the stream — and not
/// a byte of it goes near a network.
#[tokio::test]
async fn the_six_operations_run_with_no_network() {
    let events = Broadcaster::new();
    let mut watching: Subscription = events.subscribe();
    let daemon = FakeDaemon::new(events.clone());
    // One Job already running, so the Drone kill has something to kill.
    // Nothing in M1 starts a Drone, and this test is about the transport, not
    // the scheduler.
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);

    // list_jobs, empty. A list is a shape, not an absence.
    let (status, body) = call(&app, "GET", "/jobs", "").await;
    assert_eq!(status, StatusCode::OK);
    let listed: JobList = ipc::decode("job list", &body).expect("a job list");
    assert_eq!(listed.jobs.len(), 1, "only the running Job, so far");

    // propose_job. 201, and the Job is at the gate rather than running.
    let (status, body) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED);
    let proposed: JobSummary = ipc::decode("job summary", &body).expect("a summary");
    assert_eq!(proposed.status.as_wire(), "awaiting_approval");
    assert_eq!(
        proposed.title, "fix the off-by-one in the log reader",
        "the title travels the whole way: proposal, creation, answer"
    );

    // list_jobs again. The proposal is on the Board.
    let (_, body) = call(&app, "GET", "/jobs", "").await;
    let listed: JobList = ipc::decode("job list", &body).expect("a job list");
    assert_eq!(listed.jobs.len(), 2);

    // approve_dispatch. The primary autonomy control, and a human act.
    let uri = format!("/jobs/{}/approve_dispatch", proposed.id.as_str());
    let (status, body) = call(&app, "POST", &uri, "").await;
    assert_eq!(status, StatusCode::OK);
    let approved: JobSummary = ipc::decode("job summary", &body).expect("a summary");
    assert_eq!(approved.status.as_wire(), "queued");

    // The transition reached the stream, once, in order.
    let Some(Next::Send(delivered)) = watching.next().await else {
        panic!("the approval published nothing");
    };
    assert_eq!(delivered.cursor.position(), 0);
    let ipc::Event::JobStateChanged(moved) = delivered.event;
    assert_eq!(moved.from.as_wire(), "awaiting_approval");
    assert_eq!(moved.to.as_wire(), "queued");
    assert_eq!(moved.actor.as_wire(), "human");

    // kill_drone, on a Job that is running. The process goes; the Job does not.
    let (status, body) = call(&app, "POST", "/jobs/01RUNNING/kill_drone", "").await;
    assert_eq!(status, StatusCode::OK);
    let survivor: JobSummary = ipc::decode("job summary", &body).expect("a summary");
    assert_eq!(
        survivor.status.as_wire(),
        "running",
        "killing a Drone does not end its Job"
    );
    assert!(survivor.assigned_drone.is_none(), "no process is on it now");

    // kill_job, on the same Job. This is the one that ends it.
    let (status, body) = call(&app, "POST", "/jobs/01RUNNING/kill_job", "").await;
    assert_eq!(status, StatusCode::OK);
    let ended: JobSummary = ipc::decode("job summary", &body).expect("a summary");
    assert_eq!(ended.status.as_wire(), "killed");
}

/// **The case that proves the two operations are not one.** A Job at the
/// approval gate has no Drone — nothing has spawned — and the registry still
/// carries `awaiting_approval -> killed`, an operator act carrying no verdict.
/// `kill_drone` cannot express it, and says so.
#[tokio::test]
async fn a_job_with_no_drone_is_still_killable() {
    let events = Broadcaster::new();
    let mut watching: Subscription = events.subscribe();
    let daemon = FakeDaemon::new(events.clone());
    at(&daemon, "01ATGATE", "awaiting_approval");
    let app = wired(daemon, events);

    // There is no Drone here, so rung 2 has nothing to act on.
    let (status, _) = call(&app, "POST", "/jobs/01ATGATE/kill_drone", "").await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "no Drone exists at the approval gate"
    );

    // The Job ends anyway, which is the whole distinction.
    let (status, body) = call(&app, "POST", "/jobs/01ATGATE/kill_job", "").await;
    assert_eq!(status, StatusCode::OK);
    let killed: JobSummary = ipc::decode("job summary", &body).expect("a summary");
    assert_eq!(killed.status.as_wire(), "killed");

    let Some(Next::Send(delivered)) = watching.next().await else {
        panic!("the kill published nothing");
    };
    let ipc::Event::JobStateChanged(moved) = delivered.event;
    assert_eq!(moved.from.as_wire(), "awaiting_approval");
    assert_eq!(moved.to.as_wire(), "killed");
    assert_eq!(moved.actor.as_wire(), "human");
}

/// A Job already over is not killed twice. The 409 is the machine refusing the
/// move, not the id being unknown.
#[tokio::test]
async fn a_job_that_is_already_over_cannot_be_killed_again() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    at(&daemon, "01DONE", "completed_success");
    let app = wired(daemon, events);
    let (status, _) = call(&app, "POST", "/jobs/01DONE/kill_job", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_body_that_does_not_parse_is_the_callers_fault_and_says_so() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let (status, body) = call(&app, "POST", "/jobs", r#"{"urgency":"whenever"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "not a 500: nothing failed");
    let error: WireError = ipc::decode("wire error", &body).expect("an error body");
    assert_eq!(error.code, "api.undecodable_request");
    assert_eq!(
        error.run_id.as_str(),
        "01RUN",
        "the emitting process, always"
    );
    assert!(!error.chain.is_empty(), "the chain is always present");
}

#[tokio::test]
async fn a_command_against_a_job_that_is_not_there_is_a_404() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    let (status, body) = call(&app, "POST", "/jobs/01NOTHING/kill_drone", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let error: WireError = ipc::decode("wire error", &body).expect("an error body");
    assert_eq!(
        error.job_id.map(|id| id.as_str().to_string()),
        Some("01NOTHING".to_string())
    );
}

#[tokio::test]
async fn a_move_the_machine_would_refuse_is_a_409_and_not_a_404() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);
    let uri = "/jobs/01RUNNING/approve_dispatch";
    let (status, _) = call(&app, "POST", uri, "").await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "the Job exists; it is the move that is refused"
    );
}

/// The v1 bug that lost twenty-one Jobs, one layer out. A row the store could
/// not read must reach the wire as a row that could not be read.
#[tokio::test]
async fn a_row_that_would_not_load_is_on_the_wire_and_not_filtered_away() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone()).with_unreadable("status column holds `wedged`");
    let app = wired(daemon, events);
    let (_, body) = call(&app, "GET", "/jobs", "").await;
    let listed: JobList = ipc::decode("job list", &body).expect("a job list");
    assert!(listed.jobs.is_empty());
    assert_eq!(listed.unreadable.len(), 1);
    assert!(listed.unreadable[0].fault.contains("wedged"));
}

/// A dropped event is a message, not a silence.
#[tokio::test]
async fn a_subscriber_that_falls_behind_is_told_how_many_it_lost() {
    let events = Broadcaster::with_backlog(2);
    let mut watching = events.subscribe();
    for _ in 0..6 {
        events.publish(a_transition());
    }
    let Some(Next::Missed(dropped)) = watching.next().await else {
        panic!("the bound dropped events and said nothing — the quietly-wrong failure");
    };
    assert_eq!(dropped, 4, "six published, two held");
    // And the stream continues from what is still held, rather than ending.
    assert!(matches!(watching.next().await, Some(Next::Send(_))));
}

fn a_transition() -> ipc::Event {
    let message: StreamMessage = ipc::decode(
        "stream message",
        br#"{"message":"event","cursor":0,"event":{"kind":"job.state_changed",
            "job_id":"01JOB0","from":"queued","to":"running","actor":"fleet",
            "at":"2026-08-26T09:00:00.000Z"}}"#,
    )
    .expect("a stream message");
    match message {
        StreamMessage::Event(delivered) => delivered.event,
        _ => panic!("an event"),
    }
}
