//! The acts a person takes at a gate.
//!
//! **Every refusal here is one the transport can see.** Which trigger stopped a
//! step, and whether the daemon is still standing at it, are Fleet's to read off
//! the record and the slot; a `JobSummary` carries neither. So what the fake
//! checks is the shape of the body — a blank note, a blank reason, an unsaid
//! report — and it does not pretend to know the rest.

use ipc::{ChangesRequested, Instant, JobId, JobSummary};

use super::super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::Refusal;

impl FakeDaemon {
    pub(super) async fn fake_approve_dispatch(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_approval", "queued", "human")
    }
    /// The person takes the work. The gate this answers is the one after the
    /// Job ran, which is why the status it moves from is not the one
    /// `approve_dispatch` moves from.
    pub(super) async fn fake_approve_review(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_review", "running", "human")
    }
    /// The work goes back with a note. The same destination as an approval on a
    /// Job with steps left, and **a different act** — what separates them here
    /// is the note, and in Fleet it is the step that does or does not advance.
    pub(super) async fn fake_request_changes(
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
    pub(super) async fn fake_reject_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_review", "rejected", "human")
    }
    /// The one act on a refused step, faked on the one thing the route refuses
    /// on that the transport can see: **a blank reason is not an override.**
    /// Which trigger stopped the step is Fleet's to read off the record and
    /// nothing a `JobSummary` carries, so the fake does not pretend to know it.
    pub(super) async fn fake_override_verdict(
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
    pub(super) async fn fake_rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
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
    pub(super) async fn fake_file_report(
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
}
