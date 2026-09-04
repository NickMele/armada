//! The served operations, over the real router, answered by a real Fleet.
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

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use axum::Router;
use ipc::{JobList, JobSummary, RunId};
use testkit::FakeWorkProduct;

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, diff_evidence, note_evidence, worktree_directory};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

pub(crate) const A_PROPOSAL: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "fixture-workflow",
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
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));

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
    // **`queued` and not `running`, which is what `#428` moved.** The route
    // answered `running` while it dispatched inside the request that asked for
    // it — and a client that stopped waiting took the dispatch with it. What
    // crosses is the same field carrying a value Bridge already draws; the
    // `job.state_changed` to `running` follows on the turn below.
    assert_eq!(
        approved.status.as_wire(),
        "queued",
        "approval released it, and dispatch is the turn's"
    );
    assert_eq!(
        approved.current_step_id.as_ref().map(|id| id.as_str()),
        None,
        "no step is entered until something dispatches it"
    );

    fleet.turn().await.expect("the turn that admits it");
    let started = only_job(&app).await;
    assert_eq!(started.status.as_wire(), "running");
    assert_eq!(
        started.current_step_id.as_ref().map(|id| id.as_str()),
        Some("implement"),
        "and the slot was free, so it took one turn"
    );
}

/// The one Job on the Board, read back the way Bridge reads it.
async fn only_job(app: &Router) -> JobSummary {
    let (status, body) = call(app, "GET", "/jobs", "").await;
    assert_eq!(status, StatusCode::OK);
    let listed: JobList = ipc::decode("the Job list", &body).expect("a JobList");
    assert_eq!(listed.jobs.len(), 1, "one Job, and it is this one");
    listed
        .jobs
        .into_iter()
        .next()
        .expect("the Job just counted")
}

/// Poll the Board until it says so. **Nothing here turns the Fleet** — that is
/// the loop's, and waiting is how a test says it is somebody else's.
async fn until(app: &Router, what: &str, ready: impl Fn(&JobSummary) -> bool) -> JobSummary {
    for _ in 0..400 {
        let job = only_job(app).await;
        if ready(&job) {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the Job never {what} — four seconds of turns and it stood still");
}

/// **The property this whole shape exists for.** A Job proposed and approved
/// over the served API reaches a terminal state, with nobody calling
/// `Fleet::turn` by hand.
///
/// Every call below goes through the router, and every advance is the loop's.
/// The two submissions are made against Fleet directly rather than through the
/// Evidence endpoint, which `tests::evidence` drives end to end — what this one
/// is about is that the Fleet they reach is the same one the router answers
/// from, which is what the second `Arc` buys.
#[tokio::test]
async fn a_job_approved_over_the_api_reaches_a_terminal_state() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();

    // Collected rather than printed: a turn that failed is asserted on at the
    // end, so a Job that reached terminal *through* a fault cannot pass.
    let adrift: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let carried = Arc::clone(&adrift);
    let turning = crate::keep_turning(Arc::clone(&fleet), Duration::from_millis(5), move |why| {
        carried
            .lock()
            .expect("nothing panicked holding this")
            .push(why.to_string());
    });

    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));

    let (status, body) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED);
    let proposed: JobSummary = ipc::decode("a proposed Job", &body).expect("a JobSummary");
    worktree_directory(&home, &proposed.id.to_domain());

    let (status, _) = call(
        &app,
        "POST",
        &format!("/jobs/{}/approve_dispatch", proposed.id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "approval released it");

    // **Nobody presses anything else, and this is `#428`'s other half.** The
    // route answers `queued` now; the loop above is what dispatches, and this
    // is the assertion that it does — without which the fix would have traded
    // a Job a client could kill for a Job nothing ever starts.
    until(&app, "was given a Drone by the loop", |job| {
        job.current_step_id.as_ref().map(|id| id.as_str()) == Some("implement")
    })
    .await;

    // The Drone's first submission. Before this change it would have sat in the
    // inbox for the life of the process.
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the working Job's Drone submits");
    let midway = until(&app, "advanced past its first step", |job| {
        job.current_step_id.as_ref().map(|id| id.as_str()) == Some("summarise")
    })
    .await;
    assert_eq!(
        midway.status.as_wire(),
        "running",
        "a step advanced, and a step is the inner machine"
    );

    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the same Drone, on the second step");
    let ended = until(&app, "reached a terminal state", |job| {
        job.status.domain().is_terminal()
    })
    .await;
    assert_eq!(ended.status.as_wire(), "completed_success");

    turning.stopped().await;
    assert_eq!(
        adrift.lock().expect("nothing panicked holding this").len(),
        0,
        "no turn failed on the way there"
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
        "the dispatch gate is `awaiting_approval` and a Job past it is a 409"
    );
}

// ------------------------------------------------- what the first run found

/// **A Job created while a client is connected reaches that client.**
///
/// This is the bug, stated as an assertion. A Job proposed over the API while
/// Bridge was connected never appeared: creation published nothing, because
/// `ipc::Event` carried one kind and creating a Job is not a state change.
///
/// The subscription is taken **before** the proposal, which is the order the
/// listener itself takes — subscribe, then snapshot — so nothing that lands in
/// between is lost.
#[tokio::test]
async fn a_job_created_while_a_client_is_connected_reaches_that_client() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let mut watching = events.subscribe();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED);

    let Some(api::Next::Send(delivered)) = watching.next().await else {
        panic!("the proposal published nothing — the bug, exactly");
    };
    let ipc::Event::JobCreated(created) = delivered.event else {
        panic!("a creation is not a move: `job.created`, not `job.state_changed`");
    };
    assert_eq!(created.job.title, "fix the off-by-one in the log reader");
    assert_eq!(
        created.job.status.as_wire(),
        "awaiting_approval",
        "the row travels whole, so a Board draws it without a second call"
    );
    assert_eq!(created.actor.as_wire(), "human");
}

/// **A step move reaches the stream.** The complaint this answers is a Job
/// running for twenty minutes and a Board that had to be reloaded to see it
/// move: the step is what changes most often, and it published nothing.
///
/// The subscription is taken before the dispatch, in the order the listener
/// takes it, so the events are read in the order they were produced.
#[tokio::test]
async fn a_step_that_moves_while_a_client_is_connected_reaches_that_client() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("watch it advance"))
        .await
        .expect("a Job at the gate");
    worktree_directory(&home, job.id());
    let events = fleet.events();
    let mut watching = events.subscribe();

    dispatched(&fleet, job.id()).await.expect("released to run");

    // `awaiting_approval -> queued`, `queued -> running`, then the first step
    // entering `running` — which is the one that used to be silent.
    let advanced = loop {
        let Some(api::Next::Send(delivered)) = watching.next().await else {
            panic!("no step move reached the stream");
        };
        if let ipc::Event::JobStepAdvanced(advanced) = delivered.event {
            break advanced;
        }
    };

    assert_eq!(advanced.step_id.as_str(), "implement");
    assert_eq!(advanced.from.as_wire(), "not_started");
    assert_eq!(advanced.to.as_wire(), "running");
    assert_eq!(
        advanced.status.as_wire(),
        "running",
        "the status the move happened beneath — the Job did not move"
    );
    assert_eq!(
        advanced.job.current_step_id.as_ref().map(|id| id.as_str()),
        Some("implement"),
        "the row travels whole, so a Board updates it in place"
    );
    assert_eq!(advanced.actor.as_wire(), "fleet");
}

/// **A client that reconnects sees every Job that exists.**
///
/// The worse half of the same bug: reloading did not recover the missing Job.
/// A fresh connection resyncs to current state regardless of what it missed, so
/// what a client saw on the stream cannot decide what it ends up holding.
#[tokio::test]
async fn a_client_that_connects_late_is_resynced_to_every_job_that_exists() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    // Created before anything is watching. Nothing heard the event.
    let (status, _) = call(&app, "POST", "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED);

    // What a connection opens with is `list_jobs`, which is what the resync
    // carries — asserted through the same route the listener calls.
    let (status, body) = call(&app, "GET", "/jobs", "").await;
    assert_eq!(status, StatusCode::OK);
    let listed: JobList = ipc::decode("the Job list", &body).expect("a JobList");
    assert_eq!(
        listed.jobs.len(),
        1,
        "a client that missed the creation still gets the Job"
    );
}

/// **A proposal that names no model gets the configured one**, rather than
/// being stored blank and dying at dispatch as "no model was named".
#[tokio::test]
async fn a_proposal_with_no_model_is_given_the_configured_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    for body in [MODELLESS, BLANK_MODEL] {
        let (status, body) = call(&app, "POST", "/jobs", body).await;
        assert_eq!(status, StatusCode::CREATED);
        let proposed: JobSummary = ipc::decode("a proposed Job", &body).expect("a JobSummary");
        assert_eq!(
            proposed.model, "a-model",
            "the fitted default, filled in at creation"
        );
    }
}

/// A proposal naming a workflow or a Manifest Fleet does not hold is refused
/// **where it enters**, with the id in the message and the one Fleet holds
/// named. 422: the request is well-formed and the values in it are not.
#[tokio::test]
async fn a_proposal_naming_something_fleet_does_not_hold_is_refused_at_the_door() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    for (proposal, invented, held) in [
        (
            INVENTED_WORKFLOW,
            "01M0ZNVRJNC89ECG0PV1T8W08Z",
            "fixture-workflow",
        ),
        (
            INVENTED_MANIFEST,
            "01M0ZNVRJNC89ECG0PV1T8W08Y",
            "01FIXTUREMANIFEST",
        ),
    ] {
        let (status, body) = call(&app, "POST", "/jobs", proposal).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "not a 500: retrying it would fail identically forever"
        );
        let refusal: ipc::WireError = ipc::decode("a refusal", &body).expect("a WireError");
        assert!(refusal.message.contains(invented), "{}", refusal.message);
        assert!(refusal.message.contains(held), "{}", refusal.message);
    }

    let (_, body) = call(&app, "GET", "/jobs", "").await;
    let listed: JobList = ipc::decode("the Job list", &body).expect("a JobList");
    assert!(
        listed.jobs.is_empty(),
        "nothing was created, so nothing sits on the board looking approvable"
    );
}

/// What Fleet holds, served — so a composer offers values that will not be
/// refused rather than a text field for a pasted id.
#[tokio::test]
async fn what_fleet_holds_is_what_a_proposal_may_name() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(&app, "GET", "/workflows", "").await;
    assert_eq!(status, StatusCode::OK);
    let workflows: Vec<ipc::WorkflowSummary> =
        ipc::decode("the workflows", &body).expect("a workflow list");
    assert_eq!(workflows[0].id.as_str(), "fixture-workflow");
    assert_eq!(workflows[0].name, "fixture", "the name, not the id");
    assert_eq!(workflows[0].steps.len(), 2);

    let (status, body) = call(&app, "GET", "/manifests", "").await;
    assert_eq!(status, StatusCode::OK);
    let manifests: Vec<ipc::ManifestSummary> =
        ipc::decode("the Manifests", &body).expect("a Manifest list");
    assert_eq!(manifests[0].id.as_str(), "01FIXTUREMANIFEST");

    let (status, body) = call(&app, "GET", "/models", "").await;
    assert_eq!(status, StatusCode::OK);
    let models: ipc::ModelChoices = ipc::decode("the models", &body).expect("the model choices");
    assert!(
        models.models.contains(&models.default),
        "the default is a member, so a picker can select it without a lookup that misses"
    );
}

/// A proposal with the `model` key absent altogether. **The ordinary case** —
/// Bridge sends no model when a person has no opinion about one.
const MODELLESS: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "fixture-workflow",
    "owner_manifest_id": "01FIXTUREMANIFEST",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false
}"#;

/// The same thing said the other way. An empty box in a form is a caller with
/// no opinion, not a caller naming a model called "".
const BLANK_MODEL: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "fixture-workflow",
    "owner_manifest_id": "01FIXTUREMANIFEST",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false,
    "model": "   "
}"#;

/// A workflow id typed at a keyboard. It resolves to nothing, and used to be
/// stored anyway.
const INVENTED_WORKFLOW: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "01M0ZNVRJNC89ECG0PV1T8W08Z",
    "owner_manifest_id": "01FIXTUREMANIFEST",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false
}"#;

/// The same, for the other id.
const INVENTED_MANIFEST: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "fixture-workflow",
    "owner_manifest_id": "01M0ZNVRJNC89ECG0PV1T8W08Y",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false
}"#;
