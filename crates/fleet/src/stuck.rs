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
//! # It costs one filesystem stat and one read of a stopped Job's transcripts
//!
//! The Check runs are the read `serving::get_job` already makes for the step
//! detail, handed in rather than made again, and the slot and the workflow map
//! are in memory. An open of a Job that is still going costs nothing at all —
//! `Stuck::asked_of` answers first. The transcript read is the trigger's own
//! evidence — see [`Fleet::refused`]. A `gate_undecided` Job costs a second
//! read, of the Job's own log — see [`Fleet::undecided`].

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Journal;
use core_model::{
    DroneStanding, EscalationTrigger, Job, Refusals, Standing, StepCheck, StepId, Stuck,
    TransitionReason,
};

use crate::daemon::Fleet;
use crate::settling::UNDECIDED;

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
        // Asked first, so a Job that is still going costs no stat, no lock and
        // no transcript read.
        if !Stuck::asked_of(job.status()) {
            return None;
        }
        Stuck::of(
            job,
            reason,
            self.standing_of(job, ran).await,
            self.refused(job).await,
            self.undecided(job),
        )
    }

    /// What the Drone on the stopped step reached for and was refused.
    ///
    /// **Read for every stopped Job and not only a `blocked_by_policy` one.**
    /// A Drone denied the command it needed goes on to escalate as `stalled` or
    /// `silent` just as often, so evidence gathered only where the
    /// classification already named a policy would be missing from exactly the
    /// Jobs the classification got wrong.
    ///
    /// **Scoped to the step this Job stopped on, not every step it ever ran.**
    /// A Job's transcripts span its whole run, and a refusal on an earlier step
    /// that passed is not evidence for why the current step is stuck —
    /// [`stopped_step`] is the same reading [`Stuck::of`](core_model::Stuck::of)
    /// makes of the record, so the two cannot disagree about which step this is.
    ///
    /// **The Drone's process is long gone by the time a person opens this.**
    /// The events are not in memory and nothing on the record kept them; the
    /// file under `.armada/transcripts/` is what survives, and the Job's log is
    /// what names it.
    async fn refused(&self, job: &Job) -> Refusals {
        crate::transcript::refusals(&self.host().records_root, &job.handle(), stopped_step(job))
            .await
    }

    /// What the gate said when it could not decide, where that is why the
    /// stopped step stopped.
    ///
    /// **Not in the store.** [`Fleet::noted_undecided`](crate::settling)
    /// writes the sentence to the Job's log alone, so this is the same kind of
    /// read `refused` makes of the transcripts — off a file, for the step that
    /// stopped — and it costs nothing on every trigger but `gate_undecided`,
    /// which the guard below checks first.
    fn undecided(&self, job: &Job) -> Option<String> {
        let (step, trigger) = job.stopped_on()?;
        if trigger.trigger() != EscalationTrigger::GateUndecided {
            return None;
        }
        let step = ipc::StepId::from(step);
        self.job_logs()
            .read(&job.handle(), 0)
            .notes
            .into_iter()
            // The latest one: a step can be asked more than once, and this
            // says why the attempt that is still standing could not decide.
            .rev()
            .find(|note| note.msg == UNDECIDED && note.step.as_ref() == Some(&step))
            .and_then(|note| {
                note.fields
                    .into_iter()
                    .find(|field| field.name == "said")
                    .map(|field| field.value)
            })
    }

    /// What Fleet knows about this Job that its record does not say.
    async fn standing_of(&self, job: &Job, ran: &[(StepId, Vec<StepCheck>)]) -> Standing {
        Standing {
            drone: self.whats_in_the_slot(job).await,
            // The same call `restart_step` and `override_verdict` make, so the
            // classification cannot say a worktree is there that they then
            // refuse to find.
            worktree_on_disk: self.surviving_worktree(job).is_ok(),
            checks_passed: checks_passed(ran, job.stopped_on().map(|(step, _)| step)),
            workflow_held: self.workflow_named(job.workflow_id()).is_some(),
        }
    }

    /// What is standing in this Job's slot, and whether Fleet can say anything
    /// to it.
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
    /// **The two absences are told apart rather than folded**, which is #452.
    /// An empty slot and an unreadable Drone both refuse a redirect; only one
    /// of them leaves a Drone for the restart to end and a step for it to stop,
    /// and folded into one word the rule could not say which.
    ///
    /// **What is left is the act that works**: `Stuck::of`'s other arm offers a
    /// restart, which ends the unreadable Drone. An override and a redispatch
    /// end the Drone rather than speak to it, so neither is withheld.
    ///
    /// [`Session::unheard`]: crate::adopting::Session::unheard
    async fn whats_in_the_slot(&self, job: &Job) -> DroneStanding {
        let Some(slot) = self.slot_of(job.id()).await else {
            return DroneStanding::Gone;
        };
        let held = slot.lock().await;
        match held.as_ref().filter(|at_work| at_work.is(job.id())) {
            None => DroneStanding::Gone,
            Some(at_work) if at_work.session().unheard() => DroneStanding::Unheard,
            Some(_) => DroneStanding::Speakable,
        }
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

/// The step a person opening this Job asking why it stopped is asking about.
///
/// `Job::stopped_on`'s step where one stopped, or the Job's current step for a
/// Job-level escalation — `stalled` and `interrupted` stop no step, and it is
/// still the step the Job is holding. Both `refused` and the gate's own
/// undecided sentence read off this, so a screen's evidence and its trigger
/// name the same step.
fn stopped_step(job: &Job) -> Option<&StepId> {
    job.stopped_on()
        .map(|(step, _)| step)
        .or_else(|| job.current_step().map(|step| step.step_id()))
}
