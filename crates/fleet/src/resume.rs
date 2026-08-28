//! The two acts that resume a step a Job stopped on, without redispatching it.
//!
//! # Which one applies is decided by the Drone, not by the person
//!
//! [`redirect`](Fleet::redirect) needs a live session and [`restart_step`] is
//! what exists when there is none, so each refuses where the other applies.
//! `docs/concepts/job.md` is the specification: a redirect that respawns is a
//! restart that threw away the session, and a restart that reuses one is a
//! redirect that respawned for no reason.
//!
//! # Both walk the two machines in the one order the registry admits
//!
//! `escalated -> running` first, then `stopped -> running` — the exact reverse
//! of the way out. The inner machine advances only beneath `running`, so a step
//! cannot be resumed until the Job has moved, and it is why an escalated Job's
//! step move has to bracket the status move on both sides.
//!
//! # Nothing here is bounded
//!
//! A person who can redirect can redirect for ever. Whether that is capped,
//! and by what, is decided in no document — so no cap is invented here.
//!
//! [`restart_step`]: Fleet::restart_step

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{Actor, Job, JobId, JobStatus, StepId, StepState, StepTarget, Target};

use crate::adrift::Adrift;
use crate::briefing::{self, Stopped};
use crate::daemon::Fleet;
use crate::session::LiveSession;
use crate::working::Working;

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
    pub async fn redirect(&self, job_id: &JobId, instruction: &Redirection) -> Result<Job, Adrift> {
        let mut working = self.slot().lock().await;
        let job = self.load(job_id).await?;
        let step = self.resumable(&job)?;
        // The live session, which is the whole difference between the two acts
        // — and it is asked of the slot rather than of the record, because the
        // slot is the only thing holding a pipe. A record saying a Drone is on
        // a Job this Fleet did not spawn is repaired at the boot read.
        if !working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            return Err(Adrift::NoDroneToRedirect {
                job: job_id.clone(),
            });
        }

        let job = self.resumed(&job, &step, Actor::Human).await?;
        // After both moves, never before. A turn delivered to a Drone whose
        // Job then failed to move would be an instruction acted on by a
        // process nobody had unpaused.
        self.instruct(job_id, instruction, &working).await?;
        // The step is running again, so the thrashing chain is too. Without
        // this a Drone steered off one loop is never caught in the next.
        if let Some(at_work) = working.as_mut() {
            at_work.resumed(self.now());
        }
        Ok(job)
    }

    /// Put a fresh Drone on the worktree the last one left. **The second act**,
    /// and what exists when the Drone is gone.
    ///
    /// **The worktree, the branch and every earlier step's work survive.** The
    /// steps that advanced did so on real verdicts and their evidence is
    /// recorded; re-running them is a redispatch, and an expensive one.
    ///
    /// **Nothing is inherited from the Drone that went before.** The toolset,
    /// the model and the environment are resolved again from what the Manifest
    /// and the Job hold now — see `crate::spawning`.
    pub async fn restart_step(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let mut working = self.slot().lock().await;
        let job = self.load(job_id).await?;
        let step = self.resumable(&job)?;
        if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            return Err(Adrift::DroneStillThere {
                job: job_id.clone(),
            });
        }
        let worktree = self.surviving_worktree(&job)?;
        let stopped = self.what_stopped(&job, &step).await?;

        let job = self.resumed(&job, &step, Actor::Human).await?;
        let brief =
            briefing::resuming_turn(&job, job.workflow(), &step, &stopped).map_err(|cause| {
                Adrift::NotConfigurable {
                    job: job_id.clone(),
                    cause,
                }
            })?;
        self.put_a_drone_on(&job, &step, worktree, brief, &mut working)
            .await?;
        self.load(job_id).await
    }

    /// The step both acts resume, or why there is not one.
    ///
    /// **A step-level escalation is what makes either act coherent.** Only a
    /// step-level trigger reaches a step's `last_verdict`, so only a step-level
    /// escalation leaves a `stopped` row naming which step to resume — and a
    /// Job escalated on `interrupted` or `resource_exhausted` has none, which
    /// leaves redispatch and Pilot as the only moves.
    fn resumable(&self, job: &Job) -> Result<StepId, Adrift> {
        if job.status() != JobStatus::Escalated {
            return Err(Adrift::NotResumable {
                job: job.id().clone(),
                status: job.status(),
            });
        }
        job.steps()
            .iter()
            .find(|step| step.state() == StepState::Stopped)
            .map(|step| step.step_id().clone())
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
    fn surviving_worktree(&self, job: &Job) -> Result<Worktree, Adrift> {
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
