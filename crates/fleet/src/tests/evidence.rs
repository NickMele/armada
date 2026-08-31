//! The Evidence tool, reached the way a Drone reaches it.
//!
//! Every call below is JSON-RPC over the real router against a real Fleet, so
//! what is under test is the whole path a Job's only report travels:
//! bytes → `ipc::mcp` → `api`'s endpoint → `fleet::serving` → the inbox → the
//! gate. `api`'s own suite proves the transport against a fake; this proves the
//! part the fake could not — **that the submission lands against the Job Fleet
//! is working, and that a caller cannot say which Job that is.**
//!
//! What no test here can prove is the acceptance criterion: that a real Drone,
//! given this address in its strict MCP configuration, discovers the tool and
//! calls it. That needs a live Drone.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobList, JobSummary, RunId};
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::tests::daemon::{a_fleet, worktree_directory};
use crate::tests::tmp::TempDir;

type FixtureFleet = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// What the tool call answered, flattened to the two things a Drone reads.
struct Answered {
    status: StatusCode,
    body: String,
}

impl Answered {
    fn text(&self) -> String {
        let at = self
            .body
            .find("\"text\":\"")
            .unwrap_or_else(|| panic!("a tool result carrying one text block: {}", self.body));
        let rest = &self.body[at + 8..];
        let end = rest.find("\",").expect("a closed text block");
        rest[..end].to_string()
    }

    fn is_error(&self) -> bool {
        self.body.contains("\"isError\":true")
    }
}

async fn post(app: &Router, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        // **With a peer, because a Drone tool call is attributed by one.** A
        // router served by `axum::serve` carries this from the accepted
        // connection; a `oneshot` carries whatever the test puts on it, and a
        // request with none is refused rather than guessed at — see
        // `crate::peer`.
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:51000"
                .parse::<std::net::SocketAddr>()
                .expect("a loopback address"),
        ))
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

async fn tool_call(app: &Router, arguments: &str) -> Answered {
    let (status, body) = post(
        app,
        api::MCP_PATH,
        &format!(
            r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call",
                "params":{{"name":"submit_evidence","arguments":{arguments}}}}}"#
        ),
    )
    .await;
    Answered {
        status,
        body: String::from_utf8(body).expect("a JSON body"),
    }
}

/// A well-formed submission. **Every step takes these three and no others**,
/// so the same bytes are legal against any step of the fixture.
const A_DIFF: &str = r#"{
    "claimed": "The reader stops one line later.",
    "shown_by": "src/log.rs, six lines",
    "not_claimed": "The writer has the same bug and is untouched."
}"#;

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

/// A Fleet with one Job proposed, approved and running at its first step, and
/// the router in front of it.
async fn running(home: &TempDir) -> (Arc<FixtureFleet>, Router) {
    let fleet = Arc::new(a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    let (status, body) = post(&app, "/jobs", A_PROPOSAL).await;
    assert_eq!(status, StatusCode::CREATED);
    let proposed: JobSummary = ipc::decode("a proposed Job", &body).expect("a JobSummary");
    worktree_directory(home, &proposed.id.to_domain());
    let (status, _) = post(
        &app,
        &format!("/jobs/{}/approve_dispatch", proposed.id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "approval released it");
    (fleet, app)
}

async fn only_job(app: &Router) -> JobSummary {
    let request = Request::builder()
        .method("GET")
        .uri("/jobs")
        .body(Body::empty())
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body that reads")
        .to_bytes()
        .to_vec();
    let listed: JobList = ipc::decode("the Job list", &body).expect("a JobList");
    listed.jobs.into_iter().next().expect("the one Job")
}

/// **The claim of the whole step.** A submission arrives as a tool call over
/// HTTP, the receipt comes back, and the step it was for advances — with
/// nothing but `Fleet::turn` in between.
#[tokio::test]
async fn a_tool_call_records_evidence_and_the_step_advances() {
    let home = TempDir::new();
    let (fleet, app) = running(&home).await;

    let answered = tool_call(&app, A_DIFF).await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(!answered.is_error(), "{}", answered.text());
    assert_eq!(answered.text(), "recorded");
    assert_eq!(
        fleet.evidence_waiting(),
        1,
        "the call returned before anything was decided, which is the point"
    );

    fleet.turn().await.expect("the gate runs");
    let job = only_job(&app).await;
    assert_eq!(
        job.current_step_id.as_ref().map(|id| id.as_str()),
        Some("summarise"),
        "the step the submission was for advanced"
    );
}

/// **Nothing a caller sends decides which Job the evidence is for**, so a
/// forged id has no field to arrive in — and the call is refused rather than
/// having the field dropped, because a Drone that named a Job believed it
/// would be honoured.
#[tokio::test]
async fn a_call_forging_a_job_id_is_refused_and_records_nothing() {
    let home = TempDir::new();
    let (fleet, app) = running(&home).await;

    let answered = tool_call(
        &app,
        r#"{"job_id":"01SOMEOTHERJOB","claimed":"c","shown_by":"s","not_claimed":""}"#,
    )
    .await;
    assert_eq!(
        answered.status,
        StatusCode::OK,
        "never a 4xx and never a 500"
    );
    assert!(answered.is_error());
    assert!(
        answered
            .text()
            .contains("`job_id` is not a field of this tool"),
        "{}",
        answered.text()
    );
    assert_eq!(fleet.evidence_waiting(), 0, "nothing reached the inbox");
}

/// A second submission before the gate has ruled on the first. Refused rather
/// than queued: the first one advances the step, and a queued second would be
/// ruled on against the step it advanced *to*.
#[tokio::test]
async fn a_second_submission_for_a_step_already_evidenced_is_refused() {
    let home = TempDir::new();
    let (fleet, app) = running(&home).await;

    assert!(!tool_call(&app, A_DIFF).await.is_error());
    let again = tool_call(&app, A_DIFF).await;
    assert!(again.is_error());
    assert!(
        again.text().contains("has not been checked yet"),
        "{}",
        again.text()
    );
    assert_eq!(fleet.evidence_waiting(), 1, "one submission, not two");
}

/// Evidence for a step that has already advanced arrives as evidence for the
/// step the Job is on now, because the call names no step — so the case that
/// is actually distinguishable is a Job with no step at all. **Refused by
/// name, and the Drone is told to stop.**
///
/// **`#50` moved which refusal it is.** With no Drone anywhere, the call is
/// refused before it reaches a slot: nothing this Fleet spawned holds the
/// connection it arrived on, so there is no Job to record it against. The
/// sentence is `crate::peer`'s and it is the stronger of the two — it says why
/// the caller could not be placed rather than only that nothing was running.
#[tokio::test]
async fn a_submission_with_no_job_running_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    // Proposed and never approved: a Job exists and nothing is being worked.
    post(&app, "/jobs", A_PROPOSAL).await;

    let answered = tool_call(&app, A_DIFF).await;
    assert!(answered.is_error());
    assert!(
        answered.text().contains("no Job to record it against"),
        "{}",
        answered.text()
    );
    assert_eq!(fleet.evidence_waiting(), 0);
}

/// **The same three fields on every step, whatever it declares.** The fixture's
/// first step asks for a diff and its second for a `facts_note`, and the same
/// bytes are a submission on both — a Drone is never told its step's evidence
/// type, so a rule it could only discover by breaking it is a Drone run spent
/// on a refusal. That is what happened, and it is why there is no fourth field.
#[tokio::test]
async fn the_same_submission_is_accepted_whatever_the_step_declares() {
    let home = TempDir::new();
    let (fleet, app) = running(&home).await;

    assert!(
        !tool_call(&app, A_DIFF).await.is_error(),
        "step one is a diff"
    );
    fleet.turn().await.expect("the gate runs");

    let answered = tool_call(&app, A_DIFF).await;
    assert!(
        !answered.is_error(),
        "step two declares facts_note and takes the same three fields: {}",
        answered.text()
    );
    fleet.turn().await.expect("the gate runs");
    let job = only_job(&app).await;
    assert_eq!(
        job.status.as_wire(),
        "completed_success",
        "the last step advanced, and the Job is over"
    );
}

/// `note` was a field until every step was given the same three. A Drone
/// carrying the older habit is refused by name like any other invented field —
/// and told where the content goes, rather than only that the field is gone.
#[tokio::test]
async fn a_call_carrying_a_note_is_refused_by_name() {
    let home = TempDir::new();
    let (_fleet, app) = running(&home).await;

    let answered = tool_call(
        &app,
        r#"{"claimed":"The cause was an inclusive bound.",
            "shown_by":"`.armada/root-cause.md`","not_claimed":"",
            "note":"The bound was inclusive where the caller expected exclusive."}"#,
    )
    .await;
    assert!(answered.is_error(), "{}", answered.text());
    assert!(
        answered
            .text()
            .contains("`note` is not a field of this tool")
            && answered.text().contains("`shown_by`"),
        "{}",
        answered.text()
    );
}

/// The client discovers the tool under the bare name it will call, and the
/// allowlist entry `adapters` puts in argv is that name joined to the server
/// the configuration registers. **A drift between the two is a Drone denied
/// the only tool it has**, which looks exactly like a bad prompt.
#[tokio::test]
async fn the_tool_the_client_discovers_is_the_tool_the_allowlist_permits() {
    let home = TempDir::new();
    let (_, app) = running(&home).await;
    let (status, body) = post(
        &app,
        api::MCP_PATH,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let listed = String::from_utf8(body).expect("a JSON body");
    assert!(listed.contains(&format!("\"name\":\"{}\"", ipc::mcp::TOOL)));
    assert_eq!(
        adapters::evidence_tool(),
        format!("mcp__{}__{}", ipc::mcp::SERVER, ipc::mcp::TOOL),
        "the allowlist entry is the server and the tool, joined"
    );
    assert_eq!(adapters::evidence_server(), ipc::mcp::SERVER);
}

/// A tool call cannot block on a repository's Checks, so the receipt has to
/// come back before the gate runs. Asserted as elapsed time because that is
/// what a Drone's client is measuring against its own timeout.
#[tokio::test]
async fn the_call_returns_without_waiting_for_the_gate() {
    let home = TempDir::new();
    let (fleet, app) = running(&home).await;
    let started = std::time::Instant::now();
    assert!(!tool_call(&app, A_DIFF).await.is_error());
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the call returned in {:?}",
        started.elapsed()
    );
    assert_eq!(fleet.evidence_waiting(), 1, "and decided nothing");
}
