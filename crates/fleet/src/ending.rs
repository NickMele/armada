//! The acts that take something away, and what each one leaves.
//!
//! Cut out of [`daemon`](mod@crate::daemon) on that module's own three-part
//! sentence — what Fleet is made of, the slots it works in, and the things it
//! can be asked. `resume` and `reviewing` hold the rest; these are a ladder
//! rather than leftovers, each rung defined against the one below it.
//!
//! | Act | Asked by | Goes | Survives |
//! |---|---|---|---|
//! | [`kill_drone`](Fleet::kill_drone) | a person | the process, and the step it was on | the Job, its worktree, every step that advanced |
//! | [`drone_at_rest`](Fleet::drone_at_rest) | Fleet | the same two | the same three |
//! | [`kill_job`](Fleet::kill_job) | a person | the Job, at `killed` | the record, and the worktree until `armada clean` |
//! | [`forget_job`](Fleet::forget_job) | a person | the record | nothing this owns; the worktree is `armada clean`'s |
//!
//! **Reading them apart is why they are together.** Each doc below says what
//! it is *not*: a kill that is not terminal, a terminal that is not a verdict,
//! a deletion that is not a way to stop something still running, and a reap
//! nobody asked for.
//!
//! **That last one needs defending, and `Ended` is the defence.** Fleet takes
//! no process away on a judgement of its own; it takes away one whose own
//! terminating event says its run is over.
//!
//! [`stopped_by_hand`](Fleet::stopped_by_hand) is here because `kill_drone` is
//! its only caller and the two are one act as an operator means it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, EscalationTrigger, Job, JobId, StepLevelTrigger, StepState, StepTarget, Target,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drone::{aftermath, Aftermath, Ending};
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
    /// End the Drone. **The Job survives**, and its worktree is held.
    ///
    /// Where the Job then stands is `crate::aftermath`'s answer and not this
    /// method's: a process that is gone having left no evidence pauses the Job
    /// for a person. That is not terminal, so nothing here ends a Job — and it
    /// is not `running` either, which is the state the milestone refuses.
    ///
    /// **The step moves too, and that is what makes the Job recoverable.** On
    /// both endings that leave a person holding it the step the Drone was on
    /// stops under `drone_killed`; on the third it does not, because evidence
    /// is waiting and something is already queued that will rule on it.
    /// [`Fleet::stopped_by_hand`] is the move, and `#313` is what it cost.
    pub async fn kill_drone(&self, job_id: &JobId) -> Result<Job, Adrift> {
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                let ending = Ending::of(
                    &working
                        .as_ref()
                        .expect("the slot was just read as full")
                        .heard(),
                );
                let standing = self.load(job_id).await?.status();
                self.end_the_drone(&mut working).await;
                let job = self.load(job_id).await?;
                match aftermath(standing, &ending, self.left(job_id)) {
                    // **The step first, and the order is `crate::dispatch`'s.**
                    // The inner machine is frozen the moment the Job leaves
                    // `running`, so a step stopped after the move would be
                    // refused and `last_verdict` would stay unwritten — which
                    // is the whole of what this call used to leave behind.
                    Aftermath::JobMoves(target) => {
                        let job = self.stopped_by_hand(&job).await?;
                        self.move_job(&job, target, Actor::Human).await?;
                    }
                    // The Job had already stopped and stays where it is; its
                    // step has not, and it is the same reading that is now
                    // wrong. `escalated` freezes the inner machine, so this one
                    // crosses on `step_machine`'s named exception rather than
                    // on the order above.
                    Aftermath::AlreadyStopped => {
                        self.stopped_by_hand(&job).await?;
                    }
                    Aftermath::TheGateDecides => {}
                }
            }
        }
        self.load(job_id).await
    }

    /// End the Job at `killed`. Terminal, and carrying no verdict.
    ///
    /// Legal from every non-terminal status, including those with no process
    /// under them — which is why it cannot be spelled as
    /// [`kill_drone`](Fleet::kill_drone).
    pub async fn kill_job(&self, job_id: &JobId) -> Result<Job, Adrift> {
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                self.end_the_drone(&mut working).await;
            }
        }
        // And every step the *record* still names one on, which the slot cannot
        // answer for: a Fleet that died holding a Drone leaves the pointer set,
        // and `redispatch` reaches this on a Job whose process this Fleet never
        // held. Without it a killed Job reads on the Board as one with a Drone
        // still on it. `drone_left` answers `Ok` for a step holding nothing, so
        // the ordinary path pays one load.
        self.every_exit_recorded(job_id).await?;
        let job = self.load(job_id).await?;
        let killed = self.move_job(&job, Target::Killed, Actor::Human).await?;
        // **The kill is not deferred and the admission after it is** — `#428`,
        // and the sharpest instance of it: killing one Job used to be able to
        // stop the next one starting, because `admit_next` runs a whole
        // dispatch and this one ran inside a request a client could abandon.
        // The Job is `killed` before this returns; the place it gave back is
        // filled by the turn.
        Ok(killed)
    }

    /// Delete the Job's whole record. **Real deletion**, through
    /// `Store::forget_job`, and only from a terminal status — a Job still in
    /// flight has no record to erase, only a status to move, and `kill_job` is
    /// the act that ends one that is not there yet.
    ///
    /// **It does not reclaim the worktree or the branch.** `armada clean`
    /// already owns that, on its own retention schedule; folding it in here
    /// would give one call two unrelated things to fail at.
    ///
    /// `Store::forget_job` runs through the same lock every other write
    /// takes — there is no second connection opened for it, which is what
    /// makes this safe to call from inside a live Fleet in the first place.
    pub async fn forget_job(&self, job_id: &JobId) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        if !job.status().is_terminal() {
            return Err(Adrift::NotForgettable {
                job: job_id.clone(),
                status: job.status(),
            });
        }
        self.store()
            .lock()
            .await
            .forget_job(job_id)
            .map_err(Adrift::Writing)?;
        self.publish(ipc::Event::JobForgotten(ipc::JobForgotten {
            job_id: job_id.into(),
        }));
        Ok(())
    }

    /// Stop the step a person has just taken the Drone off.
    ///
    /// **What it has to get right is `Fleet::stopped_step`'s contract**, not
    /// the process's: a `stopped` row carrying `failed(<trigger>)` is what
    /// `restart_step` reads, and writing one without the gate having ruled on
    /// anything is the only place in Fleet that happens.
    ///
    /// **`Ok` and unchanged where no step is running**, which is not a fault
    /// and is the ordinary shape at a step boundary and on a Job whose steps
    /// have all advanced. `crate::drone_moves` answers the same way about a
    /// step holding no Drone, for the same reason: what a kill can be sure of
    /// is the process, and everything else is read off the record.
    pub(crate) async fn stopped_by_hand(&self, job: &Job) -> Result<Job, Adrift> {
        let why = StepLevelTrigger::of(EscalationTrigger::DroneKilled)
            .expect("`drone_killed` is step-level in the registry");
        // **Human, not Fleet.** This act exists because somebody pressed
        // something, and a row saying Fleet took the process away would claim a
        // decision it did not make. [`Fleet::stopped_at_rest`] is the row that
        // legitimately says Fleet, and the actor is the only difference between
        // the two calls.
        self.stopped_step_under(job, why, Actor::Human).await
    }

    /// Take away a Drone whose own run has ended, and stop the step it was on.
    ///
    /// **The only act on this ladder Fleet asks for**, and `DroneEvent::Ended`
    /// is what makes it defensible rather than a judgement: the Drone said its
    /// own run was over, and anything it does after saying so is outside what
    /// it told Fleet. The reading is `crate::silence`'s and so is its argument.
    ///
    /// **The cost is written down rather than discovered.** A Drone that
    /// reports its run has ended and then keeps working loses that work. It is
    /// accepted because the alternative is the row this reaps: a step saying
    /// `running` beneath a Drone that has said it is finished.
    ///
    /// **The order is `kill_drone`'s, and load-bearing for the same reason.**
    /// The process, then the step, then the Job — the inner machine freezes the
    /// moment the Job leaves `running`, so a step stopped after the move is
    /// refused and `last_verdict` stays unwritten. The step stops while the Job
    /// is still `running`, so this needs no `step_machine` exception of its own.
    ///
    /// **It will not hang on a process that will not go**, which is `#371`.
    /// `DroneSession::terminate` sends `SIGKILL`, uncatchable, and waits on
    /// Fleet's own direct child. The pipe is the other wait — a tool the Drone
    /// spawned can hold it open — and [`drained`](crate::Watching::drained)
    /// bounds it and says whether it got to the end. Both answers ride out on
    /// [`StoodDown`]: a slot handed back over a transcript cut short must not
    /// be handed back silently.
    pub(crate) async fn drone_at_rest(
        &self,
        target: Target,
        working: &mut Option<Working>,
    ) -> Result<Option<StoodDown>, Adrift> {
        let Some(at_work) = working.take() else {
            return Ok(None);
        };
        let job_id = at_work.standing().0;
        // **Not `boundary::stood_down`, and the difference is one log line.**
        // That one writes "the Drone was ended because its step ended", which
        // is the sentence this whole defect is about being false. What both
        // reach for is `stood_down_paying`, which is the ending and the spend
        // in the one order that makes the figure real; the reason a Drone was
        // ended belongs on `noted_stood_down` as a parameter rather than in a
        // second copy of that sentence here.
        let stood_down = self.stood_down_paying(at_work).await?;
        self.every_exit_recorded(&job_id).await?;
        let job = self.load(&job_id).await?;
        let job = self.stopped_at_rest(&job).await?;
        self.move_job(&job, target, Actor::Fleet).await?;
        // Ordinarily nothing, for `dispatch::reap`'s reason: the reading that
        // reaches here is one where `left` answered `Left::Nothing`. It is
        // called anyway because a Drone that submitted between that reading and
        // this line left evidence against a step that has now stopped, and a
        // drop nobody wrote down is the defect that pair closes.
        let dropped = self.empty_the_inbox(&job_id);
        self.dropped_with_the_job(&job_id, dropped);
        Ok(Some(stood_down))
    }

    /// Stop the step Fleet has just taken a finished Drone off.
    ///
    /// **Fleet, and this is the one row where that is true of a Drone being
    /// taken away.** `drone_killed` is a person's decision and says so;
    /// `run_ended` is Fleet acting on the Drone's own last word, and a reader
    /// who opens it finds nobody to ask about it.
    async fn stopped_at_rest(&self, job: &Job) -> Result<Job, Adrift> {
        let why = StepLevelTrigger::of(EscalationTrigger::RunEnded)
            .expect("`run_ended` is step-level in the registry");
        self.stopped_step_under(job, why, Actor::Fleet).await
    }

    /// Stop the step a Drone was working, under whichever trigger took it away.
    ///
    /// **What it has to get right is `Fleet::stopped_step`'s contract**, not
    /// the process's: a `stopped` row carrying `failed(<trigger>)` is what
    /// `restart_step` reads, and writing one without the gate having ruled on
    /// anything is the only place in Fleet that happens.
    ///
    /// **`Ok` and unchanged where no step is running**, which is not a fault
    /// and is the ordinary shape at a step boundary and on a Job whose steps
    /// have all advanced. `crate::drone_moves` answers the same way about a
    /// step holding no Drone, for the same reason: what an ending can be sure
    /// of is the process, and everything else is read off the record.
    async fn stopped_step_under(
        &self,
        job: &Job,
        why: StepLevelTrigger,
        by: Actor,
    ) -> Result<Job, Adrift> {
        let Some(step) = job
            .current_step()
            .filter(|row| row.state() == StepState::Running)
            .map(|row| row.step_id().clone())
        else {
            return Ok(job.clone());
        };
        self.move_step_by(job, &step, StepTarget::Stopped(why), by)
            .await
    }
}
