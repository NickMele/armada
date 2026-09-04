//! `api::Commands`, implemented over a real Fleet: every act a person takes.
//!
//! **The write half of the seam [`serving`](mod@crate::serving) holds the read
//! half of**, and a file of its own because `api::Daemon` is three traits and
//! Rust takes one impl block per trait. The redaction, the refusal path and the
//! one-way dependency are all argued there and are the same here; what is
//! different is that everything below moves something, and each says what it
//! moves it to.
//!
//! **Nothing here decides.** The move is the `Fleet` method each of these
//! calls, under the locks that make it one decision; this converts the request,
//! carries the outcome out as a DTO, and maps a refusal through
//! `Fleet::refusal`. A rule that lived here rather than beside the machine
//! would be a second opinion on a question the machine already answers.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Commands, Refusal};
use ipc::{
    ChangesRequested, JobForgotten, JobId, JobSummary, Overruled, ProposeJob, Redirection,
    Redispatched, WorktreeReclaimed,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
// The wire's `Redirection` is a struct with a public field; Fleet's is a
// newtype that cannot hold an empty instruction. Both names are in scope here,
// which is the one place they meet.
use crate::overruling::Overruling;
use crate::reporting::Filed;
use crate::resume::Redirection as Instruction;
use crate::wire::reported;

impl<H, V, W> Commands for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Draft a Job onto the approval gate. **Creation publishes `job.created`**
    /// — not a state change, because a created Job has no status it moved from.
    async fn propose_job(&self, proposal: ProposeJob) -> Result<JobSummary, Refusal> {
        let job = self
            .propose(proposal)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Read a request and draft the Job it proposes. **The same gate, and the
    /// same `job.created`** — the workflow is the only thing filled in
    /// differently.
    async fn propose_from_request(
        &self,
        request: ipc::JobRequest,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        let made = self
            .propose_from(&request.request, request.client_ref)
            .await
            .map_err(|why| self.refusal(why))?;
        let mut jobs = Vec::with_capacity(made.len());
        for job in &made {
            jobs.push(self.summarised(job).await?);
        }
        Ok(ipc::ProposedPlan { jobs })
    }

    /// Stop a proposal that is out. **Answers rather than refuses on a
    /// proposal that has gone** — see the trait's own note.
    ///
    /// It touches no store and moves no Job, which is why it is the one command
    /// here that does not end in `summarised`: there is nothing to summarise.
    async fn stop_proposal(
        &self,
        proposal_id: ipc::ProposalId,
    ) -> Result<ipc::ProposalStopped, Refusal> {
        Ok(ipc::ProposalStopped {
            stopped: Fleet::stop_proposal(self, &proposal_id),
        })
    }

    async fn approve_dispatch(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = self
            .approve(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The person takes the work, and the Job goes on or is finished.
    async fn approve_review(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::approve_review(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The work goes back with a note, to the Drone that is standing at the
    /// gate.
    ///
    /// **An empty note is refused here rather than sent**, for the reason
    /// `redirect_drone` refuses one: a Drone told nothing at all resumes with
    /// exactly the information that was not enough, which is the review
    /// appearing to work and changing nothing.
    async fn request_changes(
        &self,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        let said =
            Instruction::saying(&note.note).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = Fleet::request_changes(self, &job_id.to_domain(), &said)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The Judge refused, a person disagrees, and the step advances anyway.
    ///
    /// **A blank reason is refused here rather than recorded**, for the reason
    /// `request_changes` refuses a blank note, turned around: nothing is
    /// delivered to a Drone, so what an empty string would lose is the only
    /// account of why a verdict was overruled — and an override that says
    /// nothing is how this becomes the act somebody uses to quiet a gate.
    async fn override_verdict(
        &self,
        job_id: JobId,
        overruling: Overruled,
    ) -> Result<JobSummary, Refusal> {
        let said = Overruling::saying(&overruling.reason).ok_or_else(|| {
            self.refusal(Adrift::Unreasoned {
                job: job_id.to_domain(),
            })
        })?;
        let job = Fleet::override_verdict(self, &job_id.to_domain(), &said)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The gate could not decide, and a person asks it again on the evidence
    /// already submitted.
    ///
    /// **No reason is taken and none is refused**, which is
    /// `override_verdict`'s rule turned around: that act records why a person
    /// disagreed with a machine, and nothing here is disagreed with. What the
    /// second reading came to is written into the Job's own log by
    /// `crate::regating`, and it says more than a sentence would.
    async fn rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::rerun_gate(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A verdict on the work, and the Job is over.
    async fn reject_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::reject(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The process, not the unit of work.
    async fn kill_drone(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::kill_drone(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The unit of work, not the process.
    async fn kill_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::kill_job(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The record, gone. **Nothing is redacted here** — there is no Job left
    /// to redact, only the id it used to name.
    async fn forget_job(&self, job_id: JobId) -> Result<JobForgotten, Refusal> {
        Fleet::forget_job(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(JobForgotten { job_id })
    }

    /// The disk, given back, with the record left standing. **Nothing is
    /// redacted here either** — every field of the answer is about a directory
    /// and a branch this Job derived, and there is no path in it a person
    /// clearing their own disk should not be shown.
    async fn reclaim_worktree(&self, job_id: JobId) -> Result<WorktreeReclaimed, Refusal> {
        let id = job_id.to_domain();
        let gave_back = Fleet::reclaim_worktree(self, &id)
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(crate::wire::reclaimed(&id, gave_back))
    }

    /// **Two Jobs, redacted separately.** The failed one is now `killed`; the
    /// replacement carries `redispatched_from` and is what the caller opens
    /// next.
    async fn redispatch_job(&self, job_id: JobId) -> Result<Redispatched, Refusal> {
        let both = self
            .redispatch(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(Redispatched {
            replaced: self.summarised(&both.replaced).await?,
            dispatched: self.summarised(&both.dispatched).await?,
        })
    }

    /// A structured instruction to a Drone that is escalated and idle.
    ///
    /// **An empty instruction is refused here rather than sent.** A Drone told
    /// nothing at all resumes the step it stopped on with exactly the
    /// information that failed, which is the redirect appearing to work and
    /// changing nothing. `Redirection::saying` is where the emptiness is
    /// caught; this only carries the refusal out.
    async fn redirect_drone(
        &self,
        job_id: JobId,
        instruction: Redirection,
    ) -> Result<JobSummary, Refusal> {
        let said = Instruction::saying(&instruction.instruction)
            .ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = self
            .redirect(&job_id.to_domain(), &said)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A fresh Drone on the worktree the last one left, and what to do
    /// differently where a person had something to say.
    ///
    /// The Job resumes at the step that stopped; every earlier step's work is
    /// on the branch and is not redone. That is what separates this from a
    /// redispatch, which starts a replacement Job at the approval gate.
    ///
    /// **No note and a blank note are different requests.** Absent is the plain
    /// restart, which is what this act has always been and stays. Present and
    /// empty is refused here rather than written down, for the reason
    /// `redirect_drone` refuses one: a Drone opened with a heading and nothing
    /// under it has been given exactly the information that was not enough.
    async fn restart_step(
        &self,
        job_id: JobId,
        note: Option<ipc::RestartRequested>,
    ) -> Result<JobSummary, Refusal> {
        let said = match &note {
            Some(note) => Some(
                Instruction::saying(&note.note).ok_or_else(|| self.refusal(Adrift::Unnameable))?,
            ),
            None => None,
        };
        let job = self
            .restart_step(&job_id.to_domain(), said.as_ref())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person says this Job failed in error, and the record is filed with
    /// what they said.
    ///
    /// **Everything here is in `crate::reporting`**, including the emptiness and
    /// the two refusals it has: `crate::reporting::NotFiled` names each cause
    /// and says it, and this only carries one out as the 422 it is.
    async fn file_report(
        &self,
        job_id: JobId,
        filing: ipc::FileReport,
    ) -> Result<ipc::Report, Refusal> {
        let id = job_id.to_domain();
        let filed = Filed::saying(
            filing.claim,
            &filing.said,
            filing.step_id,
            filing.criterion_id,
        )
        .map_err(|cause| self.refusal(cause.about(&id)))?;
        let filed = Fleet::file_report(self, &id, &filed)
            .await
            .map_err(|why| self.refusal(why))?;
        reported(&filed).map_err(|why| self.refusal(why))
    }

    /// A person's answer to the question a waiting Drone asked. **Nothing
    /// moves** — the Job was `running` while it waited and is now, and the
    /// summary comes back because a caller folds the row rather than re-reading
    /// the board. The four refusals are `crate::questioning::NotAnswered`'s.
    async fn answer_question(
        &self,
        job_id: JobId,
        answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        let id = job_id.to_domain();
        Fleet::answer_question(self, &id, answer.question_id.as_str(), &answer.chose)
            .await
            .map_err(|why| self.refusal(why.about(&id)))?;
        let job = self.load(&id).await.map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }
}
