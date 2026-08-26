//! One line of a Drone's output, read.
//!
//! # A deserialiser, not a parser
//!
//! Every fact Armada needs is a named key on a typed event — measured, spike 3
//! — so nothing here regexes prose, infers from ordering, or guesses. The
//! shapes below are that stream's, which is why they are in this crate and
//! nowhere else.
//!
//! # Bytes enter the process here, and go through the one codec that reads them
//!
//! `ipc::decode` does the reading. Gate rule five scopes the untyped-JSON entry
//! points to the crates where bytes arrive, and calling through the codec is
//! how this crate reads a stream without becoming a third of them — the same
//! route `fleet` already takes for its runtime file, for the same reason.
//!
//! # Nothing is dropped, in either direction
//!
//! A line that does not decode is [`DroneEvent::Unreadable`] and a line this
//! vocabulary has no name for is [`DroneEvent::Unrecognised`]. Neither is a
//! filter: a decoder that answered "nothing happened" for output it did not
//! understand is the pre-filtered query in another shape, and the caller could
//! not tell a quiet Drone from a stream it stopped being able to read.
//!
//! # The fields that are deliberately not read
//!
//! `is_error`, `subtype`, `stop_reason` and `terminal_reason` on the
//! terminating event. All four were present, agreeing and wrong on a run that
//! did nothing — so there is no field on [`DroneEvent::Ended`] to put them in,
//! and a gate cannot read one by mistake.

use adapter_traits::DroneEvent;
use serde::Deserialize;

/// How much of an unreadable line is carried back.
///
/// Enough to recognise, not enough to be a second copy of the stream in a log.
/// A runaway line is still a line, and the reason it is unreadable is usually
/// in the first few characters.
const KEEP: usize = 240;

/// Read one line.
///
/// Never empty, and never a `Result` — see this module's note.
pub fn read(line: &str) -> Vec<DroneEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return vec![DroneEvent::Unreadable {
            line: String::new(),
            why: String::from("the line is blank"),
        }];
    }

    let read: Line = match ipc::decode("drone transcript line", trimmed.as_bytes()) {
        Ok(read) => read,
        Err(why) => {
            return vec![DroneEvent::Unreadable {
                line: keep(trimmed),
                why: why.why,
            }]
        }
    };

    match read {
        Line::System(system) => vec![system_event(system)],
        Line::Assistant(message) | Line::User(message) => blocks(message),
        Line::Result(result) => vec![DroneEvent::Ended {
            turns: result.num_turns,
            cost_micros: micros(result.total_cost_usd),
            refusals: result.permission_denials.len(),
        }],
        Line::RateLimit(event) => vec![DroneEvent::QuotaMoved {
            window: event.rate_limit_info.rate_limit_type,
            status: event.rate_limit_info.status,
        }],
        Line::Unnamed => vec![DroneEvent::Unrecognised {
            kind: kind_of(trimmed),
        }],
    }
}

/// A `system` line, told apart by its own subtype.
///
/// Two of them matter and the rest are noise from the harness's own machinery.
/// `init` is where the confinement is observable from inside the session:
/// `mcp_servers` is what the Drone actually came up holding, which is the only
/// reading that can contradict the flag Armada passed.
fn system_event(system: SystemLine) -> DroneEvent {
    match system.subtype.as_str() {
        "init" => DroneEvent::Started {
            session: system.session_id.unwrap_or_default(),
            model: system.model.unwrap_or_default(),
            mcp_servers: system.mcp_servers.len(),
        },
        "permission_denied" => DroneEvent::Refused {
            tool: system.tool_name.unwrap_or_default(),
            call: system.tool_use_id.unwrap_or_default(),
            because: system.decision_reason.unwrap_or_default(),
        },
        other => DroneEvent::Unrecognised {
            kind: format!("system/{other}"),
        },
    }
}

/// The content blocks of one turn.
///
/// A turn carrying no block Armada names is still an event, so the empty case
/// answers with one `Unrecognised` rather than with nothing — the same rule the
/// whole module follows.
fn blocks(message: MessageLine) -> Vec<DroneEvent> {
    let blocks = match message.message.content {
        // An injected turn is replayed with its content as a plain string.
        // Carried as prose, because that is what it is: Fleet's own words
        // coming back as the acknowledgement that they landed.
        Content::Prose(text) => {
            return vec![DroneEvent::Said { text }];
        }
        Content::Blocks(blocks) => blocks,
    };

    let mut events: Vec<DroneEvent> = blocks
        .into_iter()
        .filter_map(|block| match block {
            Block::ToolUse { id, name } => Some(DroneEvent::Called {
                tool: name,
                call: id,
            }),
            Block::ToolResult {
                tool_use_id,
                is_error,
            } => Some(DroneEvent::Answered {
                call: tool_use_id,
                failed: is_error,
            }),
            Block::Text { text } => Some(DroneEvent::Said { text }),
            Block::Unnamed => None,
        })
        .collect();

    if events.is_empty() {
        events.push(DroneEvent::Unrecognised {
            kind: String::from("a turn with nothing in it Armada names"),
        });
    }
    events
}

/// The `type` of a line this vocabulary has no variant for.
///
/// A second, deliberately tiny read, taken only on the branch that needs it —
/// so the ordinary line costs one decode and the unrecognised one costs the
/// name it is reported under.
fn kind_of(line: &str) -> String {
    match ipc::decode::<Tag>("drone transcript line", line.as_bytes()) {
        Ok(tag) => tag.kind,
        Err(_) => String::from("a line with no type"),
    }
}

/// Millionths of a dollar, from the fraction the harness reports.
///
/// A negative or non-finite cost is nought rather than an error: a budget is
/// what this feeds, and a budget that cannot be read is not a reason to end a
/// Job. It cannot silently overstate, which is the direction that would matter.
fn micros(usd: f64) -> u64 {
    if !usd.is_finite() || usd <= 0.0 {
        return 0;
    }
    (usd * 1_000_000.0).round() as u64
}

fn keep(line: &str) -> String {
    let cut = line
        .char_indices()
        .nth(KEEP)
        .map(|(at, _)| at)
        .unwrap_or(line.len());
    String::from(&line[..cut])
}

// ------------------------------------------------------- the stream's shapes

/// The `type` alone, for a line nothing else matched.
#[derive(Deserialize)]
struct Tag {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Line {
    #[serde(rename = "system")]
    System(SystemLine),
    #[serde(rename = "assistant")]
    Assistant(MessageLine),
    #[serde(rename = "user")]
    User(MessageLine),
    #[serde(rename = "result")]
    Result(ResultLine),
    #[serde(rename = "rate_limit_event")]
    RateLimit(RateLimitLine),
    /// Anything else the stream carries. Named rather than skipped.
    #[serde(other)]
    Unnamed,
}

#[derive(Deserialize)]
struct SystemLine {
    subtype: String,
    session_id: Option<String>,
    model: Option<String>,
    #[serde(default)]
    mcp_servers: Vec<Counted>,
    tool_name: Option<String>,
    tool_use_id: Option<String>,
    decision_reason: Option<String>,
}

/// An entry that is counted and never read, whatever shape it arrives in.
///
/// **Deliberately opaque.** What is in the operator's own inventory is not
/// Armada's business and is not something to copy into a log; how many there
/// are is the whole of what the confinement claim rests on. The refusal list on
/// the terminating event is counted the same way and for the same reason — the
/// inputs it carries are the Drone's own, and Fleet needs the count.
///
/// `IgnoredAny` rather than an empty struct, because an empty struct only
/// accepts a map: the checked-in captures carry these entries as strings, and a
/// decoder that insisted on objects would read a whole line as unreadable over
/// a field it had already decided not to look at.
type Counted = serde::de::IgnoredAny;

#[derive(Deserialize)]
struct MessageLine {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default = "Content::none")]
    content: Content,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Prose(String),
    Blocks(Vec<Block>),
}

impl Content {
    fn none() -> Content {
        Content::Blocks(Vec::new())
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Block {
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Unnamed,
}

#[derive(Deserialize)]
struct ResultLine {
    #[serde(default)]
    num_turns: u32,
    #[serde(default)]
    total_cost_usd: f64,
    #[serde(default)]
    permission_denials: Vec<Counted>,
}

#[derive(Deserialize)]
struct RateLimitLine {
    rate_limit_info: RateLimitInfo,
}

#[derive(Deserialize)]
struct RateLimitInfo {
    #[serde(default)]
    status: String,
    #[serde(rename = "rateLimitType", default)]
    rate_limit_type: String,
}
