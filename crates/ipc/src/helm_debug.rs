//! One repository's Helm session as one quotable record. `#1367`.
//!
//! **The error treatment's debug payload, one subject over** —
//! `docs/contracts/design-system.md`, *The debug payload*. A bad answer could
//! only be reported by retyping the conversation, without the brief, the
//! tools, the authority or the events that decided it.
//!
//! **Fields, not text**, for `errors/ErrorNotice/payload.ts`'s reason: one
//! producer writes the artifact and the expanded view renders that same
//! string. A Fleet that formatted the record and a Bridge that framed it would
//! be two producers of one artifact.
//!
//! **Read once, never polled.** `observe_helm` streams a live conversation to
//! the dock; this is one artifact taken at a moment.

use serde::{Deserialize, Serialize};

use crate::health::HelmActionAuthority;
use crate::ids::{Instant, ManifestId};
use crate::since::EventsSince;
use crate::version::ProtocolVersion;

/// Everything a person can carry to whoever could fix a bad Helm answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmDebugInfo {
    /// Which repository this conversation answers for.
    pub manifest_id: ManifestId,
    /// The checkout the session opens in. **A folder on this person's own
    /// disk**, which is why the surface says so before it is sent anywhere.
    pub checkout: String,
    /// The Machine setting, resolved — what this session may do, as
    /// `get_health` already answers it.
    pub authority: HelmActionAuthority,
    /// The model each message's process is started with.
    pub model: String,
    /// The brief as it was sent, whole. What bounds the answers.
    pub brief: String,
    /// The MCP server Armada's own door is registered under, which prefixes
    /// every name in [`tools`](HelmDebugInfo::tools) inside the session.
    pub door: String,
    /// Every tool the door offered this session, by name, as the inventory
    /// orders them. **The roster is the answer to "no tool here reads X"**,
    /// which is the commonest complaint about an answer.
    pub tools: Vec<String>,
    /// How many MCP servers the session came up with, as its own `init` line
    /// last said. **Not Armada's count**: a conversation resolves the person's
    /// own agent configuration (`#1373`), and this is what that resolved.
    /// Absent where nothing has run in this conversation yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub servers: Option<usize>,
    /// The agent CLI's own session id, where one is stored to resume. Absent
    /// on a conversation that has not answered yet, or one started fresh.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The thread, oldest first, bounded — see [`cut`](HelmDebugInfo::cut).
    pub thread: Vec<HelmDebugLine>,
    /// How many older lines the thread left out. **Said rather than implied**,
    /// the way a log tail says what it dropped: a record that is quietly short
    /// reads exactly like a session that was quiet.
    pub cut: u64,
    /// What this session's last `get_events_since` was answered, verbatim —
    /// a turn reads the events before it answers, so this is what it read.
    ///
    /// **Kept in memory as the door answers it**, so it is absent on a session
    /// that has not polled since this Fleet started rather than on one that
    /// never polled at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polled: Option<EventsSince>,
    /// Which Fleet process answered. **Fleet holds no application version**,
    /// the way Bridge holds none, so this and the protocol below are the pair
    /// that names what produced the answer.
    pub run_id: String,
    /// The protocol Fleet speaks.
    pub protocol_version: ProtocolVersion,
    /// When the record was taken, on Fleet's clock. **Taken, not raised**, for
    /// the debug payload's reason: nothing here happened at this instant.
    pub at: Instant,
}

/// One line of the thread, as the record holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmDebugLine {
    pub at: Instant,
    #[serde(flatten)]
    pub said: HelmDebugSaid,
}

/// What one line was.
///
/// **A narrowing of the thread's own vocabulary and not a second copy of it.**
/// `HelmMessage` carries what a live viewer draws; this carries what somebody
/// reading a bad answer needs — what was asked, what came back, what was
/// called, what was refused, and what the turn cost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum HelmDebugSaid {
    /// What the person typed. **Their own words**, which is half of why this
    /// record is read before it is sent.
    Asked { text: HelmDebugText },
    /// What Helm wrote back.
    Said { text: HelmDebugText },
    /// A tool it called, and what it called it on. `detail` is bounded where
    /// it is built, in `adapter_traits::CallDetail`.
    Called { tool: String, detail: String },
    /// A call the harness refused, in the harness's own wording. **What
    /// `#1389` made visible**: a call the person's settings deny, or an ask
    /// nobody answered, is why an answer stops where it does.
    Refused { tool: String, because: String },
    /// A turn's end: what it cost, how many turns it took, and how many calls
    /// were refused across it.
    Ended {
        turns: u32,
        cost_micros: u64,
        refusals: usize,
    },
    /// The stored session could not be resumed, so the reply started a new one.
    Fresh,
    /// No reply came, and why.
    Unanswered { why: String },
}

/// A piece of prose, bounded.
///
/// **The bound is the record's, not the thread's.** A thread keeps what was
/// said whole; a record is pasted into an issue, and one reply of forty
/// thousand characters would be the whole artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmDebugText {
    pub text: String,
    /// How long it was before it was cut. **Absent where nothing was cut**, so
    /// a reader holding a value can say what proportion of it they have rather
    /// than inferring it from a trailing character.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<usize>,
}

impl HelmDebugText {
    /// `text` whole where it fits in `most`, cut with its length where it does
    /// not. **Cut on a character boundary**, so a multi-byte character does
    /// not arrive in an issue body as half of itself.
    pub fn bounded(text: &str, most: usize) -> HelmDebugText {
        let length = text.chars().count();
        if length <= most {
            return HelmDebugText {
                text: text.to_string(),
                of: None,
            };
        }
        HelmDebugText {
            text: text.chars().take(most).collect(),
            of: Some(length),
        }
    }
}
