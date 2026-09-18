//! A scout on the wire: what a Finding records of its run, and the three acts
//! a client asks of one. `#1292`, `docs/concepts/scout.md`.
//!
//! **Two ways to start one, and they are two operations.** `ask_scout` is a
//! person typing an ask on a Studio, and makes the Finding as it starts it;
//! `start_scout` starts a Finding already Proposed — the one Helm may add
//! unasked — and is what Helm calls once a person asks it to.

use serde::{Deserialize, Serialize};

use crate::enums::ScoutSourceKind;
use crate::ids::StudioNodeId;
use crate::studio::{StudioNodeContent, StudioPosition};

/// Which state of the repository a scout read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutCheckout {
    /// The commit checked out when the scout started.
    pub commit: String,
    /// Whether anything uncommitted sat on top of it, untracked files included.
    pub uncommitted: bool,
}

/// A source Fleet fetched and handed a scout beyond the checkout. `#1293`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutSource {
    /// The Link's address, as the person pasted it.
    pub address: String,
    pub kind: ScoutSourceKind,
    /// Characters dropped from the end, where the source did not fit. `0`
    /// where the whole of it was handed over.
    pub cut: u64,
}

/// `read_in_link`: a Link on the Studio, read in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadInLink {
    /// The Link node whose address is read. It survives, and what comes back
    /// hangs off it by `produced` edges.
    pub node_id: StudioNodeId,
    /// Where the first node it makes is placed. The rest are laid out from
    /// there, because a Studio is laid out by hand and a stack is not a layout.
    ///
    /// **An Epic already read in keeps its own origin and this is ignored** —
    /// `#1405`. One Epic has one block of issues for its life, so widening it
    /// fills the gaps in that block rather than starting a second one wherever
    /// the person happens to be looking.
    pub position: StudioPosition,
    /// Which of an Epic's issues to take. **Absent on every other kind**,
    /// which has one thing to read and nothing to ask about; absent from an
    /// older peer, and read as every issue, which is what it used to do.
    /// `#1405`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take: Option<crate::studio::EpicTake>,
}

/// How a scout ended, and what it cost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutEnded {
    #[serde(flatten)]
    pub outcome: ScoutOutcome,
    /// Millionths of a dollar. **Absent where the agent reported none**, which
    /// a process ended before its turn did not — never zero in its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// `answered`, `stopped` by a person, or `failed` with Fleet's sentence why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ScoutOutcome {
    Answered,
    Stopped,
    Failed { why: String },
}

/// `ask_scout`: a person's ask, made a Finding on the Studio and started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskScout {
    /// What the person asked, verbatim. The scout is told it as it is.
    pub asked: String,
    pub position: StudioPosition,
    /// The node the ask was made from. The Studio draws the `produced` edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_by: Option<StudioNodeId>,
}

/// `start_scout`: a Proposed Finding already on the Studio, started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartScout {
    pub node_id: StudioNodeId,
}

/// `stop_scout`: the stop on a Gathering Finding's node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopScout {
    pub node_id: StudioNodeId,
}

impl ScoutCheckout {
    pub fn to_domain(self) -> core_model::ScoutCheckout {
        core_model::ScoutCheckout {
            commit: self.commit,
            uncommitted: self.uncommitted,
        }
    }
}

impl ScoutSource {
    pub fn to_domain(self) -> core_model::ScoutSource {
        core_model::ScoutSource {
            address: self.address,
            kind: self.kind.domain(),
            cut: self.cut,
        }
    }
}

impl ScoutEnded {
    pub fn to_domain(self) -> core_model::ScoutEnded {
        core_model::ScoutEnded {
            outcome: match self.outcome {
                ScoutOutcome::Answered => core_model::ScoutOutcome::Answered,
                ScoutOutcome::Stopped => core_model::ScoutOutcome::Stopped,
                ScoutOutcome::Failed { why } => core_model::ScoutOutcome::Failed { why },
            },
            cost_micros: self.cost_micros,
        }
    }
}

impl StudioNodeContent {
    /// A Finding nobody has started: its ask, and nothing a scout records.
    pub fn finding_asked(asked: &str) -> StudioNodeContent {
        finding_on_the_wire(&core_model::StudioFinding::asked(asked))
    }
}

/// A Finding's content as the wire carries it.
pub(crate) fn finding_on_the_wire(finding: &core_model::StudioFinding) -> StudioNodeContent {
    StudioNodeContent::Finding {
        asked: finding.ask().to_string(),
        checkout: finding.checkout().map(|checkout| ScoutCheckout {
            commit: checkout.commit.clone(),
            uncommitted: checkout.uncommitted,
        }),
        sources: finding
            .sources()
            .iter()
            .map(|source| ScoutSource {
                address: source.address.clone(),
                kind: source.kind.into(),
                cut: source.cut,
            })
            .collect(),
        read: finding.read().to_vec(),
        searched: finding.searched().to_vec(),
        learned: finding.learned().map(str::to_string),
        ended: finding.ended().map(|ended| ScoutEnded {
            outcome: match &ended.outcome {
                core_model::ScoutOutcome::Answered => ScoutOutcome::Answered,
                core_model::ScoutOutcome::Stopped => ScoutOutcome::Stopped,
                core_model::ScoutOutcome::Failed { why } => {
                    ScoutOutcome::Failed { why: why.clone() }
                }
            },
            cost_micros: ended.cost_micros,
        }),
    }
}
