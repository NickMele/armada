//! The three answers a person gives at a human gate.
//!
//! # Every one of them is a transition, and none writes a status
//!
//! Approving advances the step on the inner machine and then moves the Job on
//! the outer one; requesting changes moves it back to `running`; rejecting ends
//! it. All three go through `Job::transition`, so a review is a row in the same
//! log as everything else and the actor on it is **human**.
//!
//! # All three refuse anywhere but `awaiting_review`
//!
//! `awaiting_approval -> rejected` is a legal edge and it is the *dispatch*
//! gate's — `deny_dispatch`, a different act on a Job that never ran. A reject
//! that took it would be two operations sharing one route.
//!
//! # The step advances before the Job moves, and it has to
//!
//! `ADVANCING_STATUSES` is `running` and `awaiting_review`, so the inner
//! machine freezes the moment the Job leaves the gate. Both step moves an
//! approval makes are therefore made while it is still standing there, and
//! `current_step_id` is what re-admission reads to know where to put the Drone.
//!
//! # What puts a Job into `awaiting_review`
//!
//! A step gated `advance_gate: human_always` whose tiers all held: `crate::gate`
//! rules `HeldForReview` and `apply` takes the edge. The step stays `running`,
//! which is why the ordering above is what it is.
//!
//! # The branch is caught up on the far side of the person's decision
//!
//! An approval is a verdict on the step that was worked, not an authorisation
//! to merge, so the rebase does not touch what was read — it moves the tree the
//! *next* step starts from, which the reviewer was never shown. Rebasing on the
//! way *in* to `awaiting_review` buys nothing for it: the base moves while a
//! person reads, and a conflict would put markers into the diff being judged.
//!
//! The rebase is inside [`put_a_drone_on`](Fleet::put_a_drone_on), which every
//! advance reaches.
//!
//! # The gate holds no Drone and no slot
//!
//! A person's review costs no fleet time: the Drone ends when the machine
//! gates pass, and the slot goes to the next Job. So approving **re-queues**
//! rather than resuming, and `request_changes` refuses until `#207` gives its
//! note somewhere to wait. Both are on `job-statuses.toml`'s `awaiting_review`
//! row for whoever reads the registry instead of this file.
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{Actor, Job, JobId, JobStatus, StepId, StepTarget, Target};
use std::path::Path;
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::Redirection;
use crate::session::LiveSession;

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
    /// Take the work. **The counterpart to [`approve`](Fleet::approve)**, at the
    /// other end of the Job: that one releases a Job to spawn and this one
    /// accepts what it produced.
    ///
    /// The step advances, and then one of two things happens. A gate on the last
    /// step commits the work, delivers the branch and records
    /// `completed_success` — the same three things [`finish`](Fleet::finish)
    /// does, because a Job whose branch is uncommitted is correct, verified and
    /// unmergeable. A workflow with a step left goes back in the **queue**.
    ///
    /// **Approving is not a resume, and the slot is why.** The gate stood its
    /// Drone down when it opened, so a person's review costs no fleet time —
    /// and the slot it freed is very often another Job's by the time they
    /// answer. A Job that assumed a slot here would either fail or take one
    /// from underneath the Job in it. So both step moves are made where the
    /// inner machine is still live, the Job takes `awaiting_review -> queued`,
    /// and `crate::dispatch`'s re-admission puts a Drone back on it when a slot
    /// is free — immediately, on the ordinary turn where nothing else is
    /// running.
    ///
    /// **It needs no Drone and there is never one to need.** The decision is
    /// the person's and is recorded whatever the slot holds.
    pub async fn approve_review(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let mut working = self.slot().lock().await;
        let job = self.load(job_id).await?;
        let step = self.at_the_gate(&job)?;
        let passed = self.declared_step(&job, &step)?.clone();
        let next = job.workflow().after(&step).cloned();
        // Before the Job moves and never after: the inner machine is frozen
        // beneath every status but the two that advance, and `awaiting_review`
        // is one of them only until this call leaves it.
        let job = self.move_step(&job, &step, StepTarget::Advanced).await?;
        let told = OutcomeTurn::approved(&passed, next.as_ref());
        let Some(next) = next else {
            return self.completed(&job, &told, job_id, &mut working).await;
        };
        // The next step is entered here for the same reason, and it is what
        // re-admission reads to know where to put the Drone: `current_step_id`
        // moves when a step enters `running` and nothing else moves it.
        let job = self.move_step(&job, next.id(), StepTarget::Running).await?;
        // The actor is **human**. A person took the Job out of the gate; Fleet
        // only decides which turn it gets a process back.
        self.move_job(&job, Target::Queued, Actor::Human).await?;
        // Inline, exactly as `approve` dispatches inline off the other gate —
        // so a fleet with nothing else to do starts the next step now rather
        // than on the next tick, and a busy one leaves the Job in the queue
        // where a person can see it waiting.
        //
        // The `told` above is still built and still goes to the Drone on the
        // path where there is no next step: a Job that finished tells the
        // process that finished it. There is nobody to tell here. That a person
        // accepted the part crosses as a block in the next Drone's opening
        // brief, built by `crate::dispatch`'s re-admission.
        self.admit_next(&mut working).await?;
        self.load(job_id).await
    }

    /// Send the work back with a note. **The worktree and every step so far
    /// survive**: the Job returns to `running` at the same step, and the note
    /// is what the step is worked again against.
    ///
    /// The step does not move, which is the whole difference between this and an
    /// approval — the work is being done again rather than accepted.
    ///
    /// **It refuses, at every gate, until `#207`.** That is not a defect and it
    /// is not to be worked around. A Drone ends when its step's work passes the
    /// machine gates, so the gate holds none — there is nobody to tell, and a
    /// Job put back to `running` with no process on it escalates as
    /// `interrupted` a moment later having lost the note. `#207` gives a
    /// boundary something to carry the note in, at which point it opens the
    /// next Drone's brief; until it lands, approve or reject.
    ///
    /// Keeping one Drone alive for this path would keep the working slot for it
    /// too, which is the whole of what ending a Drone at a gate buys — see
    /// `job-statuses.toml`'s `awaiting_review` row, which carries the same
    /// interval for whoever reads the registry instead of the code.
    ///
    /// It is checked before anything moves, so a refused request leaves the Job
    /// at the gate rather than half-answered.
    pub async fn request_changes(&self, job_id: &JobId, note: &Redirection) -> Result<Job, Adrift> {
        let working = self.slot().lock().await;
        let job = self.load(job_id).await?;
        self.at_the_gate(&job)?;
        if !working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            return Err(Adrift::NoDroneToTell {
                job: job_id.clone(),
            });
        }
        let job = self.move_job(&job, Target::Running, Actor::Human).await?;
        self.said(job_id, note, &working).await?;
        Ok(job)
    }

    /// A verdict on the work: the Job is over and its Drone is ended.
    ///
    /// **Terminal, which is what makes it the hard stop.** `rejected` is a
    /// judgement on what was produced, unlike [`kill_job`](Fleet::kill_job),
    /// which clears a Job off the Board and carries no verdict at all. The
    /// worktree is left where it is, as every ending leaves it.
    pub async fn reject(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let mut working = self.slot().lock().await;
        let job = self.load(job_id).await?;
        self.at_the_gate(&job)?;
        if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            self.end_the_drone(&mut working).await;
        }
        let rejected = self.move_job(&job, Target::Rejected, Actor::Human).await?;
        // The slot is free and a queued Job is entitled to it, the same as
        // after a kill. A rejection that left the slot held would stop the
        // board on a Job that is over.
        self.admit_next(&mut working).await?;
        Ok(rejected)
    }

    /// The last step passed, so the work lands and the Job is over.
    ///
    /// The order is [`finish`](Fleet::finish)'s and for its reasons: the commit
    /// comes before the Job is recorded complete, so a `completed_success` on
    /// the Board is a Job whose work is on its branch, and the failure is held
    /// until the Drone has been told and the slot freed.
    pub(crate) async fn completed(
        &self,
        job: &Job,
        told: &OutcomeTurn,
        job_id: &JobId,
        working: &mut Option<crate::working::Working>,
    ) -> Result<Job, Adrift> {
        let landed = self.land_and_deliver(job, working).await;
        let job = self
            .move_job(job, Target::CompletedSuccess, Actor::Human)
            .await?;
        let said = self.tell(job_id, told, None, working).await;
        self.end_the_drone(working).await;
        self.admit_next(working).await?;
        landed?;
        said?;
        Ok(job)
    }

    /// The step a person is standing at, or why there is not one.
    ///
    /// **The status is the whole test.** A Job at `awaiting_review` is at a
    /// human gate by definition — the status carries no reason and there is
    /// nothing else to read — and the step is the one the Job's cursor names.
    fn at_the_gate(&self, job: &Job) -> Result<StepId, Adrift> {
        if job.status() != JobStatus::AwaitingReview {
            return Err(Adrift::NotUnderReview {
                job: job.id().clone(),
                status: job.status(),
            });
        }
        job.current_step_id()
            .cloned()
            .ok_or_else(|| Adrift::NoSuchStep {
                job: job.id().clone(),
                step: None,
            })
    }

    /// The frozen workflow's declaration of a step, which is what a turn is
    /// worded from. A step the workflow does not name is a fault in Fleet, not
    /// a review that can be answered.
    pub(crate) fn declared_step<'a>(
        &self,
        job: &'a Job,
        step: &StepId,
    ) -> Result<&'a core_model::ResolvedStep, Adrift> {
        job.workflow().step(step).ok_or_else(|| Adrift::NoSuchStep {
            job: job.id().clone(),
            step: Some(step.clone()),
        })
    }

    /// Say the person's words into the session the slot is holding.
    ///
    /// The same turn a redirect sends, because it is the same act on the
    /// Drone's side: a person read the work and wrote what to do about it, and
    /// the Agent Prompt Contract gives that turn no wording of Fleet's.
    async fn said(
        &self,
        job_id: &JobId,
        note: &Redirection,
        working: &Option<crate::working::Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Err(Adrift::NoDroneToTell {
                job: job_id.clone(),
            });
        };
        at_work
            .session()
            .redirect(note)
            .await
            .map_err(|cause| Adrift::NotTold {
                job: job_id.clone(),
                cause,
            })
    }

    /// What a Job's worktree holds, where it still has one.
    ///
    /// **`None` is "there is nothing to read"** — a Job at the approval gate, or
    /// one whose worktree has been reclaimed — and it is a different answer from
    /// a reading that found no change. A directory that will not open is an
    /// error rather than either.
    pub(crate) fn worktree_of(&self, job: &Job) -> Result<Option<Worktree>, Adrift> {
        let spec = WorktreeSpec::for_job(&self.host().repo_root, job.id().as_str())
            .map_err(Adrift::NoReadingWorktree)?;
        if !Path::new(&spec.worktree_path()).is_dir() {
            return Ok(None);
        }
        let branch = job
            .branch()
            .map(|branch| branch.as_str().to_string())
            .unwrap_or_else(|| spec.branch());
        Ok(Some(Worktree::at(spec.worktree_path(), branch)))
    }
}
