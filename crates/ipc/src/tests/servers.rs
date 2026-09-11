//! A server, as the stream, the list and the Drone's tool spell it.

use core_model::Timestamp;

use crate::mcp::{answer, read, Answered, Incoming, ServerReport, SERVER_TOOL};
use crate::{
    decode, encode, Cursor, Delivered, Event, Instant, JobId, ServerLink, ServerPhase, ServerPort,
    ServerState, StartServer, StartedBy, StreamMessage,
};

fn at(text: &str) -> Instant {
    Instant::from(&Timestamp::from_rfc3339(text))
}

fn serving() -> ServerState {
    ServerState {
        id: "01SERVER".to_string(),
        name: "storybook".to_string(),
        job_id: Some(JobId::carried("01JOB")),
        phase: ServerPhase::Serving,
        serve: "storybook dev -p 41207".to_string(),
        ports: vec![ServerPort {
            name: "storybook".to_string(),
            port: 41207,
        }],
        links: vec![ServerLink {
            url: "http://localhost:41207".to_string(),
            name: Some("Storybook".to_string()),
        }],
        started_by: StartedBy::Person,
        started_at: at("2026-09-11T10:00:00.000Z"),
        serving_since: Some(at("2026-09-11T10:00:04.000Z")),
        ended_at: None,
        exit_code: None,
        ended: None,
        stopped: false,
        log: ".armada/servers/jobs/01JOB/01SERVER/output.log".to_string(),
    }
}

/// Three kinds, dotted as `operations.toml` keys them, each carrying the row
/// whole — and a server with no Job says so by having no `job_id` at all.
#[test]
fn the_three_lifecycle_kinds_carry_the_instance_whole() {
    let spelled = encode(&StreamMessage::Event(Delivered {
        cursor: Cursor::at(4),
        event: Event::ServerServing(serving()),
    }))
    .expect("plain data");
    assert!(spelled.contains(r#""kind":"server.serving""#), "{spelled}");
    assert!(spelled.contains(r#""phase":"serving""#), "{spelled}");
    assert!(spelled.contains(r#""started_by":"person""#), "{spelled}");
    let back: StreamMessage = decode("a stream message", spelled.as_bytes()).expect("reads");
    assert!(matches!(
        back,
        StreamMessage::Event(Delivered { event: Event::ServerServing(state), .. }) if state == serving()
    ));

    let main_checkout = ServerState {
        job_id: None,
        ..serving()
    };
    let spelled = encode(&Event::ServerStarting(main_checkout)).expect("plain data");
    assert!(spelled.contains(r#""kind":"server.starting""#), "{spelled}");
    assert!(!spelled.contains("job_id"), "{spelled}");
}

/// The body names a server and, optionally, a Job. **No port field exists to
/// send**, so a caller cannot choose one.
#[test]
fn a_start_names_the_server_and_perhaps_a_job() {
    let asked: StartServer =
        decode("a server to start", br#"{"name":"storybook"}"#).expect("reads");
    assert_eq!(asked.job_id, None);
    let asked: StartServer = decode(
        "a server to start",
        br#"{"name":"storybook","job_id":"01JOB"}"#,
    )
    .expect("reads");
    assert_eq!(asked.job_id, Some(JobId::carried("01JOB")));
}

/// The Drone's tool takes a name and refuses a port by name, with the reason.
#[test]
fn the_drones_tool_takes_a_name_and_never_a_port() {
    let call = |arguments: &str| {
        read(
            format!(
                r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"{SERVER_TOOL}","arguments":{arguments}}}}}"#
            )
            .as_bytes(),
        )
    };
    assert!(matches!(
        call(r#"{"name":"storybook"}"#),
        Incoming::StartServer { name, .. } if name == "storybook"
    ));
    let Incoming::NotASubmission { why, .. } = call(r#"{"name":"storybook","port":6006}"#) else {
        panic!("a port is refused");
    };
    let said = why.to_string();
    assert!(said.contains("`port` is not a field"), "{said}");
    assert!(said.contains("neither you"), "{said}");
    assert!(matches!(
        call(r#"{"name":"  "}"#),
        Incoming::NotASubmission { .. }
    ));
}

/// What the Drone reads: where it is, and that it is the one instance.
#[test]
fn the_report_says_where_it_is_and_that_there_is_one() {
    let report = ServerReport {
        state: serving(),
        already_up: true,
    };
    let said = report.to_string();
    assert!(
        said.contains("http://localhost:41207 (Storybook)"),
        "{said}"
    );
    assert!(said.contains("a person had already started it"), "{said}");
    assert!(said.contains("never start another"), "{said}");

    // A call id is minted by the transport and by nothing else, so one is read.
    let Incoming::Ping { id } = read(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#) else {
        panic!("a ping");
    };
    let answered = answer(Answered::Served { id, report }).expect("plain data");
    assert!(answered.contains(r#""isError":false"#), "{answered}");
}
