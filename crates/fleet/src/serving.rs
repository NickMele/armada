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
//! **Which refusal a failure is, and the code it carries, is
//! [`refusing`](mod@crate::refusing)'s** — every `WireError` below is raised
//! through `Fleet::refusal`.
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
use ipc::mcp::{CheckReport, DeclareScope, DispatchJob, NotRecorded, Receipt, SubmitEvidence};
use ipc::{
    ChangesRequested, FleetCapacity, JobDelivery, JobDetail, JobDiff, JobEvidence, JobForgotten,
    JobHistory, JobId, JobList, JobSpend, JobSummary, ManifestId, ManifestSummary, ModelChoices,
    Overruled, ProposeJob, Redirection, Redispatched, Work, WorkflowId, WorkflowSummary,
};
use store::LoadJobError;

use crate::admitting::clear_to_run;
use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::footprint::kept;
use crate::sub_dispatch::waiting_on_children;
// The wire's `Redirection` is a struct with a public field; Fleet's is a
// newtype that cannot hold an empty instruction. Both names are in scope here,
// which is the one place they meet.
use crate::overruling::Overruling;
use crate::reporting::Filed;
use crate::resume::Redirection as Instruction;
use crate::wire::{
    canonical, declared, recorded, reported, step_facts, step_moves, submitted, told,
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

    /// How full the fleet is, and the one thing holding the next Drone back.
    ///
    /// **The same predicate again, unreduced.** `admit_next` opens with
    /// `room_for_another` and `queued_reason` folds it to one label; this is
    /// the third reader of that one answer and it takes it whole. Nothing here
    /// computes a second opinion about why a Job is waiting — `Room::hold` is a
    /// `match` over the value admission itself returned.
    ///
    /// **`occupied` is `Slots::count`.** The roster is what the bound is
    /// measured against, and a count taken from Job statuses would disagree
    /// with it: an escalated Job keeps its Drone alive and idle so a redirect
    /// costs no respawn, and it keeps its place. `count` sweeps the slots whose
    /// `Working` has gone, so what it answers is what admission will act on.
    ///
    /// The roster lock is taken once and both facts are read under it, so the
    /// bound, the count and the reason cannot be three readings of three
    /// different instants. It is admission's own lock order — roster first,
    /// then the poll lock inside `room_for_another` — so this adds no cycle.
    async fn get_capacity(&self) -> Result<FleetCapacity, Refusal> {
        let mut slots = self.slots().lock().await;
        let room = self.room_for_another(&mut slots).await;
        Ok(FleetCapacity::of(slots.cap(), slots.count(), room.hold()))
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
        let (ran, judged, flagged, moves) = {
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
            // The rows `get_job_events` serves, narrowed to the step moves.
            // **Read on every open, unlike the history**: one entry per run of
            // a step rather than a row per move, so a rail can say `Attempt 1
            // refused` without the unbounded read `history.rs` keeps off this.
            let moves =
                step_moves(&store, job.id()).map_err(|why| self.refusal(Adrift::Reading(why)))?;
            (ran, judged, flagged, moves)
        };
        // The plans are read with the footprint and only with it: they are what
        // it is measured against, and a running Job has neither — its live
        // reading is marked from the slot, where the step being watched is the
        // step that declared.
        let recorded = match job.status().is_terminal() {
            false => None,
            true => {
                let store = self.store().lock().await;
                let kept = store
                    .footprint(job.id())
                    .map_err(|why| self.refusal(Adrift::Reading(why)))?;
                let plans = match kept.is_some() {
                    false => Vec::new(),
                    true => store
                        .step_plans(job.id())
                        .map_err(|why| self.refusal(Adrift::Reading(why)))?,
                };
                kept.map(|footprint| (footprint, plans))
            }
        };
        // Read on the same terms as the footprint and for the same reason: a
        // Job that has not finished has nothing here, and a read spent on every
        // running Job would buy three nulls.
        let delivery = match job.status().is_terminal() {
            false => None,
            true => {
                let came_to = self
                    .store()
                    .lock()
                    .await
                    .delivery_for(job.id())
                    .map_err(|why| self.refusal(Adrift::Reading(why)))?;
                // Absent rather than three nulls: a Job that finished before
                // Fleet wrote this down is not a Job whose branch came to
                // nothing, and the surface says different sentences for the two.
                match came_to.is_empty() {
                    true => None,
                    false => Some(JobDelivery {
                        commit: came_to.commit,
                        pushed: came_to.pushed,
                        pull_request: came_to.pull_request,
                    }),
                }
            }
        };
        let queued = self.queued_reason(&job).await?;
        // **Read for every Job, unlike the footprint and the delivery above.**
        // Those two are absent until a Job finishes; this one is what a person
        // watching a running Job wants most, and it is one indexed query. The
        // cap travels with the figure because neither half is readable alone.
        let allowance = self.allowance();
        let spent = self
            .spend_of(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let spend = Some(JobSpend {
            cost_micros: spent.cost_micros,
            cost_cap_micros: allowance.cost().count(),
            turns: spent.turns,
            turn_cap: allowance.turns(),
            ran_ms: spent.ran_ms,
            drones: spent.drones,
        });
        // Before `step_facts`, which consumes the Check runs: the
        // classification reads them to answer whether an override is available,
        // and reading them twice would be a second answer to one question.
        let stuck = self.why_stuck(&job, reason.as_ref(), &ran).await;
        // A read, and only ever a read — `crate::overlap` says why it is
        // reachable from here and from nothing on the dispatch path.
        let overlaps = self
            .write_scope_overlaps(&job)
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(JobDetail::of(
            &job,
            reason.as_ref(),
            queued,
            self.resumption(&job),
            &step_facts(self.aloft(), &job, ran, judged, flagged, &moves),
            recorded
                .as_ref()
                .map(|(footprint, plans)| kept(footprint, plans)),
            self.redirect_awaited(job.id()).await,
            self.question_awaited(job.id()).await,
            stuck.as_ref(),
            overlaps,
            delivery,
            spend,
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

    /// The record, gone. **Nothing is redacted here** — there is no Job left
    /// to redact, only the id it used to name.
    async fn forget_job(&self, job_id: JobId) -> Result<JobForgotten, Refusal> {
        Fleet::forget_job(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(JobForgotten { job_id })
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

    /// The working Drone asking a person something it cannot answer from the
    /// repository. Binding and refusals are `Fleet::ask_question`'s, under the
    /// slot lock. **The receipt says taken, never answered**: what a person
    /// chose arrives as a later turn, which is why this does not block — see
    /// `crate::questioning`.
    async fn ask_question(
        &self,
        caller: api::Caller,
        asking: ipc::mcp::AskQuestion,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        Fleet::ask_question(self, &job, asking).await?;
        Ok(Receipt {
            word: "asked".to_string(),
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
    async fn submit_evidence(
        &self,
        caller: api::Caller,
        submission: SubmitEvidence,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        match self.record_evidence(&job, &submission).await {
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
    async fn declare_scope(
        &self,
        caller: api::Caller,
        declaration: DeclareScope,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        match Fleet::declare_scope(self, &job, &declaration).await {
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
    async fn run_checks(&self, caller: api::Caller) -> Result<CheckReport, NotRecorded> {
        let job = self.placed(&caller)?;
        Fleet::run_checks(self, &job)
            .await
            .map_err(|why| NotRecorded {
                because: why.to_string(),
            })
    }

    /// The Drone asking for one more Job to exist.
    ///
    /// The same shape as the three above: it places the caller, converts, and
    /// decides nothing. **What is different is what a success is** — the other
    /// three answer about the Job the call was made on, and this one answers
    /// with the id of a record that did not exist a moment ago.
    ///
    /// `crate::sub_dispatch` holds whether the caller was allowed to ask, and
    /// the refusal reaches the Drone as a tool error it can read rather than a
    /// status code it can only retry.
    async fn dispatch_job(
        &self,
        caller: api::Caller,
        dispatch: DispatchJob,
    ) -> Result<Receipt, NotRecorded> {
        let job = self.placed(&caller)?;
        match Fleet::sub_dispatch(self, &job, &dispatch).await {
            // The minted id, and nothing else. A Drone needs it to name this
            // Job in a later call's `after`, and it needs nothing else — the
            // Job's state is not knowable yet and a receipt implying it were
            // would be the verdict `Receipt` exists to have no room for.
            Ok(minted) => Ok(Receipt {
                word: minted.as_str().to_string(),
            }),
            Err(why) => Err(NotRecorded {
                because: why.to_string(),
            }),
        }
    }
}

// `canonical`, `declared_check`, `declared`, `recorded`, `submitted`,
// `reported` and `told` moved to `crate::wire`, and `refusal`, `unreadable`
// and `run_id` to `crate::refusing`, so this file stays the trait impl rather
// than the trait impl plus its helpers.

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
    /// Which Job made this call, as the tool's own refusal.
    ///
    /// **Every one of the three opens with this**, and none of them takes a Job
    /// id: `Fleet::caller_of` reads the connection, and a caller it cannot place
    /// is told so as a tool error the Drone can read rather than as a 4xx it can
    /// only retry.
    fn placed(&self, caller: &api::Caller) -> Result<core_model::JobId, NotRecorded> {
        self.caller_of(caller).map_err(|why| NotRecorded {
            because: why.to_string(),
        })
    }

    /// A Job as a Board row, with the reason its last transition stored and
    /// whether its Drone is waiting on somebody. **The slot read is free on
    /// every row but the working ones** — `question_awaited` answers `None`
    /// without touching the store for a Job that holds no slot.
    async fn summarised(&self, job: &Job) -> Result<JobSummary, Refusal> {
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let queued = self.queued_reason(job).await?;
        let asking = self.question_awaited(job.id()).await.is_some();
        Ok(JobSummary::of(
            job,
            reason.as_ref(),
            queued,
            asking,
            self.resumption(job),
        ))
    }

    /// Why an approved Job has not started, worked out from the board as it
    /// stands rather than read from anything.
    ///
    /// **Nothing is read for a Job that is not `queued`**, which is every row
    /// but the waiting ones — the board read is the cost, and a status that
    /// cannot have this reason must not pay it.
    ///
    /// The dependency half is [`clear_to_run`], the budget half is
    /// `Fleet::overspent` and the resource half is `Fleet::room_for_another` —
    /// all three the predicates admission itself uses, asked in the order
    /// admission asks them. A second answer here is how a Board comes to say a
    /// Job is blocked while Fleet is starting it.
    ///
    /// **`None` is the registry's `none`** — approved, unblocked, inside its
    /// budget, and there is room. **Nothing is stored**: headroom frees on its
    /// own, so a reason written down is wrong from the moment it is. The budget
    /// half does not free on its own and is still computed here, because what
    /// changes it is a person raising the cap and a stored label would survive
    /// that.
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
        // **The same label as the edge above, because it is the same fact from
        // the Board's side.** The registry gives a `queued` Job no reason
        // meaning "waiting on the Jobs it created"; a Job held for its children
        // is held for work that has to finish first, which is what the
        // dependency label says. The mechanism differs — provenance rather than
        // an edge — and that difference is not a Board fact.
        if waiting_on_children(job, &crate::sub_dispatch::children_standing(&loaded.jobs)) {
            return Ok(Some(CoreQueuedReason::BlockedByDependency));
        }
        // **Before the machine reading, and not only because admission asks it
        // first.** Headroom frees on its own and a spent budget does not, so a
        // Job that is both would be told it is waiting for something that is
        // already on its way when the thing actually holding it needs a person.
        // The dollars and the turns fold to this one label; which of the two it
        // was is on the Job's detail, where the figures are.
        if self
            .overspent(job)
            .await
            .map_err(|why| self.refusal(why))?
            .is_some()
        {
            return Ok(Some(CoreQueuedReason::OverBudget));
        }
        // **The same predicate admission opens with**, asked of the same
        // roster. The bound and each of the three machine readings fold to the
        // one label the registry gives a `queued` Job short of anything; which
        // of them it was is not a Board fact.
        let mut slots = self.slots().lock().await;
        let room = self.room_for_another(&mut slots).await;
        Ok((!room.granted()).then_some(CoreQueuedReason::WaitingOnResources))
    }
}
