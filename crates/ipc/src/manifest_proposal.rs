//! A possible `armada.yml` per workspace, built from Scan, as lines that each carry their
//! source. No edit names a provenance, so provenance stays a record.

use serde::{Deserialize, Serialize};

use crate::editing::ManifestSaved;
use crate::reading::ManifestRefused;

/// `get_manifest_proposals`: one proposal per workspace Scan found, root first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestProposals {
    pub checkout: String,
    pub proposals: Vec<ManifestProposal>,
    /// Stated at Setup and never written: no Job has run here to set them against.
    pub caps: StatedCaps,
}

/// What a Job stops at on this machine when its Manifest says nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatedCaps {
    pub cost_micros: u64,
    pub turns: u64,
}

/// One workspace's proposal, as far as it has been iterated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestProposal {
    /// `.` for the root. What an edit names.
    pub dir: String,
    /// Where the file would be, relative to the checkout.
    pub file: String,
    pub id: ProposedId,
    pub ports: Vec<ProposedPort>,
    /// In written order, which is the order the gate starts them in.
    pub checks: Vec<ProposedCheck>,
    pub commands: Vec<ProposedCommand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<ProposedSetup>,
    /// Always `auto_merge`, then `review_gate`.
    pub policy: Vec<ProposedPolicy>,
    /// The file Write would put down, from `config`'s writer. Absent where it is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Every fault that stops the file loading. Write refuses while this is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<ManifestRefused>,
    /// Absent until Write lands it; after that the proposal takes no edits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written: Option<ManifestSaved>,
    /// An `armada.yml` is at `file` now — one already set up, or one Write put down.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub present: bool,
}

/// Where a line came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum Provenance {
    /// A file said this, at `key`.
    Read {
        file: String,
        key: String,
    },
    /// A file said this, and where it was placed is a guess. `key` is absent where the
    /// whole file is the evidence.
    Convention {
        file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },
    /// No file corresponds: the parser's value for an absent key, which Write leaves absent.
    Default,
    EditedDuringSetup,
    AddedDuringSetup,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedId {
    pub value: String,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedPort {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<u16>,
    /// Absent for a compose service, which gets its port without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedCheck {
    pub name: String,
    pub run: String,
    /// Command names, in the order they run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    /// Which runner drives this Check, where the scan's own evidence named
    /// one. **`None` is every other case** — a Check nothing detected a runner
    /// for runs whole, exactly as one in a repository nobody has configured
    /// does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<ProposedRunner>,
    pub provenance: Provenance,
}

/// The runner one proposed Check names, and the package it runs in.
///
/// **Two fields and no commands.** Every way of running less than the whole
/// Check is written once in that runner's own description, never per Check —
/// `docs/concepts/runner-adapter.md`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedRunner {
    pub name: String,
    /// What `{pkg}` resolves to in that description's templates. **`None` on a
    /// workspace that is the repository root**, where there is no package
    /// below it to name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pkg: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedCommand {
    pub name: String,
    pub run: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub destructive: bool,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedSetup {
    pub requires: Vec<String>,
    pub provenance: Provenance,
}

/// `value` is in force either way. A `default` row is an absent key, so the file follows
/// the default if it moves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedPolicy {
    pub key: PolicyKey,
    pub value: String,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKey {
    AutoMerge,
    ReviewGate,
}

/// `edit_manifest_proposal`: one change to one workspace's proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditManifestProposal {
    pub dir: String,
    pub edit: ProposalEdit,
}

/// A put replaces the line it names or adds one. No variant carries provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "edit", rename_all = "snake_case")]
pub enum ProposalEdit {
    Id {
        id: String,
    },
    Port {
        name: String,
        #[serde(default)]
        container: Option<u16>,
        #[serde(default)]
        env: Option<String>,
    },
    Check {
        name: String,
        run: String,
        #[serde(default)]
        requires: Vec<String>,
    },
    Command {
        name: String,
        run: String,
        #[serde(default)]
        destructive: bool,
    },
    /// Empty removes the row.
    Setup {
        requires: Vec<String>,
    },
    /// Absent `value` goes back to the default.
    Policy {
        key: PolicyKey,
        #[serde(default)]
        value: Option<String>,
    },
    /// A Check to the Commands, or a Command to the Checks.
    Move {
        name: String,
    },
    Remove {
        band: Band,
        name: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    Ports,
    Checks,
    Commands,
}

/// `write_manifest_proposal`: one workspace's proposal, put on disk as it stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteManifestProposal {
    pub dir: String,
}
