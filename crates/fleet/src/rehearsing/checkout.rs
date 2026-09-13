//! The main checkout's entrances — Journey 9, *Running one*.
//!
//! **Everything below is a projection.** The run itself is
//! [`super`]'s, keyed on [`Owner::Checkout`](super::owner::Owner); what is
//! here is the sheet the Manifest surface reads, and the four shapes an answer
//! leaves in.
//!
//! **No Where control, and no throwaway copy.** A run from this surface
//! executes in the working tree as it is on disk — a copy would be a worktree
//! by another name, and a Job's own run sheet is already that tree.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;

use super::entries;
use super::owner::Place;
use super::record::Record;
use super::records;
use super::unrehearsable::Unrehearsable;
use crate::daemon::Fleet;

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
    /// What the Manifest surface lists for this repository.
    ///
    /// **It needs no Job and reads no store.** This is the file Fleet is
    /// already holding, which is why it answers before any Job exists.
    pub(crate) async fn checkout_run_sheet(&self) -> Result<ipc::CheckoutRunSheet, Refusal> {
        let place = Place::of_checkout();
        let tree = self.tree_at(&place);
        let manifest = self.manifest().clone();
        let listed = entries::declared(&manifest);
        let (setup, checks, commands) = listed.sheet(&[]);
        Ok(ipc::CheckoutRunSheet {
            setup,
            checks,
            commands,
            manifest_edited_at: self.edited_before(&self.now(), tree.as_ref()),
            running: self
                .rehearsals()
                .in_flight(&place.owner)
                .map(|out| out.of_checkout()),
            servers: self.declared_servers(&crate::servers::Holder::MainCheckout, &manifest),
            verify: self.rehearsals().verifies().seen(),
        })
    }

    /// Start one run in the main checkout, and answer as soon as it is
    /// underway.
    pub(crate) async fn start_checkout_rehearsal(
        self: Arc<Self>,
        asked: ipc::StartCheckoutRun,
    ) -> Result<ipc::CheckoutRunUnderway, Refusal> {
        let place = Place::of_checkout();
        let owner = place.owner.clone();
        // A Verify holds the checkout between its steps too: a run slipped in
        // there would take the slot its next step is about to be handed.
        if self.rehearsals().verifies().underway() {
            return Err(self.refused_run(&owner, Unrehearsable::VerifyUnderway));
        }
        // The whole tree, always: there is no diff of the checkout's own for a
        // narrowing to measure against, and `StartCheckoutRun` carries no flag
        // asking for one.
        let (entry, tree) = self
            .entry_at(&place, &asked.name, false)
            .await
            .map_err(|why| self.refused_run(&owner, why))?;
        let command = entry.run.clone();
        let underway = Arc::clone(&self)
            .started_at(place, tree, entry, command, false, false, None)
            .await
            .map_err(|why| self.refused_run(&owner, why))?;
        Ok(underway.of_checkout())
    }

    /// End the checkout's run, and answer with its record once it is written.
    pub(crate) async fn stop_checkout_rehearsal(
        &self,
        id: String,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        let place = Place::of_checkout();
        self.stopped_at(&place, id)
            .await
            .map(|record| record.of_checkout())
            .map_err(|why| self.refused_run(&place.owner, why))
    }

    /// Put back what one run changed, from the snapshot taken before it.
    ///
    /// **This tree holds a person's own uncommitted work**, which no Job's
    /// worktree does — so a run that took no snapshot is refused here rather
    /// than discarded from, which is [`super`]'s rule for both owners.
    pub(crate) async fn undo_checkout_rehearsal(
        &self,
        id: String,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        let place = Place::of_checkout();
        if self.rehearsals().verifies().underway() {
            return Err(self.refused_run(&place.owner, Unrehearsable::VerifyUnderway));
        }
        self.undone_at(&place, id)
            .await
            .map(|record| record.of_checkout())
            .map_err(|why| self.refused_run(&place.owner, why))
    }

    /// What one run changed, against the snapshot taken just before it.
    ///
    /// **Never against `HEAD`, and never a fallback to it.** This tree holds a
    /// person's own uncommitted work, which a patch against `HEAD` would show
    /// as the run's; a snapshot that is gone is answered as gone. A read.
    pub(crate) async fn checkout_rehearsal_diff(
        &self,
        id: String,
    ) -> Result<ipc::CheckoutRunDiff, Refusal> {
        let place = Place::of_checkout();
        let refused = |why| self.refused_run(&place.owner, why);
        if self
            .rehearsals()
            .in_flight(&place.owner)
            .is_some_and(|out| out.id == id)
        {
            return Err(refused(Unrehearsable::StillRunning { id }));
        }
        let (root, handle) = (&self.host().records_root, place.handle.as_str());
        let record = match records::read(root, handle, &id) {
            Some(Ok(record)) => record,
            Some(Err(why)) => return Err(refused(Unrehearsable::DiffUnreadable { why })),
            None => return Err(self.no_such_run(&place, id)),
        };
        let answered = |reading| ipc::CheckoutRunDiff {
            id: record.id.clone(),
            against: ipc::DiffAgainst::RunSnapshot,
            reading,
        };
        let gone = |why: &str| {
            answered(ipc::RunDiffReading::Gone {
                why: why.to_string(),
            })
        };
        let Some(reference) = record.snapshot.clone() else {
            return Ok(gone(
                record
                    .changed_unreadable
                    .as_deref()
                    .unwrap_or("no snapshot was taken before this run"),
            ));
        };
        let path = std::path::PathBuf::from(&self.host().repo_root);
        let read =
            tokio::task::spawn_blocking(move || adapters::snapshot::patch(&path, &reference)).await;
        match read {
            Ok(Ok(patch)) => Ok(answered(ipc::RunDiffReading::Read {
                files: patch.changed.iter().map(super::running::wired).collect(),
                patch: Some(patch.text).filter(|text| !text.is_empty()),
            })),
            Ok(Err(adapters::snapshot::SnapshotError::NoSuchSnapshot { .. })) => Ok(gone(
                "the snapshot this run kept is no longer in the repository",
            )),
            Ok(Err(why)) => Err(refused(Unrehearsable::DiffUnreadable {
                why: why.to_string(),
            })),
            Err(_) => Err(refused(Unrehearsable::DiffUnreadable {
                why: String::from("reading the patch did not finish"),
            })),
        }
    }

    /// The checkout's earlier runs, newest first.
    pub(crate) async fn checkout_rehearsal_history(&self) -> Result<ipc::CheckoutRunList, Refusal> {
        let (kept, unreadable) = self.history_at(&Place::of_checkout());
        Ok(ipc::CheckoutRunList {
            runs: kept.iter().map(Record::of_checkout).collect(),
            unreadable,
        })
    }

    /// One run's log. **The checkout's own runs are the allowlist**: an id
    /// that names none of them reaches no file.
    pub(crate) async fn checkout_rehearsal_output(
        &self,
        id: String,
    ) -> Result<ipc::RunOutput, Refusal> {
        let place = Place::of_checkout();
        self.output_at(&place, &id)
            .ok_or_else(|| self.no_such_run(&place, id))
    }

    /// A run's socket, resolved before it opens, for `observe_run`'s reason.
    pub(crate) async fn observe_checkout_rehearsal(
        &self,
        id: String,
    ) -> Result<api::ObservedCheckoutRun, Refusal> {
        let place = Place::of_checkout();
        let seen = self
            .observed_at(&place, &id)
            .ok_or_else(|| self.no_such_run(&place, id))?;
        Ok(api::ObservedCheckoutRun {
            id: seen.id,
            name: seen.name,
            path: seen.path,
            live: seen.live,
            history: seen.history,
            skipped: seen.skipped,
            read_to: seen.read_to,
            unreadable: seen.unreadable,
        })
    }
}
