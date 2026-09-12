//! `api::Commands`, implemented over a real Fleet: every act a person takes.
//!
//! **The write half of the seam `serving` holds the read half of**, split out
//! because `api::Daemon` is three traits and Rust takes one impl block per
//! trait. Nothing here decides: each method converts the request, calls the
//! `Fleet` method that moves it, and maps a refusal through `Fleet::refusal`.
//!
//! `#712`: most commands race their work against [`CommandBudget`] through
//! [`budgeted`], spawned so a losing race stops waiting without cancelling a
//! write already in progress. Each excluded command says why on its own `impl`.
//!
//! **Past 500 lines and staying one file.** `#712` added a wrapper to two
//! thirds of a trait already sized to one method per act; splitting by act
//! would scatter `budgeted` and `CommandBudget` across the pieces that use
//! them, which is the coupling the line count is asking about, not the count.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Commands, Refusal};
use ipc::{
    CapRaise, ChangesRequested, JobExamined, JobForgotten, JobId, JobSummary, Overruled,
    ProposeJob, Redirection, Redispatched, RemarksTakenUp, TurnRaise, WorktreeReclaimed,
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

/// How long Fleet gives a plain command before answering
/// [`Adrift::CommandTimedOut`]. Paired with Bridge's `COMMAND_MS` — see
/// `PROVISIONAL_COMMAND_BUDGET` in `crates/armada/src/serve.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandBudget(Duration);

impl CommandBudget {
    pub fn of(budget: Duration) -> CommandBudget {
        CommandBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// Race a plain command's work against [`CommandBudget`], spawned rather than
/// merely timed — see this module's own header for why.
async fn budgeted<T>(
    budget: CommandBudget,
    work: impl Future<Output = Result<T, Adrift>> + Send + 'static,
) -> Result<T, Adrift>
where
    T: Send + 'static,
{
    let waited = budget.duration();
    match tokio::time::timeout(waited, tokio::spawn(work)).await {
        Ok(Ok(answered)) => answered,
        // Resumed rather than folded into a refusal a caller might retry: a
        // panic has already unwound past every lock it held.
        Ok(Err(panicked)) => std::panic::resume_unwind(panicked.into_panic()),
        Err(_elapsed) => Err(Adrift::CommandTimedOut { job: None, waited }),
    }
}

/// [`budgeted`], naming the Job a losing race's refusal is about.
async fn budgeted_for<T>(
    budget: CommandBudget,
    job: JobId,
    work: impl Future<Output = Result<T, Adrift>> + Send + 'static,
) -> Result<T, Adrift>
where
    T: Send + 'static,
{
    match budgeted(budget, work).await {
        Err(Adrift::CommandTimedOut { waited, .. }) => Err(Adrift::CommandTimedOut {
            job: Some(job.to_domain()),
            waited,
        }),
        answered => answered,
    }
}

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
    async fn propose_job(self: Arc<Self>, proposal: ProposeJob) -> Result<JobSummary, Refusal> {
        let job = budgeted(self.command_budget(), {
            let fleet = Arc::clone(&self);
            async move { fleet.propose(proposal).await }
        })
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
            .propose_from_with_attachments(
                &request.request,
                request.client_ref,
                request.attachments,
            )
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

    async fn approve_dispatch(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { fleet.approve(&job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The person takes the work, and the Job goes on or is finished.
    /// **Included even though the last step's answer commits and delivers the
    /// branch inside it.** [`CommandBudget`] is sized to cover an ordinary
    /// local commit and push for that reason.
    async fn approve_review(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::approve_review(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The person merges the work, and then takes it.
    ///
    /// **The merge is Fleet's to perform and never Fleet's to decide.** What
    /// arrives here is a press, and what it buys over merging on the forge is
    /// the Checks that run against the tree the merge left.
    ///
    /// **Not [`budgeted`].** The forge call inside writes into a repository
    /// Fleet did not make, over the network this crate has not measured.
    async fn merge_pull_request(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::merge_pull_request(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person sends the branch back for a Drone that can edit files to
    /// bring it current with main. `#663`.
    ///
    /// **Not [`budgeted`]**, for [`Commands::merge_pull_request`]'s reason: the
    /// rebase and push inside are against a remote nothing here has measured.
    async fn resolve_pull_request_conflict(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::resolve_pull_request_conflict(self, &job_id.to_domain())
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
        self: Arc<Self>,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        let said =
            Instruction::saying(&note.note).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::request_changes(&fleet, &job_id.to_domain(), &said).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The comments a person picked off the pull request reach a Drone.
    /// Nothing is written back onto the pull request.
    ///
    /// **Nothing is decoded into a note here**, unlike `request_changes`: the
    /// body carries handles and the words come off the forge inside the act. An
    /// empty list is refused there rather than here, because it is a fact about
    /// the Job's pull request and not about the request's shape.
    ///
    /// **Not [`budgeted`]**, for [`Commands::merge_pull_request`]'s reason: the
    /// forge is read for the comments' own words before a Drone ever sees them.
    async fn take_up_remarks(
        &self,
        job_id: JobId,
        picked: RemarksTakenUp,
    ) -> Result<JobSummary, Refusal> {
        let job = Fleet::take_up_remarks(self, &job_id.to_domain(), &picked.remarks)
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
    ///
    /// **Included**, for [`Commands::approve_review`]'s reason: the step it
    /// advances may also be the workflow's last.
    async fn override_verdict(
        self: Arc<Self>,
        job_id: JobId,
        overruling: Overruled,
    ) -> Result<JobSummary, Refusal> {
        let said = Overruling::saying(&overruling.reason).ok_or_else(|| {
            self.refusal(Adrift::Unreasoned {
                job: job_id.to_domain(),
            })
        })?;
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::override_verdict(&fleet, &job_id.to_domain(), &said).await }
        })
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
    ///
    /// **Not [`budgeted`].** This asks the Judge again, on its own budget
    /// paired with Bridge's own wait for this route.
    async fn rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::rerun_gate(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person asking a Job to show its work. **The `Arc` is handed on**, so
    /// the press runs on a task of its own — `crate::showing_again`.
    ///
    /// **Not [`budgeted`].** The request waits for the whole run, however long
    /// the repository's harness takes; Bridge sends `NO_WAIT` on this route.
    async fn show_again(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> Result<ipc::ShownAgain, Refusal> {
        let refusing = std::sync::Arc::clone(&self);
        Fleet::show_again(self, &job_id.to_domain())
            .await
            .map_err(|why| refusing.refusal(why))
    }

    /// A person's run in a Job's worktree. **The `Arc` is handed on**, so the
    /// run is a task of its own — `crate::rehearsing`.
    ///
    /// **Not [`budgeted`]**: a rehearsal answers through its own refusal type,
    /// never [`Adrift`], because it is not a move on the Job at all.
    async fn start_run(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        run: ipc::StartRun,
    ) -> Result<ipc::RunUnderway, Refusal> {
        Fleet::start_rehearsal(self, &job_id.to_domain(), run).await
    }

    /// **Not [`budgeted`]**, for [`Commands::start_run`]'s reason.
    async fn stop_run(&self, job_id: JobId, run: ipc::NamedRun) -> Result<ipc::RunRecord, Refusal> {
        self.stop_rehearsal(&job_id.to_domain(), run.id).await
    }

    /// **Not [`budgeted`]**, for [`Commands::start_run`]'s reason.
    async fn undo_run(&self, job_id: JobId, run: ipc::NamedRun) -> Result<ipc::RunRecord, Refusal> {
        self.undo_rehearsal(&job_id.to_domain(), run.id).await
    }

    /// A person starting a server, for a Job or the main checkout. **The `Arc`
    /// is handed on**, so the server is a task of its own — `crate::servers`.
    ///
    /// **Not [`budgeted`]**, for [`Commands::start_run`]'s reason: a server
    /// answers through its own refusal.
    async fn start_server(
        self: std::sync::Arc<Self>,
        asked: ipc::StartServer,
    ) -> Result<ipc::ServerState, Refusal> {
        let place = match &asked.job_id {
            Some(job_id) => crate::servers::Place::Job(
                self.load(&job_id.to_domain())
                    .await
                    .map_err(|why| self.refusal(why))?,
            ),
            None => crate::servers::Place::MainCheckout,
        };
        let refusing = std::sync::Arc::clone(&self);
        Fleet::hold_server(self, place, &asked.name, ipc::StartedBy::Person)
            .await
            .map(|(state, _)| state)
            .map_err(|why| refusing.server_refusal(why, asked.job_id.as_ref()))
    }

    /// **Not [`budgeted`]**, for [`Commands::start_run`]'s reason.
    async fn stop_server(&self, named: ipc::NamedServer) -> Result<ipc::ServerState, Refusal> {
        self.stopped_server(&named.id)
            .await
            .map_err(|why| self.server_refusal(why, None))
    }

    /// More money for one Job, and nothing else changes.
    ///
    /// **The summary is re-read rather than folded from the raise.** Nothing
    /// moved, so the row a caller wants back is the row as it now stands — and
    /// the field it wants is `queued_reason`, which `summarised` computes from
    /// the board through the same predicate admission asks. A raise that reports
    /// its own success would be a second answer to *is this Job still held*.
    async fn raise_cost_cap(
        self: Arc<Self>,
        job_id: JobId,
        raise: CapRaise,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::raise_cost_cap(&fleet, &job_id.to_domain(), &raise).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// More turns for one Job, and nothing else changes. The method above's
    /// shape and its reasons, on the other ceiling.
    async fn raise_turn_cap(
        self: Arc<Self>,
        job_id: JobId,
        raise: TurnRaise,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::raise_turn_cap(&fleet, &job_id.to_domain(), &raise).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A verdict on the work, and the Job is over.
    async fn reject_job(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::reject(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The process, not the unit of work.
    async fn kill_drone(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::kill_drone(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The unit of work, not the process.
    async fn kill_job(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::kill_job(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Go and look at this Job now. **The one act here that moves nothing** —
    /// what it leaves is a line in the Job's own log saying somebody asked and
    /// what was found, which is why it is a command rather than a read.
    ///
    /// **The redaction is `crate::resources`'s.** A process crosses as its
    /// executable's name and never its argument vector, which carries absolute
    /// paths and whatever a Check was invoked with.
    async fn examine_job(self: Arc<Self>, job_id: JobId) -> Result<JobExamined, Refusal> {
        budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let job = fleet.load(&job_id.to_domain()).await?;
                fleet.examined(&job).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))
    }

    /// The record, gone. **Nothing is redacted here** — there is no Job left
    /// to redact, only the id it used to name.
    async fn forget_job(self: Arc<Self>, job_id: JobId) -> Result<JobForgotten, Refusal> {
        budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let id = job_id.clone();
            async move { Fleet::forget_job(&fleet, &id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok(JobForgotten { job_id })
    }

    /// The disk, given back, with the record left standing. **Nothing is
    /// redacted here either** — every field of the answer is about a directory
    /// and a branch this Job derived, and there is no path in it a person
    /// clearing their own disk should not be shown.
    async fn reclaim_worktree(
        self: Arc<Self>,
        job_id: JobId,
    ) -> Result<WorktreeReclaimed, Refusal> {
        let id = job_id.to_domain();
        let gave_back = budgeted_for(self.command_budget(), job_id, {
            let fleet = Arc::clone(&self);
            let id = id.clone();
            async move { Fleet::reclaim_worktree(&fleet, &id).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok(crate::wire::reclaimed(&id, gave_back))
    }

    /// **Two Jobs, redacted separately.** The failed one is now `killed`; the
    /// replacement carries `redispatched_from` and is what the caller opens
    /// next.
    async fn redispatch_job(self: Arc<Self>, job_id: JobId) -> Result<Redispatched, Refusal> {
        let both = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { fleet.redispatch(&job_id.to_domain()).await }
        })
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
        self: Arc<Self>,
        job_id: JobId,
        instruction: Redirection,
    ) -> Result<JobSummary, Refusal> {
        let said = Instruction::saying(&instruction.instruction)
            .ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { fleet.redirect(&job_id.to_domain(), &said).await }
        })
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
        self: Arc<Self>,
        job_id: JobId,
        note: Option<ipc::RestartRequested>,
    ) -> Result<JobSummary, Refusal> {
        let said = match &note {
            Some(note) => Some(
                Instruction::saying(&note.note).ok_or_else(|| self.refusal(Adrift::Unnameable))?,
            ),
            None => None,
        };
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::restart_step(&fleet, &job_id.to_domain(), said.as_ref()).await }
        })
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
        self: Arc<Self>,
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
        let filed = budgeted_for(self.command_budget(), job_id, {
            let fleet = Arc::clone(&self);
            async move { Fleet::file_report(&fleet, &id, &filed).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        reported(&filed).map_err(|why| self.refusal(why))
    }

    /// A person's answer to the question a waiting Drone asked. **Nothing
    /// moves** — the Job was `running` while it waited and is now, and the
    /// summary comes back because a caller folds the row rather than re-reading
    /// the board. The four refusals are `crate::questioning::NotAnswered`'s.
    async fn answer_question(
        self: Arc<Self>,
        job_id: JobId,
        answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                Fleet::answer_question(&fleet, &id, answer.question_id.as_str(), &answer.chose)
                    .await
                    .map_err(|why| why.about(&id))?;
                fleet.load(&id).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person's answer to a command a Drone was refused or is waiting on.
    /// The refusals are `crate::permitting::NotPermitted`'s.
    async fn answer_command(
        self: Arc<Self>,
        job_id: JobId,
        answer: ipc::AnswerCommand,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                Fleet::answer_command(&fleet, &id, &answer.call, answer.answer)
                    .await
                    .map_err(|why| why.about(&id))?;
                fleet.load(&id).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// How this Job meets a blocked command, changed. Loaded first, so an id
    /// naming nothing is a 404 rather than a setting written against nothing.
    async fn set_when_blocked(
        self: Arc<Self>,
        job_id: JobId,
        setting: ipc::SetWhenBlocked,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                let when = crate::permitting::domain_setting(setting.when_blocked);
                Fleet::set_when_blocked(&fleet, &id, when)
                    .await
                    .map_err(|why| why.about(&id))?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person's answer to the question a Judge refusal opened.
    /// `crate::asking::answer_judge`'s refusals.
    async fn answer_judge(
        self: Arc<Self>,
        job_id: JobId,
        answered: ipc::JudgeAnswered,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                Fleet::answer_judge(&fleet, &id, answered.answer, answered.note).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// How this Job meets a Judge criterion that refuses, changed. Loaded
    /// first, for `set_when_blocked`'s reason.
    async fn set_when_refused(
        self: Arc<Self>,
        job_id: JobId,
        setting: ipc::SetWhenRefused,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                let when = crate::asking::domain_when_refused(setting.when_refused);
                Fleet::set_when_refused(&fleet, &id, when).await?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The model this Job's later steps spawn on, chosen or cleared. Loaded
    /// first for `set_when_blocked`'s reason; the refusals are
    /// `crate::permitting::NotPermitted`'s.
    async fn set_model(
        self: Arc<Self>,
        job_id: JobId,
        choice: ipc::SetModel,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                Fleet::set_model(&fleet, &id, choice.model.as_deref())
                    .await
                    .map_err(|why| why.about(&id))?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A command a person allowed for this Job, taken back.
    async fn remove_allowed_command(
        self: Arc<Self>,
        job_id: JobId,
        removing: ipc::RemoveAllowedCommand,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                Fleet::remove_allowed_command(&fleet, &id, &removing.run)
                    .await
                    .map_err(|why| why.about(&id))?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }
}
