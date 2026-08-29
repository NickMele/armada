//! `api::Daemon`, implemented over a real Fleet.
//!
//! **The dependency points this way and never back.** The trait is stated in
//! `api`, where the transport is, and implemented here, where the Jobs are.
//! `cargo tree -p api` names no `fleet`: the daemon core is drivable in a test
//! with no socket, no port and no process, and `api`'s own tests were written
//! against a fake before this existed.
//!
//! **This is the redaction, and it is a visible step.** Every signature below
//! speaks `ipc` DTOs, `JobSummary::of` is called here by hand, and a field added
//! to `core_model::Job` reaches the wire only when somebody writes the line that
//! puts it there — `api` never sees a domain type.
//!
//! **Where the refusals come from.** [`Refusal`] has four variants because a
//! caller has four things to do about them, and which is chosen is decided by
//! the *typed* leaf error rather than by a message. `Adrift::IllegalMove` is a
//! 409, the machine having refused; a store fault is a 500, not the caller's
//! doing; a Job that is not there is a 404; a proposal naming a workflow, a
//! Manifest or a model that cannot work is a 422, well-formed and unworkable.
//!
//! **The reason costs a second read, and is not derived.** `JobSummary` carries
//! the reason its last transition stored, which is in `job_events` and not on
//! the `jobs` row, so each summary below reads the Job's log for it. N reads for
//! N Jobs is the honest shape at M1: a status-to-reason mapping here would be a
//! second vocabulary that agrees with the log only until something changes.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Daemon, Observed, Refusal};
use core_model::{
    Job, JobId as CoreJobId, JobStatus as CoreJobStatus, QueuedReason as CoreQueuedReason,
};
use ipc::mcp::{CheckReport, DeclareScope, NotRecorded, Receipt, SubmitEvidence};
use ipc::{
    ChangesRequested, CheckRun, DeclaredCheck, DeclaredJudge, Flagged, JobDetail, JobDiff,
    JobEvidence, JobHistory, JobId, JobList, JobSummary, Judged, ManifestId, ManifestSummary,
    ModelChoices, Overruled, ProposeJob, Redirection, Redispatched, RunId, StepFacts, StepId,
    Submitted, WireError, WireValue, Work, WorkflowId, WorkflowStep, WorkflowSummary,
};
use store::{LoadJobError, Moved, RecordedEvent, WriteError};

use crate::adrift::{Adrift, NotSubmitted};
use crate::daemon::Fleet;
use crate::dispatch::clear_to_run;
use crate::footprint::kept;
// The wire's `Redirection` is a struct with a public field; Fleet's is a
// newtype that cannot hold an empty instruction. Both names are in scope here,
// which is the one place they meet.
use crate::overruling::Overruling;
use crate::reporting::Filed;
use crate::resume::Redirection as Instruction;

/// The codes this boundary raises, declared beside the thing that raises them.
///
/// The set is closed by collection rather than by authorship — a central
/// registry would put every code far from the failure it names.
const NO_SUCH_JOB: &str = "fleet.no_such_job";
const ILLEGAL_MOVE: &str = "fleet.illegal_move";
const FAULT: &str = "fleet.fault";
/// A proposal that decoded and names something that cannot produce a Drone.
const UNACCEPTABLE: &str = "fleet.unacceptable_proposal";
/// The request was read and no workflow fits. **A refusal about the request**,
/// and the reason it has a code of its own: a caller reading `UNACCEPTABLE`
/// cannot tell it from a proposal naming a workflow that does not exist.
const NO_WORKFLOW_FITS: &str = "fleet.no_workflow_fits";
/// The proposer call could not be made. **Never the code above** — a client
/// that rendered an outage as "nothing fits" would tell a person their request
/// was refused when it was never read.
const PROPOSER_UNREACHABLE: &str = "fleet.proposer_unreachable";
/// A redispatch asked for on a Job that is not waiting for a person. A 409 like
/// a refused move, and a code of its own because the machine was never asked.
const NOT_REDISPATCHABLE: &str = "fleet.not_redispatchable";
/// A review act asked for on a Job that is not standing at a human gate. Its
/// own code because a caller reading `ILLEGAL_MOVE` would look for an edge that
/// exists — the machine was never asked.
const NOT_UNDER_REVIEW: &str = "fleet.not_under_review";
/// An act on a stopped step asked for on a Job that has no stopped step to act
/// on, or one whose step stopped for a reason the act does not answer. A 409
/// for [`NOT_UNDER_REVIEW`]'s reason — the machine was never asked, so a caller
/// reading `ILLEGAL_MOVE` would go looking for an edge that is there.
///
/// **These reached a caller as 500s.** A resume on a Job that is not escalated
/// is the caller asking for the wrong act, not Fleet breaking, and a 500 sends
/// them to retry something that will fail identically for ever.
const NOT_RESUMABLE: &str = "fleet.not_resumable";

/// A path as the filesystem knows it, or as it was given where it cannot be
/// resolved.
///
/// A Manifest that has just been read exists, so the fallback covers the case
/// where it stopped existing between the read and the ask — and a path saying
/// where Fleet looked beats an empty string saying nothing.
fn canonical(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

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
        let mut jobs = Vec::with_capacity(loaded.jobs.len());
        for job in &loaded.jobs {
            jobs.push(self.summarised(job).await?);
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

    /// One Job in full, folded from its log like every other read.
    ///
    /// # The footprint read is spent only where there is one to read
    ///
    /// This call is made on every open of a Job, which is the argument that put
    /// the history and the patch on routes of their own. A footprint is neither
    /// — it is a path and a word per file — and it is written at the terminal
    /// transition, so a Job that is still going has none. Asking only for a
    /// Job that has stopped is what keeps an open of a running Job costing
    /// exactly what it cost before, and `footprint` absent on one of them is
    /// the truth rather than an omission.
    ///
    /// **The wait a redirect left is on this read and on no other**, because it
    /// is held in the slot rather than written down — `Fleet::redirect_awaited`.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let (ran, judged, flagged) = {
            let store = self.store().lock().await;
            let ran = store
                .step_checks(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let judged = store
                .step_judgments(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let flagged = store
                .step_gaming_flags(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            (ran, judged, flagged)
        };
        let recorded = match job.status().is_terminal() {
            false => None,
            true => self
                .store()
                .lock()
                .await
                .footprint(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?,
        };
        let queued = self.queued_reason(&job).await?;
        // Before `step_facts`, which consumes the Check runs: the
        // classification reads them to answer whether an override is available,
        // and reading them twice would be a second answer to one question.
        let stuck = self.why_stuck(&job, reason.as_ref(), &ran).await;
        Ok(JobDetail::of(
            &job,
            reason.as_ref(),
            queued,
            &self.step_facts(&job, ran, judged, flagged),
            recorded.as_ref().map(kept),
            self.redirect_awaited(job.id()).await,
            stuck.as_ref(),
        ))
    }

    /// Every move one Job made, oldest first. **The log, read — not folded.**
    ///
    /// # The Job is loaded first, and that is not a wasted read
    ///
    /// It is what makes an id that names nothing a 404 rather than an empty
    /// history, which would be a lie about a Job that exists and has not moved.
    /// It also keeps this read behind the same fold every other read is behind:
    /// a log the machine would not admit refuses to load, so a history that
    /// reaches the wire is one `Job::transition` accepted. **This read cannot
    /// show a state the fold rejected**, and it does not replay anything to
    /// avoid it — `crates/store/src/fold.rs` is still the only caller.
    ///
    /// # The rows come back whole
    ///
    /// One query, in `seq` order, over the one table both machines write to.
    /// A step move ordered against the status transitions around it is what a
    /// separately keyed second log could not have offered.
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
        let plan = {
            let working = self.slot().lock().await;
            working
                .as_ref()
                .filter(|at_work| at_work.is(job.id()))
                .and_then(|at_work| at_work.declared().cloned())
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

    /// Every workflow this Fleet holds, so a caller can name one that will not
    /// be refused.
    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        Ok(self
            .workflows()
            .values()
            .map(|workflow| WorkflowSummary {
                id: WorkflowId::from(workflow.id()),
                name: workflow.name().to_string(),
                version: workflow.version(),
                steps: declared(workflow),
                manifest_id: ManifestId::from(self.manifest().id()),
            })
            .collect())
    }

    /// The one Manifest this Fleet was started against.
    ///
    /// **`repository` is not a name the Manifest declares.** `armada.yml` has
    /// no key for one — `version`, `id`, `checks` and `commands` are the whole
    /// schema — so what is carried is the directory the file was read from,
    /// which is a fact rather than an invention. A person reading a Job wants
    /// to know which project it runs against, and a ULID does not say.
    async fn list_manifests(&self) -> Result<Vec<ManifestSummary>, Refusal> {
        let manifest = self.manifest();
        let path = manifest.path();
        Ok(vec![ManifestSummary {
            id: ManifestId::from(manifest.id()),
            repository: path
                .parent()
                .and_then(|dir| dir.file_name())
                .map(|name| name.to_string_lossy().to_string())
                // A Manifest at the filesystem root has no directory to name.
                // Its own path is the next most useful true thing.
                .unwrap_or_else(|| path.to_string_lossy().to_string()),
            // **Absolute, and it has to be.** Bridge derives every artifact path
            // from this one — the worktree, the Job log, the transcripts — and
            // then hands the result to the OS to open. A relative path answers
            // a question about Fleet's working directory, which is not a fact
            // about the repository and is not a directory Bridge is in: served
            // as `./armada.yml`, every one of those opens resolved against the
            // Electron process and found nothing.
            //
            // `$HOME` therefore appears on this wire, which the log envelope
            // and the failure record both refuse. It is a different surface:
            // those are written down and read later, and this is two processes
            // on one machine agreeing where a file is.
            path: canonical(path),
            version: manifest.version(),
            checks: manifest.check_names(),
        }])
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
            .propose_from(&request.request)
            .await
            .map_err(|why| self.refusal(why))?;
        let mut jobs = Vec::with_capacity(made.len());
        for job in &made {
            jobs.push(self.summarised(job).await?);
        }
        Ok(ipc::ProposedPlan { jobs })
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

    /// A fresh Drone on the worktree the last one left.
    ///
    /// The Job resumes at the step that stopped; every earlier step's work is
    /// on the branch and is not redone. That is what separates this from a
    /// redispatch, which starts a replacement Job at the approval gate.
    async fn restart_step(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = self
            .restart_step(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person says this Job failed in error, and the record is filed with
    /// what they said.
    ///
    /// **Everything here is in `crate::reporting`**, including the emptiness:
    /// `Filed::saying` refuses a blank sentence and a half-named criterion, and
    /// this only carries the refusal out as the 422 a blank override reason
    /// gets — a report carrying the record and no sentence has added nothing
    /// that was not already served by three other routes.
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
        .ok_or_else(|| self.refusal(Adrift::Unreasoned { job: id.clone() }))?;
        let filed = Fleet::file_report(self, &id, &filed)
            .await
            .map_err(|why| self.refusal(why))?;
        reported(&filed).map_err(|why| self.refusal(why))
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

    /// The Evidence tool. **The one method here whose caller is a Drone.**
    ///
    /// It converts and maps, and decides nothing: the binding — which Job, which
    /// step, which evidence type — is `Fleet::record_evidence`'s, under the lock
    /// that makes it a single decision.
    ///
    /// Every path answers 200 with `isError` rather than a status code, because
    /// a Drone reads a tool error and can act on it, and a 4xx reaches the model
    /// as a broken server — which is something it stops trying.
    async fn submit_evidence(&self, submission: SubmitEvidence) -> Result<Receipt, NotRecorded> {
        match self.record_evidence(&submission).await {
            Ok(recorded) => Ok(Receipt {
                word: recorded.word().to_string(),
            }),
            Err(why) => Err(told(why)),
        }
    }

    /// Where the working Drone says this step's work will be.
    ///
    /// The same shape as the call above and the same reason for it: the binding
    /// — which Job, which step — is `Fleet::declare_scope`'s, under the slot
    /// lock, and every refusal answers 200 with `isError` so a Drone can read
    /// it and declare again.
    async fn declare_scope(&self, declaration: DeclareScope) -> Result<Receipt, NotRecorded> {
        match Fleet::declare_scope(self, &declaration).await {
            Ok(declared) => Ok(Receipt {
                word: declared.word().to_string(),
            }),
            Err(why) => Err(NotRecorded {
                because: why.to_string(),
            }),
        }
    }

    /// The Drone asking whether its work passes.
    ///
    /// It converts and maps like the two above, and decides as little: which
    /// Checks, what they are run against and what bounds the asking are all
    /// `Fleet::run_checks`'s, under the slot lock that binds them to one step.
    ///
    /// **What comes back is a report and never a verdict.** The step is exactly
    /// where it was when the call arrived, whatever the Checks said.
    async fn run_checks(&self) -> Result<CheckReport, NotRecorded> {
        Fleet::run_checks(self).await.map_err(|why| NotRecorded {
            because: why.to_string(),
        })
    }
}

/// The wire shape of one declared Check.
///
/// A free function rather than a `From` impl: `ipc` already depends on
/// `core-model` and could hold this, but a `DeclaredCheck` is assembled from a
/// step rather than converted from one, and the assembly is Fleet's.
///
/// **The command crosses, and it comes off the resolved workflow.** A
/// `ResolvedCheck` holds the `run` this workflow froze, which is what the gate
/// runs; serving it from the live Manifest instead would show a command that is
/// not what ran the moment somebody edits `armada.yml` under a Job.
fn declared_check(check: &core_model::ResolvedCheck) -> DeclaredCheck {
    DeclaredCheck {
        kind: check.kind().to_string(),
        name: check.name().map(str::to_string),
        run: check.run().map(str::to_string),
        expect_exit_code: check.expects(),
    }
}

/// The word a step is drawn as, with its id standing in where there is none.
///
/// A blank label is a definition that declared the key and left it empty, and a
/// blank on the rail reads as a Fleet that lost the value.
fn reads_as(label: &str, step_id: &str) -> String {
    match label.trim().is_empty() {
        true => step_id.to_string(),
        false => label.to_string(),
    }
}

/// A workflow's steps with what each one declares, in the workflow's order.
///
/// **This is what the next Job would freeze**, and `get_job` answers from what
/// its Job already froze. The two can now differ, which is the point: a
/// workflow edited under a running Job shows the new declaration here and the
/// approved one there.
///
/// Both tiers and the gate, because this is read *before* a dispatch: after the
/// fact the rail says what happened, and here a person is agreeing to it.
fn declared(workflow: &config::ResolvedWorkflow) -> Vec<WorkflowStep> {
    workflow
        .steps()
        .iter()
        .map(|step| WorkflowStep {
            step_id: StepId::from(step.id()),
            label: reads_as(step.label(), step.id().as_str()),
            checks: step.checks().iter().map(declared_check).collect(),
            // The rail's own narrowing, called rather than restated: an entry
            // that asks nothing and looks for nothing is not a Judge call, and
            // a preview that counted one would promise a call nothing makes.
            judge_checks: DeclaredJudge::firing(step.judge_checks()),
            advance_gate: step.advance_gate().into(),
        })
        .collect()
}

/// One log row, as the wire carries it. **The redaction, for a history.**
///
/// It is a plain function and not a `From` because the orphan rule puts one at
/// this boundary in `ipc`, and `ipc` has no `store` — the crate that
/// deserializes rows is deliberately not the crate that deserializes the wire.
/// So the field-by-field decision is written here, where both types are in
/// scope, and it is the same decision `JobSummary::of` makes: nothing reaches
/// Bridge that somebody did not write a line for.
///
/// It replays nothing. Every value below is copied across; none is put back
/// through `Job::transition`, which `crates/store/src/fold.rs` has already done
/// by the time this runs.
fn recorded(event: &RecordedEvent) -> ipc::Recorded {
    ipc::Recorded {
        seq: event.seq(),
        status: event.under().into(),
        moved: match event.moved() {
            Moved::Job { to, reason } => ipc::Movement::Status(ipc::StatusMoved {
                to: (*to).into(),
                reason: ipc::Reason::of(reason),
            }),
            Moved::Step {
                step_id,
                from,
                to,
                why,
            } => ipc::Movement::Step(ipc::StepMoved {
                step_id: step_id.into(),
                from: (*from).into(),
                to: (*to).into(),
                // The registry's own spelling, through the narrowing newtype —
                // a step is stopped only by a step-level trigger, and nothing
                // here restates the list.
                why: why.map(|trigger| trigger.as_wire().to_string()),
            }),
            Moved::Drone { drone_id, presence } => ipc::Movement::Drone(ipc::DroneMoved {
                drone_id: drone_id.into(),
                presence: (*presence).into(),
            }),
        },
        actor: event.actor().into(),
        at: event.at().into(),
    }
}

/// One step's evidence, as the wire carries it. **The redaction, for a claim.**
///
/// A plain function rather than a `From` for [`recorded`]'s reason: the orphan
/// rule would put the impl in `ipc`, and the pair is `(StepId, StepEvidence)`
/// rather than one type. Every field crosses — the three sentences are the
/// whole of what a submission is — and the one that does not is `source`, which
/// the record does not have either.
fn submitted(recorded: &(core_model::StepId, core_model::StepEvidence)) -> Submitted {
    let (step_id, evidence) = recorded;
    Submitted {
        step_id: step_id.into(),
        evidence_type: evidence.evidence_type.into(),
        claimed: evidence.claimed.clone(),
        shown_by: evidence.shown_by.clone(),
        // Absent rather than blank. `not_claimed` is legitimately empty on the
        // record, and an empty string on the wire reads as a boundary somebody
        // lost.
        not_claimed: Some(evidence.not_claimed.clone()).filter(|text| !text.is_empty()),
    }
}

/// One filed report, as the wire carries it. **The redaction, for a report.**
///
/// A plain function for [`recorded`]'s reason. Every field crosses, which is
/// what a report is for — and the scrubbing that makes that safe happened on
/// the way *in*, in `crate::reporting`, rather than here: a record already
/// written is a record already at rest, and this is a read.
///
/// A `claim` or an `origin` the wire has no spelling for refuses the read
/// rather than being served under a value nobody chose. Such a row is one
/// nothing but a hand-edited database can produce — the wire's own decode
/// refuses an unknown claim on the way in — and a report shown under the wrong
/// claim would be counted under it too.
fn reported(filed: &store::Report) -> Result<ipc::Report, Adrift> {
    Ok(ipc::Report {
        id: ipc::ReportId::carried(filed.report_id.clone()),
        filed_at: (&filed.filed_at).into(),
        origin: ipc::ReportOrigin::from_wire(&filed.origin)
            .ok_or_else(|| unreadable("origin", &filed.origin))?,
        claim: ipc::Claim::from_wire(&filed.claim)
            .ok_or_else(|| unreadable("claim", &filed.claim))?,
        job_id: ipc::JobId::from(&filed.job_id),
        job_title: filed.job_title.clone(),
        step_id: filed.step_id.as_ref().map(ipc::StepId::from),
        criterion_id: filed.criterion_id.as_ref().map(ipc::CriterionId::from),
        said: filed.said.clone(),
        record: filed.record.clone(),
    })
}

/// A stored spelling this build has no value for.
fn unreadable(column: &'static str, value: &str) -> Adrift {
    Adrift::Reading(LoadJobError::Unreadable(
        store::RowError::UnknownEnumValue {
            table: "reports",
            column,
            value: value.to_string(),
        },
    ))
}

/// A refusal, as the Drone reads it.
///
/// The name is the typed variant and stays on this side; what crosses is the
/// sentence it renders to, because that is the only part a Drone can act on.
fn told(why: NotSubmitted) -> NotRecorded {
    NotRecorded {
        because: why.to_string(),
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
    /// A Job as a Board row, with the reason its last transition stored.
    async fn summarised(&self, job: &Job) -> Result<JobSummary, Refusal> {
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let queued = self.queued_reason(job).await?;
        Ok(JobSummary::of(job, reason.as_ref(), queued))
    }

    /// Why an approved Job has not started, worked out from the board as it
    /// stands rather than read from anything.
    ///
    /// **Nothing is read for a Job that is not `queued`**, which is every row
    /// but the waiting ones — the board read is the cost, and a status that
    /// cannot have this reason must not pay it.
    ///
    /// The dependency half is [`clear_to_run`], the predicate admission itself
    /// uses. A second answer here is how a Board comes to say a Job is blocked
    /// while Fleet is starting it.
    ///
    /// **`None` is the registry's `none`** — approved, unblocked, and the slot
    /// is free, which is a Job about to run rather than one held.
    async fn queued_reason(&self, job: &Job) -> Result<Option<CoreQueuedReason>, Refusal> {
        if job.status() != CoreJobStatus::Queued {
            return Ok(None);
        }
        let (loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        let standing: BTreeMap<CoreJobId, CoreJobStatus> = loaded
            .jobs
            .iter()
            .map(|held| (held.id().clone(), held.status()))
            .collect();
        if !clear_to_run(job, &standing) {
            return Ok(Some(CoreQueuedReason::BlockedByDependency));
        }
        Ok(self
            .working_on()
            .await
            .map(|_| CoreQueuedReason::WaitingOnResources))
    }

    /// What Fleet knows about a Job's steps beyond the `job_steps` rows.
    ///
    /// **The declaration comes from the Job's own frozen workflow**, which is
    /// also what the gate runs — so what a person is shown and what actually
    /// gates the step are one value rather than two that can drift.
    ///
    /// `declares` stays an `Option` and is now absent only where the frozen
    /// workflow does not declare the step at all. That cannot happen through
    /// this crate, since the `job_steps` rows are seeded from those steps; it is
    /// kept because "Fleet cannot say" and "the step declares nothing" are
    /// different sentences on the wire and a row written by something else
    /// should not read as the second.
    /// `judged` and `flagged` are read from the store beside `ran` and never
    /// off the Job: the `job_steps` row carries the trigger the gate stopped on
    /// and nothing else, so a refusal's citation and a gaming finding's pattern
    /// — the whole of what an escalated Job has to say — live in their own
    /// tables and arrive here.
    fn step_facts(
        &self,
        job: &Job,
        ran: Vec<(core_model::StepId, Vec<core_model::StepCheck>)>,
        judged: Vec<(core_model::StepId, Vec<core_model::Judgment>)>,
        flagged: Vec<(core_model::StepId, Vec<core_model::GamingFlag>)>,
    ) -> Vec<StepFacts> {
        job.steps()
            .iter()
            .map(|step| StepFacts {
                step_id: StepId::from(step.step_id()),
                label: job
                    .workflow()
                    .step(step.step_id())
                    .map(|declared| declared.label().to_string()),
                declares: job
                    .workflow()
                    .step(step.step_id())
                    .map(|declared| declared.checks().iter().map(declared_check).collect()),
                ran: ran
                    .iter()
                    .find(|(at, _)| at == step.step_id())
                    .map(|(_, checks)| checks.iter().map(CheckRun::from).collect())
                    .unwrap_or_default(),
                judged: judged
                    .iter()
                    .find(|(at, _)| at == step.step_id())
                    .map(|(_, answers)| answers.iter().map(Judged::from).collect())
                    .unwrap_or_default(),
                flagged: flagged
                    .iter()
                    .find(|(at, _)| at == step.step_id())
                    .map(|(_, found)| found.iter().map(Flagged::from).collect())
                    .unwrap_or_default(),
                // The one fact here that is not a row. Read from the live slot
                // as the answer is assembled, because nothing writes it down.
                judging: self
                    .aloft()
                    .on(&ipc::JobId::from(job.id()), &StepId::from(step.step_id())),
            })
            .collect()
    }

    /// Which of the three refusals a failure is, decided from its type.
    ///
    /// **`run_id` names this process**, not Fleet's-by-assumption: it is minted
    /// where the emitter is, which is here.
    fn refusal(&self, why: Adrift) -> Refusal {
        let said = why.to_string();
        match &why {
            Adrift::Reading(LoadJobError::NoSuchJob { job_id })
            | Adrift::Writing(WriteError::NoSuchJob { job_id }) => Refusal::NoSuchJob(
                WireError::raised(NO_SUCH_JOB, said, self.run_id())
                    .about_job(ipc::JobId::from(job_id)),
            ),
            // The machine refused the move. The caller asked for something the
            // edge table does not have — approving a Job already running,
            // killing one already over — and 409 is the answer to that.
            Adrift::IllegalMove(_) | Adrift::IllegalStepMove(_) => {
                Refusal::IllegalMove(WireError::raised(ILLEGAL_MOVE, said, self.run_id()))
            }
            // The same conflict, from a request the machine never saw: the Job
            // is somewhere a replacement would mean nothing.
            // The Job is not at a human gate. A 409 like the conflicts above,
            // and never a 500: the machine was never asked, so there is no edge
            // for a caller to go looking for.
            Adrift::NotUnderReview { job, .. } | Adrift::NoDroneToTell { job } => {
                Refusal::IllegalMove(
                    WireError::raised(NOT_UNDER_REVIEW, said, self.run_id())
                        .about_job(ipc::JobId::from(job)),
                )
            }
            // What an act on a stopped step refuses with, and they are
            // conflicts of one kind: the Job is somewhere, or its step stopped
            // for something, that this act does not answer. `NotTheJudges` and
            // `CheckDidNotPass` are an override's; `NotUndecided`,
            // `NotStandingThere` and `NothingToRuleOn` are a gate re-run's, and
            // the first two of those are the same sentence as `NotTheJudges`
            // read from the other side.
            Adrift::NotResumable { job, .. }
            | Adrift::NoStepStopped { job }
            | Adrift::NotTheJudges { job, .. }
            | Adrift::CheckDidNotPass { job, .. }
            | Adrift::NotUndecided { job, .. }
            | Adrift::NotStandingThere { job }
            | Adrift::NothingToRuleOn { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOT_RESUMABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            Adrift::NotRedispatchable { job, .. }
            | Adrift::NeverRan { job }
            | Adrift::NotReplaceable { job }
            | Adrift::WorkflowWithdrawn { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOT_REDISPATCHABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // The request is well-formed and the values in it cannot work. Not
            // a 500: retrying it will fail identically forever, and the message
            // names what to send instead.
            Adrift::Unnameable
            | Adrift::Unreasoned { .. }
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::Modelless
            | Adrift::NothingToPropose
            | Adrift::AttachmentUnreadable { .. } => {
                Refusal::Unacceptable(WireError::raised(UNACCEPTABLE, said, self.run_id()))
            }
            // The request was read and declined, and it goes back on the field
            // rather than being echoed in the message: what the person retypes
            // or hands to `propose_job` is what they wrote, character for
            // character. No Job exists.
            Adrift::NoWorkflowFits { request, .. } => Refusal::Unacceptable(
                WireError::raised(NO_WORKFLOW_FITS, said, self.run_id())
                    .with_field("request", WireValue::Str(request.clone())),
            ),
            // A call that could not be made, which is not that refusal — 500,
            // because nothing about the request is wrong and asking again is
            // reasonable. It comes back on the same field either way.
            // `NotProposable` falls to the catch-all below: a proposer that
            // could not be configured is Fleet's own fault and carries no
            // request to return.
            Adrift::NotProposed { request, .. } => Refusal::Fault(
                WireError::raised(PROPOSER_UNREACHABLE, said, self.run_id())
                    .with_field("request", WireValue::Str(request.clone())),
            ),
            _ => Refusal::Fault(WireError::raised(FAULT, said, self.run_id())),
        }
    }

    /// A worktree that would not be read, named against the Job it was for.
    ///
    /// **A 500 and never an empty diff.** A repository that will not open and a
    /// Drone that changed nothing are opposite answers, and a reviewer handed
    /// the second when the first happened would take work nobody read.
    fn unreadable<E: std::error::Error + Send + Sync + 'static>(
        &self,
        job: &core_model::JobId,
        cause: E,
    ) -> Refusal {
        self.refusal(Adrift::WorkUnreadable {
            job: job.clone(),
            cause: Box::new(cause),
        })
    }

    /// This process's run id.
    ///
    /// **Not minted here per call.** It is the id of the emitter, and the
    /// emitter is one process — so it is derived from the mint once and held,
    /// which is what `run_id` names.
    fn run_id(&self) -> RunId {
        RunId::carried(self.run().as_str())
    }
}
