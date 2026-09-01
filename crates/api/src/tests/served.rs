//! Every operation, over the router, with no socket.
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

use crate::tests::fake::{at, running, FakeDaemon};
use crate::tests::shapes::{run_id, A_PROPOSAL, THE_ARGUMENT, THE_CALL};
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
///
/// `forget_job` is skipped here and proven on its own Job below it: it is the
/// one row on this table that really deletes what it is given, and calling it
/// in place with every other row would erase `01JOB0` out from under whichever
/// rows the table still has left to check.
#[tokio::test]
async fn every_operation_the_table_names_is_routed() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);
    // The id the loop substitutes, made real first. A route that is present and
    // answers "no such Job" is a 404 too, and the assertion below cannot tell
    // that apart from a route that is not there.
    call(&app, "POST", "/jobs", A_PROPOSAL).await;
    for route in SERVED {
        if route.operation == "forget_job" {
            continue;
        }
        let uri = route
            .path
            .replace(":job_id", "01JOB0")
            .replace(":call_id", THE_CALL);
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

    // `forget_job`'s own Job, terminal before the route is asked to delete it —
    // sharing `01JOB0` would leave nothing for a route later in the table to
    // answer about.
    let (_, body) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    let proposed: JobSummary = ipc::decode("a proposed Job", &body).expect("a summary");
    let kill_uri = format!("/jobs/{}/kill_job", proposed.id.as_str());
    let (status, _) = call(&app, "POST", &kill_uri, "").await;
    assert_eq!(status, StatusCode::OK, "terminal before it is forgettable");
    let forget_uri = format!("/jobs/{}/forget_job", proposed.id.as_str());
    let (status, _) = call(&app, "POST", &forget_uri, "").await;
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "forget_job is in the table and not in the router: POST /jobs/:job_id/forget_job"
    );
    assert_ne!(
        status,
        StatusCode::METHOD_NOT_ALLOWED,
        "forget_job is routed but not for POST"
    );
}

/// One Job, through every operation M1 serves, watched on the stream — and not
/// a byte of it goes near a network.
#[tokio::test]
async fn every_operation_runs_with_no_network() {
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

    // The creation reached the stream, first and whole. **This is the bug that
    // was here**: proposing published nothing, so a client connected at the
    // time was never told the row existed.
    let Some(Next::Send(delivered)) = watching.next().await else {
        panic!("the proposal published nothing");
    };
    assert_eq!(delivered.cursor.position(), 0);
    let ipc::Event::JobCreated(created) = delivered.event else {
        panic!("a creation, not a move");
    };
    assert_eq!(
        created.job.id, proposed.id,
        "the row travels whole, so a Board inserts it rather than re-reading"
    );
    assert_eq!(created.job.status.as_wire(), "awaiting_approval");

    // The transition reached it next, in order.
    let Some(Next::Send(delivered)) = watching.next().await else {
        panic!("the approval published nothing");
    };
    assert_eq!(delivered.cursor.position(), 1);
    let ipc::Event::JobStateChanged(moved) = delivered.event else {
        panic!("a move, not a creation");
    };
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

/// **A step's gates cross before any of them acts, over HTTP and whole.**
///
/// This is the JSON a rail is drawn from, and until it carried these two keys
/// a step gated `human_always` — where the Job halts for a person — drew as a
/// step with nothing on it. What is asserted here is the serialised body
/// rather than the struct: the defect was a field that never left Rust.
#[tokio::test]
async fn a_steps_judge_and_its_human_gate_cross_before_either_acts() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);

    let (status, body) = call(&app, "GET", "/jobs/01RUNNING", "").await;
    assert_eq!(status, StatusCode::OK);
    let json = String::from_utf8(body.clone()).expect("a JSON body");
    let detail: ipc::JobDetail = ipc::decode("a Job in full", &body).expect("a detail");

    let gated = &detail.steps[1];
    assert_eq!(
        gated.advance_gate.expect("the gate crosses").as_wire(),
        "human_always",
        "the step the Job stops on says so: {json}"
    );

    let judged = detail.steps[0]
        .judge_checks
        .as_ref()
        .expect("the declaration crosses");
    assert_eq!(judged[0].criteria, 2);
    assert_eq!(judged[0].panel_size, Some(3), "the owner asked by name");
    assert!(judged[0].gaming_check);
    assert!(
        gated
            .judge_checks
            .as_ref()
            .expect("an answer, not a gap")
            .is_empty(),
        "asking the Judge nothing is not the same as stopping for nobody: {json}"
    );
}

/// **The same two declarations, one moment earlier.** The rail says what a
/// running Job's step did; this is the preview a person approves a dispatch
/// from, and it is the more consequential of the two — a workflow that will
/// stop and wait at `handoff`, and spend Judge calls at `implement`, previewed
/// as neither.
#[tokio::test]
async fn a_workflow_previews_the_judge_and_the_gate_it_will_stop_at() {
    let events = Broadcaster::new();
    let app = wired(FakeDaemon::new(events.clone()), events);

    let (status, body) = call(&app, "GET", "/workflows", "").await;
    assert_eq!(status, StatusCode::OK);
    let json = String::from_utf8(body.clone()).expect("a JSON body");
    let held: Vec<ipc::WorkflowSummary> = ipc::decode("the workflows", &body).expect("a list");
    let steps = &held[0].steps;

    assert_eq!(
        steps[0].advance_gate.as_wire(),
        "auto_if_judge_passes",
        "what the first step advances on: {json}"
    );
    assert_eq!(
        steps[0].judge_checks[0].criteria, 2,
        "and how many questions that costs"
    );
    assert_eq!(
        steps[1].advance_gate.as_wire(),
        "human_always",
        "the step the dispatch will stop and wait on, said before it is approved"
    );
    assert!(
        steps[1].judge_checks.is_empty(),
        "which is not the same as a step the Judge is asked about: {json}"
    );
    assert!(
        !json.contains("question") && !json.contains("does the diff"),
        "counts and states cross, and a criterion's wording does not: {json}"
    );
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
    let ipc::Event::JobStateChanged(moved) = delivered.event else {
        panic!("a move, not a creation");
    };
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

/// A Job still running has a record `kill_job` could still end, not one
/// `forget_job` may erase — the 409 says which act was wanted.
#[tokio::test]
async fn a_job_that_is_not_yet_over_cannot_be_forgotten() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);
    let (status, body) = call(&app, "POST", "/jobs/01RUNNING/forget_job", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    let refused: WireError = ipc::decode("wire error", &body).expect("an error body");
    assert_eq!(refused.code, "fake.not_forgettable");
}

/// The record, gone. **`get_job` is a 404 from then on** — this is the case
/// `forget_job` exists for: a Board that no longer shows a Job it cleared.
#[tokio::test]
async fn a_terminal_job_can_be_forgotten_and_then_is_really_gone() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    at(&daemon, "01DONE", "completed_success");
    let app = wired(daemon, events);

    let (status, body) = call(&app, "POST", "/jobs/01DONE/forget_job", "").await;
    assert_eq!(status, StatusCode::OK);
    let forgotten: ipc::JobForgotten = ipc::decode("a forgotten Job", &body).expect("the id");
    assert_eq!(forgotten.job_id.as_str(), "01DONE");

    let (status, _) = call(&app, "GET", "/jobs/01DONE", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the row is really gone");
}

/// **The gesture that opens a row gets the argument.** The socket sends a line
/// and a size; this is the route the rest comes back on, and it comes back
/// uncollapsed — a heredoc read as one line is not the thing that was run.
#[tokio::test]
async fn one_calls_arguments_come_back_whole_and_keep_their_newlines() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);

    let uri = format!("/jobs/01RUNNING/calls/{THE_CALL}");
    let (status, body) = call(&app, "GET", &uri, "").await;
    assert_eq!(status, StatusCode::OK);
    let served: ipc::CallArguments = ipc::decode("a call", &body).expect("a call");
    assert_eq!(served.arguments, THE_ARGUMENT);
    assert!(served.arguments.contains('\n'), "not collapsed to a row");
    assert!(served.whole);
    assert_eq!(served.length, Some(THE_ARGUMENT.chars().count()));
}

/// A call the record does not hold is **not** the Job being absent, and the two
/// answers must not be one: a person told the Job is gone goes looking for the
/// wrong thing.
#[tokio::test]
async fn a_call_the_record_does_not_hold_is_not_a_missing_job() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    let app = wired(daemon, events);

    let (status, _) = call(&app, "GET", "/jobs/01RUNNING/calls/toolu_never", "").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _) = call(&app, "GET", "/jobs/01NOTHERE/calls/toolu_never", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "and a missing Job still is");
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
