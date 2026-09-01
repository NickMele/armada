//! One line of a Drone's output, read.
//!
//! **A deserialiser, not a parser.** Every fact Armada needs is a named key on
//! a typed event — measured, spike 3 — so nothing here regexes prose, infers
//! from ordering, or guesses. The shapes below are that stream's, which is why
//! they are in this crate and nowhere else.
//!
//! **Bytes enter the process here, through the one codec that reads them.**
//! `ipc::decode` does the reading: gate rule five scopes the untyped-JSON entry
//! points to the crates where bytes arrive, and calling through the codec is
//! how this crate reads a stream without becoming a third of them — the same
//! route `fleet` already takes for its runtime file.
//!
//! **Nothing is dropped, in either direction.** A line that does not decode is
//! [`DroneEvent::Unreadable`] and one this vocabulary has no name for is
//! [`DroneEvent::Unrecognised`]. A decoder answering "nothing happened" is the
//! pre-filtered query in another shape, and the caller could not tell a quiet
//! Drone from a stream it stopped reading. **It holds at the block too**: a
//! turn is several lines, each answering for itself.
//!
//! **Four fields are deliberately not read**: `is_error`, `subtype`,
//! `stop_reason` and `terminal_reason` on the terminating event. All four were
//! present, agreeing and wrong on a run that did nothing — so there is no field
//! on [`DroneEvent::Ended`] to put them in, and a gate cannot read one by
//! mistake.

use adapter_traits::{CallDetail, DroneEvent};
use serde::Deserialize;

/// How much of an unreadable line is carried back.
///
/// Enough to recognise, not enough to be a second copy of the stream in a log.
/// A runaway line is still a line, and the reason it is unreadable is usually
/// in the first few characters.
const KEEP: usize = 240;

/// A turn that carried the model's working, named without carrying it.
///
/// **The text is deliberately not read.** Reasoning is the model's working
/// rather than its answer, it is long, and `docs/scope.md` records that reading
/// a transcript is the thing Armada exists to escape. What a reader cannot do
/// without is knowing the turn had reasoning in it at all — a turn that thought
/// for thirty seconds and then made one call rendered as one call, and the
/// removal was invisible. Whether the text is ever shown is a separate decision
/// and is not this decoder's.
const REASONED: &str = "the Drone's reasoning, not carried";

/// A block whose `type` this vocabulary has no variant for.
///
/// The spelling is not carried: serde's catch-all variant consumes the tag, and
/// a second read of the block to recover it would cost every ordinary turn.
const UNNAMED_BLOCK: &str = "a block Armada does not name";

/// A turn whose content held nothing at all — which is a different fact from a
/// turn holding blocks this vocabulary does not name, and they say so one row
/// each.
const EMPTY_TURN: &str = "a turn with nothing in it Armada names";

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
/// **No block leaves without a row.** A block this vocabulary has no variant
/// for is a [`DroneEvent::Unrecognised`] carrying what it was — the same answer
/// [`system_event`] gives a subtype it does not name, and the rule the whole
/// module follows, applied one level down.
///
/// What shipped applied it only to a turn that had nothing else, so a turn that
/// reasoned and then made a call rendered as the call alone and a reader could
/// not tell that anything had been taken out.
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

    let mut reasoned = false;
    let mut events: Vec<DroneEvent> = blocks
        .into_iter()
        .filter_map(|block| match block {
            Block::ToolUse { id, name, input } => Some(DroneEvent::Called {
                detail: detail(&input),
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
            // **One row per turn, not one per block.** What a reader is owed is
            // that the turn reasoned, which is a fact about the turn; a turn's
            // working arrives as several blocks, and a row each would put
            // volume into the fold that already reads as noise.
            Block::Thinking => match reasoned {
                true => None,
                false => {
                    reasoned = true;
                    Some(DroneEvent::Unrecognised {
                        kind: String::from(REASONED),
                    })
                }
            },
            Block::Unnamed => Some(DroneEvent::Unrecognised {
                kind: String::from(UNNAMED_BLOCK),
            }),
        })
        .collect();

    if events.is_empty() {
        events.push(DroneEvent::Unrecognised {
            kind: String::from(EMPTY_TURN),
        });
    }
    events
}

/// What the call was on, in the words a person would use — a path, a command,
/// a pattern.
///
/// The keys are the tools' own and the order is first-present-wins rather than
/// one arm per tool: a tool the list does not name still shows its path or its
/// command, and a tool that grows an argument does not need an arm here.
///
/// **What is deliberately not read is content.** `Write`'s `content` is not a
/// field on [`ToolInput`] at all, so the largest argument in the stream has
/// nowhere to arrive.
fn detail(input: &ToolInput) -> CallDetail {
    if let Some(file_path) = &input.file_path {
        let path = under_home(file_path);
        return match edited(input) {
            Some(size) => CallDetail::of(&format!("{path} {size}")),
            None => CallDetail::of(&path),
        };
    }
    if let Some(command) = &input.command {
        return CallDetail::of(command);
    }
    if let Some(pattern) = &input.pattern {
        return match &input.path {
            Some(path) => CallDetail::of(&format!("{pattern} in {}", under_home(path))),
            None => CallDetail::of(pattern),
        };
    }
    if let Some(declared) = &input.context_paths {
        return CallDetail::of(&declared_as(declared));
    }
    match [&input.query, &input.url, &input.description, &input.claimed]
        .into_iter()
        .flatten()
        .next()
    {
        Some(text) => CallDetail::of(text),
        None => CallDetail::none(),
    }
}

/// A declared scope on one line.
///
/// **An empty declaration reads as `[]`, not as nothing.** A step that will
/// change nothing has declared that, and it is a different fact from a step
/// that never called — which is what a blank detail looks like. Each path goes
/// through [`under_home`] even though the tool asks for repository-relative
/// ones: what a Drone types is what a Drone types, and a row is read by people.
///
/// The join is assembled before it is bounded, unlike a command. A path list
/// is many short strings rather than one that could be the size of a file, so
/// what [`CallDetail`] throws away here is bytes serde already materialised.
fn declared_as(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::from("[]");
    }
    paths
        .iter()
        .map(|path| under_home(path))
        .collect::<Vec<String>>()
        .join(", ")
}

/// An edit's size, the way a diff states it.
///
/// The two strings are read for their line counts and **neither is carried** —
/// what an edit changed a line to is the work, not the row.
fn edited(input: &ToolInput) -> Option<String> {
    let new = input.new_string.as_deref()?;
    let old = input.old_string.as_deref().unwrap_or_default();
    Some(format!("+{} -{}", counted(new), counted(old)))
}

fn counted(text: &str) -> usize {
    text.lines().count()
}

/// A path with the operator's home elided to `~`.
///
/// Spike 3 measured the concrete leak: the session's opening event carried the
/// home path and had to be scrubbed by hand before this repository could hold
/// the capture. A row naming a file the Drone read would carry it again on
/// every call, so the two leading components of a home-shaped path are replaced
/// rather than sent.
fn under_home(path: &str) -> String {
    let mut parts = path.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(""), Some("Users") | Some("home"), Some(_)) => {
            format!("~/{}", parts.collect::<Vec<&str>>().join("/"))
        }
        _ => String::from(path),
    }
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
    ToolUse {
        id: String,
        name: String,
        /// Boxed: this is the one block with more than two fields, and an enum
        /// is as large as its largest variant.
        #[serde(default)]
        input: Box<ToolInput>,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(rename = "text")]
    Text { text: String },
    /// The model's working. **A unit variant on purpose** — the working and its
    /// signature are the largest fields in an ordinary turn, and there is no
    /// field here for either to arrive in, so this decoder cannot carry the
    /// text even by mistake.
    ///
    /// The redacted spelling is the same fact with the working withheld
    /// upstream, and reads as the same row.
    #[serde(rename = "thinking", alias = "redacted_thinking")]
    Thinking,
    #[serde(other)]
    Unnamed,
}

/// The argument keys the tools a Drone can reach use, and no others.
///
/// **A struct rather than a map**, so what a row can carry is the list below
/// rather than whatever the tool was given — a `Write` argument is a whole file
/// and `content` is deliberately not a field here.
///
/// **Most keys are one vendor's and the last two are `ipc::mcp`'s.** Leaving
/// those out is the whole of why every `mcp__armada__*` call recorded an empty
/// detail: a key with no field is a key serde drops, so Armada's own surface
/// was the one place a call said nothing about itself. A fall-through rather
/// than a missing mechanism — `fleet::transcript::row` already carries
/// `detail` and `truncated`, and would have written whatever arrived. The two
/// names are asserted against [`ipc::mcp::SCOPE_FIELDS`] and
/// [`ipc::mcp::EVIDENCE_FIELDS`] in this crate's tests, so a rename there fails
/// here rather than emptying a row again. `run_checks` is absent and stays
/// absent: it takes no arguments, so its empty detail is already the truth.
#[derive(Deserialize, Default)]
struct ToolInput {
    /// Read, Write, Edit.
    file_path: Option<String>,
    /// Bash.
    command: Option<String>,
    /// Glob, Grep.
    pattern: Option<String>,
    /// The path a pattern is searched under.
    path: Option<String>,
    /// A search.
    query: Option<String>,
    /// A fetch.
    url: Option<String>,
    /// Bash's own summary, and a subagent's task.
    description: Option<String>,
    /// Edit. Read for its line count and never carried.
    old_string: Option<String>,
    /// Edit. Read for its line count and never carried.
    new_string: Option<String>,
    /// `declare_scope`. **The one argument in the stream that is recoverable
    /// nowhere else** — no route, no log line and no store field holds what a
    /// step declared, so a drift refusal could not be read against the claim it
    /// refused.
    context_paths: Option<Vec<String>>,
    /// `submit_evidence`, and the only one of its three carried.
    ///
    /// `shown_by` and `not_claimed` are recorded in full as `StepEvidence`, so
    /// what the row is for is saying which submission this call was; three
    /// prose fields joined would be one truncated field and two elisions.
    claimed: Option<String>,
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
