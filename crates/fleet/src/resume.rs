//! The two acts that put a person back on a Job, and what writes their step.
//!
//! # Which one applies is decided by the Drone, not by the person
//! [`redirect`](Fleet::redirect) needs a live session and [`restart_step`] is
//! what exists when there is none, so each refuses where the other applies —
//! `docs/concepts/job.md` is the specification. **Neither asks the other's
//! question**: a redirect wanted a stopped step until `stalled` escalated a Job
//! over a live Drone.
//!
//! # A redirect moves a stopped step; a restart moves nothing
//! Where a step stopped, a redirect moves both machines in the one order the
//! registry admits — `escalated -> running`, then `stopped -> running`. Where
//! none stopped there is nothing to unfreeze, the Job stays `escalated` and
//! returns on [`watch_redirect`](Fleet::watch_redirect) seeing the Drone turn:
//! moved on the sending it would read as recovered whether or not anything
//! woke.
//!
//! **A restart takes neither move**, because it spawns and since #50 nothing
//! spawns outside admission: `escalated -> queued`, step left at `stopped`,
//! and `crate::readmitting` makes both moves when there is room.
//!
//! **Nothing here is bounded.** A person who can redirect can redirect for
//! ever; whether that is capped is decided in no document.
//!
//! [`restart_step`]: Fleet::restart_step

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, Job, JobId, JobStatus, Level, StepId,
    StepLevelTrigger, StepState, StepTarget, Target,
};

use crate::adrift::Adrift;
use crate::briefing::Stopped;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};
use crate::transcript;
use crate::working::Working;

/// A Drone that took a turn after a person redirected it, and the Job that came
/// back to `running` with it.
///
/// **The Drone's evidence, reported rather than inferred.** It is on `Turned`
/// beside what the liveness vigil did, because the two are one question read in
/// the two directions: one says a Drone stopped and the other says it started.
#[derive(Debug)]
pub struct Roused {
    pub job: JobId,
    pub step: StepId,
}

/// A person's instruction to a Drone that is there.
///
/// **Never empty.** There is no constructor that takes a blank string: an
/// instruction that says nothing is a poke, the poke is a different turn with
/// its own wording, and a Drone told nothing at all would resume the step it
/// stopped on with exactly the information that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redirection(String);

impl Redirection {
    /// `None` where there is nothing in it a Drone could act on.
    pub fn saying(instruction: &str) -> Option<Redirection> {
        let said = instruction.trim();
        (!said.is_empty()).then(|| Redirection(said.to_string()))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

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
    /// Steer the Drone that is already on the Job. **Intervention Ladder rung
    /// one**, and the cheapest of the four acts: the process is there holding
    /// its session, its context and its worktree, and the instruction is a turn
    /// injected into it.
    ///
    /// The actor is **human**. Fleet redirects nothing of its own accord —
    /// deciding what to say is exactly the part a person was escalated to.
    ///
    /// **The Drone is told the person's words and nothing else.** It is not
    /// told the Judge's citation, because the person read that citation and
    /// wrote the instruction from it; `docs/contracts/agent-prompt.md` gives
    /// this turn no Fleet wording for the same reason.
    ///
    /// **What it asks for is a live session, and nothing about a step.** A
    /// redirect is a turn injected into a process; whether some step of the Job
    /// is frozen decides what else has to move, never whether this act applies.
    /// It was gated on a stopped step until `stalled` arrived — the first
    /// trigger that pauses a Job over a Drone that is still there — and the
    /// only move left on the case a redirect most obviously fits was
    /// kill-and-redispatch.
    pub async fn redirect(&self, job_id: &JobId, instruction: &Redirection) -> Result<Job, Adrift> {
        let Some(slot) = self.slot_of(job_id).await else {
            return Err(Adrift::NoDroneToRedirect {
                job: job_id.clone(),
            });
        };
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        self.held_for_a_person(&job)?;
        // The live session, which is the whole difference between the two acts
        // — and it is asked of the slot rather than of the record, because the
        // slot is the only thing holding a pipe. A record saying a Drone is on
        // a Job this Fleet did not spawn is repaired at the boot read.
        let Some(at_work) = working.as_ref().filter(|at_work| at_work.is(job_id)) else {
            return Err(Adrift::NoDroneToRedirect {
                job: job_id.clone(),
            });
        };
        // **Before the write**, for `Working::answering`'s reason: an answer
        // that arrived between the two would be inside a baseline taken after
        // and would read as a Drone that never turned.
        let turned = at_work.turned();

        // The step, where one stopped. `Err` is the `stalled` shape and is not
        // a refusal here — a Job-level escalation freezes nothing underneath
        // it, so there is nothing for this act to unfreeze.
        let stopped = self.stopped_step(&job).ok();
        let job = match stopped.as_ref() {
            // After both moves, never before. A turn delivered to a Drone whose
            // Job then failed to move would be an instruction acted on by a
            // process nobody had unpaused.
            Some(step) => self.resumed(&job, step, Actor::Human).await?,
            None => job,
        };
        self.instruct(job_id, instruction, &working).await?;
        // The step is running again, so the thrashing chain is too. Without
        // this a Drone steered off one loop is never caught in the next.
        if let Some(at_work) = working.as_mut() {
            at_work.resumed(self.now());
            // And where nothing was unfrozen, the Job is still `escalated` and
            // stays there until the Drone proves it heard. See
            // [`watch_redirect`](Fleet::watch_redirect).
            if stopped.is_none() {
                at_work.awaiting_answer(turned, self.now());
            }
        }
        Ok(job)
    }

    /// A Job comes back to `running` when its Drone turns, and never on the
    /// send. **The deferred half of [`redirect`](Fleet::redirect)**, kept beside
    /// it rather than in a watcher of its own so that the act and what completes
    /// it are read together.
    ///
    /// **Cold on every turn but one.** The slot answers `false` unless a
    /// redirect is outstanding, so a Drone nobody has spoken to reaches no
    /// store — the property [`watch_silence`](Fleet::watch_silence) has, for the
    /// same reason.
    ///
    /// The actor is **human**. A person took a Job out of `escalated` by
    /// redirecting it; Fleet only chose the instant, once the Drone had shown
    /// the instruction landed. `job-statuses.toml` says a person is who acts on
    /// an escalated Job, and a row saying Fleet un-escalated one would be Fleet
    /// claiming a decision it did not take.
    pub(crate) async fn watch_redirect(
        &self,
        working: &mut Option<Working>,
    ) -> Result<Option<Roused>, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(None);
        };
        if !at_work.turned_since_redirect() {
            return Ok(None);
        }
        let (job, step, _) = at_work.standing();
        // From here it costs a store read, and only on the turn a redirect is
        // answered.
        let record = self.load(&job).await?;
        // The Job left `escalated` some other way while the redirect was out —
        // killed, piloted, failed. Nothing is owed and nothing is moved; what is
        // dropped is only the wait.
        if record.status() == JobStatus::Escalated {
            self.move_job(&record, Target::Running, Actor::Human)
                .await?;
            self.noted_roused(&job, &step);
        }
        if let Some(at_work) = working.as_mut() {
            at_work.answered();
        }
        Ok(Some(Roused { job, step }))
    }

    /// The redirect this Job's Drone has not answered yet, for `get_job`.
    ///
    /// **The third part of the act, and the one a person reads.** The send is
    /// [`redirect`](Fleet::redirect) and the return to `running` is
    /// [`watch_redirect`](Fleet::watch_redirect); between them the Job sits
    /// `escalated`, looking exactly like a Job nobody has spoken to. This is
    /// what tells the two apart, and it is on the wire rather than in the window
    /// that pressed the button because that window's memory of it does not
    /// survive a reload and never existed in a second one.
    ///
    /// **A fact about the last act, and not a status.** The Job stays where the
    /// escalation put it; nothing mints a seventh status for a Job already in
    /// the one it belongs in. It says Fleet wrote to the pipe and nothing more
    /// — whether the Drone read it is the turn `watch_redirect` waits for.
    ///
    /// `None` where nothing is outstanding, where the slot holds some other Job,
    /// and on every redirect that landed on a stopped step: that one moved both
    /// machines on the send, so there was never anything to wait for.
    pub(crate) async fn redirect_awaited(&self, job: &JobId) -> Option<ipc::RedirectInFlight> {
        let slot = self.slot_of(job).await?;
        let working = slot.lock().await;
        working
            .as_ref()
            .filter(|at_work| at_work.is(job))
            .and_then(|at_work| at_work.awaiting_since())
            .map(|sent_at| ipc::RedirectInFlight {
                sent_at: sent_at.into(),
            })
    }

    /// Write the answer into the Job's own log. **Fields say who and when; the
    /// wording says what happened**, and neither carries anything the Drone
    /// said — that the Drone turned is a count, and what it turned about is a
    /// claim the gate exists to refuse.
    fn noted_roused(&self, job: &JobId, step: &StepId) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "the Drone took a turn after being redirected and the Job is running again",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str());
        // A log line that will not write does not undo the move, for
        // `silence::noted_quiet`'s reason: the transition is its own record.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }

    /// Ask for a fresh Drone on the worktree the last one left. **The second
    /// act**, and what exists when the Drone is gone.
    ///
    /// **It asks; it does not start one.** The Job takes `escalated -> queued`
    /// and `crate::readmitting` spawns when `concurrency-cap` has room, which
    /// is the shape `approve_review` has had since #50 — and the reversal
    /// `job-transitions.toml` records on the edge itself. **Never refused
    /// because the cap is spent**: the act lands and the Job says
    /// `waiting_on_resources`.
    ///
    /// **The step does not move here either**, because `store::attempt` counts
    /// entries into `running` as runs and a run belongs to a Drone. It waits at
    /// `stopped`, which is both what a rail should render and how re-admission
    /// knows which act to answer.
    ///
    /// **Every guard below does run here**, because each is a question about
    /// *now* — a Drone still standing, a worktree still there, an exit still
    /// unrecorded. Deferred, they would answer about a different instant.
    ///
    /// **The worktree, the branch and every earlier step's work survive**, and
    /// the branch is caught up inside
    /// [`put_a_drone_on`](Fleet::put_a_drone_on) rather than here — #180. A
    /// restart is the case a rebase most often conflicts on, since it re-runs
    /// the same step on a tree already holding an attempt; the markers ride the
    /// opening brief. **Nothing is inherited from the Drone before it.**
    pub async fn restart_step(&self, job_id: &JobId) -> Result<Job, Adrift> {
        // Looked up rather than opened: this act starts nothing, so it needs no
        // place in the roster. What it wants the slot for is the one question
        // only the slot can answer — whether a Drone is still standing here —
        // and a Job with none has no slot to hold.
        let slot = self.slot_of(job_id).await;
        let job = self.load(job_id).await?;
        // **First, and the order is the refusals'.** A Job that is `running`,
        // or escalated with no step stopped, hears which act applies instead —
        // and hears it before "a Drone is still there", which is true of both
        // and names neither.
        self.stopped_step(&job)?;
        let standing = match &slot {
            Some(slot) => slot.lock().await.as_ref().is_some_and(|at| at.is(job_id)),
            None => false,
        };
        if standing {
            return Err(Adrift::DroneStillThere {
                job: job_id.clone(),
            });
        }
        // Read and discarded, for the reason it is read: a restart onto a
        // worktree that has been reclaimed is a redispatch, and saying so
        // before the Job moves leaves it exactly where the person found it.
        // `crate::readmitting` reaches the same directory again at the spawn.
        self.surviving_worktree(&job)?;
        // **Before the Job leaves `escalated`, and this used to be before the
        // spawn.** The record can still name a Drone on the step being
        // restarted: a Fleet that died holding one leaves exactly that, and
        // `reconcile` clears it only for the Jobs it read at boot.
        // `drone_spawned` refuses over a live pointer, so a restart onto a step
        // whose pointer nobody cleared is a restart that cannot happen — and
        // hearing that now is better than hearing it when the queue reaches
        // this Job.
        self.every_exit_recorded(job_id).await?;
        // The actor is **human**. A person took the Job out of `escalated`;
        // Fleet only chooses which turn it gets a process back, which is the
        // `queued -> running` row `crate::readmitting` writes as its own.
        self.move_job(&job, Target::Queued, Actor::Human).await?;
        // Inline, exactly as an approval re-admits inline — so a fleet with
        // nothing else to do restarts the step now rather than on the next
        // tick, and a busy one leaves the Job in the queue where a person can
        // see it waiting.
        self.admit_next().await?;
        self.load(job_id).await
    }

    /// The Job is one a person is holding. **What both acts need, and all
    /// either of them needs of the status.**
    ///
    /// It is `escalated` and nothing weaker: `escalated -> running` is the edge
    /// both acts take, and the registry has no other status it leaves from.
    fn held_for_a_person(&self, job: &Job) -> Result<(), Adrift> {
        (job.status() == JobStatus::Escalated)
            .then_some(())
            .ok_or_else(|| Adrift::NotResumable {
                job: job.id().clone(),
                status: job.status(),
            })
    }

    /// Stop the step a person has just taken the Drone off.
    ///
    /// **The other end of this file's subject.** Everything above consumes a
    /// stopped step; this is the one act that makes one without the gate having
    /// ruled on anything, and it is here rather than beside `kill_drone`
    /// because what it has to get right is `stopped_step`'s contract and not
    /// the process's.
    ///
    /// **`Ok` and unchanged where no step is running**, which is not a fault
    /// and is the ordinary shape at a step boundary and on a Job whose steps
    /// have all advanced. `crate::drone_moves` answers the same way about a
    /// step holding no Drone, for the same reason: what a kill can be sure of
    /// is the process, and everything else is read off the record.
    pub(crate) async fn stopped_by_hand(&self, job: &Job) -> Result<Job, Adrift> {
        let Some(step) = job
            .current_step()
            .filter(|row| row.state() == StepState::Running)
            .map(|row| row.step_id().clone())
        else {
            return Ok(job.clone());
        };
        let why = StepLevelTrigger::of(EscalationTrigger::DroneKilled)
            .expect("`drone_killed` is step-level in the registry");
        // **Human, not Fleet.** Fleet ends a Drone of its own accord nowhere;
        // this act exists because somebody pressed something, and a row saying
        // Fleet took the process away would claim a decision it did not make.
        self.move_step_by(job, &step, StepTarget::Stopped(why), Actor::Human)
            .await
    }

    /// The step a restart lands on, or why there is not one.
    ///
    /// **A step-level escalation is what makes a restart coherent.** Only a
    /// step-level trigger reaches a step's `last_verdict`, so only a step-level
    /// escalation leaves a `stopped` row naming which step to run again — and a
    /// Job escalated on `interrupted`, `resource_exhausted` or `stalled` has
    /// none. For the first two the Drone is gone as well, which leaves
    /// redispatch and Pilot; for the third it is alive, and a redirect is what
    /// answers it.
    ///
    /// **A redirect does not call this to decide whether it applies.** It calls
    /// it to learn whether anything has to be unfrozen, which is a different
    /// question and is why this stopped being one predicate.
    fn stopped_step(&self, job: &Job) -> Result<StepId, Adrift> {
        self.held_for_a_person(job)?;
        // `Job::stopped_on` is the reading `crate::overruling`,
        // `crate::regating` and the classification all make. It answers the
        // step and the trigger together; only the step is wanted here, because
        // a restart lands on a stopped step whatever stopped it.
        job.stopped_on()
            .map(|(step, _)| step.clone())
            .ok_or_else(|| Adrift::NoStepStopped {
                job: job.id().clone(),
            })
    }

    /// The two moves, in the one order the machines admit.
    async fn resumed(&self, job: &Job, step: &StepId, by: Actor) -> Result<Job, Adrift> {
        let job = self.move_job(job, Target::Running, by).await?;
        self.move_step(&job, step, StepTarget::Running).await
    }

    /// The worktree the stopped Drone was working in, if it is still there.
    ///
    /// **A restart with no worktree is a redispatch and says so** rather than
    /// silently becoming one. `armada clean` keeps a branch the base cannot
    /// reach, and a worktree can still be reclaimed — at which point the
    /// earlier steps' work is not on disk and there is nothing to resume onto.
    pub(crate) fn surviving_worktree(&self, job: &Job) -> Result<Worktree, Adrift> {
        let spec =
            WorktreeSpec::for_job(&self.host().repo_root, job.id().as_str()).map_err(|cause| {
                Adrift::Unworkable {
                    job: job.id().clone(),
                    cause,
                }
            })?;
        if !Path::new(&spec.worktree_path()).is_dir() {
            return Err(Adrift::WorktreeGone {
                job: job.id().clone(),
                path: spec.worktree_path(),
            });
        }
        // The branch the record holds, never the one the spec derives: a
        // branch a reader recomputes cannot have been renamed.
        let branch = job
            .branch()
            .map(|branch| branch.as_str().to_string())
            .unwrap_or_else(|| spec.branch());
        Ok(Worktree::at(spec.worktree_path(), branch))
    }

    /// Why this step stopped, read off the record.
    ///
    /// The same three a person reads on the detail view — the verdict, the
    /// Judge's answers, and what the gaming check flagged. **Nothing is
    /// composed here**: a restarted Drone is told what the log says and not
    /// what Fleet infers.
    ///
    /// **`crate::readmitting` is the caller**, because a restart asks for a
    /// Drone here and gets one there. It is read at the spawn and not at the
    /// act for the reason the step move is deferred: the judgments and flags
    /// are filed by attempt, and this must read the attempt that stopped.
    pub(crate) async fn what_stopped(&self, job: &Job, step: &StepId) -> Result<Stopped, Adrift> {
        let store = self.store().lock().await;
        let judged = store.step_judgments(job.id()).map_err(Adrift::Reading)?;
        let flagged = store.step_gaming_flags(job.id()).map_err(Adrift::Reading)?;
        Ok(Stopped {
            verdict: job.step(step).and_then(|row| row.last_verdict()),
            judged: for_step(judged, step),
            flagged: for_step(flagged, step),
        })
    }

    /// Say it, into the session the slot is holding.
    async fn instruct(
        &self,
        job_id: &JobId,
        instruction: &Redirection,
        working: &Option<Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Err(Adrift::NoDroneToRedirect {
                job: job_id.clone(),
            });
        };
        at_work.instructed(Occasion::Redirect, instruction.text());
        at_work
            .session()
            .redirect(instruction)
            .await
            .map_err(|cause| Adrift::NotTold {
                job: job_id.clone(),
                cause,
            })
    }
}

/// One step's rows out of a whole Job's, which is the shape every step read
/// answers in.
fn for_step<T>(rows: Vec<(StepId, Vec<T>)>, step: &StepId) -> Vec<T> {
    rows.into_iter()
        .find(|(id, _)| id == step)
        .map(|(_, rows)| rows)
        .unwrap_or_default()
}
