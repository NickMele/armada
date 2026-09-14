//! The claims a Job is part of, read for its detail. `crate::fixing::waiting`
//! gives them back.

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
        for reported in store
            .breakages_reported_by(job.id())
            .map_err(Adrift::Reading)?
        {
            if !claims.contains(&reported) {
                claims.push(reported);
            }
        }
        // The claims this Job is pointed at, where they still stand. #1001.
        for pointer in store
            .fixes_waited_on_by(job.id())
            .map_err(Adrift::Reading)?
        {
            let standing = store
                .breakage_claimed(&pointer.repository, &pointer.check, &pointer.test)
                .map_err(Adrift::Reading)?;
            if let Some(claim) = standing.filter(|claim| !claims.contains(claim)) {
                claims.push(claim);
            }
        }
        let title = |id: &JobId| {
            store
                .load_job(id)
                .ok()
                .map(|found| found.title().as_str().to_string())
        };
        let mut drawn = Vec::with_capacity(claims.len());
        for claim in claims {
            let waiting = store
                .waiting_on_fix(&claim.fix)
                .map_err(Adrift::Reading)?
                .into_iter()
                .filter(|pointer| {
                    pointer.check == claim.breakage.check && pointer.test == claim.breakage.test
                })
                .map(|pointer| ipc::WaitingOnFix {
                    title: title(&pointer.waiting),
                    job_id: ipc::JobId::from(&pointer.waiting),
                })
                .collect();
            drawn.push(ipc::ClaimedBreakage {
                fix_title: title(&claim.fix).unwrap_or_default(),
                reported_by_title: title(&claim.reported_by),
                fix: ipc::JobId::from(&claim.fix),
                reported_by: ipc::JobId::from(&claim.reported_by),
                check: claim.breakage.check,
                test: claim.breakage.test,
                failure: claim.breakage.failure,
                waiting,
            });
        }
        Ok(drawn)
    }
}
