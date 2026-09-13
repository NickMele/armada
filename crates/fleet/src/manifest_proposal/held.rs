//! The proposals a Fleet holds, and the operations over them. A proposal is built the
//! first time its workspace is asked about and kept, so edits survive later reads.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    EditManifestProposal, Instant, ManifestProposal, ManifestProposals, StatedCaps, WireError,
    WireValue, WriteManifestProposal,
};

use super::writing::NotWritten;
use super::{propose, Draft, NotAmended};
use crate::daemon::Fleet;
use crate::scanning::{scan, Checkout};

/// An edit or a Write naming a workspace Scan does not find. A 422.
const NO_SUCH_WORKSPACE: &str = "fleet.no_such_workspace";
/// An edit that cannot apply: no such line, or a move that would drop a key. A 422.
const PROPOSAL_NOT_AMENDED: &str = "fleet.proposal_not_amended";
/// An edit or a second Write after Write landed. A 409: the file is what to change now.
const PROPOSAL_WRITTEN: &str = "fleet.proposal_written";
/// A Write whose file would not load. A 422 carrying `faults` as `[key, fault]` pairs.
const PROPOSAL_REFUSED: &str = "fleet.proposal_refused";
/// A Write where a file is already at the path. A 409 carrying `on_disk` where it reads.
const MANIFEST_APPEARED: &str = "fleet.manifest_appeared";
/// A Write whose bytes would not go down. A 500.
const PROPOSAL_UNWRITABLE: &str = "fleet.proposal_unwritable";

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

    /// `edit_manifest_proposal`.
    pub(crate) fn edit_manifest_proposal(
        &self,
        asked: EditManifestProposal,
    ) -> Result<ManifestProposal, Refusal> {
        self.with_draft(&asked.dir, |draft| {
            draft.amend(asked.edit).map_err(|why| {
                let (code, refusal): (_, fn(WireError) -> Refusal) = match why {
                    NotAmended::Written { .. } => (PROPOSAL_WRITTEN, Refusal::IllegalMove),
                    _ => (PROPOSAL_NOT_AMENDED, Refusal::Unacceptable),
                };
                refusal(WireError::raised(code, why.to_string(), self.run_id()))
            })?;
            Ok(draft.answer())
        })
    }

    /// `write_manifest_proposal` — the file, from the proposal as it stands.
    pub(crate) fn write_manifest_proposal(
        &self,
        asked: WriteManifestProposal,
    ) -> Result<ManifestProposal, Refusal> {
        let at = Instant::from(&self.now());
        self.with_draft(&asked.dir, |draft| {
            let path = draft.path.display().to_string();
            let raised = |code, said: String| WireError::raised(code, said, self.run_id());
            draft.write(at).map_err(|why| match why {
                NotWritten::Written => Refusal::IllegalMove(raised(
                    PROPOSAL_WRITTEN,
                    format!("{path} was already written from this proposal"),
                )),
                NotWritten::Refused(refused) => {
                    let faults = refused
                        .faults
                        .iter()
                        .map(|one| {
                            WireValue::List(vec![
                                WireValue::Str(one.key.clone()),
                                WireValue::Str(one.fault.clone()),
                            ])
                        })
                        .collect();
                    let said = format!(
                        "{path} would not load, so nothing was written: {}",
                        refused.summary
                    );
                    Refusal::Unacceptable(
                        raised(PROPOSAL_REFUSED, said)
                            .with_field("faults", WireValue::List(faults)),
                    )
                }
                NotWritten::Appeared(on_disk) => {
                    let said = format!("{path} is already there, so nothing was written over it");
                    let refused = raised(MANIFEST_APPEARED, said);
                    Refusal::IllegalMove(match on_disk {
                        Some(text) => refused.with_field("on_disk", WireValue::Str(text)),
                        None => refused,
                    })
                }
                NotWritten::Unwritable(cause) => Refusal::Fault(raised(
                    PROPOSAL_UNWRITABLE,
                    format!("{path} could not be written: {cause}. Nothing was put there"),
                )),
            })?;
            Ok(draft.answer())
        })
    }

    /// Act on `dir`'s proposal, building every proposal first where it was never asked about.
    fn with_draft(
        &self,
        dir: &str,
        act: impl FnOnce(&mut Draft) -> Result<ManifestProposal, Refusal>,
    ) -> Result<ManifestProposal, Refusal> {
        if !self.held_proposals().lock().contains_key(dir) {
            self.manifest_proposals();
        }
        let mut held = self.held_proposals().lock();
        let Some(draft) = held.get_mut(dir) else {
            let said = format!(
                "Scan found no workspace `{dir}` in {}. A proposal is named by the `dir` \
                 `get_manifest_proposals` answers with, `.` for the root",
                self.host().repo_root
            );
            return Err(Refusal::Unacceptable(WireError::raised(
                NO_SUCH_WORKSPACE,
                said,
                self.run_id(),
            )));
        };
        act(draft)
    }
}
