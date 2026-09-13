//! A possible `armada.yml` per workspace, from what Scan read, iterated before
//! anything is written — the *Proposal* step of the Setup journey.
//!
//! **Not [`crate::proposing`]**, which proposes Jobs. Nothing here makes a Job,
//! calls a model or reaches the gate; it turns files into the lines of a file.
//!
//! **Every line carries where it came from**, and only this module moves it:
//! [`building`] cites the file, [`amending`] moves a line a person touched to
//! *edited* or *added*. No edit on the wire names a provenance.
//!
//! **No text is produced here.** `config`'s writer (#721) is the one writer of
//! `armada.yml`; Write expresses a proposal as edits to an empty text through
//! it and puts the result down through `crate::editing::create`.
//!
//! **Held in Fleet's memory for the Fleet's life, and never written down.** A
//! proposal is a draft nobody has committed to; a store row would outlive the
//! Scan it was built from, and a restart costs one more Scan. Where a proposal
//! lives between edits was left open by the journey — [`held`] is the choice.

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
