//! Ending a Drone, ending a Job, and putting one back on its feet.
//!
//! **The Drone and the Job are separate acts on separate things**, which is why
//! `kill_drone` and `kill_job` are two operations and refuse on different
//! statuses. Terminality is read through the DTO from the domain rather than
//! restated here: a second list of which statuses are over is a second
//! vocabulary.

use ipc::{
    Actor, Event, Instant, JobCreated, JobForgotten, JobId, JobSummary, Redispatched,
    WorktreeReclaimed,
};
use std::sync::atomic::Ordering;

use super::super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::{run_id, status};
use crate::Refusal;

impl FakeDaemon {
    /// The process, not the unit of work. The Job is handed back where it
    /// stood, with no Drone on it: `assigned_drone` is presence rather than
    /// state, and **the registry names no edge a killed Drone fires**, so a
    /// transition invented here would be the fake asserting something about a
    /// machine it does not own.
    pub(super) async fn fake_kill_drone(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter_mut().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        // The one status this fake knows carries a live Drone. Nothing else
        // does: a Job at the approval gate or in the queue has no process.
        if job.status.as_wire() != "running" {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.no_drone",
                format!("a Job at {} has no Drone to kill", job.status.as_wire()),
                run_id(),
            )));
        }
        job.assigned_drone = None;
        Ok(job.clone())
    }
    /// The unit of work, not the process. Legal from wherever the Job is, so
    /// long as it is not already over — including the statuses `kill_drone`
    /// refuses, which is the whole reason the two are separate operations.
    pub(super) async fn fake_kill_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let from = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            // Terminality is the domain's, read through the DTO rather than
            // restated: a second list of which statuses are over is a second
            // vocabulary.
            if job.status.domain().is_terminal() {
                return Err(Refusal::IllegalMove(ipc::WireError::raised(
                    "fake.illegal_move",
                    format!("a Job at {} is already over", job.status.as_wire()),
                    run_id(),
                )));
            }
            job.status.as_wire()
        };
        self.move_to(&job_id, from, "killed", "human")
    }
    /// The record, gone. **The fake actually removes it**, the same as
    /// `Store::forget_job` does on the real one, so a caller that forgets a
    /// Job and then asks for it again sees exactly what asking for an id that
    /// never existed sees.
    pub(super) async fn fake_forget_job(&self, job_id: JobId) -> Result<JobForgotten, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        // Terminality is the domain's, read through the DTO rather than
        // restated, for `kill_job`'s reason.
        if !job.status.domain().is_terminal() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.not_forgettable",
                format!("a Job at {} cannot be forgotten", job.status.as_wire()),
                run_id(),
            )));
        }
        jobs.retain(|job| job.id != job_id);
        Ok(JobForgotten { job_id })
    }
    /// The disk, given back. **The record stays** — that is the whole
    /// difference from the method above, and the fake keeps the row so a test
    /// that reclaims and then reads the Job still finds it.
    ///
    /// Terminality is the domain's, read through the DTO rather than restated,
    /// for `kill_job`'s reason.
    pub(super) async fn fake_reclaim_worktree(
        &self,
        job_id: JobId,
    ) -> Result<WorktreeReclaimed, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if !job.status.domain().is_terminal() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.not_reclaimable",
                format!("a Job at {} has no disk to give back", job.status.as_wire()),
                run_id(),
            )));
        }
        Ok(shapes::reclaimed(job_id))
    }
    /// Mint a replacement for a stopped Job. **The fake asserts one thing about
    /// the domain and no more**: a Job that ran and stopped is replaceable and
    /// anything else is refused, because that is the whole of what the route
    /// refuses on.
    /// Both resume acts, faked the way the real ones are told apart: **which
    /// one applies is decided by the Drone, not by the caller.** A redirect
    /// needs one alive; a restart is what exists when it is gone. A fake that
    /// let either work on any Job would let a test pass against a rule the
    /// real Fleet enforces.
    pub(super) async fn fake_redirect_drone(
        &self,
        job_id: JobId,
        _instruction: ipc::Redirection,
    ) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.assigned_drone.is_none() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.no_drone_to_redirect",
                "this Job has no Drone to redirect".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }
    /// The answer to a question, faked on the one thing a `JobSummary` shows:
    /// **there is no Drone waiting.** Which question is outstanding and which
    /// labels it offered come off a working slot this daemon has none of.
    pub(super) async fn fake_answer_question(
        &self,
        job_id: JobId,
        _answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.assigned_drone.is_none() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.nothing_is_asking",
                "this Job has no Drone waiting on an answer".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }
    /// **The note is refused blank and otherwise ignored here.** Where it goes
    /// is onto the record, and this fake holds summaries — so what it can check
    /// is the one thing the route promises about the body, which is that an
    /// empty note is a refusal rather than a restart with nothing said.
    pub(super) async fn fake_restart_step(
        &self,
        job_id: JobId,
        note: Option<ipc::RestartRequested>,
    ) -> Result<JobSummary, Refusal> {
        if note
            .as_ref()
            .is_some_and(|note| note.note.trim().is_empty())
        {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.blank_note",
                "a restart note with nothing in it says nothing to the Drone it asks for"
                    .to_string(),
                run_id(),
            )));
        }
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.assigned_drone.is_some() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.drone_still_there",
                "this Job still has a Drone — redirect it rather than restarting".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }
    pub(super) async fn fake_redispatch_job(&self, job_id: JobId) -> Result<Redispatched, Refusal> {
        let failed = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            if !matches!(
                job.status.as_wire(),
                "escalated" | "completed_failed" | "killed"
            ) {
                return Err(Refusal::IllegalMove(ipc::WireError::raised(
                    "fake.not_redispatchable",
                    format!("a Job at {} has not run and stopped", job.status.as_wire()),
                    run_id(),
                )));
            }
            job.clone()
        };
        let minted = self.minted.fetch_add(1, Ordering::SeqCst);
        let dispatched = JobSummary {
            id: JobId::carried(format!("01JOB{minted}")),
            status: status("awaiting_approval"),
            created_at: Instant::carried("2026-01-01T00:00:00.000Z"),
            // No worktree exists at the approval gate, so no branch is claimed.
            branch: None,
            reason: None,
            queued_reason: None,
            resumption: None,
            redispatched_from: Some(failed.id.clone()),
            ..failed.clone()
        };
        self.jobs
            .lock()
            .expect("not poisoned")
            .push(dispatched.clone());
        self.events.publish(Event::JobCreated(JobCreated {
            job: dispatched.clone(),
            actor: Actor::from_wire("human").expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }));
        // Only an escalated original moves. A terminal one has no outbound
        // edge, and the fake does not invent one.
        let replaced = if failed.status.as_wire() == "escalated" {
            self.move_to(&job_id, "escalated", "killed", "human")?
        } else {
            failed
        };
        Ok(Redispatched {
            replaced,
            dispatched,
        })
    }
}
