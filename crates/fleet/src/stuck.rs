//! The four live facts a classification needs, gathered.
//!
//! `core_model::Stuck` is the rule — which acts a stopped Job admits, given
//! what is true about it. This is the half that reads what is true: the slot,
//! the filesystem, the store and the workflows Fleet holds. **The rule is in
//! `core-model` because it is a statement about the trigger vocabulary and
//! nothing else; the reading is here because none of it is in the record.**
//!
//! # Why a person could not be told this before
//!
//! Four acts each answered "does this apply to me" by refusing, so a person
//! learned which one applied by pressing buttons until one worked. Bridge then
//! re-derived four of those refusals in TypeScript and could not derive the
//! fifth at all: whether the worktree survived is a `path.is_dir()` and a
//! renderer reads no filesystem, so a restart was offered on a Job that had
//! none. Every fact below is one only Fleet holds.
//!
//! # It costs one filesystem stat and nothing else
//!
//! The Check runs are the read `serving::get_job` already makes for the step
//! detail, handed in rather than made again, and the slot and the workflow map
//! are in memory. An open of a Job that is still going costs nothing at all —
//! `Stuck::asked_of` answers first.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, Standing, StepCheck, StepId, Stuck, TransitionReason};

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
    /// Why this Job is stuck and what moves it, or `None` where it is not.
    ///
    /// **The facts are read before the rule is applied, and none of them is
    /// guessed.** A read that cannot be made is reported as the pessimistic
    /// answer — a worktree spec that will not resolve is a worktree that is not
    /// there, which is exactly what `restart_step` says on the press.
    pub(crate) async fn why_stuck(
        &self,
        job: &Job,
        reason: Option<&TransitionReason>,
        ran: &[(StepId, Vec<StepCheck>)],
    ) -> Option<Stuck> {
        // Asked first, so a Job that is still going costs no stat and no lock.
        if !Stuck::asked_of(job.status()) {
            return None;
        }
        Stuck::of(job, reason, self.standing_of(job, ran).await)
    }

    /// What Fleet knows about this Job that its record does not say.
    async fn standing_of(&self, job: &Job, ran: &[(StepId, Vec<StepCheck>)]) -> Standing {
        Standing {
            drone_holding: self.a_drone_to_speak_to(job).await,
            // The same call `restart_step` and `override_verdict` make, so the
            // classification cannot say a worktree is there that they then
            // refuse to find.
            worktree_on_disk: self.surviving_worktree(job).is_ok(),
            checks_passed: checks_passed(ran, job.stopped_on().map(|(step, _)| step)),
            workflow_held: self.workflow_named(job.workflow_id()).is_some(),
        }
    }

    /// Whether a Drone is on this Job **and Fleet can say something to it**.
    ///
    /// The slot and never `assigned_drone`: the record's pointer survives a
    /// Fleet restart and the pipe does not, and it is the pipe a redirect and a
    /// gate re-run both need.
    ///
    /// **And a full slot is not a pipe either**, which is the half that was
    /// false. `crate::adopting` puts a Drone that outlived its Fleet back in
    /// the working slot with both pipes dead, so a person offered a redirect on
    /// one typed a note and had it refused. [`Session::unheard`] is the
    /// reading, and it is the condition rather than the cause — the same call
    /// `crate::silence` makes to tell `stalled` from `unheard`, so a second way
    /// to lose the pipe withholds the same two acts unasked. #442.
    ///
    /// **What is left is the act that works**: `Stuck::of`'s other arm offers a
    /// restart, which ends the unreadable Drone. An override and a redispatch
    /// end the Drone rather than speak to it, so neither is withheld.
    ///
    /// [`Session::unheard`]: crate::adopting::Session::unheard
    async fn a_drone_to_speak_to(&self, job: &Job) -> bool {
        let Some(slot) = self.slot_of(job.id()).await else {
            return false;
        };
        let held = slot.lock().await;
        held.as_ref()
            .is_some_and(|at_work| at_work.is(job.id()) && !at_work.session().unheard())
    }
}

/// Whether every Check the gate recorded on the stopped step passed.
///
/// **True where none ran**, which is the same answer `overruling`'s guard
/// gives: it looks for a failure and finds none. An ungated step is not a step
/// whose Checks failed.
///
/// **A skipped Check is true here too**, for the same reason and by the same
/// method: `advances` asks whether anything failed, and a Check the step's
/// paths did not reach failed nothing. Asking `passed` would tell a person the
/// Checks failed on a step where nothing was run.
fn checks_passed(ran: &[(StepId, Vec<StepCheck>)], step: Option<&StepId>) -> bool {
    let Some(step) = step else {
        return true;
    };
    ran.iter()
        .filter(|(id, _)| id == step)
        .flat_map(|(_, checks)| checks.iter())
        .all(|check| check.outcome.advances())
}
