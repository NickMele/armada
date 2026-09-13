//! A possible `armada.yml` per workspace, built from what Scan read — the
//! *Proposal* and *Write* steps of the Setup journey.
//!
//! **Every line says where it came from, and a person cannot say otherwise.**
//! [`Provenance`] is on each line and on no edit: an edit moves it to
//! `edited_during_setup` or `added_during_setup`, and nothing on this wire sets
//! it directly. A record, not a value.
//!
//! **Placement is a guess; a port is not.** A compose file proves a port, so a
//! port line is `read`. Nothing proves a script gates code, so every script
//! line is `convention`, whichever registry it landed in.
//!
//! **Not the Job proposer's.** [`ProposeJob`](crate::ProposeJob) is a Job on its
//! way to the gate; this is a file on its way to disk, and no Job exists.

use serde::{Deserialize, Serialize};

use crate::editing::ManifestSaved;
use crate::reading::ManifestRefused;

/// `get_manifest_proposals`: one proposal per workspace Scan found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestProposals {
    /// The checkout the proposals are for, as Scan spelled it.
    pub checkout: String,
    /// The root first, then by path — Scan's order.
    pub proposals: Vec<ManifestProposal>,
    /// The ceilings a Job here would stop at. **Stated, never written**: at
    /// Setup no Job has run here, so a control would ask for a guess.
    pub caps: StatedCaps,
}

/// What a Job stops at on this machine when its Manifest says nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatedCaps {
    pub cost_micros: u64,
    pub turns: u64,
}

/// A possible `armada.yml` for one workspace, as far as it has been iterated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestProposal {
    /// The workspace, `.` for the root — what an edit and a Write name.
    pub dir: String,
    /// Where Write puts the file, relative to the checkout.
    pub file: String,
    /// The header, not a row: what every Job here is keyed to once written.
    pub id: ProposedId,
    /// First, because the lists below read them.
    pub ports: Vec<ProposedPort>,
    /// In the order they would be written, which is the order the gate starts
    /// them in.
    pub checks: Vec<ProposedCheck>,
    pub commands: Vec<ProposedCommand>,
    /// `setup.requires`, **absent where nothing runs first**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<ProposedSetup>,
    /// `auto_merge` and `review_gate`, always both.
    pub policy: Vec<ProposedPolicy>,
    /// The file Write would put down, exactly.
    pub text: String,
    /// What `config` refuses in [`text`](Self::text), key by key, **absent
    /// where it loads**. Write refuses while this is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<ManifestRefused>,
    /// **Absent until Write lands it.** After that the file is the thing to
    /// edit, and the proposal takes no more edits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written: Option<ManifestSaved>,
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
    /// A file said this, and where it was put is a guess — a script placed as
    /// a Check or a Command, or a command a lockfile's tool conventionally
    /// takes. `key` is absent where the whole file is the evidence.
    Convention {
        file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },
    /// Nothing in a repository corresponds to it: the parser's own value for
    /// an absent key, which writes nothing.
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
    /// Commands this proposal declares, in the order they run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    pub provenance: Provenance,
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

/// One policy row. `value` is what is in force either way; **`default` as the
/// provenance is an absent key**, so the file follows the default if it moves.
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

/// **No provenance on any of these**, which is what keeps it a record. A put
/// naming a line that exists replaces it; one naming nothing adds it.
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
    /// Absent `value` goes back to the default, and writes nothing.
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

/// `write_manifest_proposal`: put one workspace's proposal on disk, as it
/// stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteManifestProposal {
    pub dir: String,
}
