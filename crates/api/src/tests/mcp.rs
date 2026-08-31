//! The Evidence endpoint, over the router, with no socket and no Drone.
//!
//! The four methods are the ones the real client's own server log names in
//! `docs/spikes/006-will-a-drone-use-the-evidence-tool.md`, and the bodies
//! below are that log's shapes. What cannot be proved here is that a Drone
//! calls it — that is the acceptance criterion and it needs a live Drone.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::{at, run_id, running, FakeDaemon};
use crate::{router, Broadcaster, Served, MCP_PATH, SERVED};

fn wired(daemon: FakeDaemon) -> Router {
    let events = Broadcaster::new();
    router(Served::by(daemon, run_id(), events))
}

/// The body, and the text of the one tool result inside it. `is_error` is what
/// tells a refusal from a receipt, and both are HTTP 200 by design.
struct Answered {
    status: StatusCode,
    body: String,
}

impl Answered {
    fn text(&self) -> String {
        // The transport is under test, not a JSON reader, and `serde_json` is
        // not a dependency of this crate. The tool result carries exactly one
        // text block, so the slice between its quotes is the message.
        let at = self
            .body
            .find("\"text\":\"")
            .expect("a tool result carrying one text block");
        let rest = &self.body[at + 8..];
        let end = rest.find("\",").expect("a closed text block");
        rest[..end].to_string()
    }

    fn is_error(&self) -> bool {
        self.body.contains("\"isError\":true")
    }
}

async fn call(app: &Router, body: &str) -> Answered {
    method(app, "POST", body).await
}

async fn method(app: &Router, method: &str, body: &str) -> Answered {
    let request = Request::builder()
        .method(method)
        .uri(MCP_PATH)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
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
    Answered {
        status,
        body: String::from_utf8(body).expect("a JSON body"),
    }
}

const HANDSHAKE: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize",
    "params":{"protocolVersion":"2025-06-18","capabilities":{},
    "clientInfo":{"name":"a-client","version":"1"}}}"#;

const SUBMISSION: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call",
    "params":{"name":"submit_evidence","arguments":{
        "claimed":"The reader stops one line later.",
        "shown_by":"`cargo test -p store` exit 0",
        "not_claimed":"The writer has the same bug and is untouched."}}}"#;

/// **The whole point of the endpoint**, and the failure it was built for: the
/// address a Drone is handed answered 404 and every Job went quiet on it.
#[tokio::test]
async fn the_address_a_drone_is_given_is_routed() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let answered = call(&app, HANDSHAKE).await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(
        answered.body.contains("\"protocolVersion\":\"2025-06-18\""),
        "the handshake echoes the revision the client asked for: {}",
        answered.body
    );
}

/// The client sends this before it will call anything, and JSON-RPC forbids a
/// reply. A body here would be a response with no id to match it to.
#[tokio::test]
async fn a_notification_is_acknowledged_and_not_answered() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::ACCEPTED);
    assert!(answered.body.is_empty());
}

/// Four tools, named as the allowlist names them and as `adapters` joins them.
#[tokio::test]
async fn the_tool_list_carries_four_tools_and_no_fifth() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let answered = call(&app, r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.body.contains("\"name\":\"submit_evidence\""));
    assert!(answered.body.contains("\"name\":\"declare_scope\""));
    assert!(answered.body.contains("\"name\":\"run_checks\""));
    assert!(answered.body.contains("\"name\":\"ask_question\""));
    // Four, and no fifth. **Only one of them reports** — the count matters
    // because a Drone choosing between reporting-shaped tools is spike 6's one
    // miss, and none of a declaration, a dry run or a question is a report.
    //
    // `ask_question`'s own schema nests an object, so the count is of the
    // top-level key: `inputSchema` appears once per tool and the option shape
    // inside it is `items`.
    assert_eq!(answered.body.matches("\"inputSchema\"").count(), 4);
}

/// A question is taken and answered with a receipt that is **not** the answer.
///
/// The point of the assertion is the word: `asked` rather than `recorded`,
/// because a Drone that read this as its answer would carry on having been told
/// nothing. What a person chose arrives later as a turn, which no router test
/// can see and `fleet`'s own suite asserts.
#[tokio::test]
async fn a_question_is_taken_and_the_receipt_is_not_the_answer() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(&app, QUESTION).await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(!answered.is_error());
    assert_eq!(answered.text(), "asked");
}

/// **The structure is the requirement, so a shapeless question is refused by
/// name.** One option is not a question, and a Drone told so can fix it and call
/// again — which is what a tool error is for and what a 4xx would not be.
#[tokio::test]
async fn a_question_with_one_answer_is_refused_and_takes_nothing() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"ask_question","arguments":{
                "question":"Split by crate?",
                "options":[{"label":"Yes","consequence":"one Job per crate"}]}}}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.is_error(), "a refusal the Drone reads: {}", answered.text());
    assert!(
        answered.text().contains("Offer between"),
        "and it names the bound: {}",
        answered.text()
    );
}

/// Two answers a person could not tell apart are two answers that mean either.
#[tokio::test]
async fn two_answers_with_one_label_are_refused() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call",
            "params":{"name":"ask_question","arguments":{
                "question":"Split by crate?",
                "options":[{"label":"Yes","consequence":"one Job per crate"},
                           {"label":"Yes","consequence":"one Job for all of them"}]}}}"#,
    )
    .await;
    assert!(answered.is_error());
    assert!(answered.text().contains("labelled `Yes`"), "{}", answered.text());
}

/// One well-formed question, reused by the tests above.
const QUESTION: &str = r#"{"jsonrpc":"2.0","id":8,"method":"tools/call",
    "params":{"name":"ask_question","arguments":{
        "question":"Should the store schema change be its own Job?",
        "options":[
            {"label":"Its own Job","consequence":"dispatch a migration Job first and make the rest depend on it"},
            {"label":"Fold it in","consequence":"the first Job that needs the column adds it"}]}}}"#;

/// The happy path, end to end through the router: a call becomes a receipt and
/// the daemon holds the submission.
#[tokio::test]
async fn a_submission_is_taken_and_answers_with_the_receipt() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(&app, SUBMISSION).await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(!answered.is_error());
    assert_eq!(answered.text(), "recorded");
}

/// **A Drone cannot say which Job its evidence is for**, so there is nothing to
/// forge — and a call that invents the field is refused by name rather than
/// having it dropped, because a Drone that named a Job believed it would be
/// honoured.
#[tokio::test]
async fn a_call_naming_a_job_is_refused_and_records_nothing() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"submit_evidence","arguments":{
                "job_id":"01JOBSOMEONEELSE",
                "claimed":"c","shown_by":"s","not_claimed":""}}}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.is_error());
    assert!(
        answered
            .text()
            .contains("`job_id` is not a field of this tool"),
        "the refusal names the field: {}",
        answered.text()
    );
}

/// A step id is refused for the same reason and with the same sentence: Fleet
/// knows where the Job is standing, and a Drone naming a step could only agree
/// or disagree.
#[tokio::test]
async fn a_call_naming_a_step_is_refused() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"submit_evidence","arguments":{
                "step_id":"plan",
                "claimed":"c","shown_by":"s","not_claimed":""}}}"#,
    )
    .await;
    assert!(answered.is_error());
    assert!(answered.text().contains("Fleet knows which Job"));
}

/// Empty is a legal answer and absent is not, and the difference is a refusal
/// the Drone can act on rather than a field silently defaulted.
#[tokio::test]
async fn a_call_omitting_not_claimed_is_refused_by_name() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"submit_evidence","arguments":{
                "claimed":"c","shown_by":"s"}}}"#,
    )
    .await;
    assert!(answered.is_error());
    assert!(answered.text().contains("`not_claimed` is missing"));
}

/// The daemon's own refusal reaches the Drone the same way an argument refusal
/// does: 200, `isError`, and prose it can read. **Never a 500** — a broken
/// server is something a model stops trying.
#[tokio::test]
async fn a_refusal_from_the_daemon_is_a_tool_error_and_not_a_fault() {
    // No Job at `running`: nothing is being worked.
    let daemon = FakeDaemon::new(Broadcaster::new());
    at(&daemon, "01JOB0", "awaiting_approval");
    let app = wired(daemon);

    let answered = call(&app, SUBMISSION).await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.is_error());
    assert!(answered.text().contains("no Job is being worked"));
}

/// A tool that is not the one tool. Refused as a tool error rather than a
/// JSON-RPC error, because the caller is a model choosing a name.
#[tokio::test]
async fn a_call_of_another_tool_is_refused_by_name() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{"name":"report_findings","arguments":{}}}"#,
    )
    .await;
    assert!(answered.is_error());
    assert!(answered.text().contains("there is no tool called"));
}

/// A method outside the four. A JSON-RPC error, because this one is the
/// client's mistake rather than the model's.
#[tokio::test]
async fn a_method_outside_the_four_is_a_json_rpc_error() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":6,"method":"resources/list"}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.body.contains("-32601"));
    assert!(answered.body.contains("resources/list"));
}

/// Bytes that are not JSON-RPC. Answered rather than dropped, and answered
/// with an id of `null`, which is what a parse error has to carry.
#[tokio::test]
async fn bytes_that_are_not_json_rpc_are_answered() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let answered = call(&app, "not json at all").await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.body.contains("-32700"));
}

/// There is no server-initiated stream and no session to delete, and 405 is
/// what the transport says a server without either answers. **This endpoint
/// queues nothing**, so it adds nothing to the unbounded-sink risk the event
/// socket carries.
#[tokio::test]
async fn there_is_no_stream_to_open_and_no_session_to_end() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    for verb in ["GET", "DELETE"] {
        let answered = method(&app, verb, "").await;
        assert_eq!(
            answered.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{verb} {MCP_PATH}"
        );
    }
}

/// The Evidence endpoint is on the listener and deliberately not in the
/// inventory the gate checks `SERVED` against: a different peer, a different
/// vocabulary, a different version to negotiate. Written as a test so that
/// adding a row is a deliberate act rather than a tidy-up.
#[test]
fn the_evidence_endpoint_is_not_a_bridge_operation() {
    assert!(SERVED.iter().all(|route| route.path != MCP_PATH));
}

/// The scope tool, end to end through the router that ships: a declaration is
/// taken and answers with its own receipt word, so a Drone can tell which call
/// landed.
#[tokio::test]
async fn a_declaration_is_taken_and_answers_with_its_own_receipt() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call",
            "params":{"name":"declare_scope","arguments":{
                "context_paths":["docs","crates/config/src"]}}}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(!answered.is_error());
    assert_eq!(answered.text(), "declared");
}

/// A declaration arriving when nothing is being worked is a tool error the
/// Drone reads, never a status code it can only retry.
#[tokio::test]
async fn a_declaration_with_nothing_working_is_a_tool_error() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call",
            "params":{"name":"declare_scope","arguments":{"context_paths":["docs"]}}}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.is_error());
}

/// **A report naming a failed Check is a tool call that worked.** `isError`
/// would tell the client the server is broken and put the one answer the Drone
/// asked for behind a retry — so the failure is in the text and the call is a
/// success.
#[tokio::test]
async fn a_report_carrying_a_failure_is_not_a_tool_error() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB0");
    let app = wired(daemon);

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"run_checks"}}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(!answered.is_error(), "{}", answered.text());
    let said = answered.text();
    assert!(said.contains("tests") && said.contains("FAILED"), "{said}");
    assert!(said.contains("exit code 101, expected 0"), "{said}");
    assert!(said.contains("implement.dry.1.log"), "{said}");
    assert!(
        said.contains("not a verdict"),
        "the answer says so itself, not only the briefing: {said}"
    );
}

/// And a call arriving when nothing is being worked is a tool error, like every
/// other refusal on this endpoint.
#[tokio::test]
async fn a_checks_call_with_nothing_working_is_a_tool_error() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));

    let answered = call(
        &app,
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"run_checks"}}"#,
    )
    .await;
    assert_eq!(answered.status, StatusCode::OK);
    assert!(answered.is_error());
}
