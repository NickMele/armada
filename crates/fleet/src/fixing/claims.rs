//! The claims a Job is part of: read for its detail, given back when it ends.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId};

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
    /// Every claimed fix a Job is part of, from either side, for its detail.
    /// A reporter that has been forgotten keeps its id and loses its title.
    pub(crate) async fn breakages_of(
        &self,
        job: &Job,
    ) -> Result<Vec<ipc::ClaimedBreakage>, Adrift> {
        let store = self.store().lock().await;
        let mut claims = store
            .breakages_claimed_by(job.id())
            .map_err(Adrift::Reading)?;
        claims.extend(
            store
                .breakages_reported_by(job.id())
                .map_err(Adrift::Reading)?,
        );
        let title = |id: &JobId| {
            store
                .load_job(id)
                .ok()
                .map(|found| found.title().as_str().to_string())
        };
        Ok(claims
            .into_iter()
            .map(|claim| ipc::ClaimedBreakage {
                fix_title: title(&claim.fix).unwrap_or_default(),
                reported_by_title: title(&claim.reported_by),
                fix: ipc::JobId::from(&claim.fix),
                reported_by: ipc::JobId::from(&claim.reported_by),
                check: claim.breakage.check,
                test: claim.breakage.test,
                failure: claim.breakage.failure,
            })
            .collect())
    }

    /// Give back every breakage a Job that just ended was claiming.
    pub(crate) async fn released_breakages(&self, job: &Job) {
        let _ = self.store().lock().await.release_breakages(job.id());
    }
}
