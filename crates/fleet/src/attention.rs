//! The four narrowings of the board, and the one rule each is.
//!
//! **Split from `serving` for `summarising`'s reason.** Those are the
//! operations; this is what decides, once, which Jobs each of them is about —
//! a rule every surface would otherwise state for itself.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Job, JobStatus};
use ipc::{Alert, AlertList, JobList, JobSummary};
use store::Moved;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Jobs no Drone has been started on: approved and waiting for a slot, or
/// waiting to be approved at all.
fn not_yet_started(job: &Job) -> bool {
    matches!(
        job.status(),
        JobStatus::AwaitingApproval | JobStatus::Queued
    )
}

/// Jobs resting at a gate a person has to answer.
///
/// **Resting, not stopped.** An escalated Job also needs somebody and is not
/// here: it is holding a Drone while it waits, which is `list_alerts`' split.
fn awaiting_sign_off(job: &Job) -> bool {
    matches!(
        job.status(),
        JobStatus::AwaitingReview | JobStatus::AwaitingAttestation
    )
}

/// Jobs that stopped mid-flight and are holding what they held.
///
/// `piloted` is not here: a person has already taken it over, so it is being
/// acted on rather than waiting for somebody to notice.
fn stopped_mid_flight(job: &Job) -> bool {
    matches!(
        job.status(),
        JobStatus::Escalated | JobStatus::AwaitingRepair
    )
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
    /// `list_job_board` — the open queue.
    pub(crate) async fn job_board(&self) -> Result<JobList, Refusal> {
        self.selected(not_yet_started, false).await
    }

    /// `list_reviews` — what is waiting to be signed off.
    pub(crate) async fn reviews(&self) -> Result<JobList, Refusal> {
        self.selected(awaiting_sign_off, false).await
    }

    /// `get_activity_feed` — what is over, newest first.
    ///
    /// **Terminal by the registry's own column**, never by a list of status
    /// names kept here: a status added to `job-statuses.toml` joins this answer
    /// with nothing in this workspace edited.
    pub(crate) async fn activity_feed(&self) -> Result<JobList, Refusal> {
        self.selected(|job| job.status().is_terminal(), true).await
    }

    /// `list_alerts` — what is waiting on a person, in two buckets.
    ///
    /// **One board read for both.** Two calls would be two readings of one
    /// board at two instants, which is how a Job comes to be in neither bucket
    /// or in both.
    pub(crate) async fn alerts(&self) -> Result<AlertList, Refusal> {
        let (loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        let (mut blocked, mut waiting) = (Vec::new(), Vec::new());
        for job in &loaded.jobs {
            let stopped = stopped_mid_flight(job);
            let resting = awaiting_sign_off(job) || job.status() == JobStatus::AwaitingApproval;
            if !stopped && !resting {
                continue;
            }
            let summary = self.summarised(job).await?;
            // **A Drone waiting on an answer is blocked whatever its status
            // says.** It is stopped inside a tool call, holding its session and
            // its spend, and `running` is what the record carries meanwhile.
            match stopped || summary.asking {
                true => blocked.push(self.alerted(job, &summary).await?),
                false => waiting.push(self.alerted(job, &summary).await?),
            }
        }
        Ok(AlertList { blocked, waiting })
    }

    /// One row, with when the Job stopped where it is.
    async fn alerted(&self, job: &Job, summary: &JobSummary) -> Result<Alert, Refusal> {
        let events = self
            .store()
            .lock()
            .await
            .events_for(job.id())
            .map_err(|cause| {
                self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause)))
            })?;
        let moved = events
            .iter()
            .rev()
            .find(|event| matches!(event.moved(), Moved::Job { .. }));
        Ok(Alert {
            job_id: summary.id.clone(),
            handle: summary.handle.clone(),
            status: job.status().as_wire().to_string(),
            why: summary
                .reason
                .as_ref()
                .and_then(|reason| reason.named.clone()),
            since: moved.map(|event| event.at().into()),
        })
    }

    /// Every Job the predicate keeps, as Board rows.
    ///
    /// **Filtered before it is summarised.** A summary costs a log read per
    /// Job, and a narrowing that summarised the whole board to throw most of it
    /// away would make each of these four as expensive as `list_jobs`.
    async fn selected(
        &self,
        keep: impl Fn(&Job) -> bool,
        newest_first: bool,
    ) -> Result<JobList, Refusal> {
        let (loaded, unreadable) = self.every_job().await.map_err(|why| self.refusal(why))?;
        let landed = self
            .store()
            .lock()
            .await
            .landed_by_job()
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let mut jobs = Vec::new();
        for job in loaded.jobs.iter().filter(|job| keep(job)) {
            let mut summary = self.summarised(job).await?;
            summary.landed = landed.get(job.id()).and_then(crate::noticing::settled);
            jobs.push(summary);
        }
        if newest_first {
            jobs.sort_by(|one, two| two.created_at.as_str().cmp(one.created_at.as_str()));
        }
        Ok(JobList {
            jobs,
            // **Carried, never dropped.** A narrowing cannot know whether a row
            // that would not load belonged in it, and a list that quietly lost
            // one is the v1 defect this shape exists against.
            unreadable: unreadable
                .into_iter()
                .map(|fault| ipc::UnreadableJob {
                    job_id: None,
                    fault,
                })
                .collect(),
        })
    }
}
