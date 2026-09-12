//! The agent's door: a tool call becomes a request on the HTTP surface, and
//! an answer too large to carry says so rather than arriving short.
//!
//! **The third seam in this crate.** Bridge's wire is versioned, the Drone's
//! is the MCP revision, and this one is the HTTP surface spoken as MCP — so
//! what these cases hold is that nothing is invented between the two.

use serde_json::{json, Value};

use crate::door::{answer, read, Answer, Answered, Asked, Shape, MOST_A_TOOL_MAY_ANSWER, REACHABLE};

/// The shapes a test drives: one read with a segment, one with a query, and
/// one command.
fn shapes() -> Vec<Shape> {
    vec![
        Shape {
            operation: "get_job",
            method: "GET",
            path: "/jobs/:job_id",
            query: &[],
        },
        Shape {
            operation: "get_events_since",
            method: "GET",
            path: "/events/since",
            query: &["since"],
        },
        Shape {
            operation: "redirect_drone",
            method: "POST",
            path: "/jobs/:job_id/redirect",
            query: &[],
        },
        Shape {
            operation: "list_alerts",
            method: "GET",
            path: "",
            query: &[],
        },
    ]
}

fn call(name: &str, arguments: Value) -> Vec<u8> {
    json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments },
    })
    .to_string()
    .into_bytes()
}

#[test]
fn the_tool_set_is_the_inventorys_own_column() {
    assert!(
        REACHABLE.len() > 20,
        "the emitted set is {} rows, which is not an inventory",
        REACHABLE.len()
    );
    assert!(REACHABLE.iter().any(|row| row.operation == "list_jobs"));
    assert!(
        !REACHABLE.iter().any(|row| row.operation == "observe_job"),
        "a `Bridge only` row reached the agent's tool set"
    );
    assert!(
        REACHABLE.iter().all(|row| !row.description.is_empty()),
        "a tool with no description is one a model cannot choose"
    );
}

#[test]
fn a_segment_is_filled_from_the_arguments() {
    let asked = read(&call("get_job", json!({ "job_id": "12" })), &shapes());
    let Asked::Call { call, .. } = asked else {
        panic!("expected a call, got {asked:?}");
    };
    assert_eq!(call.path, "/jobs/12");
    assert_eq!(call.method, "GET");
    assert_eq!(call.body, None);
}

/// **Refused rather than defaulted.** An empty segment resolves to a different
/// Job's worth of nothing, which is the failure this arm exists for.
#[test]
fn a_missing_segment_is_a_tool_error_and_never_a_request() {
    let asked = read(&call("get_job", json!({})), &shapes());
    let Asked::NotACall { why, .. } = asked else {
        panic!("expected a refusal, got {asked:?}");
    };
    assert!(why.contains("job_id"), "the refusal must name the field: {why}");
}

#[test]
fn a_query_parameter_rides_on_the_path() {
    let asked = read(&call("get_events_since", json!({ "since": 41 })), &shapes());
    let Asked::Call { call, .. } = asked else {
        panic!("expected a call, got {asked:?}");
    };
    assert_eq!(call.path, "/events/since?since=41");
}

/// A handle with a slash in it must not become two segments.
#[test]
fn an_argument_is_encoded_into_its_segment() {
    let asked = read(&call("get_job", json!({ "job_id": "a/b c" })), &shapes());
    let Asked::Call { call, .. } = asked else {
        panic!("expected a call, got {asked:?}");
    };
    assert_eq!(call.path, "/jobs/a%2Fb%20c");
}

/// Every command on this seam decodes a body, so a call that sent none is an
/// empty object rather than nothing at all.
#[test]
fn a_command_carries_a_body_even_where_none_was_sent() {
    let asked = read(&call("redirect_drone", json!({ "job_id": "12" })), &shapes());
    let Asked::Call { call, .. } = asked else {
        panic!("expected a call, got {asked:?}");
    };
    assert_eq!(call.path, "/jobs/12/redirect");
    assert_eq!(call.body.as_deref(), Some("{}"));
}

#[test]
fn a_body_is_passed_through_as_it_arrived() {
    let asked = read(
        &call(
            "redirect_drone",
            json!({ "job_id": "12", "body": { "instruction": "try again" } }),
        ),
        &shapes(),
    );
    let Asked::Call { call, .. } = asked else {
        panic!("expected a call, got {asked:?}");
    };
    assert_eq!(call.body.as_deref(), Some(r#"{"instruction":"try again"}"#));
}

/// The gate refuses this state; the door still answers it as something an
/// agent can read rather than as a request to nowhere.
#[test]
fn an_operation_with_no_route_is_refused_at_the_door() {
    let asked = read(&call("list_alerts", json!({})), &shapes());
    let Asked::NotACall { why, .. } = asked else {
        panic!("expected a refusal, got {asked:?}");
    };
    assert!(why.contains("no \nroute") || why.contains("no route"), "{why}");
}

#[test]
fn a_tool_nobody_offers_is_a_tool_error_and_not_a_transport_failure() {
    let asked = read(&call("rm_rf", json!({})), &shapes());
    assert!(matches!(asked, Asked::NotACall { .. }), "{asked:?}");
}

#[test]
fn the_handshake_names_the_manifest_every_answer_is_inside() {
    let written = answer(Answered::Handshake {
        id: id(),
        revision: "2025-06-18".to_string(),
        scope: "01MF".to_string(),
    })
    .expect("a handshake encodes");
    let back: Value = serde_json::from_str(&written).expect("valid JSON");
    let instructions = back["result"]["instructions"]
        .as_str()
        .expect("instructions are said");
    assert!(instructions.contains("01MF"), "{instructions}");
    assert!(
        instructions.contains("get_events_since"),
        "the polling obligation is what stands in for the stream: {instructions}"
    );
}

/// **The cap says it capped.** An answer that arrived short with nothing said
/// is one an agent would read as the whole thing.
#[test]
fn an_answer_over_the_cap_is_cut_and_names_the_route_that_serves_it_whole() {
    let written = answer(Answered::Served {
        id: id(),
        answer: Answer {
            status: 200,
            media_type: "application/json".to_string(),
            body: vec![b'x'; MOST_A_TOOL_MAY_ANSWER + 4_096],
            whole_at: "/jobs/12/diff".to_string(),
        },
    })
    .expect("an answer encodes");
    let back: Value = serde_json::from_str(&written).expect("valid JSON");
    let said = back["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(said.contains("cut at"), "{}", &said[said.len() - 200..]);
    assert!(said.contains("/jobs/12/diff"));
    assert_eq!(back["result"]["isError"], json!(false));
}

#[test]
fn an_answer_under_the_cap_is_untouched() {
    let written = answer(Answered::Served {
        id: id(),
        answer: Answer {
            status: 200,
            media_type: "application/json".to_string(),
            body: br#"{"jobs":[]}"#.to_vec(),
            whole_at: "/jobs".to_string(),
        },
    })
    .expect("an answer encodes");
    let back: Value = serde_json::from_str(&written).expect("valid JSON");
    assert_eq!(back["result"]["content"][0]["text"], json!(r#"{"jobs":[]}"#));
}

/// A refusal from the HTTP surface reaches the model as something to read,
/// which is what `isError` on a JSON-RPC success means.
#[test]
fn a_refusal_from_the_surface_is_a_tool_error_and_keeps_its_words() {
    let written = answer(Answered::Served {
        id: id(),
        answer: Answer {
            status: 404,
            media_type: "application/json".to_string(),
            body: br#"{"code":"fleet.no_such_job"}"#.to_vec(),
            whole_at: "/jobs/99".to_string(),
        },
    })
    .expect("an answer encodes");
    let back: Value = serde_json::from_str(&written).expect("valid JSON");
    assert_eq!(back["result"]["isError"], json!(true));
    assert!(back["result"]["content"][0]["text"]
        .as_str()
        .expect("text")
        .contains("fleet.no_such_job"));
}

/// **An image has no window**, so one over the cap is the route rather than a
/// shorter PNG.
#[test]
fn a_frame_is_answered_as_an_image_and_a_large_one_as_its_route() {
    let small = answer(Answered::Served {
        id: id(),
        answer: Answer {
            status: 200,
            media_type: "image/png".to_string(),
            body: vec![0x89, 0x50, 0x4E],
            whole_at: "/jobs/12/frames/1/home.png".to_string(),
        },
    })
    .expect("an image encodes");
    let back: Value = serde_json::from_str(&small).expect("valid JSON");
    assert_eq!(back["result"]["content"][0]["type"], json!("image"));
    assert_eq!(back["result"]["content"][0]["data"], json!("iVBO"));
    assert_eq!(back["result"]["content"][0]["mimeType"], json!("image/png"));

    let large = answer(Answered::Served {
        id: id(),
        answer: Answer {
            status: 200,
            media_type: "image/png".to_string(),
            body: vec![0; MOST_A_TOOL_MAY_ANSWER + 1],
            whole_at: "/jobs/12/frames/1/home.png".to_string(),
        },
    })
    .expect("a refusal encodes");
    let back: Value = serde_json::from_str(&large).expect("valid JSON");
    assert_eq!(back["result"]["isError"], json!(true));
    assert!(back["result"]["content"][0]["text"]
        .as_str()
        .expect("text")
        .contains("/jobs/12/frames/1/home.png"));
}

#[test]
fn a_notification_is_not_answered() {
    let bytes = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
        .to_string()
        .into_bytes();
    assert!(matches!(read(&bytes, &shapes()), Asked::Nothing));
}

#[test]
fn bytes_that_are_not_json_rpc_are_an_answer_rather_than_a_panic() {
    assert!(matches!(
        read(b"not json at all", &shapes()),
        Asked::Unreadable { .. }
    ));
}

/// Every tool a client is shown names its route and takes only what that route
/// has segments for.
#[test]
fn a_listed_tool_states_the_route_and_its_own_arguments() {
    let written = answer(Answered::Tools {
        id: id(),
        shapes: shapes(),
    })
    .expect("a tool list encodes");
    let back: Value = serde_json::from_str(&written).expect("valid JSON");
    let tools = back["result"]["tools"].as_array().expect("an array");
    let job = tools
        .iter()
        .find(|tool| tool["name"] == json!("get_job"))
        .expect("get_job is offered");
    assert!(job["description"]
        .as_str()
        .expect("a description")
        .contains("GET /jobs/:job_id"));
    assert_eq!(job["inputSchema"]["required"], json!(["job_id"]));
    let unserved = tools
        .iter()
        .find(|tool| tool["name"] == json!("list_alerts"))
        .expect("an unrouted operation is still listed");
    assert!(unserved["description"]
        .as_str()
        .expect("a description")
        .contains("NOT SERVED"));
}

fn id() -> crate::mcp::CallId {
    let Asked::Ping { id } = read(
        &json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" })
            .to_string()
            .into_bytes(),
        &[],
    ) else {
        panic!("a ping reads as a ping");
    };
    id
}
