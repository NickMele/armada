//! A Job's plan, read for the wire. `store::work_plan` is the record; this is
//! what every summary and detail asks of it, so none of them reads it its own way.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{JobId, WorkPlan};

use crate::adrift::Adrift;
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
    /// The plan this Job's history leaves, or `None` where none was recorded.
    pub(crate) async fn plan_of(&self, job: &JobId) -> Result<Option<WorkPlan>, Adrift> {
        self.store()
            .lock()
            .await
            .work_plan(job)
            .map_err(Adrift::Reading)
    }

    /// A row's task counts. **Absent is a Job with no plan**, which a Board
    /// draws as no task field rather than an empty one.
    pub(crate) async fn task_counts(&self, job: &JobId) -> Result<Option<ipc::TaskCounts>, Adrift> {
        Ok(self.plan_of(job).await?.map(|plan| plan.counts().into()))
    }
}
