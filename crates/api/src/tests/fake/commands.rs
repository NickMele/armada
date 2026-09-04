//! The nineteen commands, in one list, each delegating to its subject.
//!
//! **A trait implementation cannot be split across files**, so this is the whole
//! `Commands` block and the bodies are inherent methods next door. What that
//! buys is the roster: every command Fleet answers, in the order the inventory
//! names them, on one screen — and three files sized by subject rather than one
//! file sized by the trait.
//!
//! It asserts nothing about the status machine. That machine is `core-model`'s
//! and is tested there against the edge table. What is asserted here is that the
//! route reached the daemon, that the body arrived, and that a refusal comes
//! back as the refusal the transport is supposed to map.
//!
//! [`FakeDaemon::move_to`](super::FakeDaemon) is the one move, and most commands
//! below are a pair of statuses handed to it.

mod gating;
mod proposing;
mod recovering;

use ipc::{
    ChangesRequested, JobExamined, JobForgotten, JobId, JobSummary, ProposeJob, Redispatched,
    WorktreeReclaimed,
};

use super::FakeDaemon;
use crate::{Commands, Refusal};

impl Commands for FakeDaemon {
    async fn propose_from_request(
        &self,
        request: ipc::JobRequest,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        self.fake_propose_from_request(request).await
    }
    async fn propose_job(&self, proposal: ProposeJob) -> Result<JobSummary, Refusal> {
        self.fake_propose_job(proposal).await
    }
    async fn stop_proposal(
        &self,
        _proposal_id: ipc::ProposalId,
    ) -> Result<ipc::ProposalStopped, Refusal> {
        self.fake_stop_proposal(_proposal_id).await
    }
    async fn examine_job(&self, job_id: JobId) -> Result<JobExamined, Refusal> {
        self.fake_examine_job(job_id).await
    }
    async fn approve_dispatch(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_approve_dispatch(job_id).await
    }
    async fn approve_review(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_approve_review(job_id).await
    }
    async fn request_changes(
        &self,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        self.fake_request_changes(job_id, note).await
    }
    async fn reject_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_reject_job(job_id).await
    }
    async fn override_verdict(
        &self,
        job_id: JobId,
        overruling: ipc::Overruled,
    ) -> Result<JobSummary, Refusal> {
        self.fake_override_verdict(job_id, overruling).await
    }
    async fn rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_rerun_gate(job_id).await
    }
    async fn file_report(
        &self,
        job_id: JobId,
        filing: ipc::FileReport,
    ) -> Result<ipc::Report, Refusal> {
        self.fake_file_report(job_id, filing).await
    }
    async fn kill_drone(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_kill_drone(job_id).await
    }
    async fn kill_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.fake_kill_job(job_id).await
    }
    async fn forget_job(&self, job_id: JobId) -> Result<JobForgotten, Refusal> {
        self.fake_forget_job(job_id).await
    }
    async fn reclaim_worktree(&self, job_id: JobId) -> Result<WorktreeReclaimed, Refusal> {
        self.fake_reclaim_worktree(job_id).await
    }
    async fn redirect_drone(
        &self,
        job_id: JobId,
        _instruction: ipc::Redirection,
    ) -> Result<JobSummary, Refusal> {
        self.fake_redirect_drone(job_id, _instruction).await
    }
    async fn answer_question(
        &self,
        job_id: JobId,
        _answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        self.fake_answer_question(job_id, _answer).await
    }
    async fn restart_step(
        &self,
        job_id: JobId,
        note: Option<ipc::RestartRequested>,
    ) -> Result<JobSummary, Refusal> {
        self.fake_restart_step(job_id, note).await
    }
    async fn redispatch_job(&self, job_id: JobId) -> Result<Redispatched, Refusal> {
        self.fake_redispatch_job(job_id).await
    }
}
