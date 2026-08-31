//! Which approved Job runs next, and how many may run at once.
//!
//! Split from [`dispatch`](mod@crate::dispatch), which is what happens to a Job
//! once the answer is yes. They were one file while the answer was "the slot is
//! free"; the bound, the queue's ordering and the dependency release together
//! are a subject, and it is the one `#44`, `#48` and `#51` each arrive at.
//!
//! # The queue is a status, not a structure
//!
//! `queued` is the queue. There is no ordering held in memory that a restart
//! could lose or that could disagree with the log, so [`Fleet::next_queued`]
//! reads the board and sorts by the sequence of the approving event.
//!
//! [`Fleet::next_queued`]: crate::Fleet

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId, JobStatus};
use store::Moved;

use crate::adrift::Adrift;
use crate::coupling::{coupling, Coupling};
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
    /// Start every approved Job the bound has room for.
    ///
    /// **`Slots::room` is the whole of what the bound means here**, and it is
    /// the same predicate `queued_reason` answers `waiting_on_resources` from —
    /// one answer, because a Board saying a Job is blocked while Fleet is
    /// starting it is worse than a Board saying nothing.
    ///
    /// **The roster lock is held across the loop, and nothing else is.** Two
    /// admissions running at once would each read the same `queued` Job as
    /// next and dispatch it twice; holding the roster is what makes admission
    /// one act. It is released the moment the last Drone is spawned, which is
    /// what keeps it from being the single working slot again — a slot is held
    /// for as long as a Job is worked, and this is held for as long as a Job is
    /// *started*.
    ///
    /// Admission stops at the first Job that will not start. The failure is
    /// returned, its Job is left `escalated` by `dispatch`, and the next turn
    /// asks again — Fleet does not walk on down the queue to find one that
    /// works, because the reason the first failed is ordinarily the disk.
    pub(crate) async fn admit_next(&self) -> Result<Vec<JobId>, Adrift> {
        let mut slots = self.slots().lock().await;
        let mut admitted = Vec::new();
        while slots.room() {
            let Some(job) = self.next_queued().await? else {
                break;
            };
            let job_id = job.id().clone();
            let slot = slots.opened_for(&job_id);
            // The slot exists and counts against the bound before the dispatch
            // that fills it runs, so a `next_queued` inside this same loop
            // cannot hand back the Job being started. Nothing else can be
            // holding this lock — the roster is held, and the Job had none.
            let mut working = slot.lock().await;
            if let Err(cause) = self.dispatch(job, &mut working).await {
                drop(working);
                // Nothing started, so nothing is being worked, so the bound is
                // not spent. Left in place it would be a Job with no Drone
                // holding a place in the roster for ever.
                slots.closed(&job_id);
                return Err(cause);
            }
            drop(working);
            admitted.push(job_id);
        }
        Ok(admitted)
    }

    /// The Job that has been waiting longest, by when it was approved.
    ///
    /// Ordered by the sequence of the event that put it at `queued`, not by id
    /// and not by creation: two Jobs created in either order can be approved in
    /// the other, and the person who approved first is entitled to expect
    /// theirs to run first.
    ///
    /// A row that would not rebuild is not dispatchable, and is reported by
    /// every read that returns a list rather than being silently completed
    /// here.
    async fn next_queued(&self) -> Result<Option<Job>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        let standing: BTreeMap<JobId, JobStatus> = loaded
            .jobs
            .iter()
            .map(|job| (job.id().clone(), job.status()))
            .collect();
        let mut waiting = Vec::new();
        for job in loaded.jobs {
            if job.status() != JobStatus::Queued {
                continue;
            }
            // Approved and still waiting on a peer. It is skipped rather than
            // reordered — the queue is by approval and this is not a Job's turn
            // being taken, it is a Job that has nothing to work against yet.
            if !clear_to_run(&job, &standing) {
                continue;
            }
            waiting.push((self.approved_at(job.id()).await?, job));
        }
        waiting.sort_by_key(|(seq, _)| *seq);
        Ok(waiting.into_iter().next().map(|(_, job)| job))
    }

    /// When the Job was released to run, as the log's own sequence.
    ///
    /// A Job with no such event was **created** at `queued` — a sub-dispatch,
    /// approved as part of its parent — and is therefore older than anything
    /// that had to be approved on its own.
    async fn approved_at(&self, job_id: &JobId) -> Result<i64, Adrift> {
        let events = self
            .store()
            .lock()
            .await
            .events_for(job_id)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        Ok(events
            .iter()
            .rev()
            .find(|event| {
                matches!(
                    event.moved(),
                    Moved::Job {
                        to: JobStatus::Queued,
                        ..
                    }
                )
            })
            .map(|event| event.seq())
            .unwrap_or(i64::MIN))
    }
}

/// Whether every Job this one waits on has finished, and finished well.
///
/// `pub(crate)` because `serving` renders the same fact as a label. **One
/// answer, not two** — a Board saying a Job is blocked while admission
/// disagrees is worse than a Board saying nothing.
///
/// **No terminal weaker than `completed_success` or `superseded`.** A dependent
/// admitted after a failed upstream would do its work against a base that never
/// landed, which is the half-landed upstream the linked-DAG shape exists to
/// prevent. A superseded upstream is not that case: the work landed outside the
/// Job, so the base is there and only the record has nothing to say.
///
/// The three-way weighing, and which peer failed, are
/// [`coupling`](crate::coupling::coupling)'s — this is the yes-or-no admission
/// reads, and it is the same call `serving` labels a Board row from.
pub(crate) fn clear_to_run(job: &Job, standing: &BTreeMap<JobId, JobStatus>) -> bool {
    matches!(coupling(job, standing), Coupling::Clear { .. })
}
