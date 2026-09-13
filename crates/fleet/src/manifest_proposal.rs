//! A possible `armada.yml` per workspace, from what Scan read, iterated and
//! then written — the *Proposal* and *Write* steps of the Setup journey.
//!
//! **Not [`crate::proposing`]**, which proposes Jobs. Nothing here makes a Job,
//! calls a model or reaches the gate; it turns files into a file.
//!
//! **Every line carries where it came from**, and only this module moves it:
//! [`building`] cites the file, [`amending`] moves a line a person touched to
//! *edited* or *added*. No edit on the wire names a provenance.
//!
//! **Held in Fleet's memory for the Fleet's life, and never written down.** A
//! proposal is a draft nobody has committed to; a store row would outlive the
//! Scan it was built from, and a restart costs one more Scan. Where a proposal
//! lives between edits was left open by the journey — [`held`] is the choice.

use std::path::PathBuf;

use ipc::{
    ManifestProposal, ManifestSaved, PolicyKey, ProposedCheck, ProposedCommand, ProposedId,
    ProposedPolicy, ProposedPort, ProposedSetup, Provenance, RepositoryScan,
};

mod amending;
mod building;
mod held;
mod rendering;

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
    /// Where Write would put it. What `config` cites a fault against, too.
    path: PathBuf,
    id: ProposedId,
    ports: Vec<ProposedPort>,
    checks: Vec<ProposedCheck>,
    commands: Vec<ProposedCommand>,
    setup: Option<ProposedSetup>,
    policy: Vec<ProposedPolicy>,
    written: Option<ManifestSaved>,
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

    /// The proposal as a surface draws it: the lines, the file they make, and
    /// what `config` would refuse in it.
    pub fn answer(&self) -> ManifestProposal {
        let text = rendering::text(self);
        ManifestProposal {
            dir: self.dir.clone(),
            file: self.file.clone(),
            id: self.id.clone(),
            ports: self.ports.clone(),
            checks: self.checks.clone(),
            commands: self.commands.clone(),
            setup: self.setup.clone(),
            policy: self.policy.clone(),
            refused: rendering::refused(&self.path, &text),
            text,
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
