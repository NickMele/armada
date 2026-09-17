//! Runs a Studio holds: starting one from a Studio, and keeping what it said
//! when retention comes for it. `#1289`, `docs/concepts/studio.md`.
//!
//! **A Run node references its run and copies no status.** While the run's
//! record is on disk the node is a reference, and what colour it draws is read
//! off the run — so a node cannot disagree with the run it names.
//!
//! **What a sweep would strand is taken before the sweep.** A Studio is kept
//! until a person deletes it and a run's log is not, so the moment
//! `settings.ad-hoc-run-log-retention` is about to take a run away is the
//! moment the node keeps its result and its log's last lines.
//!
//! **A Run on a Studio writes no Evidence**, the same as every run outside a
//! Job: it is `start_checkout_run`'s own rehearsal, started in the checkout.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Redirector, Refusal};
use core_model::{
    StudioAuthor, StudioEdgeId, StudioNode, StudioNodeContent, StudioNodeId, StudioRunKept,
};
use ipc::{CheckoutRunRecord, RunOutput, StartStudioRun, StudioRunStarted};

use crate::daemon::Fleet;

/// How many of a log's last lines a Run node keeps once its run is swept.
///
/// **The tail, for the reason `checks_runner` captures the tail**: a test
/// runner prints its failures last, and a runaway command prints forever, so
/// keeping the beginning keeps the part nobody started the run for.
///
/// **Nearer `check_output`'s excerpt than its reading.** A person opening a
/// run's panel is served 2,000 lines out of a file; this is carried inside the
/// Studio itself, which crosses the wire whole on every write to it and is
/// kept until a person deletes it — so it is bounded like the excerpt that
/// rides in a tool call rather than like a file somebody opened.
const LAST_LINES: usize = 200;

/// The byte bound paired with [`LAST_LINES`], for the reason `check_output`
/// pairs one with its own: a single minified line can be a whole bundle.
const LAST_BYTES: usize = 16 * 1024;

/// What a Run node keeps of a run that is about to be swept: its result, and
/// the tail of what it printed.
///
/// **Built from the record and the log, never from the node.** The node holds
/// a reference and nothing else until this is written onto it, so there is no
/// earlier copy of a status for this to disagree with.
///
/// `printed` is `None` where the log would not read, and the node keeps the
/// result with no lines under it rather than nothing at all.
pub fn kept(record: &CheckoutRunRecord, printed: Option<&RunOutput>) -> StudioRunKept {
    let (lines, whole) = match printed {
        None => (Vec::new(), false),
        Some(printed) => {
            let (window, first, _) = crate::check_output::windowed(
                printed.lines.iter().cloned(),
                LAST_LINES,
                LAST_BYTES,
            );
            (Vec::from(window), printed.whole && first == 1)
        }
    };
    StudioRunKept {
        name: record.name.clone(),
        command: record.command.clone(),
        exit_code: record.exit_code,
        expect_exit_code: record.expect_exit_code,
        stopped: record.stopped,
        duration_ms: record.duration_ms,
        lines,
        total_lines: printed
            .map(|printed| printed.total_lines)
            .unwrap_or_default(),
        whole,
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
    /// Start one run in the checkout the Studio's repository stands in, and put
    /// a Run node on the Studio for it.
    ///
    /// **The run starts first.** A node written before the run would be left
    /// naming a run that was refused — for a name nothing declares, for a
    /// server, for a run already out — and a refused start is no Run at all.
    ///
    /// **The Studio names the repository**, so nothing on this call can run an
    /// entry in a checkout the Studio does not belong to.
    pub(crate) async fn started_studio_run(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        asked: StartStudioRun,
        by: Redirector,
        within: Option<ipc::ManifestId>,
    ) -> Result<StudioRunStarted, Refusal> {
        let repository = {
            let store = self.store().lock().await;
            let graph = self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?;
            ipc::ManifestId::from(&graph.studio.manifest_id)
        };
        let checkout = self.checkout_named(Some(&repository), None)?;
        let run = Arc::clone(&self)
            .start_checkout_rehearsal(
                ipc::StartCheckoutRun {
                    name: asked.name,
                    workspace: asked.workspace,
                },
                checkout,
            )
            .await?;
        let at = self.now();
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            StudioNodeContent::Run {
                run_id: run.id.clone(),
                kept: None,
            },
            asked.position.to_domain(),
            at.clone(),
            match by {
                Redirector::Person => StudioAuthor::Person,
                Redirector::Helm => StudioAuthor::Helm,
            },
        );
        let produced_by = asked
            .produced_by
            .map(|from| (from.to_domain(), StudioEdgeId::carried(self.mint().ulid())));
        let studio = self
            .written(&studio_id, within, |store, id| {
                let produced_by = produced_by
                    .as_ref()
                    .map(|(from, edge)| (from, edge.clone()));
                store.add_studio_node(id, &node, produced_by, &at)
            })
            .await?;
        Ok(StudioRunStarted {
            studio,
            node_id: ipc::StudioNodeId::from(node.id()),
            run,
        })
    }

    /// Write what each run about to be swept said onto every Run node holding
    /// it, and name the runs the sweep must leave where they are.
    ///
    /// **Called with the runs, before they are removed.** Each log is read
    /// here while it is still on disk; after the sweep there is nothing to
    /// read and the node would be left pointing at nothing.
    ///
    /// **A run whose tail could not be written is not swept.** The alternative
    /// to holding one run's directory for another retention cycle is a node
    /// pointing at nothing, which is the failure this whole path exists
    /// against; the next run started tries it again.
    pub(crate) async fn kept_what_studios_hold(
        &self,
        root: &str,
        handle: &str,
        going: &[CheckoutRunRecord],
    ) -> Vec<String> {
        if going.is_empty() {
            return Vec::new();
        }
        let (mut touched, mut held_back) = (Vec::new(), Vec::new());
        {
            let mut store = self.store().lock().await;
            for record in going {
                let printed = crate::rehearsing::records::output(
                    root,
                    handle,
                    &record.id,
                    record.name.clone(),
                );
                let kept = kept(record, printed.as_ref());
                match store.keep_studio_run(&record.id, &kept) {
                    Ok(studios) => touched.extend(studios),
                    Err(_) => held_back.push(record.id.clone()),
                }
            }
        }
        for studio_id in touched {
            let read = {
                let store = self.store().lock().await;
                store.studio(&studio_id)
            };
            if let Ok(graph) = read {
                self.events()
                    .publish(ipc::Event::StudioChanged(ipc::Studio::of(
                        &graph,
                        &adapters::forge_named,
                    )));
            }
        }
        held_back
    }
}
