//! A result told the Drone as it lands, while the rest of its run goes on.
//! #1062.
//!
//! **Never the last turn.** A result that ends the run — the last to finish,
//! or the failure that stops the rest — is in the report
//! [`Fleet::dry_run_ends`] sends, so no result is told twice.

use std::future::Future;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::JobId;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{ChecksReported, HEADING};
use crate::checking::Completed;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};
use crate::underway::Landed;

impl ChecksReported {
    /// One result, landed while others still run.
    pub(super) fn landed(landed: &Landed) -> ChecksReported {
        let came_to = landed
            .ran
            .as_ref()
            .map_or("finished", |run| run.outcome.as_wire())
            .replace('_', " ");
        let still = landed
            .still
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        ChecksReported(format!(
            "{HEADING}\n\n`{}` {came_to} in {:.1}s. Still going: {still}. Each result \
             arrives as its own turn, and the last one says the run is over.",
            landed.name,
            landed.took.as_secs_f64(),
        ))
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
    /// Run `running`, telling the Drone each result that lands before it ends.
    pub(super) async fn heard_while(
        &self,
        caller: &JobId,
        run: u64,
        running: impl Future<Output = Vec<Completed>>,
        mut hearing: UnboundedReceiver<Landed>,
    ) -> Vec<Completed> {
        tokio::pin!(running);
        let completed = loop {
            tokio::select! {
                biased;
                Some(landed) = hearing.recv() => self.told_landed(caller, run, &landed).await,
                completed = &mut running => break completed,
            }
        };
        // A result heard just before the run ended still goes before the report.
        while let Ok(landed) = hearing.try_recv() {
            self.told_landed(caller, run, &landed).await;
        }
        completed
    }

    /// Tell the Drone one result, only while its run is still the one in flight.
    async fn told_landed(&self, caller: &JobId, run: u64, landed: &Landed) {
        let Some(slot) = self.slot_of(caller).await else {
            return;
        };
        let working = slot.lock().await;
        let Some(at_work) = working
            .as_ref()
            .filter(|at_work| at_work.checks_in_flight(run))
        else {
            return;
        };
        let told = ChecksReported::landed(landed);
        // Written down before the send, `Fleet::tell`'s order.
        at_work.instructed(Occasion::Checks, told.text());
        let _ = at_work.session().checks(&told).await;
    }
}
