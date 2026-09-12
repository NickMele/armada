//! Resolving a pull request's conflicts with main from the review gate,
//! without asking the Drone that cannot. `#663`.
//!
//! **Fleet rebases, never a Drone** — [`crate::currency`]'s sweep engine. Clean, it pushes and the pull
//! request updates, nobody spawned; conflicted, a Drone that can edit files is needed, and the gate step
//! is often exactly the one that cannot (`#660` spent a turn on `handoff`, which only summarises).
//!
//! **So a conflict routes the Job back to the step before delivery** — the same [`StepTarget::Returned`]
//! edge `verdict_routing` uses (`crate::reviewing::route_back` is that caller; this chooses its own
//! target). A Drone there reads `crate::spawning`'s catch-up markers, and once Checks pass, the ordinary
//! forward walk carries the Job through delivery again — commit, push, pull request updated.
//!
//! **Not the gate step itself**: `crate::landing::sent_out_on_entry` commits whatever the worktree holds
//! the moment delivery is *entered*, so a conflicted rebase there would commit the markers; a single-step
//! workflow has none to redo, and [`Fleet::resolve_pull_request_conflict`] refuses rather than risk it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Actor, Component, Envelope, FieldValue, Job, JobId, Level, StepTarget, Target};

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
    /// Send a Job's branch back for a Drone that can edit files to bring it
    /// current with main, from the human gate its pull request is waiting at.
    ///
    /// **Refuses off the gate.** [`crate::reviewing::at_the_gate`] is the same
    /// read [`approve_review`](Fleet::approve_review) and
    /// [`request_changes`](Fleet::request_changes) refuse from, because this is
    /// a third answer at the same gate and not a fourth act with its own
    /// entry.
    ///
    /// **Fleet does not read the branch before moving the Job.** The rebase
    /// happens where every rebase on this Job happens — inside
    /// [`put_a_drone_on`](Fleet::put_a_drone_on), on the turn a slot admits the
    /// step this returns to — so a Job whose branch turns out not to be behind
    /// after all costs one no-op turn rather than a second reading of git here.
    pub async fn resolve_pull_request_conflict(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        let gate = self.at_the_gate(&job)?;

        let delivery = self
            .store()
            .lock()
            .await
            .delivery_for(job_id)
            .map_err(Adrift::Reading)?;
        if delivery.pull_request.is_none() || delivery.landed.is_some() {
            return Err(Adrift::NothingToResolve {
                job: job_id.clone(),
            });
        }

        let Some(delivering) = job.workflow().delivering_step() else {
            return Err(Adrift::NothingToResolve {
                job: job_id.clone(),
            });
        };
        let Some(target) = job
            .workflow()
            .before(delivering.id())
            .map(|step| step.id().clone())
        else {
            return Err(Adrift::NoStepToRedo {
                job: job_id.clone(),
            });
        };

        // A Drone standing on the gate's own step has nothing left to do —
        // the gate stood it down — but one left over from a Fleet restart that
        // never reaped it is ended rather than left racing the fresh one.
        if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            self.end_the_drone(&mut working).await;
        }
        // Before anything moves, so a refusal leaves the Job at the gate
        // rather than half sent back — `route_back`'s own ordering, for its
        // own reason: there is nowhere for the next pass to happen without a
        // worktree.
        if self.surviving_worktree(&job).is_err() {
            return Err(Adrift::NoDroneToTell {
                job: job_id.clone(),
            });
        }

        self.logged(
            job_id,
            Envelope::new(
                self.now(),
                Level::Info,
                Component::Fleet,
                self.run().clone(),
                "a person sent the pull request's branch back for a Drone to bring \
                 current with main — the gate itself has not moved",
            )
            .in_job(job_id.as_ulid().clone())
            .at_step(gate.as_str())
            .with_field("redoing", FieldValue::Str(target.as_str().to_string())),
        );

        // While the Job is still `awaiting_review`, which is in
        // `ADVANCING_STATUSES` only until the move below leaves it —
        // `route_back`'s own reason for the same ordering.
        let job = self
            .move_step_by(&job, &target, StepTarget::Returned(gate), Actor::Human)
            .await?;
        drop(working);
        // `queued`, and the turn is what makes it `running` — `#428`, the
        // same edge every other re-admission after a gate takes.
        self.move_job(&job, Target::Queued, Actor::Human).await
    }
}
