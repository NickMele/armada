//! Sending a Job at its review gate back for a Drone to clear its pull
//! request's conflicts with the base. `#663`, and Fleet's own act since `#1131`.
//!
//! **The step before the delivering one is redone**, on the `Returned` edge
//! `crate::reviewing::route_back` takes too. Its spawn merges the base in and
//! leaves the markers for the Drone; once its Checks pass, the walk forward
//! re-enters the delivering step, whose commit finishes the merge and whose push
//! updates the pull request. Not the delivering step itself: it commits on
//! entry, and would commit the markers.
//!
//! **Fleet sends it where the sweep finds a conflict** — `crate::currency` — as
//! often as the gate's `iteration_cap` allows and at least once. A person's press
//! is the other road, until Bridge drops it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, Level, StepLevelTrigger,
    StepTarget, Target,
};

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
    pub async fn resolve_pull_request_conflict(&self, job_id: &JobId) -> Result<Job, Adrift> {
        self.sent_to_clear_conflicts(job_id, Actor::Human).await
    }

    /// The same act, with the actor named.
    ///
    /// **Fleet's sends are bounded and a person's are not.** A base that keeps
    /// moving while a Drone clears it would send the Job round for ever, so a
    /// send past the gate's `iteration_cap` — or past one, where the gate closes
    /// no loop — escalates as `loop_cap` instead.
    ///
    /// **Fleet does not read the branch before moving the Job.** The merge
    /// happens where every catch-up on this Job happens — inside
    /// [`put_a_drone_on`](Fleet::put_a_drone_on), on the turn a slot admits the
    /// step this returns to.
    pub(crate) async fn sent_to_clear_conflicts(
        &self,
        job_id: &JobId,
        by: Actor,
    ) -> Result<Job, Adrift> {
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

        if by == Actor::Fleet {
            let events = self
                .store()
                .lock()
                .await
                .events_for(job_id)
                .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
            let allowed = job
                .workflow()
                .step(&gate)
                .map_or(0, |step| step.iteration_cap())
                .max(1);
            let spent = StepLevelTrigger::of(EscalationTrigger::LoopCap);
            if let Some(spent) =
                spent.filter(|_| crate::clearing::times_sent(&events, &gate) >= allowed)
            {
                self.logged(
                    job_id,
                    Envelope::new(
                        self.now(),
                        Level::Warn,
                        Component::Fleet,
                        self.run().clone(),
                        "the pull request's branch conflicts with its base again, and Fleet has \
                         already sent it back to clear conflicts as often as this gate allows",
                    )
                    .in_job(job_id.as_ulid().clone())
                    .at_step(gate.as_str()),
                );
                let escalated = self
                    .loop_is_spent(&job, &gate, spent, &mut working, Actor::Fleet)
                    .await;
                drop(working);
                return escalated;
            }
        }

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

        let said = match by {
            Actor::Fleet => {
                "the pull request's branch conflicts with its base, so Fleet sent it back for a \
                 Drone to clear the conflicts — the gate itself has not moved"
            }
            _ => {
                "a person sent the pull request's branch back for a Drone to bring current with \
                 main — the gate itself has not moved"
            }
        };
        self.logged(
            job_id,
            Envelope::new(
                self.now(),
                Level::Info,
                Component::Fleet,
                self.run().clone(),
                said,
            )
            .in_job(job_id.as_ulid().clone())
            .at_step(gate.as_str())
            .with_field("redoing", FieldValue::Str(target.as_str().to_string())),
        );

        // While the Job is still `awaiting_review`, which is in
        // `ADVANCING_STATUSES` only until the move below leaves it —
        // `route_back`'s own reason for the same ordering.
        let job = self
            .move_step_by(&job, &target, StepTarget::Returned(gate), by)
            .await?;
        drop(working);
        // `queued`, and the turn is what makes it `running` — `#428`, the
        // same edge every other re-admission after a gate takes.
        self.move_job(&job, Target::Queued, by).await
    }
}
