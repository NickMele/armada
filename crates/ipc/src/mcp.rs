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
//! # The tool takes three prose fields and an optional note
//!
//! No job id, no step id, no evidence type: Fleet knows all three, and a value
//! a Drone supplies is a value a Drone chose. A call carrying one is refused by
//! name rather than ignored — a field nothing reads is a promise the call makes
//! and the system does not keep.
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::codec::{encode, Unencodable};

/// The name Armada's server is registered under in a Drone's MCP
/// configuration, and therefore the middle of the tool name a Drone is
/// allowed to call.
pub const SERVER: &str = "armada";

/// The tool's own name, bare. The client joins it to [`SERVER`] to make the
/// allowlist entry, and `adapters` spells that joined form — the one place a
/// vendor's joining convention is allowed to live.
pub const TOOL: &str = "submit_evidence";

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
    /// The tool list. There is one tool and there is no parameter through
    /// which a second could arrive.
    Tools {
        id: CallId,
    },
    /// A call of the Evidence tool that read as a submission. Whether it *is*
    /// one is the daemon's answer, not this module's.
    Submit {
        id: CallId,
        submission: SubmitEvidence,
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

/// What a Drone hands over. **The Agent Copy Contract's Work submission
/// fields, spelled as the Drone is asked for them**, and nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmitEvidence {
    pub claimed: String,
    pub shown_by: String,
    /// Required, and legitimately empty — which is why it is not an `Option`
    /// here either. A Drone that left nothing behind has answered; a Drone
    /// that omitted the field has not, and is refused by name.
    pub not_claimed: String,
    pub note: Option<String>,
}

/// Why a tool call did not read as a submission.
///
/// **None of these is a gate failure.** Nothing was verified and the step has
/// neither advanced nor failed; the call was malformed and what the Drone is
/// told is to fix it and submit again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnArgument {
    /// A tool that is not the one tool.
    NoSuchTool {
        named: String,
    },
    /// `arguments` was absent, or was not an object.
    NoArguments,
    Missing {
        field: &'static str,
    },
    NotText {
        field: &'static str,
    },
    /// A field the tool does not take. Named rather than dropped.
    NotAField {
        named: String,
    },
}

impl core::fmt::Display for NotAnArgument {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAnArgument::NoSuchTool { named } => write!(
                out,
                "there is no tool called `{named}`. The one tool is `{TOOL}`"
            ),
            NotAnArgument::NoArguments => write!(
                out,
                "the call carried no arguments. `{TOOL}` takes claimed, shown_by \
                 and not_claimed"
            ),
            NotAnArgument::Missing { field } => write!(
                out,
                "`{field}` is missing. It is required and may be empty only \
                 where it is not_claimed — submit again with it"
            ),
            NotAnArgument::NotText { field } => {
                write!(out, "`{field}` is not text. Submit again with a string")
            }
            // The two a Drone is likeliest to invent get the reason, because
            // "no such field" reads as an oversight it should work around.
            NotAnArgument::NotAField { named } if named == "job_id" || named == "step_id" => {
                write!(
                    out,
                    "`{named}` is not a field of this tool. Fleet knows which Job \
                 and which step you are on and binds your submission to them; \
                 remove it and submit again"
                )
            }
            NotAnArgument::NotAField { named } => write!(
                out,
                "`{named}` is not a field of this tool. It takes claimed, \
                 shown_by, not_claimed and note — remove it and submit again"
            ),
        }
    }
}

impl std::error::Error for NotAnArgument {}

/// The receipt, as the daemon answers it. **One word**, and the type carries
/// no room for a verdict — the outcome is not known when this returns.
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
    let named = params
        .and_then(|params| params.get("name"))
        .and_then(|name| name.as_str())
        .unwrap_or_default();
    if named != TOOL {
        return Incoming::NotASubmission {
            id,
            why: NotAnArgument::NoSuchTool {
                named: named.to_string(),
            },
        };
    }
    let Some(arguments) = params
        .and_then(|params| params.get("arguments"))
        .and_then(|arguments| arguments.as_object())
    else {
        return Incoming::NotASubmission {
            id,
            why: NotAnArgument::NoArguments,
        };
    };
    match submission(arguments) {
        Ok(submission) => Incoming::Submit { id, submission },
        Err(why) => Incoming::NotASubmission { id, why },
    }
}

/// The three prose fields and the note, and a refusal for anything else.
const FIELDS: &[&str] = &["claimed", "shown_by", "not_claimed", "note"];

fn submission(arguments: &Map<String, Value>) -> Result<SubmitEvidence, NotAnArgument> {
    for named in arguments.keys() {
        if !FIELDS.contains(&named.as_str()) {
            return Err(NotAnArgument::NotAField {
                named: named.clone(),
            });
        }
    }
    Ok(SubmitEvidence {
        claimed: text(arguments, "claimed")?,
        shown_by: text(arguments, "shown_by")?,
        not_claimed: text(arguments, "not_claimed")?,
        note: match arguments.get("note") {
            None | Some(Value::Null) => None,
            Some(note) => Some(
                note.as_str()
                    .ok_or(NotAnArgument::NotText { field: "note" })?
                    .to_string(),
            ),
        },
    })
}

fn text(arguments: &Map<String, Value>, field: &'static str) -> Result<String, NotAnArgument> {
    let value = arguments
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?;
    Ok(value
        .as_str()
        .ok_or(NotAnArgument::NotText { field })?
        .to_string())
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
        Answered::Tools { id } => result(id, json!({ "tools": [tool()] })),
        Answered::Recorded { id, receipt } => result(id, said(&receipt.word, false)),
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

/// The one tool, as the client is shown it.
///
/// The description is the wording spike 6 measured — the `silent` arm proved a
/// description alone does not make a Drone call the tool, which is why the
/// obligation is in the baseline prompt; what the description still has to do
/// is say what the call is for and that the receipt is not a verdict.
///
/// **`additionalProperties` is false and the server checks it anyway.** The
/// schema is advice a client may enforce; the check in [`submission`] is what
/// makes a forged field a named refusal rather than a silently accepted one.
fn tool() -> Value {
    json!({
        "name": TOOL,
        "description":
            "Report the outcome of the step you were given. This is the only way \
             to report: the result is not read from anything you write in prose. \
             Returns a receipt, not a verdict — the receipt does not mean the \
             step passed.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "claimed": {
                    "type": "string",
                    "description":
                        "What the work now does, as an observable. Behaviour, not \
                         a description of the change you made.",
                },
                "shown_by": {
                    "type": "string",
                    "description":
                        "The artifact that demonstrates it — a named test, a \
                         command and its exit code, a rendered string.",
                },
                "not_claimed": {
                    "type": "string",
                    "description":
                        "Everything the claim does not assert: the gap you left \
                         and the side effect you caused. Empty is a legal answer; \
                         omitting it is not.",
                },
                "note": {
                    "type": "string",
                    "description":
                        "The finding itself, where this step's work product is a \
                         written note rather than a change to the repository. A \
                         step that needs one and does not get it refuses the call \
                         and says so, and so does a step that has no use for one.",
                },
            },
            "required": ["claimed", "shown_by", "not_claimed"],
            "additionalProperties": false,
        },
    })
}
