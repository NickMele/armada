//! One line of a **watched one-turn call**, read for what it says about how far
//! the call has got.
//!
//! # It is not `crate::transcript`, and the difference is the question
//!
//! That module reads a Drone's session for **what the Drone did** — turns, tool
//! calls, what it was refused — and every line it cannot name is carried back
//! so nothing is dropped. This one reads a single call for **whether it is
//! still moving**, which is a much smaller question with a much smaller
//! vocabulary: five readings, and everything else on the stream is silence.
//!
//! Folding the two together was considered and is wrong in both directions. A
//! `DroneEvent` carries a speaker, a turn index and a tool call, none of which
//! a one-turn call has; and this has to name the moment the request reached the
//! vendor, which a Drone's reader has no variant for because a Drone's session
//! is a hundred of those.
//!
//! # Dropping a line is the correct behaviour here, and it is the opposite rule
//!
//! `transcript::read` never answers "nothing happened", because a caller could
//! not then tell a quiet Drone from a stream it stopped reading. **This one
//! answers [`Heard::Nothing`] constantly and that is the ordinary case**: a call
//! emits hook lines, signature deltas and per-token frames that say nothing
//! about progress, and reporting each as an unrecognised event would put
//! several hundred messages on a channel whose whole purpose is to say that
//! something is happening. What a caller here cannot tell — a quiet call from a
//! stream that stopped — it answers with the process, which it is holding.
//!
//! # Bytes enter through the one codec
//!
//! `ipc::decode`, for `crate::transcript`'s reason: gate rule five scopes the
//! untyped-JSON entry points, and calling through the codec is how this crate
//! reads a stream without becoming a third of them.

use adapter_traits::{CallProgress, Heard};
use serde::Deserialize;

/// Read one line.
///
/// [`Heard::Nothing`] for most lines, which is this module's whole difference
/// from `crate::transcript`. See its note on why that is the right answer here
/// and the wrong one next door.
///
/// The type is `adapter_traits`', because Fleet reads a stream through
/// [`ModelClient::heard`](adapter_traits::ModelClient::heard) rather than by
/// calling in here — the same seam `AgentHarness::read` already draws for a
/// Drone's transcript.
pub fn heard(line: &str) -> Heard {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Heard::Nothing;
    }
    let Ok(read) = ipc::decode::<Line>("watched call line", trimmed.as_bytes()) else {
        return Heard::Nothing;
    };
    match read {
        Line::Assistant(message) => match text_of(&message) {
            // A turn that said nothing is not an answer, and reporting one as
            // an empty answer would let a call that printed nothing look like a
            // call that answered. The runner's `SaidNothing` names that, and
            // it is the runner's to raise.
            said if said.is_empty() => Heard::Nothing,
            said => Heard::Answer(said),
        },
        Line::System(system) => moved(match system.subtype.as_str() {
            // The harness is up and has told us what it came up holding. **The
            // one that separates a hung harness from a slow model**: without
            // it, a call that never reached the vendor and a call the vendor is
            // thinking hard about are the same elapsed count.
            "init" => Some(CallProgress::Started),
            // The question is at the vendor. Everything after this is the
            // model's time rather than the harness's, which is the distinction
            // a person deciding whether to keep waiting is making.
            "status" if system.status.as_deref() == Some("requesting") => {
                Some(CallProgress::Requesting)
            }
            // Cumulative within the call, and the harness's own estimate — the
            // field says so in its name and `CallProgress::Thinking` carries
            // that forward rather than rounding it into a fact.
            "thinking_tokens" => system
                .estimated_tokens
                .map(|tokens| CallProgress::Thinking { tokens }),
            _ => None,
        }),
        // The answer is being written. **Counted, not carried**: the text
        // itself arrives whole on the `assistant` line, and a progress channel
        // that also carried it would be a second copy of the answer arriving
        // before the answer.
        Line::StreamEvent(event) => moved(match event.event.delta {
            Some(Delta {
                text: Some(text), ..
            }) => Some(CallProgress::Answering {
                characters: text.chars().count() as u64,
            }),
            _ => None,
        }),
        Line::Result(result) => moved(Some(CallProgress::Ended {
            cost_usd: result.total_cost_usd,
            api_ms: result.duration_api_ms,
        })),
        Line::Unnamed => Heard::Nothing,
    }
}

/// A progress reading, or nothing. One place, so no arm above spells the
/// absence its own way.
fn moved(progress: Option<CallProgress>) -> Heard {
    match progress {
        Some(progress) => Heard::Moved(progress),
        None => Heard::Nothing,
    }
}

/// Every text block on a turn, joined. **Text only**: a thinking block is the
/// model's working rather than its answer, and `crate::transcript` states at
/// length why Armada does not read one.
fn text_of(message: &AssistantLine) -> String {
    message
        .message
        .content
        .iter()
        .filter_map(|block| match block {
            Block::Text { text } => Some(text.as_str()),
            Block::Other => None,
        })
        .collect::<Vec<&str>>()
        .join("")
        .trim()
        .to_string()
}

/// What a watched call prints, as far as this cares.
///
/// **Four named shapes and a catch-all**, where `crate::transcript::Line` has
/// six: there is no `user` arm and no rate-limit arm, because neither says
/// anything about a one-turn call — a call Armada never injects a turn into
/// cannot receive one, and a quota window moving is a fact about the machine
/// rather than about this call.
#[derive(Deserialize)]
#[serde(tag = "type")]
enum Line {
    #[serde(rename = "assistant")]
    Assistant(AssistantLine),
    #[serde(rename = "system")]
    System(SystemLine),
    #[serde(rename = "stream_event")]
    StreamEvent(StreamEventLine),
    #[serde(rename = "result")]
    Result(ResultLine),
    #[serde(other)]
    Unnamed,
}

/// The harness's own machinery talking. Told apart by `subtype`, exactly as
/// `crate::transcript` tells one apart.
#[derive(Deserialize)]
struct SystemLine {
    subtype: String,
    /// Present on `status` and nowhere else.
    #[serde(default)]
    status: Option<String>,
    /// Present on `thinking_tokens`. The cumulative estimate, not the delta —
    /// the stream carries both and the delta is only useful to something
    /// keeping a running total, which this is not.
    #[serde(default)]
    estimated_tokens: Option<u64>,
}

/// The model's turn, whole. **The answer's one arrival** — see
/// [`Heard::Answer`] for why it is read here rather than reassembled from the
/// frames below.
#[derive(Deserialize)]
struct AssistantLine {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    #[serde(default)]
    content: Vec<Block>,
}

/// One block of a turn. Text is the answer; everything else is named `Other`
/// and dropped, which is the opposite of `crate::transcript`'s rule and for
/// this module's stated reason.
#[derive(Deserialize)]
#[serde(tag = "type")]
enum Block {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

/// One frame of the model's own streamed message.
#[derive(Deserialize)]
struct StreamEventLine {
    event: StreamEvent,
}

#[derive(Deserialize)]
struct StreamEvent {
    #[serde(default)]
    delta: Option<Delta>,
}

/// A content delta. **Only `text_delta` is read**: a `thinking_delta` carries
/// the model's working, which `docs/scope.md` records as the thing Armada
/// exists to escape reading, and the token count beside it is already reported
/// through the `thinking_tokens` line without the text.
#[derive(Deserialize)]
struct Delta {
    #[serde(default)]
    text: Option<String>,
}

/// The terminating line. **Both fields are optional** because a call that was
/// killed never prints one at all, and a harness that changes what it reports
/// should narrow the readout rather than fail the read.
#[derive(Deserialize)]
struct ResultLine {
    #[serde(default)]
    total_cost_usd: Option<f64>,
    #[serde(default)]
    duration_api_ms: Option<u64>,
}
