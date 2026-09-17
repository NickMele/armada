//! A scout on the wire: what a Finding records of its run, and the three acts
//! a client asks of one. `#1292`, `docs/concepts/scout.md`.
//!
//! **Two ways to start one, and they are two operations.** `ask_scout` is a
//! person typing an ask on a Studio, and makes the Finding as it starts it;
//! `start_scout` starts a Finding already Proposed — the one Helm may add
//! unasked — and is what Helm calls once a person asks it to.

use serde::{Deserialize, Serialize};

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
