//! `api::Queries`, implemented over a real Fleet: every read a client makes.
//!
//! **The dependency points this way and never back.** The traits are stated in
//! `api`, where the transport is, and implemented here. `cargo tree -p api`
//! names no `fleet`, the daemon core is drivable with no socket and no process,
//! and `api`'s own tests were written against a fake before this existed.
//!
//! **This is the redaction, and it is a visible step.** Every signature below
//! speaks `ipc` DTOs, `JobSummary::of` is called here by hand, and a field added
//! to `core_model::Job` reaches the wire only when somebody writes the line that
//! puts it there — `api` never sees a domain type.
//!
//! **One of three files, because `api::Daemon` is three traits** — #434. The
//! writes are [`commanding`](mod@crate::commanding)'s and the tools are
//! [`tooling`](mod@crate::tooling)'s, each whole, so no file delegates.
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

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Observed, Queries, Refusal};
use ipc::{
    CallArguments, FleetCapacity, JobDelivery, JobDetail, JobDiff, JobEvidence, JobHistory, JobId,
    JobList, JobResources, JobSpend, ManifestSummary, ModelChoices, Work, WorkflowSummary,
    WorktreesHeld,
};
use store::LoadJobError;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::footprint::kept;
use crate::wire::{
    manifest_summary, recorded, reported, step_facts, step_moves, submitted, workflow_summary,
    worktree_held,
};

impl<H, V, W> Queries for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
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
        let (ran, flagged, moves, ran_every_attempt, judged_every_attempt) = {
            let store = self.store().lock().await;
            let ran = store
                .step_checks(job.id())
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
            // **Every attempt's rows, beside the latest-only `ran` above.**
            // `ran` stays latest-only because `why_stuck` below reads it as
            // that; `step_facts` wants every run's Checks and Judge answers
            // stamped with the attempt they belong to, which these two give.
            let ran_every_attempt = store
                .step_checks_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let judged_every_attempt = store
                .step_judgments_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            (ran, flagged, moves, ran_every_attempt, judged_every_attempt)
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
                        landed: came_to.landed.as_ref().and_then(crate::noticing::settled),
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
            &step_facts(
                self.aloft(),
                &self.host().repo_root,
                &job,
                ran_every_attempt,
                judged_every_attempt,
                flagged,
                &moves,
            ),
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
    /// What this Job holds on this machine. **The reading `crate::resources`
    /// takes, redacted there** — a process crosses as its executable's name and
    /// never its arguments.
    ///
    /// The Job is loaded first, so an id naming nothing is a 404 rather than a
    /// panel of empty lists.
    async fn get_job_resources(&self, job_id: JobId) -> Result<JobResources, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.job_resources(&job)
            .await
            .map_err(|why| self.refusal(why))
    }

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
}
