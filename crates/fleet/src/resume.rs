//! The two acts that put a person back on a Job without redispatching it.
//!
//! # Which one applies is decided by the Drone, not by the person
//!
//! [`redirect`](Fleet::redirect) needs a live session and [`restart_step`] is
//! what exists when there is none, so each refuses where the other applies —
//! `docs/concepts/job.md` is the specification. **Neither asks the other's
//! question**: a redirect wanted a stopped step only because both were once one
//! predicate, which held until `stalled` escalated a Job over a live Drone.
//!
//! # A step that stopped is moved; a Job that is only quiet is not
//!
//! Where a step stopped, both machines move in the one order the registry
//! admits — `escalated -> running`, then `stopped -> running` — because the
//! inner machine advances only beneath `running`. Where none stopped there is
//! nothing to unfreeze, the Job stays `escalated`, and it comes back on
//! [`watch_redirect`](Fleet::watch_redirect) seeing the Drone turn: a Job
//! moved on the sending would read as recovered whether or not anything woke.
//!
//! # Both acts leave the branch current, and neither one rebases
//!
//! A redirect is a turn into a live session mid-step and moves no boundary, so
//! there is nothing to catch up to that the step boundary either side of it
//! does not already do. A restart spawns, and every spawn catches the branch up
//! inside `crate::spawning` — one funnel, reached rather than called. See
//! `docs/concepts/fleet.md`, *Catching a branch up*.
//!
//! # Nothing here is bounded
//!
//! A person who can redirect can redirect for ever. Whether that is capped is
//! decided in no document, so no cap is invented here.
//!
//! [`restart_step`]: Fleet::restart_step

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{
    Actor, Component, Envelope, Job, JobId, JobStatus, Level, StepId, StepTarget, Target,
};

use crate::adrift::Adrift;
use crate::briefing::{Opening, Stopped};
use crate::daemon::Fleet;
use crate::session::LiveSession;
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
        let mut working = self.slot().lock().await;
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
        let working = self.slot().lock().await;
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

    /// Put a fresh Drone on the worktree the last one left. **The second act**,
    /// and what exists when the Drone is gone.
    ///
    /// **The worktree, the branch and every earlier step's work survive.** The
    /// steps that advanced did so on real verdicts and their evidence is
    /// recorded; re-running them is a redispatch, and an expensive one.
    ///
    /// **The branch is caught up, and that is not in tension with the line
    /// above.** A rebase updates the worktree that is there — the same path,
    /// the same branch, the same work, with the base moved underneath it.
    /// Nothing is created and nothing is discarded, so `#62`'s surviving
    /// worktree survives and the base it is measured against is current. The
    /// catch-up runs inside [`put_a_drone_on`](Fleet::put_a_drone_on), which is
    /// the one funnel every spawn goes through; this method does not call it,
    /// which is `#180`'s point.
    ///
    /// **A restart is the case a rebase most often conflicts on.** It re-runs
    /// the *same* step on a worktree already holding an attempt at it, so a
    /// moved base is being reconciled against edits already made. Where that
    /// conflicts the markers ride the opening brief and are the new Drone's
    /// first piece of work — there is no session to inject a turn into at the
    /// moment the rebase runs, so the spawn carries it.
    ///
    /// **Nothing is inherited from the Drone that went before.** The toolset,
    /// the model and the environment are resolved again from what the Manifest
    /// and the Job hold now — see `crate::spawning`.
    pub async fn restart_step(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let mut working = self.slot().lock().await;
        let job = self.load(job_id).await?;
        let step = self.stopped_step(&job)?;
        if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            return Err(Adrift::DroneStillThere {
                job: job_id.clone(),
            });
        }
        let worktree = self.surviving_worktree(&job)?;
        let stopped = self.what_stopped(&job, &step).await?;

        let job = self.resumed(&job, &step, Actor::Human).await?;
        self.put_a_drone_on(
            &job,
            &step,
            worktree,
            Opening::Resuming(stopped),
            &mut working,
        )
        .await?;
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
    async fn what_stopped(&self, job: &Job, step: &StepId) -> Result<Stopped, Adrift> {
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
