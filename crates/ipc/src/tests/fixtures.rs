//! Every Armada tool call in a fixture is one the server would accept.
//!
//! **The fixtures are a specification of Drone output, and nothing was reading
//! them.** They were written before the detectors they describe, which is the
//! point — but it also meant they were never decoded, and they drifted: all
//! three `submit_evidence` calls carried `summary`, `check_command` and
//! `exit_code`, none of which the tool has taken since it went to three prose
//! fields. `ipc::mcp::closed` would have refused every one of them by name. No
//! fixture called `declare_scope` at all, so the argument that is recoverable
//! nowhere else appeared in no specification of a Drone's stream.
//!
//! Found while fixing `#225`. This is the test that stops it happening again:
//! it puts each call into the JSON-RPC envelope the server actually receives
//! and asserts [`mcp::read`] accepts it — so a field renamed on either side
//! fails here rather than in a live Job.
//!
//! **In `ipc` and not in `testkit`, because the schema is `ipc`'s.** Reading a
//! sibling crate's directory follows `adapters`' transcript tests, which read
//! the pinned captures under `docs/spikes` the same way; and untyped JSON is
//! `store`'s and `ipc`'s alone, which is a gate rather than a preference.

use std::path::PathBuf;

use crate::mcp::{self, Incoming};
use serde_json::{json, Map, Value};

/// One tool call as a fixture holds it: the file it came from, for the message.
struct Called {
    fixture: String,
    tool: String,
    input: Map<String, Value>,
}

fn every_armada_call() -> Vec<Called> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../testkit/fixtures/ndjson");
    let mut found = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|why| panic!("{} holds the fixtures: {why}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().and_then(|it| it.to_str()) != Some("ndjson") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|it| it.to_str())
            .expect("a named file")
            .to_string();
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|why| panic!("{} is a checked-in fixture: {why}", path.display()));
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let row: Value = serde_json::from_str(line)
                .unwrap_or_else(|why| panic!("{name} holds one JSON object per line: {why}"));
            let Some(content) = row.pointer("/message/content").and_then(Value::as_array) else {
                continue;
            };
            for block in content {
                let Some(tool) = block.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if block.get("type").and_then(Value::as_str) != Some("tool_use")
                    || !tool.contains("__armada__")
                {
                    continue;
                }
                found.push(Called {
                    fixture: name.clone(),
                    tool: tool.to_string(),
                    input: block
                        .get("input")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default(),
                });
            }
        }
    }
    found
}

/// The envelope the server receives, around the arguments a fixture holds.
///
/// **The bare name, not the prefixed one.** A Drone calls
/// `mcp__armada__submit_evidence` because its harness prefixes every tool with
/// the server it came from; what reaches the server over stdio is the name the
/// server registered.
fn as_the_server_receives_it(called: &Called) -> Vec<u8> {
    let bare = called
        .tool
        .rsplit("__")
        .next()
        .expect("a name with at least one segment");
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": bare, "arguments": called.input },
    })
    .to_string()
    .into_bytes()
}

#[test]
fn every_armada_tool_call_in_a_fixture_is_one_the_server_would_accept() {
    let calls = every_armada_call();
    assert!(
        !calls.is_empty(),
        "the fixtures are the specification of a Drone's stream, and one that \
         calls none of Armada's own tools specifies nothing about them"
    );
    let refused: Vec<String> = calls
        .iter()
        .filter_map(
            |called| match mcp::read(&as_the_server_receives_it(called)) {
                Incoming::Submit { .. } | Incoming::Declare { .. } | Incoming::RunChecks { .. } => {
                    None
                }
                other => Some(format!(
                    "{}: {} would be refused — {other:?}",
                    called.fixture, called.tool
                )),
            },
        )
        .collect();
    assert!(
        refused.is_empty(),
        "a fixture describes a call the server would reject, so it specifies a \
         Drone that could not have run:\n{}",
        refused.join("\n")
    );
}

/// **`declare_scope` is the one worth insisting on.** Its arguments are
/// recoverable nowhere else — no route, no log line, no store field holds what
/// a step declared — so a stream specification that never shows one leaves the
/// only argument that matters undescribed. `#225`.
#[test]
fn some_fixture_declares_a_scope() {
    let calls = every_armada_call();
    assert!(
        calls
            .iter()
            .any(|called| called.tool.ends_with("declare_scope")),
        "no fixture calls declare_scope, so nothing specifies the shape of the \
         one argument that is recorded in no other place"
    );
}
