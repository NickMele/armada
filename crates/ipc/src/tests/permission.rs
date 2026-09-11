//! The permission tool: what the harness asks, and the two answers it takes.
//!
//! **The opposite of every other tool's rule on unknown fields.** The harness
//! writes these arguments and adds fields between releases, so an extra one is
//! read past rather than refused — and the answer's text is asserted exactly,
//! because the harness parses it and a key spelled differently is a command
//! that neither runs nor is refused.

use serde_json::{json, Value};

use crate::mcp::{answer, read, Answered, CallId, Incoming, NotAnArgument, PermissionAsked};

/// A `tools/call` of the permission tool, with `arguments` as given.
fn asked(arguments: Value) -> Incoming {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": { "name": "permission", "arguments": arguments },
    });
    read(body.to_string().as_bytes())
}

/// The one text item of an answer, and whether it was marked an error.
fn answered_text(answered: Answered) -> (String, bool) {
    let body = answer(answered).expect("an answer encodes");
    let envelope: Value = crate::decode("an answer", body.as_bytes()).expect("it is JSON");
    let content = envelope["result"]["content"]
        .as_array()
        .expect("a tool result carries content");
    assert_eq!(content.len(), 1, "one text item and no other: {body}");
    let text = content[0]["text"]
        .as_str()
        .expect("a text item")
        .to_string();
    let is_error = envelope["result"]["isError"].as_bool().expect("isError");
    (text, is_error)
}

/// A shell call that read as a permission question, with the id it came under.
fn shell_call() -> (CallId, PermissionAsked) {
    let called = asked(json!({
        "tool_name": "Bash",
        "input": { "command": "touch x", "description": "Make x" },
        "tool_use_id": "toolu_01",
    }));
    let Incoming::Permission { id, asked } = called else {
        panic!("a permission call is its own variant: {called:?}");
    };
    (id, asked)
}

#[test]
fn a_shell_call_reads_as_a_question_about_its_command() {
    let (_, asked) = shell_call();
    assert_eq!(asked.tool, "Bash");
    assert_eq!(asked.call, "toolu_01");
    assert_eq!(asked.command(), Some("touch x"));
    assert_eq!(
        asked.input,
        json!({ "command": "touch x", "description": "Make x" }),
        "the input is kept whole, because an allow hands it back"
    );
}

/// Any tool can reach the prompt, and only a shell call has a command to show.
#[test]
fn a_call_on_another_tool_is_a_question_with_no_command() {
    let called = asked(json!({
        "tool_name": "Write",
        "input": { "file_path": "notes.md", "content": "x" },
        "tool_use_id": "toolu_02",
    }));
    let Incoming::Permission { asked, .. } = called else {
        panic!("a permission call on any tool is still a permission call: {called:?}");
    };
    assert_eq!(asked.tool, "Write");
    assert_eq!(asked.command(), None);
}

/// **The harness writes these, not the model**, so a field it adds in a later
/// release is read past. Refusing it would refuse every command a Drone asks
/// about the day the harness is upgraded.
#[test]
fn a_field_the_harness_adds_is_read_past_rather_than_refused() {
    let called = asked(json!({
        "tool_name": "Bash",
        "input": { "command": "ls" },
        "tool_use_id": "toolu_03",
        "permission_suggestions": [{ "type": "addRules" }],
        "something_new": 1,
    }));
    assert!(
        matches!(called, Incoming::Permission { ref asked, .. } if asked.call == "toolu_03"),
        "{called:?}"
    );
}

/// The id is what joins the question to the transcript row a person reads, so a
/// call without one is refused by name rather than asked about blind.
#[test]
fn a_question_with_no_call_id_is_refused_by_name() {
    let called = asked(json!({
        "tool_name": "Bash",
        "input": { "command": "ls" },
    }));
    assert!(
        matches!(
            called,
            Incoming::NotASubmission {
                why: NotAnArgument::Missing {
                    field: "tool_use_id"
                },
                ..
            }
        ),
        "{called:?}"
    );
}

/// An allow is a success whose text names the input the call runs with — the
/// one the harness sent, unchanged.
#[test]
fn an_allow_hands_the_input_back_in_the_shape_the_harness_reads() {
    let (id, asked) = shell_call();
    let (text, is_error) = answered_text(Answered::Permitted {
        id,
        input: asked.input,
    });
    assert_eq!(
        text,
        r#"{"behavior":"allow","updatedInput":{"command":"touch x","description":"Make x"}}"#
    );
    assert!(!is_error, "a yes is not a broken tool");
}

/// A deny is a success too. `isError` would read as the tool breaking, and the
/// message is what the Drone is told.
#[test]
fn a_deny_carries_its_message_in_the_shape_the_harness_reads() {
    let (id, _) = shell_call();
    let (text, is_error) = answered_text(Answered::Withheld {
        id,
        message: String::from("A person turned this command down."),
    });
    assert_eq!(
        text,
        r#"{"behavior":"deny","message":"A person turned this command down."}"#
    );
    assert!(!is_error, "a no is an answer, not a broken tool");
}

/// **Listed, because the harness will not use a prompt tool it cannot find**,
/// and never offered to the model in a refusal: the model has nothing to call
/// it for.
#[test]
fn the_tool_is_listed_and_a_refusal_never_offers_it() {
    let listed = answer(Answered::Tools { id: shell_call().0 }).expect("the list encodes");
    assert!(listed.contains("\"name\":\"permission\""), "{listed}");

    let called = read(
        br#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"Permit",
            "arguments":{}}}"#,
    );
    let Incoming::NotASubmission { why, .. } = called else {
        panic!("a tool nobody serves is refused as a tool error");
    };
    assert!(!why.to_string().contains("permission"), "{why}");
}
