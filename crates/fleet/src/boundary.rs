//! Crossing a step boundary: the Drone that finished ends, and a fresh one is
//! put on the same worktree for the next step.
//!
//! A Drone belongs to a workflow step — `docs/concepts/drone.md`. The worktree
//! and the branch survive, because nothing in this workspace can remove one;
//! what crosses is a directory holding every step's work so far, uncommitted,
//! as `crate::landing` requires.
//!
//! # The order is fixed, and each part of it answers a failure
//!
//! 1. **Terminate explicitly.** [`put_a_drone_on`](Fleet::put_a_drone_on)
//!    assigns straight over the slot, and dropping a [`Working`] drops its
//!    `DroneSession` without signalling. The child is `setsid`-detached, so it
//!    survives its parent's handle going away and keeps spending.
//! 2. **Drain before dropping, bounded.** `Drop` aborts the reader over the
//!    Drone's last lines, and a tool it spawned holds the pipe open — `#211`.
//! 3. **Record the exit before the spawn.** `Job::drone_spawned` refuses over a
//!    live pointer — `#137`'s `AlreadyAssigned` — so a spawn ordered first is
//!    refused by the record it had just failed to update.
//!
//! The first two are [`Working::stood_down`], which consumes the slot; the
//! third and the spawn are here, because both reach a store.
//!
//! What crosses is `crate::crossing`'s value and not the injected turn, for the
//! reason that module's own doc gives: a rendered turn cannot be re-tensed.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level, StepId};

use crate::adrift::Adrift;
use crate::briefing::Opening;
use crate::crossing::Crossed;
use crate::daemon::Fleet;
use crate::drone::Ending;
use crate::drone_moves::steps_holding_a_drone;
use crate::transcript;
use crate::watch::Drained;
use crate::working::{StoodDown, Working};

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
    /// The whole boundary: end the Drone that finished its step, and put a
    /// fresh one on the same worktree for `next`.
    ///
    /// **Every advance a Drone in a slot makes comes here**, which is the
    /// mechanical one in `crate::dispatch` and, since #50's follow-on, only
    /// that one. A person's advance at a human gate and a person's override
    /// both cross a step boundary too, and both take the Job back to `queued`
    /// instead — because the boundary they cross also has to consult
    /// `concurrency-cap`, and only admission does. `crate::readmitting` is
    /// where they arrive; what they hand a fresh Drone is still a `Crossed`,
    /// which is the point of that type.
    ///
    /// **A slot that is already empty is not an error.** The Fleet holding the
    /// process restarted, or the Drone ended on its own, and the answer is the
    /// same either way: the worktree is still on disk and a fresh Drone is put
    /// on it. That is the arm `crate::overruling` used to spell for itself.
    ///
    /// **The catch-up, the brief and the baseline are all
    /// [`put_a_drone_on`](Fleet::put_a_drone_on)'s**, which is what makes the
    /// rebase run once per boundary and its outcome ride the opening turn.
    /// There is no session to inject it into any more, which is `#180`'s
    /// ordering arriving everywhere at once.
    pub(crate) async fn crossed_onto(
        &self,
        job: &Job,
        next: &StepId,
        crossed: Crossed,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        let stood_down = self.stood_down(job.id(), working).await?;
        // The slot's worktree where there was a slot, and the directory on disk
        // where there was not. Both name the same place for the same Job; the
        // first is free and the second is a `stat`.
        let worktree = match stood_down {
            Some(stood_down) => stood_down.worktree,
            None => self.surviving_worktree(job)?,
        };
        self.put_a_drone_on(
            job,
            next,
            worktree,
            Opening::fresh().carrying(crossed),
            working,
        )
        .await
    }

    /// End the Drone in the slot, drain what it said, and record that it left.
    ///
    /// **`Ok(None)` is a slot that was already empty**, which is a legitimate
    /// state and not a failure — see [`crossed_onto`](Fleet::crossed_onto).
    ///
    /// **The exit is returned rather than noted.** `end_the_drone` swallows the
    /// same refusal into the Job's log, and it is right to: its callers are
    /// ending a Drone as part of a move the Job has already made, so there is
    /// nothing left to abandon. Here there is. A spawn follows, and
    /// `drone_spawned` refuses over a pointer that is still set — so an exit
    /// that silently failed would become a boundary that could not put a Drone
    /// on the next step, with nothing saying why.
    ///
    /// It sweeps the record as well as the slot. A step the record still names
    /// a Drone on and the slot does not is the ordinary shape after a Fleet
    /// restart, and it refuses the spawn exactly as a live pointer would.
    pub(crate) async fn stood_down(
        &self,
        job_id: &JobId,
        working: &mut Option<Working>,
    ) -> Result<Option<StoodDown>, Adrift> {
        // **The spend is folded inside the stand-down and before the exit is
        // recorded**, and this returns if it will not write. The figure is what
        // the Job's cap is compared against, and one that silently failed to
        // land would leave a Job with an allowance it can never exhaust.
        // Recording twice is harmless: the row is keyed on the Drone, so
        // `dispatch::reap` having already folded this run writes the same row
        // again. See `Fleet::stood_down_paying`.
        let stood_down = match working.take() {
            Some(at_work) => Some(self.stood_down_paying(at_work).await?),
            None => None,
        };
        if let Some(stood_down) = &stood_down {
            self.noted_stood_down(stood_down);
        }
        self.every_exit_recorded(job_id).await?;
        Ok(stood_down)
    }

    /// Record a departure for every step of this Job the record still names a
    /// Drone on.
    ///
    /// **Read off the record and not off the slot**, because the two answer
    /// different questions: the slot says what this Fleet is holding and the
    /// record says what a spawn will be refused over. `drone_left` answers `Ok`
    /// for a step holding nothing, so this is a no-op on the ordinary path and
    /// costs one load.
    ///
    /// Its other two callers are `restart_step` and `kill_job`, which both
    /// leave a pointer standing today — the first then spawns onto the step
    /// whose pointer it left, and the second leaves a killed Job reading as
    /// though a Drone were still on it.
    pub(crate) async fn every_exit_recorded(&self, job_id: &JobId) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        for step in steps_holding_a_drone(&job) {
            self.drone_left(job_id, &step).await?;
        }
        Ok(())
    }

    /// Write the ending into the Job's own log.
    ///
    /// **The fold, never the words.** What goes down is whether the Drone came
    /// to rest, whether it reached for anything and how often it was refused —
    /// three counts, which is what `Ending` is. Nothing the Drone said is
    /// written here; the transcript holds that, and it now holds the last lines
    /// too, because the drain waited for them.
    ///
    /// **Unless it could not, and then the line says so.** A drain that was cut
    /// short is a `warn` with a sentence of its own, because the `Ending` beside
    /// it is a fold over a prefix: a Drone whose terminating event was still in
    /// the pipe reads as one that vanished, which is a different thing for a
    /// person to do about it. See [`Drained`].
    fn noted_stood_down(&self, stood_down: &StoodDown) {
        let envelope = Envelope::new(
            self.now(),
            match stood_down.drained {
                Drained::ToTheEnd => Level::Info,
                Drained::CutShort { .. } => Level::Warn,
            },
            Component::Fleet,
            self.run().clone(),
            match stood_down.drained {
                Drained::ToTheEnd => "the Drone was ended because its step ended",
                Drained::CutShort { .. } => {
                    "the Drone was ended because its step ended, and its \
                     transcript was cut short: something the Drone spawned \
                     outlived it holding the same output"
                }
            },
        )
        .in_job(stood_down.job.as_ulid().clone())
        .at_step(stood_down.step.as_str())
        .with_field(
            "drone_id",
            FieldValue::Str(stood_down.drone.as_str().to_string()),
        )
        .with_field("ending", FieldValue::Str(said(&stood_down.ending)))
        .with_field("signalled", FieldValue::Bool(stood_down.terminated.is_ok()))
        .with_field("transcript", FieldValue::Str(heard(&stood_down.drained)));
        // A log line that will not write does not undo the ending, for
        // `resume::noted_roused`'s reason: the departure is its own record, in
        // the same log, written by `drone_left`.
        let _ = transcript::note(&self.host().repo_root, &stood_down.job, &envelope);
    }
}

/// How the run finished, in one word, for the log.
///
/// **Not a verdict and it cannot become one.** `Ending` has no `Succeeded` for
/// the reason its own doc gives, and a boundary is reached because the *gate*
/// passed the step — what the process did on its way out is a separate fact and
/// this is all of it that is legible.
fn said(ending: &Ending) -> String {
    match ending {
        Ending::Reported {
            refusals,
            called_something,
        } => format!("reported, {refusals} refusal(s), called_something={called_something}"),
        Ending::Vanished => String::from("no terminating event ever arrived"),
    }
}

/// How much of the transcript was read, in one phrase, for the same line.
///
/// **It qualifies `ending` and is written beside it for that reason.** Cut
/// short, the ending is a fold over as much as arrived before the bound, and
/// nothing else on the line would say so.
fn heard(drained: &Drained) -> String {
    match drained {
        Drained::ToTheEnd => String::from("read to the end of the pipe"),
        Drained::CutShort { waited } => format!(
            "cut short after {}s: the pipe was still held open",
            waited.as_secs()
        ),
    }
}
