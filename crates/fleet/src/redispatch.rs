//! Trying a failed Job again, which is minting a new one.
//!
//! # It does not reopen the Job, and the registry is why
//!
//! `job-fields.toml`'s `redispatched_from` row says a redispatch is always a
//! new Job carrying a reference back and never reopens an escalated one, and
//! `job-transitions.toml` puts redispatch on `escalated -> killed`. The
//! `escalated -> running` edge is a redirect's — context injected mid-step —
//! not this.
//!
//! The disk agrees. A worktree is `armada/<job_id>` and
//! `GitVcs::create_worktree` refuses an existing branch, so a Job moved back to
//! `running` under its own id could only be dispatched by deleting the branch
//! its failure is recorded on. **A failed Job's worktree and branch are
//! evidence** — nothing in the workspace removes either — so the id has to
//! change.
//!
//! # The replacement is minted before the failure is killed
//!
//! The two writes are not one transaction. Killed first and minting failing
//! leaves a terminal Job with no replacement and no way to ask again, which is
//! the complaint this exists to answer. This order leaves, at worst, a Job at
//! the approval gate beside a still-escalated one: visible, harmless, and
//! recoverable by hand.

use adapter_traits::{AgentHarness, Vcs, WorkProduct};
use core_model::{Job, JobId, JobStatus, NewJob, StepSeed};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// What a redispatch left behind: the failed Job, and the one replacing it.
#[derive(Clone, Debug)]
pub struct Replacement {
    /// The Job that failed, now `killed`.
    pub replaced: Job,
    /// The new Job, at its entry status, carrying `redispatched_from`.
    pub dispatched: Job,
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Replace a Job that stopped and is waiting for a person.
    ///
    /// **`escalated` and nothing else.** A running Job is redirected rather
    /// than replaced, a Job at a gate has not failed, and whether a `rejected`
    /// or `completed_failed` Job may be redispatched is an open question on
    /// the `redispatched_from` registry row — refused here rather than decided.
    ///
    /// The actor is **human**. Fleet never redispatches of its own accord: a
    /// Job reaches `escalated` precisely because Fleet stopped deciding.
    pub async fn redispatch(&self, job_id: &JobId) -> Result<Replacement, Adrift> {
        let failed = self.load(job_id).await?;
        if failed.status() != JobStatus::Escalated {
            return Err(Adrift::NotRedispatchable {
                job: failed.id().clone(),
                status: failed.status(),
            });
        }
        let dispatched = self.mint_replacement(&failed).await?;
        let replaced = Fleet::kill_job(self, job_id).await?;
        Ok(Replacement {
            replaced,
            dispatched,
        })
    }

    /// The new Job, written and published.
    ///
    /// The workflow and its steps are read from what Fleet holds **now**, not
    /// copied from the failed Job. A redispatch is a new Job, and a new Job
    /// freezes the definition as it currently stands — which is how an edit
    /// made in response to the failure reaches the retry. Everything else is
    /// carried across unchanged.
    async fn mint_replacement(&self, failed: &Job) -> Result<Job, Adrift> {
        let at = self.now();
        let new = NewJob {
            id: JobId::carried(self.mint().ulid()),
            title: failed.title().clone(),
            workflow: self.workflow().frozen().clone(),
            owner_manifest_id: failed.owner_manifest_id().clone(),
            urgency: failed.urgency(),
            atomic: failed.atomic(),
            model: failed.model().clone(),
            acceptance_criteria: failed.acceptance_criteria().to_vec(),
            steps: self
                .workflow()
                .frozen()
                .steps()
                .iter()
                .enumerate()
                .map(|(ordinal, step)| StepSeed {
                    step_id: step.id().clone(),
                    ordinal: ordinal as u32,
                })
                .collect(),
            dependencies: failed.dependencies().to_vec(),
            gate_manifests: failed.gate_manifests().to_vec(),
            write_targets: failed.write_targets().cloned(),
            subject: failed.subject().cloned(),
            redispatched_from: Some(failed.id().clone()),
            facts: failed.facts().clone(),
            scope_revisions: failed.scope_revisions().to_vec(),
        };
        // A replacement enters where its original entered, so the approval gate
        // is neither skipped nor imposed. A sub-dispatched Job has no top-level
        // origin, and replacing one is its parent's act rather than this one.
        let origin = failed
            .origin()
            .top_level()
            .ok_or_else(|| Adrift::NotReplaceable {
                job: failed.id().clone(),
            })?;
        let job = Job::create_top_level(new, origin, at.clone());
        self.store()
            .lock()
            .await
            .insert_job(&job, &at)
            .map_err(Adrift::Writing)?;
        self.publish(ipc::Event::JobCreated(ipc::JobCreated {
            job: ipc::JobSummary::from(&job),
            actor: core_model::Actor::Human.into(),
            at: (&at).into(),
        }));
        Ok(job)
    }
}
