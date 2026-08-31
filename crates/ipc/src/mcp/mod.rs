//! The Evidence tool's transport: JSON-RPC in, JSON-RPC out.
//!
//! **A different seam from the rest of this crate.** Everything else here is
//! the Fleet/Bridge wire, versioned by `protocol-version.toml`. This is the
//! Fleet/Drone wire, versioned by the MCP revision the client names. So no type
//! below is in `operations.toml` or in `api`'s `SERVED` table, and none is
//! subject to the minor/major rules — a Drone is spawned by the Fleet it
//! reports to and cannot be skewed against it.
//!
//! # Four methods, hand-written, and no library
//!
//! `initialize`, `notifications/*`, `tools/list`, `tools/call` — the whole set
//! a client needs to discover one tool and call it, measured against the real
//! one in `docs/spikes/006-will-a-drone-use-the-evidence-tool.md`, whose server
//! log names those and nothing else. An MCP crate would put a codegen-shaped
//! dependency under the one path a Job cannot finish without.
//!
//! # The tools are next door
//!
//! What a tool takes, and what it is described as, is [`tools`](mod@tools).
//! What one of them answers with is [`report`](mod@report). This module decides
//! which method a message is and what is answered; it never decides what a
//! field means.
mod report;
mod tools;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::codec::{encode, Unencodable};

pub use report::{CheckRan, CheckReport};
pub use tools::{
    AskQuestion, AskedOption, DeclareScope, NotAnArgument, SubmitEvidence, ASK_FIELDS, ASK_TOOL,
    CHECKS_FIELDS, CHECKS_TOOL, EVIDENCE_FIELDS, FEWEST_OPTIONS, MOST_OPTIONS, SCOPE_FIELDS,
    SCOPE_TOOL, TOOL,
};

/// The name Armada's server is registered under in a Drone's MCP
/// configuration, and therefore the middle of the tool name a Drone is
/// allowed to call.
pub const SERVER: &str = "armada";

/// The MCP revision answered when a client names none.
///
/// The four methods below have been identical across every revision, so what
/// this server supports is not revision-dependent; the client's own value is
/// echoed back where it sends one, which is what the spec asks of a server
/// that can speak what it was asked for.
const ASSUMED_REVISION: &str = "2025-06-18";

/// A method that is not `tools/call` and that this server does not implement.
const NO_SUCH_METHOD: i64 = -32601;

/// Bytes that were not JSON-RPC at all.
const UNREADABLE: i64 = -32700;

/// The JSON-RPC call id, carried back untouched.
///
/// Opaque on purpose: the spec allows a string or a number and a server that
/// re-typed one would answer a call the client cannot match to its request.
/// Nothing outside this module constructs one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallId(Value);

/// One message read off the transport.
///
/// Every variant but [`Incoming::Nothing`] is answered, and the answer is
/// decided here rather than by the transport — which is what keeps the JSON
/// shapes in this crate and out of `api`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Incoming {
    /// The handshake. `revision` is what the client asked for, or
    /// [`ASSUMED_REVISION`] where it asked for nothing.
    Handshake {
        id: CallId,
        revision: String,
    },
    /// A notification. **JSON-RPC forbids answering one**, so the transport
    /// acknowledges the delivery and sends no body.
    Nothing,
    Ping {
        id: CallId,
    },
    /// The tool list. Every tool, always — there is no parameter through
    /// which a client could ask for a subset.
    Tools {
        id: CallId,
    },
    /// A call of the Evidence tool that read as a submission. Whether it *is*
    /// one is the daemon's answer, not this module's.
    Submit {
        id: CallId,
        submission: SubmitEvidence,
    },
    /// A call of the scope tool that read as a declaration. **Not evidence**:
    /// it arrives before the work rather than after it, and it moves no step.
    Declare {
        id: CallId,
        declaration: DeclareScope,
    },
    /// A call of the dry-run tool. **It carries nothing** — the tool takes no
    /// arguments, so there is no value here for a Drone to have chosen.
    ///
    /// Not evidence either, and not a gate: what comes back is what the Checks
    /// printed, and no step moves on it in either direction.
    RunChecks {
        id: CallId,
    },
    /// A call of the asking tool that read as a question. Whether it *is* one
    /// Fleet may hold is the daemon's answer, not this module's.
    ///
    /// **The one call whose answer is not in the reply.** Every other arm here
    /// is answered from something Fleet already has; this one is answered by a
    /// person, later, as a turn injected into the Drone's session. What comes
    /// back on this connection is a receipt saying the question was taken.
    Ask {
        id: CallId,
        asking: AskQuestion,
    },
    /// A tool call this server would not take. **Answered as a tool error and
    /// never as a transport failure** — a Drone reads a tool error and can act
    /// on it, and a 500 is something it can only retry.
    NotASubmission {
        id: CallId,
        why: NotAnArgument,
    },
    /// A method that is not one of the four.
    NoSuchMethod {
        id: CallId,
        named: String,
    },
    /// Bytes that did not read as JSON-RPC.
    Unreadable {
        why: String,
    },
}

/// The receipt, as the daemon answers it. **One word**, and the type carries
/// no room for a verdict — the outcome is not known when either call returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub word: String,
}

/// A submission the daemon would not take.
///
/// One field, because the Drone can act on exactly one thing: what is wrong
/// and what to do about it. The refusal's *name* is the caller's typed error;
/// this is what reaches the other end of the tool call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotRecorded {
    pub because: String,
}

/// What the transport sends back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answered {
    Handshake {
        id: CallId,
        revision: String,
    },
    Ping {
        id: CallId,
    },
    Tools {
        id: CallId,
    },
    Recorded {
        id: CallId,
        receipt: Receipt,
    },
    /// What the Checks did, in full.
    ///
    /// **A success carrying failures.** A report naming three failed Checks is
    /// still a tool call that worked, so `isError` is false — setting it would
    /// tell the client the server is broken and put the one answer a Drone
    /// asked for behind a retry.
    Checked {
        id: CallId,
        report: CheckReport,
    },
    /// A tool error: a JSON-RPC *success* carrying `isError`. That is the
    /// shape a client surfaces to its model as something to read, and a
    /// JSON-RPC error is the shape it surfaces as a broken server.
    Refused {
        id: CallId,
        why: NotRecorded,
    },
    NoSuchMethod {
        id: CallId,
        named: String,
    },
    Unreadable {
        why: String,
    },
}

/// The envelope, before anything is known about the method.
#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// Read one message. **The only place on this seam that parses JSON.**
///
/// Nothing here fails: bytes that are not JSON-RPC become
/// [`Incoming::Unreadable`], which is itself an answer. A transport that had
/// to handle a `Result` would have to decide what to say, and deciding what to
/// say is this module's whole job.
pub fn read(bytes: &[u8]) -> Incoming {
    let envelope: Envelope = match serde_json::from_slice(bytes) {
        Ok(envelope) => envelope,
        Err(why) => {
            return Incoming::Unreadable {
                why: why.to_string(),
            }
        }
    };
    // A notification must not be answered, and every one this server receives —
    // `initialized`, `cancelled` — needs nothing done, so the prefix is one arm
    // rather than a set to keep current. It is matched before the id because
    // some clients spell a notification with `"id": null` rather than by
    // omitting the member, and answering one of those is a reply the client
    // cannot match to anything.
    if envelope.method.starts_with("notifications/") {
        return Incoming::Nothing;
    }
    // Anything else with no id cannot be answered either, whatever it meant.
    let Some(id) = envelope.id.map(CallId) else {
        return Incoming::Nothing;
    };
    match envelope.method.as_str() {
        "initialize" => Incoming::Handshake {
            id,
            revision: revision(envelope.params.as_ref()),
        },
        "ping" => Incoming::Ping { id },
        "tools/list" => Incoming::Tools { id },
        "tools/call" => called(id, envelope.params.as_ref()),
        other => Incoming::NoSuchMethod {
            id,
            named: other.to_string(),
        },
    }
}

/// The revision the client asked for, echoed. A client that named none is
/// answered with the one this server assumes.
fn revision(params: Option<&Value>) -> String {
    params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(|version| version.as_str())
        .unwrap_or(ASSUMED_REVISION)
        .to_string()
}

fn called(id: CallId, params: Option<&Value>) -> Incoming {
    let tool = match tools::named(
        params
            .and_then(|params| params.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or_default(),
    ) {
        Ok(tool) => tool,
        Err(why) => return Incoming::NotASubmission { id, why },
    };
    // **Before the arguments are looked for**, because this tool takes none and
    // a client is entitled to omit the member entirely. A client that sends an
    // empty object is answered by the same arm, one branch down.
    if tool == CHECKS_TOOL {
        let carried = params
            .and_then(|params| params.get("arguments"))
            .and_then(|arguments| arguments.as_object());
        return match carried.map(tools::nothing).unwrap_or(Ok(())) {
            Ok(()) => Incoming::RunChecks { id },
            Err(why) => Incoming::NotASubmission { id, why },
        };
    }
    let Some(arguments) = params
        .and_then(|params| params.get("arguments"))
        .and_then(|arguments| arguments.as_object())
    else {
        return Incoming::NotASubmission {
            id,
            why: tools::argumentless(tool),
        };
    };
    if tool == SCOPE_TOOL {
        return match tools::declaration(arguments) {
            Ok(declaration) => Incoming::Declare { id, declaration },
            Err(why) => Incoming::NotASubmission { id, why },
        };
    }
    if tool == ASK_TOOL {
        return match tools::question(arguments) {
            Ok(asking) => Incoming::Ask { id, asking },
            Err(why) => Incoming::NotASubmission { id, why },
        };
    }
    match tools::submission(arguments) {
        Ok(submission) => Incoming::Submit { id, submission },
        Err(why) => Incoming::NotASubmission { id, why },
    }
}

/// Write one answer. The JSON shapes live here and nowhere else.
pub fn answer(answered: Answered) -> Result<String, Unencodable> {
    let value = match answered {
        Answered::Handshake { id, revision } => result(
            id,
            json!({
                "protocolVersion": revision,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER, "version": env!("CARGO_PKG_VERSION") },
            }),
        ),
        Answered::Ping { id } => result(id, json!({})),
        Answered::Tools { id } => result(id, json!({ "tools": tools::listed() })),
        Answered::Recorded { id, receipt } => result(id, said(&receipt.word, false)),
        Answered::Checked { id, report } => result(id, said(&report.to_string(), false)),
        Answered::Refused { id, why } => result(id, said(&why.because, true)),
        Answered::NoSuchMethod { id, named } => {
            failed(Some(id), NO_SUCH_METHOD, format!("no such method: {named}"))
        }
        Answered::Unreadable { why } => failed(None, UNREADABLE, format!("not JSON-RPC: {why}")),
    };
    encode(&value)
}

fn result(id: CallId, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn failed(id: Option<CallId>, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

/// A tool result. `isError` is what makes a refusal something the model on the
/// other end reads and acts on, rather than something its client swallows.
fn said(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}
