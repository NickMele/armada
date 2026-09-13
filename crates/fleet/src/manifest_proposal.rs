//! A possible `armada.yml` per workspace, from what Scan read, iterated before
//! anything is written — Setup's *Proposal* step. Not [`crate::proposing`],
//! which proposes Jobs.
//!
//! **Every line carries where it came from**, and only this module moves it:
//! [`building`] cites the file, [`amending`] moves a touched line to *edited*
//! or *added*. No edit on the wire names a provenance.
//!
//! **No text is produced here.** `config`'s writer (#721) is the one writer of
//! `armada.yml`; Write goes through it and `crate::editing::create`.
//!
//! **Held in Fleet's memory for its life, never written down**: a store row
//! would outlive the Scan it came from, and a restart costs one Scan. The
//! journey left where a proposal lives open — [`held`] is the choice.

use ipc::{
    ManifestProposal, PolicyKey, ProposedCheck, ProposedCommand, ProposedId, ProposedPolicy,
    ProposedPort, ProposedSetup, Provenance, RepositoryScan,
};

mod amending;
mod building;
mod held;

#[cfg(test)]
mod tests;

pub use amending::NotAmended;
pub(crate) use held::Held;

/// What `config` reads an absent `auto_merge` as. **Restated, not imported**:
/// the word list is private to `config`, and `tests` holds this to the parser.
pub const AUTO_MERGE_DEFAULT: &str = "never";
/// What `config` reads an absent `review_gate` as, for the same reason.
pub const REVIEW_GATE_DEFAULT: &str = "human_always";

/// One workspace's proposal, as far as it has been iterated.
#[derive(Clone, Debug)]
pub struct Draft {
    dir: String,
    file: String,
    id: ProposedId,
    ports: Vec<ProposedPort>,
    checks: Vec<ProposedCheck>,
    commands: Vec<ProposedCommand>,
    setup: Option<ProposedSetup>,
    policy: Vec<ProposedPolicy>,
}

/// A proposal for every workspace in `scan`, in its order. **Reads nothing**:
/// every line comes from a finding, and a finding carries its file.
pub fn propose(scan: &RepositoryScan) -> Vec<Draft> {
    building::drafts(scan)
}

impl Draft {
    pub fn dir(&self) -> &str {
        &self.dir
    }

    /// The proposal as a surface draws it: every line, with where it came from.
    pub fn answer(&self) -> ManifestProposal {
        ManifestProposal {
            dir: self.dir.clone(),
            file: self.file.clone(),
            id: self.id.clone(),
            ports: self.ports.clone(),
            checks: self.checks.clone(),
            commands: self.commands.clone(),
            setup: self.setup.clone(),
            policy: self.policy.clone(),
        }
    }
}

fn default_policy() -> Vec<ProposedPolicy> {
    [
        (PolicyKey::AutoMerge, AUTO_MERGE_DEFAULT),
        (PolicyKey::ReviewGate, REVIEW_GATE_DEFAULT),
    ]
    .into_iter()
    .map(|(key, value)| ProposedPolicy {
        key,
        value: value.to_string(),
        provenance: Provenance::Default,
    })
    .collect()
}
