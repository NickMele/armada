//! What a step's own history says before its next look sees new work. Job
//! `01M28RVVN200232YNHWF8CFFKH`'s mid-step look answered `converging` three
//! times across six attempts of the same step, each going on to fail the same
//! Check — a finding the look had no way to check itself against.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Attempt, JobId, StepCheck, StepId};

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
    /// What the immediately preceding attempt of this step did not let it
    /// advance on. Empty on a first attempt, and empty where the store will
    /// not read — a look grounded in nothing is the look this had before.
    pub(crate) async fn precedent_failures(&self, job: &JobId, step: &StepId) -> Vec<StepCheck> {
        let store = self.store().lock().await;
        let Ok(current) = store.step_attempt(job, step) else {
            return Vec::new();
        };
        if current == Attempt::FIRST {
            return Vec::new();
        }
        let Ok(every) = store.step_checks_every_attempt(job) else {
            return Vec::new();
        };
        every
            .into_iter()
            .filter(|run| run.step_id == *step && run.attempt < current)
            .next_back()
            .into_iter()
            .flat_map(|run| run.record)
            .filter(|check| !check.outcome.advances())
            .collect()
    }
}
