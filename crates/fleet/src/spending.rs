//! What the fleet has spent, and which Jobs a ceiling is holding.
//!
//! **Summed here and stored nowhere.** A total written down is wrong the
//! moment a Drone ends, and the per-Job rows it sums are the record.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FleetUsage, Overspending};

use crate::allowance::Overspent;
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
    /// `get_usage` — the fleet's spend, and what the ceilings are holding.
    ///
    /// **`Fleet::overspent` is asked, never re-derived.** It is admission's own
    /// predicate and the Board's `over_budget` label comes from it; a second
    /// comparison here is how a page comes to say a Job is held while Fleet is
    /// starting it.
    pub(crate) async fn usage(&self) -> Result<FleetUsage, Refusal> {
        let (loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        let mut usage = FleetUsage {
            cost_micros: 0,
            turns: 0,
            drones: 0,
            unpriced: 0,
            jobs: loaded.jobs.len() as u64,
            over_budget: Vec::new(),
        };
        for job in &loaded.jobs {
            let spent = self
                .spend_of(job.id())
                .await
                .map_err(|why| self.refusal(why))?;
            usage.cost_micros = usage.cost_micros.saturating_add(spent.cost_micros);
            usage.turns = usage.turns.saturating_add(spent.turns);
            usage.drones = usage.drones.saturating_add(spent.drones);
            usage.unpriced = usage.unpriced.saturating_add(spent.unpriced);
            let Some(over) = self.overspent(job).await.map_err(|why| self.refusal(why))? else {
                continue;
            };
            let allowed = self.allowance_for(job);
            usage.over_budget.push(Overspending {
                handle: job.handle(),
                ceiling: over.to_string(),
                spent: match over {
                    Overspent::Cost => spent.cost_micros,
                    Overspent::Turns => spent.turns,
                },
                allowed: match over {
                    Overspent::Cost => allowed.cost().count(),
                    Overspent::Turns => allowed.turns(),
                },
            });
        }
        Ok(usage)
    }
}
