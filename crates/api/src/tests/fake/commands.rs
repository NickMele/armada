//! What the fake does when it is told to move something.
//!
//! **The `Commands` block, in its own file because the trait is three traits.**
//! It asserts nothing about the status machine: that machine is `core-model`'s
//! and is tested there, against the edge table. What is asserted here is that
//! the route reached the daemon, that the body arrived, and that a refusal
//! comes back as the refusal the transport is supposed to map.
//!
//! [`FakeDaemon::move_to`](super::FakeDaemon) is the one move, and every
//! command below is a pair of statuses handed to it.

use ipc::{
    Actor, ChangesRequested, Event, Instant, JobCreated, JobForgotten, JobId, JobSummary,
    ManifestId, Origin, ProposeJob, Redispatched, Urgency, WorkflowId, WorktreeReclaimed,
};
use std::sync::atomic::Ordering;

use super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::{run_id, status};
use crate::{Commands, Refusal};

impl Commands for FakeDaemon {
    /// Stop a proposal. **The fake never has one out** — it answers a proposal
    /// without making a call — so this always reports that there was nothing to
    /// stop, which is the arm a route test needs: it is the one that must be a
    /// 200 and not a 404.
    async fn stop_proposal(
        &self,
        _proposal_id: ipc::ProposalId,
    ) -> Result<ipc::ProposalStopped, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(ipc::ProposalStopped { stopped: false })
    }

    /// The proposer's own path, faked at the seam above it: the fake names the
    /// workflow the request asked for by naming none of its own reasoning. What
    /// `api` is under test for is the route, the status and the body.
    async fn propose_from_request(
        &self,
        request: ipc::JobRequest,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        if request.request.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.unacceptable_proposal",
                "a request needs something in it to read",
                run_id(),
            )));
        }
        self.propose_job(ProposeJob {
            title: request.request.clone(),
            workflow_id: WorkflowId::carried("bug"),
            owner_manifest_id: ManifestId::carried("01MANIFEST"),
            origin: ipc::TopLevelOrigin::from_wire("auto_detected").expect("an origin"),
            urgency: Urgency::from_wire("normal").expect("an urgency"),
            atomic: false,
            model: None,
            acceptance_criteria: Vec::new(),
            subject: None,
            facts: request.request,
            write_targets: None,
            dependencies: Vec::new(),
            attachments: Vec::new(),
        })
        .await
        .map(|job| ipc::ProposedPlan { jobs: vec![job] })
    }

    async fn propose_job(&self, proposal: ProposeJob) -> Result<JobSummary, Refusal> {
        let minted = self.minted.fetch_add(1, Ordering::SeqCst);
        let job = JobSummary {
            id: JobId::carried(format!("01JOB{minted}")),
            title: proposal.title,
            // The entry status of a top-level Job. Creation is not a
            // transition, and `job.created` is what carries it — a
            // `job.state_changed` here would name a move from a status the Job
            // was never in.
            status: status("awaiting_approval"),
            created_at: Instant::carried("2026-01-01T00:00:00.000Z"),
            // No worktree exists at the approval gate, so no branch is claimed.
            branch: None,
            reason: None,
            queued_reason: None,
            resumption: None,
            workflow_id: proposal.workflow_id,
            owner_manifest_id: proposal.owner_manifest_id,
            origin: Origin::from_wire(proposal.origin.as_wire()).expect("a top-level origin"),
            urgency: proposal.urgency,
            atomic: proposal.atomic,
            // Absent is the ordinary case: Fleet fills it from configuration.
            // The fake has none, so it names what the fixture names.
            model: proposal.model.unwrap_or_else(|| "a-model".to_string()),
            current_step_id: None,
            assigned_drone: None,
            redispatched_from: None,
            // No slot, so nothing is waiting. See `Tools::ask_question`.
            asking: false,
            landed: None,
        };
        self.jobs.lock().expect("not poisoned").push(job.clone());
        self.events.publish(Event::JobCreated(JobCreated {
            job: job.clone(),
            actor: Actor::from_wire("human").expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }));
        Ok(job)
    }

    async fn approve_dispatch(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_approval", "queued", "human")
    }

    /// The person takes the work. The gate this answers is the one after the
    /// Job ran, which is why the status it moves from is not the one
    /// `approve_dispatch` moves from.
    async fn approve_review(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_review", "running", "human")
    }

    /// The work goes back with a note. The same destination as an approval on a
    /// Job with steps left, and **a different act** — what separates them here
    /// is the note, and in Fleet it is the step that does or does not advance.
    async fn request_changes(
        &self,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        if note.note.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.blank_note",
                "a review note with nothing in it says nothing to change",
                run_id(),
            )));
        }
        self.move_to(&job_id, "awaiting_review", "running", "human")
    }

    /// A verdict on the work. Terminal, and from the review gate alone: the
    /// other edge into `rejected` is the dispatch gate's and belongs to
    /// `deny_dispatch`.
    async fn reject_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_review", "rejected", "human")
    }

    /// The process, not the unit of work. The Job is handed back where it
    /// stood, with no Drone on it: `assigned_drone` is presence rather than
    /// state, and **the registry names no edge a killed Drone fires**, so a
    /// transition invented here would be the fake asserting something about a
    /// machine it does not own.
    async fn kill_drone(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
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
    async fn kill_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
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
    async fn forget_job(&self, job_id: JobId) -> Result<JobForgotten, Refusal> {
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
    async fn reclaim_worktree(&self, job_id: JobId) -> Result<WorktreeReclaimed, Refusal> {
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
    async fn redirect_drone(
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
    async fn answer_question(
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
    async fn restart_step(
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

    /// The one act on a refused step, faked on the one thing the route refuses
    /// on that the transport can see: **a blank reason is not an override.**
    /// Which trigger stopped the step is Fleet's to read off the record and
    /// nothing a `JobSummary` carries, so the fake does not pretend to know it.
    async fn override_verdict(
        &self,
        job_id: JobId,
        overruling: ipc::Overruled,
    ) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if overruling.reason.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.unreasoned_override",
                "overruling a verdict needs a reason".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }

    /// The act on a step nothing ruled on, faked on the one thing the route
    /// refuses on that the transport can see: **a Job that is not escalated
    /// reaches no act on a stopped step.** Which trigger stopped the step, and
    /// whether the daemon is still standing at it, are Fleet's to read off the
    /// record and the slot; a `JobSummary` carries neither, so the fake does
    /// not pretend to know them.
    async fn rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.status.as_wire() != "escalated" {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.not_resumable",
                format!("a Job at {} has no stopped step", job.status.as_wire()),
                run_id(),
            )));
        }
        Ok(job.clone())
    }

    /// Filing, faked on the two things the transport can see: the Job has to
    /// exist, and **a report with no sentence is not a report.** What Fleet
    /// attaches around the sentence is Fleet's — three reads and a render —
    /// and a fake that produced a record would be asserting about a bundle it
    /// invented.
    async fn file_report(
        &self,
        job_id: JobId,
        filing: ipc::FileReport,
    ) -> Result<ipc::Report, Refusal> {
        let job = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            job.clone()
        };
        if filing.said.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.unsaid_report",
                "a report needs the sentence the record is context for".to_string(),
                run_id(),
            )));
        }
        let report = ipc::Report {
            id: ipc::ReportId::carried("01REPORT"),
            filed_at: Instant::carried("2026-08-28T21:00:00.000Z"),
            origin: ipc::ReportOrigin::Human,
            claim: filing.claim,
            job_id,
            job_title: job.title.clone(),
            step_id: filing.step_id,
            criterion_id: filing.criterion_id,
            said: filing.said,
            record: "## Every move it made".to_string(),
        };
        self.reports
            .lock()
            .expect("not poisoned")
            .push(report.clone());
        Ok(report)
    }

    async fn redispatch_job(&self, job_id: JobId) -> Result<Redispatched, Refusal> {
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
