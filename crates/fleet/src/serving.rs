//! `api::Daemon`, implemented over a real Fleet.
//!
//! **The dependency points this way and never back**, for the reasons
//! `api::daemon`'s own header gives and does not need repeating here.
//!
//! **What is local is that this is the redaction, and that it is a visible
//! step.** `JobSummary::of` is called here by hand, so a field added to
//! `core_model::Job` reaches the wire only when somebody writes the line that
//! puts it there.
//!
//! **Which refusal a failure is, and the code it carries, is
//! [`refusing`](mod@crate::refusing)'s** — every `WireError` below is raised
//! through `Fleet::refusal`. The trait's other half, whose caller is a Drone
//! and which refuses through no status code at all, is
//! [`tooling`](mod@crate::tooling)'s.
//!
//! **The reason costs a second read, and is not derived.** `JobSummary` carries
//! the reason its last transition stored, which is in `job_events` and not on
//! the `jobs` row, so each summary below reads the Job's log for it. N reads for
//! N Jobs is the honest shape at M1: a status-to-reason mapping here would be a
//! second vocabulary that agrees with the log only until something changes.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Daemon, Observed, Refusal};
use ipc::mcp::{
    CheckReport, DeclareScope, DispatchJob, NotRecorded, Receipt, RequestScope, SubmitEvidence,
};
use ipc::{
    CallArguments, ChangesRequested, FleetCapacity, JobDetail, JobDiff, JobEvidence, JobForgotten,
    JobHistory, JobId, JobList, JobSummary, ManifestSummary, ModelChoices, Overruled, ProposeJob,
    Redirection, Redispatched, Work, WorkflowSummary, WorktreeReclaimed, WorktreesHeld,
};
use store::LoadJobError;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
// The wire's `Redirection` is a struct with a public field; Fleet's is a
// newtype that cannot hold an empty instruction. Both names are in scope here,
// which is the one place they meet.
use crate::overruling::Overruling;
use crate::reporting::Filed;
use crate::resume::Redirection as Instruction;
use crate::wire::{
    manifest_summary, recorded, reported, submitted, workflow_summary, worktree_held,
};

impl<H, V, W> Daemon for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Every Job, **and every row that would not load**.
    ///
    /// The unreadable half travels with the readable one all the way to the
    /// Board. A Board that shows nine of ten Jobs and says so is honest; one
    /// that shows nine is not.
    async fn list_jobs(&self) -> Result<JobList, Refusal> {
        let (loaded, unreadable) = self.every_job().await.map_err(|why| self.refusal(why))?;
        // **One read for the whole list**, filled in afterwards rather than
        // passed into `JobSummary::of`. What became of a Job's pull request is
        // not on `core_model::Job` — it is in the delivery columns beside the
        // row — and reading it per Job would be a query per row on a list that
        // redraws on every event. Ordinarily an empty map: it holds only the
        // Jobs somebody has merged or closed.
        let landed = self
            .store()
            .lock()
            .await
            .landed_by_job()
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let mut jobs = Vec::with_capacity(loaded.jobs.len());
        for job in &loaded.jobs {
            let mut summary = self.summarised(job).await?;
            summary.landed = landed.get(job.id()).and_then(crate::noticing::settled);
            jobs.push(summary);
        }
        Ok(JobList {
            jobs,
            unreadable: unreadable
                .into_iter()
                .map(|fault| ipc::UnreadableJob {
                    job_id: None,
                    fault,
                })
                .collect(),
        })
    }

    /// How full the fleet is, and the one thing holding the next Drone back.
    ///
    /// **The same predicate again, unreduced.** `admit_next` opens with
    /// `room_for_another` and `queued_reason` folds it to one label; this is the
    /// third reader of that one answer and takes it whole — `Room::hold` is a
    /// `match` over the value admission returned, never a second opinion.
    ///
    /// **`occupied` is `Slots::count`**, because the roster is what the bound is
    /// measured against. A count over Job statuses would disagree: an escalated
    /// Job keeps its Drone alive and idle so a redirect costs no respawn, and it
    /// keeps its place. `count` sweeps the slots whose `Working` has gone.
    ///
    /// The roster lock is taken once and both facts are read under it, so the
    /// bound, the count and the reason cannot be three readings of three
    /// instants. It is admission's own lock order — roster first, then the poll
    /// lock inside `room_for_another` — so this adds no cycle.
    async fn get_capacity(&self) -> Result<FleetCapacity, Refusal> {
        let mut slots = self.slots().lock().await;
        let room = self.room_for_another(&mut slots).await;
        Ok(FleetCapacity::of(slots.cap(), slots.count(), room.hold()))
    }

    /// One Job in full. **The assembly is `crate::detail`'s** — the only
    /// operation here whose body is more than a conversion, and the reasoning
    /// about what is read and when is beside it.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        crate::detail::of(self, job_id).await
    }

    /// Every move one Job made, oldest first. **The log, read — not folded.**
    ///
    /// **The Job is loaded first, and that is not a wasted read.** It makes an
    /// id that names nothing a 404 rather than an empty history — a lie about a
    /// Job that exists and has not moved — and it keeps this read behind the
    /// same fold every other read is behind: a log the machine would not admit
    /// refuses to load, so a history that reaches the wire is one
    /// `Job::transition` accepted. **This read cannot show a state the fold
    /// rejected**, and nothing is replayed to get that: `crates/store/src/fold.rs`
    /// is still the only caller.
    ///
    /// The rows come back whole: one query, in `seq` order, over the one table
    /// both machines write to. A step move ordered against the status
    /// transitions around it is what a separately keyed second log could not
    /// have offered.
    async fn get_job_events(&self, job_id: JobId) -> Result<JobHistory, Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        let events = self
            .store()
            .lock()
            .await
            .events_for(&id)
            .map_err(|cause| self.refusal(Adrift::Reading(LoadJobError::Unreadable(cause))))?;
        Ok(JobHistory {
            job_id,
            moves: events.iter().map(recorded).collect(),
        })
    }

    /// Every claim this Job's Drones have submitted, step by step.
    ///
    /// **The Job is loaded first**, for `get_job_events`' reason: an id naming
    /// nothing is a 404, and never an empty list. Empty is a real answer and it
    /// means no step has submitted anything yet.
    async fn get_evidence(&self, job_id: JobId) -> Result<JobEvidence, Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(&id)
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        Ok(JobEvidence {
            job_id,
            steps: recorded.iter().map(submitted).collect(),
        })
    }

    /// One Job's whole patch, with the file list beside it.
    ///
    /// **The expensive read, and the only place Fleet spends it for a person.**
    /// `WorkProduct` keeps the patch behind its own call because the bytes are
    /// large and most steps ask no semantic question; the two calls are made
    /// together here because this is the one caller that wants both.
    ///
    /// A Job with no worktree answers `work: None` rather than an empty
    /// reading, and a worktree that will not open is a 500 rather than a patch
    /// nobody read. `plan_declared` is false unless this Job is the one holding
    /// the working slot: a declaration belongs to the Drone that made it, and
    /// there is nowhere else it survives.
    async fn get_diff(&self, job_id: JobId) -> Result<JobDiff, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let Some(worktree) = self.worktree_of(&job).map_err(|why| self.refusal(why))? else {
            return Ok(JobDiff { job_id, work: None });
        };
        let plan = match self.slot_of(job.id()).await {
            Some(slot) => slot
                .lock()
                .await
                .as_ref()
                .filter(|at_work| at_work.is(job.id()))
                .and_then(|at_work| at_work.declared().cloned()),
            None => None,
        };
        // **The whole branch, which is what a person opening a Job is reading.**
        let changed = self
            .work()
            .changed_files(&worktree)
            .map_err(|cause| self.unreadable(job.id(), cause))?;
        let patch = self
            .work()
            .patch(&worktree)
            .map_err(|cause| self.unreadable(job.id(), cause))?;
        Ok(JobDiff {
            job_id,
            work: Some(Work {
                files: crate::footprint::seen(&changed, plan.as_ref()),
                plan_declared: plan.is_some(),
                // Absent where there is nothing in it. An empty string reads as
                // a reading that broke, and a reading that broke is the refusal
                // above rather than a field.
                patch: Some(patch.as_str().to_string()).filter(|text| !text.is_empty()),
            }),
        })
    }

    /// One tool call's arguments, read back out of the Job's transcripts.
    ///
    /// **The other end of a row that was cut.** A `called` row on the socket
    /// carries a line and the argument's true size, so opening it can say how
    /// much there is; this is where the rest comes from, fetched once by the
    /// person who opened it rather than streamed to everyone watching.
    ///
    /// The Job is loaded first, so an id naming nothing is a 404 before any
    /// file is opened. A call the record does not carry is a different answer
    /// and says so: [`Adrift::NoSuchCall`], which is a 422 — the Job is there,
    /// and the id names nothing in it.
    async fn get_call(&self, job_id: JobId, call_id: String) -> Result<CallArguments, Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        crate::transcript::arguments(&self.host().repo_root, &id, &call_id)
            .await
            .ok_or_else(|| self.refusal(Adrift::NoSuchCall { named: call_id }))
    }

    /// Every workflow this Fleet holds, so a caller can name one that will not
    /// be refused.
    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        let manifest_id = self.manifest().id();
        Ok(self
            .workflows()
            .values()
            .map(|workflow| workflow_summary(workflow, manifest_id))
            .collect())
    }

    /// The one Manifest this Fleet was started against.
    async fn list_manifests(&self) -> Result<Vec<ManifestSummary>, Refusal> {
        Ok(vec![manifest_summary(self.manifest())])
    }

    /// What a Job may be spawned as, resolved once by the composition root.
    async fn list_models(&self) -> Result<ModelChoices, Refusal> {
        Ok(self.models().clone())
    }

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

    /// Every worktree Fleet is holding disk for, and the test each one failed.
    ///
    /// **`Fleet::worktrees_held` is the derivation and this only redacts it.**
    /// The five tests are written once, in `crate::holding`, and the sweep and
    /// `armada clean` already read that same answer; a filter here that decided
    /// for itself what is safe would be the third opinion the sharing exists to
    /// prevent.
    ///
    /// **The one thing dropped is a piloted Job's checkout**, and it is dropped
    /// through `Holding::offerable` rather than by matching a status here —
    /// `#367`, and the predicate belongs beside the tests it reads.
    async fn list_worktrees(&self) -> Result<WorktreesHeld, Refusal> {
        let holding = Fleet::worktrees_held(self)
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(WorktreesHeld {
            worktrees: holding
                .iter()
                .filter(|one| one.offerable())
                .map(worktree_held)
                .collect(),
        })
    }

    /// Every report filed, newest first, with the counts they are read beside.
    async fn list_reports(&self) -> Result<ipc::ReportList, Refusal> {
        let (filed, counted) = Fleet::reports(self)
            .await
            .map_err(|why| self.refusal(why))?;
        let mut reports = Vec::with_capacity(filed.len());
        for report in &filed {
            reports.push(reported(report).map_err(|why| self.refusal(why))?);
        }
        Ok(ipc::ReportList {
            reports,
            calibration: ipc::Calibration {
                refusals_recorded: counted.refusals_recorded,
                refusals_disputed: counted.refusals_disputed,
                passes_disputed: counted.passes_disputed,
                reports_filed: counted.reports_filed,
            },
        })
    }

    /// One Job's turns. **Subscribe, then read the history** — the one order,
    /// in the one place that can take both halves.
    ///
    /// The other order loses a row that arrives between the two. This order can
    /// repeat one, and a repeat is detectable from the call id a row carries
    /// while a gap is not detectable at all. It is the ordering
    /// `api::Broadcaster::subscribe` already documents for the same reason.
    ///
    /// The Job is loaded first so that an id naming nothing is a 404 rather
    /// than a socket that opens on an empty history. **Nothing else about the
    /// Job is read**: watching is a property of a connection, not of the work,
    /// and a status that would refuse a viewer does not exist.
    async fn observe_job(&self, job_id: JobId) -> Result<Observed, Refusal> {
        self.load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let live = self.turns().watching(&job_id);
        let (history, skipped) =
            crate::transcript::history(&self.host().repo_root, &job_id.to_domain()).await;
        Ok(Observed {
            job_id,
            live,
            history,
            skipped,
        })
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

    // The rest of the trait is the other caller. **A Drone makes these calls,
    // not a person**, and no row of `crates/ipc/operations.toml` names one:
    // they answer a `Receipt` through `NotRecorded` rather than a DTO through
    // `Fleet::refusal`, so nothing said above about the redaction or the
    // refusal path is theirs. They are [`tooling`](mod@crate::tooling)'s.

    async fn ask_question(
        &self,
        caller: api::Caller,
        asking: ipc::mcp::AskQuestion,
    ) -> Result<Receipt, NotRecorded> {
        self.asked(caller, asking).await
    }

    async fn submit_evidence(
        &self,
        caller: api::Caller,
        submission: SubmitEvidence,
    ) -> Result<Receipt, NotRecorded> {
        self.submitted(caller, submission).await
    }

    async fn declare_scope(
        &self,
        caller: api::Caller,
        declaration: DeclareScope,
    ) -> Result<Receipt, NotRecorded> {
        self.declared(caller, declaration).await
    }

    async fn request_scope(
        &self,
        caller: api::Caller,
        request: RequestScope,
    ) -> Result<Receipt, NotRecorded> {
        self.widened(caller, request).await
    }

    async fn run_checks(&self, caller: api::Caller) -> Result<CheckReport, NotRecorded> {
        self.checked(caller).await
    }

    async fn dispatch_job(
        &self,
        caller: api::Caller,
        dispatch: DispatchJob,
    ) -> Result<Receipt, NotRecorded> {
        self.dispatched(caller, dispatch).await
    }
}
