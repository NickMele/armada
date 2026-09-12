//! The agent's door: the HTTP surface, spoken as MCP.
//!
//! **A second path on the listener, not a second surface.** Every tool below
//! is one row of `operations.toml` reached at the route `api` already serves,
//! so nothing here answers a question the HTTP surface does not.
//!
//! The tool set is [`REACHABLE`], emitted by `build.rs` from the inventory's
//! own `agent_access` column. A list written by hand is how a tool gets added
//! to the file and stays unreachable.
//!
//! The Drone's endpoint is [`crate::mcp`] and is a different seam: its peer is
//! a process Fleet spawned, and its eight tools are in no inventory.

use serde_json::{json, Map, Value};

use crate::codec::{encode, Unencodable};
use crate::mcp::CallId;

/// One operation an agent may reach, as the inventory names it.
pub struct Reachable {
    pub operation: &'static str,
    pub kind: &'static str,
    pub description: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/reachable.rs"));

/// The name this server is registered under in an agent's MCP configuration.
///
/// **Not [`crate::mcp::SERVER`].** A Drone's `armada` server is the Evidence
/// tool set; this one is fleet control, and a shared name would let a Drone's
/// configuration and an agent's be mistaken for each other.
pub const SERVER: &str = "armada-fleet";

/// The MCP revision answered when a client names none. [`crate::mcp`]'s, for
/// its reason: the methods have been identical across every revision.
const ASSUMED_REVISION: &str = "2025-06-18";

/// A method this server does not implement.
const NO_SUCH_METHOD: i64 = -32601;
/// Bytes that were not JSON-RPC at all.
const UNREADABLE: i64 = -32700;

/// The most one tool answer may weigh.
///
/// **A cap that says it capped.** Everything an agent pulls in stays in its
/// session for the rest of it, so a whole patch is a real cost — and a door
/// that refused instead would leave an agent unable to read its own work.
pub const MOST_A_TOOL_MAY_ANSWER: usize = 64 * 1024;

/// Where one operation is served, handed in by the crate that holds the route
/// table. **`path` empty is an operation the inventory names and nothing
/// serves**, which the gate refuses and this still answers honestly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub operation: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    /// The query parameters the route reads, if any. A handler's fact, so it
    /// comes from the same file the path does.
    pub query: &'static [&'static str],
}

impl Shape {
    /// The `:segment` names in the path, in order.
    pub fn segments(&self) -> impl Iterator<Item = &'static str> {
        self.path
            .split('/')
            .filter_map(|part| part.strip_prefix(':'))
    }

    fn takes_a_body(&self) -> bool {
        self.method == "POST"
    }
}

/// One tool call, as a request on the HTTP surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub operation: &'static str,
    pub method: &'static str,
    /// The path with every segment filled and the query appended.
    pub path: String,
    /// The JSON body, on a command. `None` on a read.
    pub body: Option<String>,
}

/// One message read off the door.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// A notification, or a message with no id. Acknowledged, never answered.
    Nothing,
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
    Call {
        id: CallId,
        call: Call,
    },
    /// A call this door would not make. **A tool error and never a status
    /// code**: an agent reads the one and can only retry the other.
    NotACall {
        id: CallId,
        why: String,
    },
    NoSuchMethod {
        id: CallId,
        named: String,
    },
    Unreadable {
        why: String,
    },
}

/// What the HTTP surface answered one tool call with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    /// What the response said it was. An `image/*` is answered as an image
    /// block, and everything else as text.
    pub media_type: String,
    pub body: Vec<u8>,
    /// The route that serves this whole, named in a capped answer so an agent
    /// has somewhere to go for the rest.
    pub whole_at: String,
}

/// What the door sends back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answered {
    Handshake {
        id: CallId,
        revision: String,
        /// The Manifest every answer to this session is given inside.
        scope: String,
    },
    /// A handshake answered by something standing in front of a Fleet it
    /// cannot reach — `#698`'s relay, where the caller's directory resolves to
    /// no Manifest, or to one this machine's Fleet is not serving, or where no
    /// Fleet is running at all.
    ///
    /// **A handshake that succeeds and offers nothing**, rather than a
    /// connection that fails. A client whose server would not start shows its
    /// person a red line and shows the model nothing; this way the reason is
    /// in the session, in the place the model reads before it asks.
    Unreachable {
        id: CallId,
        revision: String,
        why: String,
    },
    Ping {
        id: CallId,
    },
    Tools {
        id: CallId,
        shapes: Vec<Shape>,
    },
    Served {
        id: CallId,
        answer: Answer,
    },
    Refused {
        id: CallId,
        why: String,
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
#[derive(serde::Deserialize)]
struct Envelope {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// Read one message. **The only place on this door that parses JSON.**
pub fn read(bytes: &[u8], shapes: &[Shape]) -> Asked {
    let envelope: Envelope = match serde_json::from_slice(bytes) {
        Ok(envelope) => envelope,
        Err(why) => {
            return Asked::Unreadable {
                why: why.to_string(),
            }
        }
    };
    if envelope.method.starts_with("notifications/") {
        return Asked::Nothing;
    }
    let Some(id) = envelope.id.map(CallId::of) else {
        return Asked::Nothing;
    };
    match envelope.method.as_str() {
        "initialize" => Asked::Handshake {
            id,
            revision: revision(envelope.params.as_ref()),
        },
        "ping" => Asked::Ping { id },
        "tools/list" => Asked::Tools { id },
        "tools/call" => called(id, envelope.params.as_ref(), shapes),
        other => Asked::NoSuchMethod {
            id,
            named: other.to_string(),
        },
    }
}

fn revision(params: Option<&Value>) -> String {
    params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(|version| version.as_str())
        .unwrap_or(ASSUMED_REVISION)
        .to_string()
}

/// One `tools/call`, turned into a request or refused.
fn called(id: CallId, params: Option<&Value>, shapes: &[Shape]) -> Asked {
    let named = params
        .and_then(|params| params.get("name"))
        .and_then(|name| name.as_str())
        .unwrap_or_default();
    let Some(shape) = shapes.iter().find(|shape| shape.operation == named) else {
        return Asked::NotACall {
            id,
            why: format!("`{named}` is not a tool this Fleet offers. Call `tools/list`"),
        };
    };
    if shape.path.is_empty() {
        return Asked::NotACall {
            id,
            why: format!(
                "`{named}` is named in this Fleet's operation inventory and is served at no \
                 route, so there is nothing to call"
            ),
        };
    }
    let empty = Map::new();
    let arguments = params
        .and_then(|params| params.get("arguments"))
        .and_then(|arguments| arguments.as_object())
        .unwrap_or(&empty);
    match requested(shape, arguments) {
        Ok(call) => Asked::Call { id, call },
        Err(why) => Asked::NotACall { id, why },
    }
}

/// Fill the path's segments, append the query, and take the body.
fn requested(shape: &Shape, arguments: &Map<String, Value>) -> Result<Call, String> {
    let mut path = String::new();
    for part in shape.path.split('/').skip(1) {
        path.push('/');
        match part.strip_prefix(':') {
            None => path.push_str(part),
            Some(name) => match arguments.get(name).and_then(said) {
                // **Refused rather than defaulted.** A missing segment would
                // otherwise reach a route as the empty string, which resolves
                // to a different Job's worth of nothing.
                None => return Err(format!("`{name}` is required and was not given")),
                Some(value) => path.push_str(&encoded(&value)),
            },
        }
    }
    let asked: Vec<String> = shape
        .query
        .iter()
        .filter_map(|name| {
            let value = arguments.get(*name).and_then(said)?;
            Some(format!("{name}={}", encoded(&value)))
        })
        .collect();
    if !asked.is_empty() {
        path.push('?');
        path.push_str(&asked.join("&"));
    }
    let body = match shape.takes_a_body() {
        false => None,
        // **An empty object where nothing was sent**, because every command on
        // this seam decodes a body and a request with none is a 400 an agent
        // cannot act on. A command that takes no fields decodes `{}`.
        true => Some(match arguments.get("body") {
            Some(body) => serde_json::to_string(body).map_err(|why| why.to_string())?,
            None => String::from("{}"),
        }),
    };
    Ok(Call {
        operation: shape.operation,
        method: shape.method,
        path,
        body,
    })
}

/// One argument as text. A number or a boolean is taken as written, because a
/// client that sent `since: 12` meant the same thing as `since: "12"`.
fn said(value: &Value) -> Option<String> {
    match value {
        Value::String(said) => Some(said.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(yes) => Some(yes.to_string()),
        _ => None,
    }
}

/// Percent-encode what cannot ride in a path segment or a query value.
///
/// **By hand, because the gate keeps this crate's dependencies to two.** The
/// unreserved set is RFC 3986's; everything else is escaped, which is safe in
/// both positions even where it need not have been.
fn encoded(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Write one answer. The JSON shapes live here and nowhere else.
pub fn answer(answered: Answered) -> Result<String, Unencodable> {
    let value = match answered {
        Answered::Handshake {
            id,
            revision,
            scope,
        } => result(
            id,
            json!({
                "protocolVersion": revision,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER, "version": env!("CARGO_PKG_VERSION") },
                "instructions": instructions(&scope),
            }),
        ),
        Answered::Unreachable { id, revision, why } => result(
            id,
            json!({
                "protocolVersion": revision,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER, "version": env!("CARGO_PKG_VERSION") },
                "instructions": why,
            }),
        ),
        Answered::Ping { id } => result(id, json!({})),
        Answered::Tools { id, shapes } => result(id, json!({ "tools": listed(&shapes) })),
        Answered::Served { id, answer } => result(id, served(&answer)),
        Answered::Refused { id, why } => result(id, said_as(&why, true)),
        Answered::NoSuchMethod { id, named } => {
            failed(Some(id), NO_SUCH_METHOD, format!("no such method: {named}"))
        }
        Answered::Unreadable { why } => failed(None, UNREADABLE, format!("not JSON-RPC: {why}")),
    };
    encode(&value)
}

/// What the session is told at the handshake.
///
/// **The scope and the polling obligation**, which `docs/contracts/agent-prompt.md`
/// says an agent is told at session start. Said here as well as in a prompt so
/// a client that reaches this door with no prompt behind it still learns both.
fn instructions(scope: &str) -> String {
    format!(
        "Every answer is inside Manifest {scope}; nothing outside it is reachable through this \
         server. There is no event stream here — call `get_events_since` with the `upto` cursor \
         from your last call at the start of each turn, and fetch detail through the other tools \
         only where it bears on what was asked. An answer larger than {MOST_A_TOOL_MAY_ANSWER} \
         bytes is cut and says so, naming the route that serves it whole."
    )
}

/// One sentence added to an answer a relay is carrying through.
///
/// **For `#698`'s one disclosure**: a session resolved upward from a
/// subdirectory is answered about a repository its person may not have thought
/// they were standing in, and the handshake is where a model reads what its
/// session is. A relay cannot write it — the JSON on this door is this module's
/// — and the answer it is adding to is Fleet's, not its own.
///
/// `None` where the bytes are not a handshake carrying instructions, so the
/// caller writes what it was given through unchanged.
pub fn also_saying(answer: &[u8], note: &str) -> Option<String> {
    let mut message: Value = serde_json::from_slice(answer).ok()?;
    let instructions = message.get_mut("result")?.get_mut("instructions")?;
    let said = instructions.as_str()?;
    *instructions = Value::String(format!("{note} {said}"));
    encode(&message).ok()
}

/// Every tool, as a client is shown it.
fn listed(shapes: &[Shape]) -> Vec<Value> {
    shapes.iter().map(tool).collect()
}

fn tool(shape: &Shape) -> Value {
    let described = REACHABLE
        .iter()
        .find(|row| row.operation == shape.operation)
        .map(|row| row.description)
        .unwrap_or_default();
    let mut properties = Map::new();
    let mut required = Vec::new();
    for segment in shape.segments() {
        properties.insert(
            segment.to_string(),
            json!({ "type": "string", "description": describe(segment) }),
        );
        required.push(Value::String(segment.to_string()));
    }
    for name in shape.query {
        properties.insert(
            (*name).to_string(),
            json!({ "type": "string", "description": describe(name) }),
        );
    }
    if shape.takes_a_body() {
        properties.insert(
            "body".to_string(),
            json!({
                "type": "object",
                "description":
                    "The command's own fields, as JSON. Send it empty where the act takes \
                     none; what a field is called is answered by the refusal you get for \
                     leaving it out.",
            }),
        );
    }
    json!({
        "name": shape.operation,
        "description": match shape.path.is_empty() {
            true => format!("{described}. NOT SERVED by this Fleet."),
            false => format!("{described}. {} {}", shape.method, shape.path),
        },
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    })
}

/// What one argument names. The three that appear on more than one route are
/// worth a sentence; the rest are their own name.
fn describe(name: &str) -> String {
    match name {
        "job_id" => String::from(
            "The Job: its id, the handle a person reads, or the number that handle starts with",
        ),
        "manifest_id" => String::from(
            "The Manifest. This Fleet serves one, and it is the one your session is scoped to",
        ),
        "drone_id" => String::from("The Drone, as `list_drones` names it"),
        "since" => String::from(
            "The cursor your last call answered with, as `upto`. Nought is the whole stream",
        ),
        "q" => String::from("What a person has typed so far. Empty matches everything"),
        other => format!("The `{other}` this route names"),
    }
}

/// One answer, capped, with the cut said out loud.
fn served(answer: &Answer) -> Value {
    let failed = answer.status >= 400;
    if answer.media_type.starts_with("image/") {
        return match answer.body.len() <= MOST_A_TOOL_MAY_ANSWER {
            true => json!({
                "content": [{
                    "type": "image",
                    "data": base64(&answer.body),
                    "mimeType": answer.media_type,
                }],
                "isError": failed,
            }),
            // **An image has no window.** A truncated PNG is not a shorter
            // PNG, so there is nothing partial to send and the route is the
            // answer.
            false => said_as(
                &format!(
                    "that image is {} bytes, over the {MOST_A_TOOL_MAY_ANSWER} a tool answer \
                     may carry, and an image cannot be cut. It is at `GET {}` on this Fleet's \
                     HTTP surface",
                    answer.body.len(),
                    answer.whole_at
                ),
                true,
            ),
        };
    }
    let whole = String::from_utf8_lossy(&answer.body);
    if whole.len() <= MOST_A_TOOL_MAY_ANSWER {
        return said_as(&whole, failed);
    }
    let mut cut = MOST_A_TOOL_MAY_ANSWER;
    while cut > 0 && !whole.is_char_boundary(cut) {
        cut -= 1;
    }
    said_as(
        &format!(
            "{}\n\n[cut at {cut} of {} bytes. The whole answer is at `GET {}` on this Fleet's \
             HTTP surface, on loopback. Ask a narrower question if you can — everything you \
             pull in stays in this session.]",
            &whole[..cut],
            whole.len(),
            answer.whole_at
        ),
        failed,
    )
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
/// other end reads and acts on rather than something its client swallows.
fn said_as(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

/// Standard base64, for the one answer that is bytes.
///
/// **Twenty lines rather than a dependency.** This crate is held to two, and
/// an encoder has one shape with a test beneath it.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let packed = chunk.iter().enumerate().fold(0u32, |packed, (at, byte)| {
            packed | (*byte as u32) << (16 - 8 * at)
        });
        for at in 0..4 {
            match at <= chunk.len() {
                true => out.push(ALPHABET[(packed >> (18 - 6 * at)) as usize & 0x3F] as char),
                false => out.push('='),
            }
        }
    }
    out
}
