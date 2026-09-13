//! A possible `armada.yml` per workspace, built from Scan and iterated before Write.
//! Not `crate::proposing`, which proposes Jobs.

use std::path::PathBuf;

use ipc::{
    ManifestProposal, ManifestSaved, PolicyKey, ProposedCheck, ProposedCommand, ProposedId,
    ProposedPolicy, ProposedPort, ProposedSetup, Provenance, RepositoryScan,
};

mod amending;
mod building;
mod held;
mod writing;

#[cfg(test)]
mod tests;

pub use amending::NotAmended;
pub(crate) use held::Held;

/// What `config` reads an absent `auto_merge` as. Its word list is private, so a test
/// holds this to the parser.
pub const AUTO_MERGE_DEFAULT: &str = "never";
/// What `config` reads an absent `review_gate` as.
pub const REVIEW_GATE_DEFAULT: &str = "human_always";

/// One workspace's proposal. Held in memory and never stored: it should not outlive its Scan.
#[derive(Clone, Debug)]
pub struct Draft {
    dir: String,
    file: String,
    /// Where Write puts the file, and what a refusal cites.
    path: PathBuf,
    id: ProposedId,
    ports: Vec<ProposedPort>,
    checks: Vec<ProposedCheck>,
    commands: Vec<ProposedCommand>,
    setup: Option<ProposedSetup>,
    policy: Vec<ProposedPolicy>,
    written: Option<ManifestSaved>,
}

/// A proposal per workspace, from the findings alone, so every line carries a file.
pub fn propose(scan: &RepositoryScan) -> Vec<Draft> {
    building::drafts(scan)
}

impl Draft {
    pub fn dir(&self) -> &str {
        &self.dir
    }

    pub fn answer(&self) -> ManifestProposal {
        let (text, refused) = match self.text() {
            Ok(text) => (Some(text), None),
            Err(refused) => (None, Some(refused)),
        };
        ManifestProposal {
            dir: self.dir.clone(),
            file: self.file.clone(),
            id: self.id.clone(),
            ports: self.ports.clone(),
            checks: self.checks.clone(),
            commands: self.commands.clone(),
            setup: self.setup.clone(),
            policy: self.policy.clone(),
            text,
            refused,
            written: self.written.clone(),
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
