//! What a Job looks like as a Board row, and why one that is queued has not
//! started.
//!
//! **Split from the trait impls, not from a line count.** `serving` and
//! `commanding` are the operations — one method per row of the inventory, each
//! answering what a caller asked. These are what several of them need, and
//! neither is an operation: `summarised` is called on every Job in a list, and
//! `queued_reason` is a read of the whole board taken to answer about one row.
//!
//! **Nothing here is stored.** A `queued` Job's reason is worked out from the
//! board as it stands, because headroom frees on its own and a reason written
//! down is wrong from the moment it is written.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{
    Job, JobId as CoreJobId, JobStatus as CoreJobStatus, QueuedReason as CoreQueuedReason,
};

use ipc::JobSummary;

use crate::admitting::clear_to_run;
use crate::daemon::Fleet;
use crate::sub_dispatch::waiting_on_children;

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
    /// A Job as a Board row, with the reason its last transition stored and
    /// whether its Drone is waiting on somebody. **The slot read is free on
    /// every row but the working ones** — `question_awaited` answers `None`
    /// without touching the store for a Job that holds no slot.
    pub(crate) async fn summarised(&self, job: &Job) -> Result<JobSummary, Refusal> {
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let queued = self.queued_reason(job).await?;
        let asking = self.question_awaited(job.id()).await.is_some();
        Ok(JobSummary::of(
            job,
            reason.as_ref(),
            queued,
            asking,
            self.resumption(job),
        ))
    }

    /// Why an approved Job has not started, worked out from the board as it
    /// stands rather than read from anything.
    ///
    /// **Nothing is read for a Job that is not `queued`**, which is every row
    /// but the waiting ones — the board read is the cost, and a status that
    /// cannot have this reason must not pay it.
    ///
    /// The dependency half is [`clear_to_run`], the budget half is
    /// `Fleet::overspent` and the resource half is `Fleet::room_for_another` —
    /// all three the predicates admission itself uses, asked in the order
    /// admission asks them. A second answer here is how a Board comes to say a
    /// Job is blocked while Fleet is starting it.
    ///
    /// **`None` is the registry's `none`** — approved, unblocked, inside its
    /// budget, and there is room. **Nothing is stored**: headroom frees on its
    /// own, so a reason written down is wrong from the moment it is. The budget
    /// half does not free on its own and is still computed here, because what
    /// changes it is a person raising the cap and a stored label would survive
    /// that.
    pub(crate) async fn queued_reason(
        &self,
        job: &Job,
    ) -> Result<Option<CoreQueuedReason>, Refusal> {
        if job.status() != CoreJobStatus::Queued {
            return Ok(None);
        }
        let (loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        let standing: BTreeMap<CoreJobId, CoreJobStatus> = loaded
            .jobs
            .iter()
            .map(|held| (held.id().clone(), held.status()))
            .collect();
        if !clear_to_run(job, &standing) {
            return Ok(Some(CoreQueuedReason::BlockedByDependency));
        }
        // **The same label as the edge above, because it is the same fact from
        // the Board's side.** The registry gives a `queued` Job no reason
        // meaning "waiting on the Jobs it created"; a Job held for its children
        // is held for work that has to finish first, which is what the
        // dependency label says. The mechanism differs — provenance rather than
        // an edge — and that difference is not a Board fact.
        if waiting_on_children(job, &crate::sub_dispatch::children_standing(&loaded.jobs)) {
            return Ok(Some(CoreQueuedReason::BlockedByDependency));
        }
        // **Before the machine reading, and not only because admission asks it
        // first.** Headroom frees on its own and a spent budget does not, so a
        // Job that is both would be told it is waiting for something that is
        // already on its way when the thing actually holding it needs a person.
        // The dollars and the turns fold to this one label; which of the two it
        // was is on the Job's detail, where the figures are.
        if self
            .overspent(job)
            .await
            .map_err(|why| self.refusal(why))?
            .is_some()
        {
            return Ok(Some(CoreQueuedReason::OverBudget));
        }
        // **The same predicate admission opens with**, asked of the same
        // roster. The bound and each of the three machine readings fold to the
        // one label the registry gives a `queued` Job short of anything; which
        // of them it was is not a Board fact.
        let mut slots = self.slots().lock().await;
        let room = self.room_for_another(&mut slots).await;
        Ok((!room.granted()).then_some(CoreQueuedReason::WaitingOnResources))
    }
}
