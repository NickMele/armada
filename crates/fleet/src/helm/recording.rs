//! One repository's Helm session as one quotable record. `#1367`.
//!
//! **What decides an answer, in one artifact**: the repository, the authority
//! and the model; the brief as it was sent; the tools the door offered; the
//! thread with each turn's cost; and what the last poll was told. A bad answer
//! could only be reported by retyping the conversation without any of it.
//!
//! **The roster is read from the same predicate the door enforces** — `may`
//! over `api::offerable` — so the record cannot name a set the door does not.

use ipc::{
    HelmActionAuthority, HelmDebugLine, HelmDebugSaid, HelmDebugText, HelmMessage, Saw, Voice,
};

use super::reach::{may, Authority};

/// How many of the thread's own lines a record carries. **A record is pasted
/// into an issue**: past this, what it proves is drowned by what it holds.
const LINES: usize = 160;

/// How much of one piece of prose a line carries. A reply of forty thousand
/// characters is the whole artifact otherwise.
const MOST: usize = 2_000;

/// The tools the door offers a session under `authority`, by name, as the
/// inventory orders them.
pub(super) fn tools(authority: Authority) -> Vec<String> {
    api::offerable()
        .filter(|row| may(authority, row))
        .map(|row| row.operation.to_string())
        .collect()
}

/// The Machine setting as the wire spells it.
pub(super) fn authority_on_the_wire(authority: Authority) -> HelmActionAuthority {
    match authority {
        Authority::Acting => HelmActionAuthority::Acting,
        Authority::ReadOnly => HelmActionAuthority::ReadOnly,
    }
}

/// The thread's last [`LINES`] lines, and how many were left out.
///
/// **What was cut is counted, never inferred.** `dropped` is the thread's own
/// losses plus what this bound left behind, so a record that is short says by
/// how much.
pub(super) fn thread(messages: &[HelmMessage], dropped: u64) -> (Vec<HelmDebugLine>, u64) {
    let lines: Vec<HelmDebugLine> = messages.iter().filter_map(line).collect();
    let cut = lines.len().saturating_sub(LINES);
    (lines.into_iter().skip(cut).collect(), dropped + cut as u64)
}

/// How many MCP servers the session came up with, as its own last `init` said.
pub(super) fn servers(messages: &[HelmMessage]) -> Option<usize> {
    messages.iter().rev().find_map(|message| match message {
        HelmMessage::Row(shown) => match shown.row().saw {
            Saw::Started { mcp_servers, .. } => Some(mcp_servers),
            _ => None,
        },
        _ => None,
    })
}

/// One thread message as the record holds it, or nothing where it holds none.
///
/// **Prose in Armada's own voice is dropped**: that is the brief coming back
/// off the session's input channel, and the record carries the brief whole.
fn line(message: &HelmMessage) -> Option<HelmDebugLine> {
    let (at, said) = match message {
        HelmMessage::Asked(asked) => (
            asked.ts.clone(),
            HelmDebugSaid::Asked {
                text: HelmDebugText::bounded(&asked.text, MOST),
            },
        ),
        HelmMessage::Fresh(fresh) => (fresh.ts.clone(), HelmDebugSaid::Fresh),
        HelmMessage::Unanswered(unanswered) => (
            unanswered.ts.clone(),
            HelmDebugSaid::Unanswered {
                why: unanswered.why.clone(),
            },
        ),
        HelmMessage::Row(shown) => {
            let row = shown.row();
            let said = match &row.saw {
                Saw::Said { text } if row.by == Voice::Drone => HelmDebugSaid::Said {
                    text: HelmDebugText::bounded(text, MOST),
                },
                Saw::Called { tool, detail, .. } => HelmDebugSaid::Called {
                    tool: tool.clone(),
                    detail: detail.clone(),
                },
                Saw::Refused { tool, because, .. } => HelmDebugSaid::Refused {
                    tool: tool.clone(),
                    because: because.clone(),
                },
                Saw::Ended {
                    turns,
                    cost_micros,
                    refusals,
                } => HelmDebugSaid::Ended {
                    turns: *turns,
                    cost_micros: *cost_micros,
                    refusals: *refusals,
                },
                _ => return None,
            };
            (row.ts.clone(), said)
        }
        _ => return None,
    };
    Some(HelmDebugLine { at, said })
}
