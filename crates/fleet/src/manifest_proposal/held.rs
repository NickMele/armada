//! The proposals a Fleet holds, and the operations over them. A proposal is built the
//! first time its workspace is asked about and kept, so edits survive later reads.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{EditManifestProposal, ManifestProposal, ManifestProposals, StatedCaps, WireError};

use super::{propose, Draft};
use crate::daemon::Fleet;
use crate::scanning::{scan, Checkout};

/// An edit naming a workspace Scan does not find. A 422.
const NO_SUCH_WORKSPACE: &str = "fleet.no_such_workspace";
/// An edit that cannot apply: no such line, or a move that would drop a key. A 422.
const PROPOSAL_NOT_AMENDED: &str = "fleet.proposal_not_amended";

/// Every proposal this Fleet holds, by workspace.
#[derive(Default)]
pub(crate) struct Held(Mutex<BTreeMap<String, Draft>>);

impl Held {
    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, Draft>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_manifest_proposals`. A workspace already held is not rebuilt over.
    pub(crate) fn manifest_proposals(&self) -> ManifestProposals {
        let checkout = self.host().repo_root.clone();
        let found = scan(&checkout, &Checkout::at(&checkout));
        let mut held = self.held_proposals().lock();
        for draft in propose(&found) {
            held.entry(draft.dir.clone()).or_insert(draft);
        }
        let allowance = self.machine_allowance();
        ManifestProposals {
            proposals: found
                .workspaces
                .iter()
                .filter_map(|one| held.get(&one.dir).map(Draft::answer))
                .collect(),
            checkout,
            caps: StatedCaps {
                cost_micros: allowance.cost().count(),
                turns: allowance.turns(),
            },
        }
    }

    /// `edit_manifest_proposal`. A workspace nobody has asked about is built first.
    pub(crate) fn edit_manifest_proposal(
        &self,
        asked: EditManifestProposal,
    ) -> Result<ManifestProposal, Refusal> {
        if !self.held_proposals().lock().contains_key(&asked.dir) {
            self.manifest_proposals();
        }
        let mut held = self.held_proposals().lock();
        let Some(draft) = held.get_mut(&asked.dir) else {
            let said = format!(
                "Scan found no workspace `{}` in {}. A proposal is named by the `dir` \
                 `get_manifest_proposals` answers with, `.` for the root",
                asked.dir,
                self.host().repo_root
            );
            return Err(Refusal::Unacceptable(WireError::raised(
                NO_SUCH_WORKSPACE,
                said,
                self.run_id(),
            )));
        };
        draft.amend(asked.edit).map_err(|why| {
            Refusal::Unacceptable(WireError::raised(
                PROPOSAL_NOT_AMENDED,
                why.to_string(),
                self.run_id(),
            ))
        })?;
        Ok(draft.answer())
    }
}
