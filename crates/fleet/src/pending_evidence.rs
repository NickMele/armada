//! Evidence a restart found still waiting for the gate, ruled on before
//! Fleet decides which Drones survived. #796.
//!
//! Ruled through `crate::rechecking::ruled_with_no_drone`, not through
//! `crate::settling::settle`: the Drone that submitted did not survive the
//! restart, so there is no slot for `settle` to ever find.
//!
//! **`Ruling::Advanced` is queued rather than handed to `act_on`**, which
//! starts the next Drone outright — right for the turn loop, where the Job
//! already held a slot, wrong on a boot that has counted nothing against
//! `concurrency-cap` yet. `crate::rechecking::carried_on` makes the same
//! override.
//!
//! **A pointer a ruling leaves looking live is the adoption pass right
//! after this one's to find**, `crate::mending`'s ordering.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use core_model::{
    Actor, Component, Envelope, FieldValue, Job, JobId, JobStatus, Level, StepId, StepTarget,
    Target,
};
use verification::{Claimed, NotClaimed, ShownBy, Submission};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::regating::came_to;

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
    /// Every row this served repository's Jobs left pending, ruled on and
    /// cleared. **Called from `reconcile`, before the Drone-adoption pass
    /// reads the same rows** — `crate::mending`'s own ordering, for this
    /// module's reason.
    pub(crate) async fn ruled_on_pending_evidence(
        &self,
        jobs: &mut [Job],
    ) -> Result<Vec<JobId>, Adrift> {
        let pending = self
            .store()
            .lock()
            .await
            .pending_evidence()
            .map_err(Adrift::BootRead)?;
        let mut ruled = Vec::new();
        for row in pending {
            // Not this repository's Job — served, and reconciled, whenever
            // the repository that owns it is added. `crate::mending`'s note.
            let Some(slot) = jobs.iter_mut().find(|job| job.id() == &row.job_id) else {
                continue;
            };
            match self.ruled_on_one_pending(slot, &row).await {
                Ok(true) => ruled.push(slot.id().clone()),
                Ok(false) => {}
                Err(why) => self.noted_adrift(&why),
            }
        }
        Ok(ruled)
    }

    /// One row. `Ok(true)` where the gate ruled on it, `Ok(false)` where the
    /// row named nothing left to rule on.
    async fn ruled_on_one_pending(
        &self,
        slot: &mut Job,
        row: &store::PendingEvidence,
    ) -> Result<bool, Adrift> {
        let job_id = slot.id().clone();
        // A Job that left `running` before Fleet stopped, or a row an
        // earlier crash left between the ruling and the clear — either way
        // the gate has nothing left to say about it.
        if slot.status() != JobStatus::Running {
            self.store()
                .lock()
                .await
                .forget_pending_evidence(&job_id)
                .map_err(Adrift::Writing)?;
            return Ok(false);
        }
        let submission = Submission::submitted(
            row.evidence_type,
            Claimed(&row.claimed),
            ShownBy(&row.shown_by),
            NotClaimed(&row.not_claimed),
        )
        // Unreachable in practice — `kept_pending` never writes an empty
        // `claimed` or `shown_by` — kept for `submitted_already`'s reason:
        // a row that will not pass this was written by something that did
        // not share the constructor, which is a corrupt record and not
        // evidence to rule on.
        .map_err(|_| Adrift::NothingToRuleOn {
            job: job_id.clone(),
            step: row.step_id.clone(),
        })?;
        let worktree = self.surviving_worktree(slot)?;
        let ruling = self
            .ruled_with_no_drone(slot, &row.step_id, &worktree, &submission)
            .await?;
        self.noted_recovered(&job_id, &row.step_id, &ruling);
        self.noted_undecided(&job_id, &row.step_id, &ruling);
        *slot = self
            .carried_without_a_drone(&ruling, &job_id, &row.step_id, &worktree)
            .await?;
        self.store()
            .lock()
            .await
            .forget_pending_evidence(&job_id)
            .map_err(Adrift::Writing)?;
        Ok(true)
    }

    /// The ruling, applied with no live Drone to hand `act_on`. **`Advanced`
    /// is queued rather than acted on** — this module's own header. Every
    /// other ruling starts no Drone, and `act_on` already treats an empty
    /// slot as the ordinary shape of a Fleet that restarted —
    /// `crate::boundary::crossed_onto`'s own doc.
    async fn carried_without_a_drone(
        &self,
        ruling: &Ruling,
        job_id: &JobId,
        step: &StepId,
        worktree: &Worktree,
    ) -> Result<Job, Adrift> {
        if let Ruling::Advanced { .. } = ruling {
            let job = self.load(job_id).await?;
            let job = self.move_step(&job, step, StepTarget::Advanced).await?;
            return self.move_job(&job, Target::Queued, Actor::Fleet).await;
        }
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        self.act_on(ruling, job_id, step, &mut working).await?;
        drop(working);
        if matches!(ruling, Ruling::HeldForReview { .. }) {
            let held = self.load(job_id).await?;
            self.compose_review_at_gate(&held, step, worktree).await;
        }
        self.load(job_id).await
    }

    fn noted_recovered(&self, job: &JobId, step: &StepId, ruling: &Ruling) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "evidence this Job's Drone submitted survived a restart; Fleet ruled on it at boot",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("came_to", FieldValue::Str(came_to(ruling).to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}
