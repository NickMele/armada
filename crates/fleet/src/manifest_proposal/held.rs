//! Fleet's side: the proposals it holds, the three operations over them, and
//! each refusal's code.
//!
//! **A proposal is built the first time its workspace is asked about and kept
//! after**, so a person's edits survive every later read. A Scan runs on each
//! read to find a workspace nobody has asked about yet; one already held is not
//! rebuilt over.

use std::collections::BTreeMap;
use std::io;
use std::sync::{Mutex, MutexGuard, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    EditManifestProposal, Instant, ManifestProposal, ManifestProposals, ManifestSaved, StatedCaps,
    WireError, WireValue, WriteManifestProposal,
};

use super::{amending::NotAmended, propose, rendering, Draft};
use crate::daemon::Fleet;
use crate::editing::{create, NotCreated};
use crate::scanning::{scan, Checkout};

/// An edit or a Write naming a workspace Scan does not find. A 422.
const NO_SUCH_WORKSPACE: &str = "fleet.no_such_workspace";
/// An edit that cannot be applied — a line that is not there, a move that would
/// drop a key. A 422: the request decoded and names something that cannot work.
const PROPOSAL_NOT_AMENDED: &str = "fleet.proposal_not_amended";
/// An edit or a second Write after Write landed. A 409: the proposal is done,
/// and what a person does next is change the file.
const PROPOSAL_WRITTEN: &str = "fleet.proposal_written";
/// A Write `config` would refuse. A 422 carrying `keys`; the faults themselves
/// are on the proposal's `refused`, where they were before Write was asked.
const PROPOSAL_REFUSED: &str = "fleet.proposal_refused";
/// A Write where a file is already at the path. A 409 carrying `on_disk`, for
/// `fleet.manifest_moved_under_the_edit`'s reason: nothing broke, and what a
/// person does next is look at what is there.
const MANIFEST_APPEARED: &str = "fleet.manifest_appeared";
/// A Write whose bytes would not go down. A 500: nothing about the request is
/// wrong.
const PROPOSAL_UNWRITABLE: &str = "fleet.proposal_unwritable";

/// Every proposal this Fleet is holding, by workspace.
#[derive(Default)]
pub(crate) struct Held(Mutex<BTreeMap<String, Draft>>);

impl Held {
    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, Draft>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Why a Write did not land.
#[derive(Debug)]
pub(crate) enum NotWritten {
    Written,
    /// The keys `config` refused, or the parser's sentence where it found no
    /// document to have keys.
    Refused(Vec<String>),
    Appeared(Option<String>),
    Unwritable(io::Error),
}

impl Draft {
    /// Put this proposal on disk as it stands — **only where `config` loads it
    /// and nothing is at the path**.
    pub(crate) fn write(&mut self, at: Instant) -> Result<ManifestSaved, NotWritten> {
        if self.written.is_some() {
            return Err(NotWritten::Written);
        }
        let text = rendering::text(self);
        if let Some(refused) = rendering::refused(&self.path, &text) {
            let keys = match refused.faults.is_empty() {
                true => vec![refused.summary],
                false => refused.faults.into_iter().map(|fault| fault.key).collect(),
            };
            return Err(NotWritten::Refused(keys));
        }
        create(&self.path, &text).map_err(|why| match why {
            NotCreated::Appeared(on_disk) => NotWritten::Appeared(on_disk),
            NotCreated::Unwritable(cause) => NotWritten::Unwritable(cause),
        })?;
        let saved = ManifestSaved {
            path: self.path.display().to_string(),
            at,
        };
        self.written = Some(saved.clone());
        Ok(saved)
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
    /// `get_manifest_proposals` — one per workspace in the checkout.
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

    /// `edit_manifest_proposal` — one edit, answered with the proposal after it.
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
                    format!(
                        "{path} was already written from this proposal; nothing was written again"
                    ),
                )),
                NotWritten::Refused(keys) => Refusal::Unacceptable(
                    raised(
                        PROPOSAL_REFUSED,
                        format!(
                            "{path} would not load, so nothing was written. What is refused is on \
                             the proposal's `refused`, at {}",
                            keys.join(", ")
                        ),
                    )
                    .with_field(
                        "keys",
                        WireValue::List(keys.into_iter().map(WireValue::Str).collect()),
                    ),
                ),
                NotWritten::Appeared(on_disk) => {
                    let said = format!(
                        "{path} is already there, so nothing was written over it. What it holds \
                         is on `on_disk` where it reads"
                    );
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

    /// Run `act` on the proposal for `dir`, building every proposal first if
    /// that one has never been asked about.
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
