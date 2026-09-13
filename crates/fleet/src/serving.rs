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
use api::{FramePart, FrameSpan, Observed, ObservedCheckoutRun, Queries, Refusal, Resolved};
use core_model::JobReference;
use ipc::{
    AlertList, CallArguments, DroneDetail, DroneId, DroneList, FleetCapacity, FleetHealth,
    FleetUsage, JobDetail, JobDiff, JobEvidence, JobHistory, JobId, JobList, JobRemarks,
    JobResources, ManifestConfig, ManifestDrift, ManifestFile, ManifestId, ManifestReading,
    ManifestSummary, ModelChoices, Work, WorkflowSummary, WorktreesHeld,
};
use store::{LoadJobError, ResolveJobError};

/// One Job in full. **Its own file**, because this one crossed the 900-line
/// rule and that read is a quarter of it.
mod detail;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::wire::{
    manifest_summary, recorded, redacted, reported, submitted, workflow_summary, worktree_held,
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
    /// What a caller said, turned into the Job they meant.
    ///
    /// **Three reads and not one, deliberately.** `store::Store::resolve_job`
    /// answers with the id and the handle together, so nothing downstream loads
    /// the Job a second time to derive a path from it.
    ///
    /// **A number counts within a Manifest**, and Fleet serves several: one
    /// that more than one of them holds is refused rather than guessed.
    async fn resolve_job(&self, named: String) -> Result<Resolved, Refusal> {
        let Some(reference) = JobReference::read(&named) else {
            return Err(self.refusal(Adrift::Unresolvable(ResolveJobError::NoSuchJob { named })));
        };
        let found = self
            .resolve_job_across(&reference, &self.repositories().served())
            .await
            .map_err(|why| self.refusal(Adrift::Unresolvable(why)))?;
        Ok(Resolved::of(JobId::from(&found.job_id), found.handle))
    }

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

    /// The Manifest every door answer is given inside — `#698` is what lets a
    /// caller name one.
    async fn scope(&self) -> Result<ManifestId, Refusal> {
        // The agent door answers inside the repository Fleet was started in.
        Ok(ManifestId::from(
            self.repositories().first().manifest().id(),
        ))
    }

    /// The four narrowings, each one rule, stated in `crate::attention`.
    async fn list_job_board(&self) -> Result<JobList, Refusal> {
        self.job_board().await
    }

    async fn list_reviews(&self) -> Result<JobList, Refusal> {
        self.reviews().await
    }

    async fn get_activity_feed(&self) -> Result<JobList, Refusal> {
        self.activity_feed().await
    }

    async fn list_alerts(&self) -> Result<AlertList, Refusal> {
        self.alerts().await
    }

    /// The roster, read without taking a working slot — `crate::rostered`.
    async fn list_drones(&self) -> Result<DroneList, Refusal> {
        self.drone_list().await
    }

    async fn get_drone(&self, drone_id: DroneId) -> Result<DroneDetail, Refusal> {
        self.drone_detail(drone_id).await
    }

    /// The probes Fleet can run on itself — `crate::probing`.
    async fn get_health(&self) -> Result<FleetHealth, Refusal> {
        self.health().await
    }

    /// The spend, and what the ceilings hold — `crate::spending`.
    async fn get_usage(&self) -> Result<FleetUsage, Refusal> {
        self.usage().await
    }

    /// The costliest and the longest Job against this Manifest — the budget
    /// form's warning, read beside the caps it is set against.
    async fn get_manifest_spend(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestSpend, Refusal> {
        let past = self
            .store()
            .lock()
            .await
            .past_spend_for(self.served_named(manifest_id.as_ref())?.manifest().id())
            .map_err(|why| self.refusal(crate::adrift::Adrift::Reading(why)))?;
        Ok(ipc::ManifestSpend {
            jobs: past.jobs,
            most_cost_micros: past.most_cost_micros,
            most_turns: past.most_turns,
        })
    }

    /// What this repository declares — `crate::configured`.
    async fn get_manifest(&self, manifest_id: ManifestId) -> Result<ManifestConfig, Refusal> {
        self.manifest_config(manifest_id)
    }

    /// How full the fleet is, and the one thing holding the next Drone back.
    ///
    /// **The same predicate again, unreduced.** `admit_next` opens with
    /// `room_for_another` and `queued_reason` folds it to one label; this is
    /// the third reader, and takes the whole answer — `Room::hold` is a
    /// `match` over the value admission itself returned.
    ///
    /// **`occupied` is `Slots::count`.** A count taken from Job statuses would
    /// disagree with the roster: an escalated Job keeps its Drone alive and
    /// idle, keeping its place. `count` sweeps the slots whose `Working` has
    /// gone, so it answers what admission will act on.
    ///
    /// The roster lock is taken once, so the bound, count and reason cannot be
    /// three readings of three different instants — admission's own lock
    /// order, roster first, so this adds no cycle.
    async fn get_capacity(&self) -> Result<FleetCapacity, Refusal> {
        let mut slots = self.slots().lock().await;
        let room = self.room_for_another(&mut slots).await;
        Ok(FleetCapacity::of(slots.cap(), slots.count(), room.hold()))
    }

    /// The limits in force and what shipped — [`crate::limits`].
    async fn get_limits(&self) -> Result<ipc::FleetLimits, Refusal> {
        Ok(self.limits_in_force().await)
    }

    /// Every rule a person always-allowed for this Manifest's repository —
    /// `crate::permitting::repository`.
    async fn get_repository_allowed_commands(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RepositoryAllowedCommands, Refusal> {
        Ok(ipc::RepositoryAllowedCommands {
            commands: self
                .repository_allowed(&self.served_named(manifest_id.as_ref())?)
                .await
                .iter()
                .map(ipc::AllowedCommandRow::from)
                .collect(),
        })
    }

    /// What the last re-read of `armada.yml` came to, straight off what Fleet
    /// is holding.
    ///
    /// **No lock but its own and no work at all.** The reading was assembled
    /// when the file was read; this hands back the copy. `None` is a Fleet that
    /// has not re-read the file since it started with it, which is a real
    /// answer rather than a gap — the configuration in force is then the one it
    /// booted on.
    async fn get_manifest_reading(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<Option<ManifestReading>, Refusal> {
        Ok(self
            .served_named(manifest_id.as_ref())?
            .repository()
            .reading())
    }

    /// `armada.yml` as it is on disk, for the view that edits it —
    /// [`editing`](mod@crate::editing).
    ///
    /// **Unparsed, and whole.** A file that does not parse is the case a
    /// person opening this is most likely to be in, and what the parse came to
    /// is `get_manifest_reading` beside it.
    async fn get_manifest_file(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestFile, Refusal> {
        self.read_manifest_file(&self.served_named(manifest_id.as_ref())?)
    }

    /// Whether the repository still has what `armada.yml` names —
    /// `crate::drifting`, which carries the whole argument.
    ///
    /// **Against the main checkout, not a Job's worktree**, and it takes no
    /// lock and starts no process.
    async fn get_manifest_drift(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestDrift, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let checkout = std::path::Path::new(served.root());
        Ok(crate::drifting::drift(served.manifest(), checkout))
    }

    /// What Scan finds in the checkout Fleet was started in —
    /// [`scanning`](mod@crate::scanning). **It never reads `self.manifest()`**:
    /// Scan is for a repository with none, and the one Fleet holds is beside
    /// the point until Locate (#821) names another checkout.
    async fn get_repository_scan(
        &self,
        repository: Option<String>,
    ) -> Result<ipc::RepositoryScan, Refusal> {
        let repository = self.repository_named(repository.as_deref())?;
        let root = repository.root();
        let tree = crate::scanning::Checkout::at(root);
        Ok(crate::scanning::scan(
            root,
            &tree,
            &**self.ci_configuration(),
        ))
    }

    /// A proposal per workspace — `crate::manifest_proposal`, which holds
    /// them between calls.
    async fn get_manifest_proposals(
        &self,
        repository: Option<String>,
    ) -> Result<ipc::ManifestProposals, Refusal> {
        Ok(self.manifest_proposals(self.repository_named(repository.as_deref())?.as_ref()))
    }

    /// One Job in full — [`detail`](mod@detail), which is a quarter of this
    /// file's length and the read made on every open of a Job.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        self.job_detail(job_id).await
    }

    /// Every move one Job made, oldest first. **The log, read — not folded.**
    ///
    /// # The Job is loaded first, and that is not a wasted read
    ///
    /// It is what makes an id that names nothing a 404 rather than an empty
    /// history — a lie about a Job that exists and has not moved — and it
    /// keeps this read behind the same fold every other read is behind: a log
    /// the machine would not admit refuses to load, so a history that reaches
    /// the wire is one `Job::transition` accepted. **This read cannot show a
    /// state the fold rejected**, and does not replay to avoid it —
    /// `crates/store/src/fold.rs` is still the only caller.
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

    /// What anybody has written on one Job's open pull request.
    ///
    /// **The redaction is that a `Remark` becomes plain strings, an address and
    /// a number.** `adapter_traits::FromOutside` is what keeps text somebody
    /// outside this machine wrote from being swept into a sentence by accident,
    /// and JSON has no shape for it — so
    /// [`as_written`](adapter_traits::FromOutside::as_written) is called by
    /// hand here, which is exactly the visible step this file exists to make
    /// people take. What crosses is what was written, uncleaned;
    /// `crates/ipc/src/remarks.rs` says where the guard is on the far side.
    ///
    /// `taken_up` is the one field a forge did not write, and it is the reason
    /// the record is read beside the reading.
    async fn get_remarks(&self, job_id: JobId) -> Result<JobRemarks, Refusal> {
        let said = self
            .what_was_said(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(JobRemarks {
            job_id,
            pull_request: said.pull_request,
            remarks: said
                .remarks
                .iter()
                .map(|remark| redacted(remark, &said.taken_up))
                .collect(),
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
        // Which ref the two readings above were taken against. **Carried rather
        // than assumed**: a patch measured from the branch's own tip is the
        // uncommitted remainder and not the Job's work, and until this field
        // existed the sheet said "the Job's patch" over either one.
        let measured = self.work().measured(&worktree);
        Ok(JobDiff {
            job_id,
            work: Some(Work {
                files: crate::footprint::seen(&changed, plan.as_ref()),
                measured_from: measured.from,
                measured_whole: measured.whole,
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
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let served = self.served_by(&job).map_err(|why| self.refusal(why))?;
        crate::transcript::arguments(served.records_root(), &job.handle(), &call_id)
            .await
            .ok_or_else(|| self.refusal(Adrift::NoSuchCall { named: call_id }))
    }

    /// What one command a Drone was not granted does, read by a model.
    ///
    /// **The one read on this trait that spends money**, and it still moves
    /// nothing: `crate::explaining` makes the call, resolves the call id
    /// against the two places a still-answerable command lives, and names every
    /// refusal it can answer with. This only carries one out.
    async fn explain_command(
        &self,
        job_id: JobId,
        call_id: String,
    ) -> Result<ipc::CommandExplained, Refusal> {
        self.explained(&job_id.to_domain(), &call_id)
            .await
            .map_err(|why| self.refusal(why))
    }

    /// One Check's own output, read back out of the file its row points at.
    ///
    /// **The other end of a path that opened nothing in Armada.** A row has
    /// carried `output_path` since the record kept it, and Bridge could only
    /// hand that path to the operating system — so the audit the Checks chapter
    /// exists for happened outside the app. This is the read that brings it in.
    ///
    /// The Job is loaded first, so an id naming nothing is a 404 before any
    /// file is opened. A name no row of the Job kept is a different answer and
    /// says so: [`Adrift::NoSuchCheckOutput`], a 422 — the Job is there, and
    /// the id names nothing in its record.
    ///
    /// **The rows are the allowlist**, which is why they are read before the
    /// file: `kept` is a row's own file name and `check_output::kept_output`
    /// resolves it against them, so a caller cannot name a file Fleet did not
    /// write for this Job.
    async fn get_check_output(
        &self,
        job_id: JobId,
        kept: String,
    ) -> Result<ipc::CheckOutput, Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        let ran = {
            let store = self.store().lock().await;
            store
                .step_checks_every_attempt(&id)
                .map_err(|why| self.refusal(Adrift::Reading(why)))?
        };
        crate::check_output::kept_output(
            self.served_by_id(&id)
                .map_err(|why| self.refusal(why))?
                .records_root(),
            &kept,
            &ran,
        )
        .ok_or_else(|| self.refusal(Adrift::NoSuchCheckOutput { named: kept }))
    }

    /// One running Check's log, as it is written.
    ///
    /// **The live set is the allowlist**, which is `get_check_output`'s rule
    /// with the Checks the gate is running standing in for the rows: `kept`
    /// resolves through `Underway::log` or not at all, so a caller cannot name
    /// a file no running gate of this Job wrote.
    async fn observe_check_output(
        &self,
        job_id: JobId,
        kept: String,
    ) -> Result<api::LiveOutput, Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        let Some(log) = self.underway().log(&job_id, &kept) else {
            return Err(self.refusal(Adrift::NoSuchCheckOutput { named: kept }));
        };
        Ok(api::LiveOutput {
            name: log.name,
            attempt: log.attempt,
            path: log.path,
            follow: std::sync::Arc::new(crate::following::LiveFollow {
                file: log.file,
                job: job_id,
                kept,
                underway: self.underway().clone(),
            }),
        })
    }

    /// One frame a step's harness produced, as the file itself.
    ///
    /// **The rows are the allowlist**, which is `get_check_output`'s rule and
    /// the whole of what makes a caller-supplied name safe to open a file with:
    /// `showing::frame_bytes` resolves the name against the frames this Job
    /// kept before it opens anything, so a name no row holds reaches no file,
    /// whatever it spells.
    ///
    /// A row whose file will not open is the same answer as a name that was
    /// never one — [`Adrift::NoSuchFrame`], a 422. The Job is there and the
    /// image is not, which is a reclaimed `.armada/frames` and is a different
    /// thing from the Job being absent.
    async fn get_frame(
        &self,
        job_id: JobId,
        kept: String,
    ) -> Result<(ipc::KeptFrame, Vec<u8>), Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        // Every frame the record holds, including a person's presses — the
        // second list the allowlist is. `showing::frames_held` composes it.
        let frames = self.frames_held(&id).await?;
        let (held, bytes) = crate::showing::frame_bytes(
            self.served_by_id(&id)
                .map_err(|why| self.refusal(why))?
                .records_root(),
            &kept,
            &frames,
        )
        .ok_or_else(|| self.refusal(Adrift::NoSuchFrame { named: kept }))?;
        Ok((crate::showing::as_wire(&held), bytes))
    }

    /// One span of a recording, resolved against the same record.
    ///
    /// **[`Self::get_frame`]'s allowlist and a seek instead of a read.** The
    /// name is resolved before a file is opened, exactly as the whole read
    /// does; what changes is that a two-minute capture costs the window rather
    /// than its length. A span past the end of a file that is there is not a
    /// refusal — the caller answers it as a 416.
    async fn get_frame_part(
        &self,
        job_id: JobId,
        kept: String,
        span: FrameSpan,
    ) -> Result<(ipc::KeptFrame, FramePart), Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        let frames = self.frames_held(&id).await?;
        let (held, part) = crate::showing::frame_part(
            self.served_by_id(&id)
                .map_err(|why| self.refusal(why))?
                .records_root(),
            &kept,
            &frames,
            span,
        )
        .ok_or_else(|| self.refusal(Adrift::NoSuchFrame { named: kept }))?;
        Ok((crate::showing::as_wire(&held), part))
    }

    /// The run sheet and what its runs left — `crate::rehearsing`.
    async fn get_run_sheet(&self, job_id: JobId) -> Result<ipc::RunSheet, Refusal> {
        self.run_sheet(&job_id.to_domain()).await
    }

    async fn list_runs(&self, job_id: JobId) -> Result<ipc::RunList, Refusal> {
        self.rehearsal_history(&job_id.to_domain()).await
    }

    async fn get_run_output(
        &self,
        job_id: JobId,
        run_id: String,
    ) -> Result<ipc::RunOutput, Refusal> {
        self.rehearsal_output(&job_id.to_domain(), run_id).await
    }

    async fn observe_run(
        &self,
        job_id: JobId,
        run_id: String,
    ) -> Result<api::ObservedRun, Refusal> {
        self.observe_rehearsal(&job_id.to_domain(), run_id).await
    }

    /// The Manifest surface's five reads — `crate::rehearsing::checkout`.
    async fn get_checkout_run_sheet(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::CheckoutRunSheet, Refusal> {
        self.checkout_run_sheet(self.served_named(manifest_id.as_ref())?)
            .await
    }

    async fn list_checkout_runs(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::CheckoutRunList, Refusal> {
        self.checkout_rehearsal_history(self.served_named(manifest_id.as_ref())?)
            .await
    }

    async fn get_checkout_run_output(
        &self,
        run_id: String,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RunOutput, Refusal> {
        self.checkout_rehearsal_output(run_id, self.served_named(manifest_id.as_ref())?)
            .await
    }

    async fn get_checkout_run_diff(
        &self,
        run_id: String,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::CheckoutRunDiff, Refusal> {
        self.checkout_rehearsal_diff(run_id, self.served_named(manifest_id.as_ref())?)
            .await
    }

    async fn observe_checkout_run(
        &self,
        run_id: String,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ObservedCheckoutRun, Refusal> {
        self.observe_checkout_rehearsal(run_id, self.served_named(manifest_id.as_ref())?)
            .await
    }

    /// Every server Fleet holds — `crate::servers`.
    async fn list_servers(&self) -> Result<ipc::ServerList, Refusal> {
        Ok(self.server_list())
    }

    async fn observe_server(&self, server_id: String) -> Result<api::ObservedServer, Refusal> {
        self.observed_server(&server_id)
            .map_err(|why| self.server_refusal(why, None))
    }

    /// Every workflow this Fleet holds, so a caller can name one that will not
    /// be refused.
    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        let served = self.repositories().served();
        Ok(served
            .iter()
            .flat_map(|one| {
                one.workflows()
                    .values()
                    .map(|workflow| workflow_summary(workflow, one.manifest().id()))
            })
            .collect())
    }

    /// The Kit and carried definitions this Fleet runs without, each with why.
    async fn list_left_out_workflows(&self) -> Result<Vec<ipc::LeftOutWorkflow>, Refusal> {
        Ok(self.left_out().to_vec())
    }

    /// Every Manifest this Fleet serves, the one it was started in first.
    async fn list_manifests(&self) -> Result<Vec<ManifestSummary>, Refusal> {
        let served = self.repositories().served();
        Ok(served
            .iter()
            .map(|one| manifest_summary(one.manifest(), one.records_root()))
            .collect())
    }

    /// Every repository served, Manifest or none — `crate::repositories`.
    async fn list_repositories(&self) -> Result<ipc::RepositoryList, Refusal> {
        Ok(self.repository_list())
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
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let live = self.turns().watching(&job_id);
        let (history, skipped) = crate::transcript::history(
            self.served_by(&job)
                .map_err(|why| self.refusal(why))?
                .records_root(),
            &job.handle(),
        )
        .await;
        Ok(Observed {
            job_id,
            live,
            history,
            skipped,
        })
    }

    /// Paths under the checkout narrowed against typed text, for Bridge's `@`
    /// mention popup. `crate::files::search` is the walk; this only names the
    /// root it walks and cannot refuse.
    async fn search_files(
        &self,
        query: String,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::FilesFound, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let root = std::path::Path::new(served.root());
        Ok(ipc::FilesFound {
            paths: crate::files::search(root, &query),
        })
    }
}
