//! One Manifest as Fleet resolved it, past the summary a picker reads.
//!
//! **`ManifestSummary` names it; this says what it declares.** The split is
//! `get_job`'s against `list_jobs`: a list is read to choose from and this is
//! read once, by somebody asking what a Job in this repository will be held to.

use serde::{Deserialize, Serialize};

use crate::ids::ManifestId;
use crate::setup::ManifestSummary;

/// What an `armada.yml` declares, resolved.
///
/// **What it does not carry: the resolution chain.** `operations.toml` records
/// that the Kit/Machine/Manifest chain is not yet stated anywhere, so every
/// value below is the Manifest's own reading and none of it claims a tier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestConfig {
    pub id: ManifestId,
    /// The naming half, so a caller holding this needs no second call.
    pub summary: ManifestSummary,
    /// The branch worktrees are cut from, where the Manifest names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// The Checks as the file writes them, before any resolution — the order a
    /// gate runs them in.
    pub checks_as_written: Vec<String>,
    /// The Checks re-run after a merge, by name. Empty is the ordinary case.
    pub proved_after_a_merge: Vec<String>,
    /// Commands a Drone may be granted, by name.
    pub commands: Vec<String>,
    /// Servers the Manifest declares, by name. What `start_server` may name.
    pub servers: Vec<String>,
    /// Port spans the Manifest names, which a server's `${port.NAME}` resolves
    /// through.
    pub ports: Vec<String>,
    /// Paths no Job here may write, as the file spells them.
    pub exclude_paths: Vec<String>,
    /// The repository tier's cost ceiling per Job, where it states one.
    /// **Absent defers to the machine**, and is not a ceiling of zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_cap_micros: Option<u32>,
    /// The repository tier's turn ceiling per Job, on the same rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_cap: Option<u32>,
    /// How long a Drone may say nothing before Fleet pokes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_after_seconds: Option<u32>,
    /// How many pokes before Fleet stops poking and escalates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poke_limit: Option<u32>,
}
