//! The one vigil whose subject is a Job rather than a Drone.
//!
//! Between a person approving a Job and its first Drone starting there is a
//! worktree to cut, a Manifest's `setup.requires` to run and an opening brief
//! to assemble. [`silence`](mod@crate::silence) measures a Drone that has
//! stopped speaking and [`converging`](mod@crate::converging) one that is
//! getting nowhere; here there is no Drone for either to have a subject. So a
//! Job that stopped in this span read as healthy on the Board, carried no
//! `stuck`, offered no act, and stayed that way for as long as Fleet stayed
//! up. `#436`.
//!
//! # What used to wedge, and what still can
//!
//! `approve_dispatch` ran the whole of [`crate::dispatch`] **inside the HTTP
//! request that approved the Job**, Bridge waits five seconds, and a cold
//! `pnpm install` is not five seconds. When the wait was spent axum dropped
//! the handler's future and `kill_on_drop` took the install. That is `#435`'s
//! third answer: not spawned and lost, not never spawned, but **spawned and
//! then taken away with everything that would have said so** — the
//! `tokio::time::timeout` bounding the command included.
//!
//! **`#428` moved the dispatch off that future**, so a client stopping can no
//! longer reach one. What is left is Fleet's own doing: `Turning::drop` aborts
//! the turn in flight, so a Fleet stopping during a long preparation leaves
//! exactly this state, and the reading and the answer are identical.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    StepState, Target,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::transcript;

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
    /// Escalate every Job that is `running` before its first step with nothing
    /// attending it.
    ///
    /// **The slot is the attendance, so there is no clock in here.** A Job
    /// being dispatched holds a slot in [`crate::slots`], and holds it *locked*
    /// for the whole of `admit_next`; `Slots::sweep` keeps a slot that is
    /// locked or that holds a `Working`, and discards one that is neither. The
    /// roster therefore already answers, exactly, whether anything in this
    /// process has a hand on a Job — and a ten-minute `playwright install` on a
    /// laptop that has never run one is never asked how long it has been. A
    /// bound on the span would be the regression `#436` names first.
    ///
    /// | Clause | Excludes |
    /// |---|---|
    /// | `running` | a Job queued, at a gate, finished, or already escalated by dispatch's own failure paths |
    /// | every step `not_started` | every Job past its first spawn — `dispatch` moves step one to `running` before it asks for a Drone |
    /// | in no slot | the dispatch in flight, the Drone at work, the gate running a Check, the adopted Drone `crate::readopting` put back |
    ///
    /// **Once for the turn and before `admit_next`**, for `strand_dependents`'s
    /// reason: the subject is in no slot, so a walk over slots could not reach
    /// it however often it ran. The roster is read into a list and let go
    /// before anything is written — holding it across a store write would take
    /// the two locks in the order [`crate::slots`] forbids.
    pub(crate) async fn watch_unattended(&self) -> Result<Vec<JobId>, Adrift> {
        let attended = self.working_on().await;
        let (loaded, _) = self.every_job().await?;
        let mut abandoned = Vec::new();
        for job in &loaded.jobs {
            if !unattended(job) || attended.contains(job.id()) {
                continue;
            }
            self.noted_unattended(job);
            // **`not_prepared`, and this is the decision the module makes that
            // nobody else had.** The row reads as a failure — "a command … did
            // not succeed" — and here nothing ran at all. It is still right:
            // the clause a person acts on is the second, "nothing had been
            // spawned and no step had been entered", and that is exactly true.
            // `interrupted` would send them after a Drone that never existed,
            // and a trigger meaning *abandoned* would be a word whose only
            // difference from this one is how Fleet found out.
            self.move_job(
                job,
                Target::Escalated(EscalationTrigger::NotPrepared),
                Actor::Fleet,
            )
            .await?;
            abandoned.push(job.id().clone());
        }
        Ok(abandoned)
    }

    /// The line in the Job's own log, written **before** the move.
    ///
    /// `Warn` and not `Info`: every other line this Job has says a preparation
    /// started, and the one saying nobody is holding it any more is what a
    /// person scanning for the moment it went wrong is looking for.
    fn noted_unattended(&self, job: &Job) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "no Drone was ever started on this Job and nothing in Fleet is preparing it",
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("attending", FieldValue::Str("nothing".to_string()));
        // A log line that will not write does not undo the move, for
        // `silence::noted_quiet`'s reason: the transition is its own record.
        let _ = transcript::note(&self.host().repo_root, job.id(), &envelope);
    }
}

/// Whether this Job is in the span the vigil watches: `running`, and no step
/// has ever been entered.
///
/// **Nothing reaches this state by design.** `restart_step` leaves a Job
/// `queued` and needs a stopped step, a stand-down leaves it `queued`, and the
/// boot reconciliation runs before the first turn and answers `interrupted`
/// for a Job a dead Fleet left running. What is left is an abandoned dispatch.
///
/// **A detection and not a repair.** The caller escalates rather than
/// re-running the dispatch, because the worktree now holds whatever a
/// half-finished install left in it and Fleet cannot tell what that is.
/// `Recourse::Redispatch` is what a person is offered, which `crate::stuck`
/// already gives a top-level Job at `escalated` — non-empty, which `#436`
/// requires. **The cause a client could reach is gone**: `#428` took the
/// dispatch off the request's future, so what reaches this now is a turn Fleet
/// itself abandoned rather than a browser that stopped waiting.
///
/// A free function, so the reading is one expression and testable without a
/// Fleet. Whether anything is *attending* it is the roster's answer and is
/// deliberately not in here: the two facts come from different places, and
/// folding them would make the predicate need a lock.
fn unattended(job: &Job) -> bool {
    job.status() == JobStatus::Running
        && job
            .steps()
            .iter()
            .all(|step| step.state() == StepState::NotStarted)
}
