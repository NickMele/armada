//! What the fake answers a read with.
//!
//! **The `Queries` block, in its own file because the trait is three traits.**
//! `api::Daemon` stated every surface at once until #434, so one implementation
//! meant one impl block and one file; the fake was 856 lines of it. Nothing
//! here is new — it is the reads, moved.
//!
//! Every fixed value is [`shapes`](crate::tests::shapes)'s, still built out of
//! `ipc` alone.

use ipc::{
    AlertList, CallArguments, CheckOutput, CommandExplained, DroneDetail, DroneId, DroneList,
    FilesFound, FleetCapacity, FleetHealth, FleetUsage, JobDetail, JobDiff, JobEvidence,
    JobHistory, JobId, JobList, JobRemarks, JobResources, KeptFrame, ManifestConfig, ManifestDrift,
    ManifestFile, ManifestId, ManifestReading, ManifestSummary, ModelChoices, WorkflowSummary,
    WorktreesHeld,
};

use super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::run_id;
use crate::{FramePart, FrameSpan, Observed, Queries, Refusal, Resolved};

impl FakeDaemon {
    /// The row a frame id resolves to, and the bytes behind it.
    ///
    /// **The record is the allowlist here too.** Both frame routes go through
    /// this, so a name no row of this Job holds reaches nothing on either —
    /// which is the rule a ranged read must not be able to route around.
    fn frame_held(
        &self,
        job_id: &JobId,
        kept: String,
    ) -> Result<(KeptFrame, &'static [u8]), Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| &job.id == job_id) {
            return Err(self.no_such_job(job_id));
        }
        if kept != shapes::THE_FRAME && kept != shapes::THE_RECORDING {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_frame",
                format!("no step of this Job kept a frame named `{kept}`"),
                run_id(),
            )));
        }
        let bytes = shapes::bytes_of(&kept);
        Ok((shapes::frame(kept), bytes))
    }

    /// The run sheet's six operations, which the fake does not run: a 404 for
    /// a Job it does not hold, and a 422 naming that for one it does. What the
    /// routes prove is that each is wired, and `fleet` proves what they do.
    pub(super) fn runs_nothing<T>(&self, job_id: &JobId) -> Result<T, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| &job.id == job_id) {
            return Err(self.no_such_job(job_id));
        }
        self.runs_nothing_here()
    }

    /// The checkout's own, which name no Job to be missing.
    pub(super) fn runs_nothing_here<T>(&self) -> Result<T, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.cannot_run_here",
            "the fake daemon runs nothing in a worktree",
            run_id(),
        )))
    }
}

impl FakeDaemon {
    /// The fake's own rows, kept where their status is one of `wanted`.
    fn narrowed(&self, wanted: &[&str], within: Option<&ManifestId>) -> Result<JobList, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        let jobs = self.jobs.lock().expect("not poisoned");
        Ok(JobList {
            jobs: jobs
                .iter()
                .filter(|job| wanted.contains(&job.status.as_wire()))
                .filter(|job| within.is_none_or(|id| &job.owner_manifest_id == id))
                .cloned()
                .collect(),
            unreadable: self.unreadable.lock().expect("not poisoned").clone(),
        })
    }
}

impl FakeDaemon {
    /// Whether a Job the fake holds belongs to `within`; everything does where
    /// none is named.
    fn owns(&self, within: Option<&ManifestId>, job_id: &JobId) -> bool {
        within.is_none_or(|named| {
            self.jobs
                .lock()
                .expect("not poisoned")
                .iter()
                .any(|job| &job.id == job_id && &job.owner_manifest_id == named)
        })
    }
}

impl Queries for FakeDaemon {
    /// **The three forms, against the summaries the fake is holding.** A real
    /// daemon asks the store; this asks the list, which is the same three
    /// answers and lets a route test drive a handle through the extractor
    /// without a store behind it.
    async fn resolve_job(
        &self,
        named: String,
        within: Option<ManifestId>,
    ) -> Result<Resolved, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let found = jobs
            .iter()
            .filter(|job| {
                within
                    .as_ref()
                    .is_none_or(|id| &job.owner_manifest_id == id)
            })
            .find(|job| {
                job.id.as_str() == named
                    || job.handle == named
                    || job.handle.split('-').next() == Some(named.as_str())
            });
        match found {
            Some(job) => Ok(Resolved::of(job.id.clone(), job.handle.clone())),
            None => Err(self.no_such_job(&JobId::carried(named))),
        }
    }

    async fn list_jobs(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(JobList {
            jobs: self
                .jobs
                .lock()
                .expect("not poisoned")
                .iter()
                .filter(|job| {
                    manifest_id
                        .as_ref()
                        .is_none_or(|id| &job.owner_manifest_id == id)
                })
                .cloned()
                .collect(),
            unreadable: self.unreadable.lock().expect("not poisoned").clone(),
        })
    }

    /// The Manifest named, where the fake holds it; the first where none was.
    async fn scope(&self, named: Option<ManifestId>) -> Result<ManifestId, Refusal> {
        let held = shapes::manifests();
        match named {
            None => Ok(held[0].id.clone()),
            Some(named) if held.iter().any(|one| one.id == named) => Ok(named),
            Some(named) => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_manifest",
                format!("this Fleet serves no Manifest `{}`", named.as_str()),
                run_id(),
            ))),
        }
    }

    /// **The narrowings are the real rule applied to the fake's own rows.** A
    /// fixed list here would let a route pass while the predicate behind it
    /// said something else.
    async fn list_job_board(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        self.narrowed(&["awaiting_approval", "queued"], manifest_id.as_ref())
    }

    async fn list_reviews(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        self.narrowed(
            &["awaiting_review", "awaiting_attestation"],
            manifest_id.as_ref(),
        )
    }

    async fn get_activity_feed(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        self.narrowed(
            &[
                "completed_success",
                "completed_failed",
                "killed",
                "rejected",
                "superseded",
            ],
            manifest_id.as_ref(),
        )
    }

    async fn list_alerts(&self, _manifest_id: Option<ManifestId>) -> Result<AlertList, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::alerts())
    }

    async fn list_drones(&self, manifest_id: Option<ManifestId>) -> Result<DroneList, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(DroneList {
            drones: vec![shapes::drone()]
                .into_iter()
                .filter(|drone| self.owns(manifest_id.as_ref(), &drone.job_id))
                .collect(),
        })
    }

    async fn owned_jobs(&self, manifest_id: ManifestId) -> Result<Vec<JobId>, Refusal> {
        let manifest_id = self.scope(Some(manifest_id)).await?;
        let jobs = self.jobs.lock().expect("not poisoned");
        Ok(jobs
            .iter()
            .filter(|job| job.owner_manifest_id == manifest_id)
            .map(|job| job.id.clone())
            .collect())
    }

    /// **The refusal is what matters**: an id naming no live Drone is a 404,
    /// never a detail with nothing in it.
    async fn get_drone(&self, drone_id: DroneId) -> Result<DroneDetail, Refusal> {
        match drone_id == shapes::drone().drone_id {
            true => Ok(shapes::drone_detail()),
            false => Err(self.no_such_job(&JobId::carried(drone_id.as_str()))),
        }
    }

    async fn get_health(&self) -> Result<FleetHealth, Refusal> {
        Ok(shapes::health())
    }

    async fn get_usage(&self) -> Result<FleetUsage, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::usage())
    }

    async fn get_manifest_spend(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestSpend, Refusal> {
        Ok(ipc::ManifestSpend {
            jobs: 2,
            most_cost_micros: 7_120_000,
            most_turns: 212,
        })
    }

    /// A Manifest this fake does not hold is a 422 and never a 404: the request
    /// is well-formed and names something not in the record.
    async fn get_manifest(&self, manifest_id: ManifestId) -> Result<ManifestConfig, Refusal> {
        match manifest_id == shapes::manifests()[0].id {
            true => Ok(shapes::manifest_config()),
            false => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.unacceptable_proposal",
                "this Fleet holds no such Manifest",
                run_id(),
            ))),
        }
    }

    async fn get_capacity(&self) -> Result<FleetCapacity, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::capacity())
    }

    /// Whatever the fake's own saves have left, so a route test can read back
    /// what it saved.
    async fn get_limits(&self) -> Result<ipc::FleetLimits, Refusal> {
        Ok(*self.limits.lock().expect("not poisoned"))
    }

    /// Whatever the fake's own saves have left, `get_limits`' reason.
    async fn get_preferences(&self) -> Result<ipc::Preferences, Refusal> {
        Ok(*self.preferences.lock().expect("not poisoned"))
    }

    /// Whatever a test planted, unfiltered — `#836`.
    async fn get_repository_allowed_commands(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RepositoryAllowedCommands, Refusal> {
        Ok(ipc::RepositoryAllowedCommands {
            commands: self
                .repository_allowed
                .lock()
                .expect("not poisoned")
                .clone(),
        })
    }

    /// **Always a reading, and always one worth saying.** The fake exists so a
    /// route test has a shape to assert on; a `None` here would make the
    /// ordinary case a test of the empty answer.
    async fn get_manifest_reading(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<Option<ManifestReading>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(Some(shapes::manifest_reading()))
    }

    /// **A file that does not parse**, matching the reading beside it: the two
    /// answers a surface draws together are about one `armada.yml`.
    async fn get_manifest_file(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestFile, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::manifest_file())
    }

    /// **Always two rows, one of each verdict.** A fake answering all `current`
    /// or all `gone` would let a route test pass against a constant.
    async fn get_manifest_drift(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestDrift, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::manifest_drift())
    }

    async fn get_repository_scan(
        &self,
        _repository: Option<String>,
    ) -> Result<ipc::RepositoryScan, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::repository_scan())
    }

    /// **No proposals and a stated cap**: what a proposal holds is
    /// `fleet::manifest_proposal`'s and tested there against a Scan.
    async fn get_manifest_proposals(
        &self,
        _repository: Option<String>,
    ) -> Result<ipc::ManifestProposals, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(ipc::ManifestProposals {
            checkout: "/repo".to_string(),
            proposals: Vec::new(),
            caps: ipc::StatedCaps {
                cost_micros: 5_000_000,
                turns: 200,
            },
        })
    }

    /// **`replaced_by` is read the way Fleet reads it** — every Job's own
    /// `redispatched_from`, as a predicate — so the fake cannot hold a forward
    /// link the record does not support.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        let mut detail = shapes::detail(job.clone());
        detail.replaced_by = jobs
            .iter()
            .rev()
            .find(|other| other.redispatched_from.as_ref() == Some(&job_id))
            .map(|found| ipc::ReplacedBy {
                job_id: found.id.clone(),
                handle: found.handle.clone(),
            });
        Ok(detail)
    }

    /// **The refusal is what matters here**: an id that names nothing is a 404,
    /// never an empty list. What the history says is [`shapes::history`].
    async fn get_job_events(&self, job_id: JobId) -> Result<JobHistory, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        let at = job.status;
        Ok(shapes::history(job_id, at))
    }

    /// **The refusal is what matters here too**: an id naming nothing is a 404
    /// rather than a panel of empty lists, which is the one thing a client
    /// cannot tell apart on its own.
    async fn get_job_resources(&self, job_id: JobId) -> Result<JobResources, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        Ok(shapes::resources(job_id))
    }

    async fn get_evidence(&self, job_id: JobId) -> Result<JobEvidence, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        Ok(shapes::evidence(job_id))
    }

    /// **Two comments, one already spent.** The forge is not reached from here
    /// — what a route test can see is the shape, and `taken_up` is the field
    /// that decides whether a surface offers a comment at all.
    async fn get_remarks(&self, job_id: JobId) -> Result<JobRemarks, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        Ok(shapes::remarks(job_id))
    }

    async fn get_diff(&self, job_id: JobId) -> Result<JobDiff, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        Ok(shapes::diff(job_id))
    }

    /// **The two refusals are what this side holds**: a Job that is not there,
    /// and an id the record does not carry, which are different answers.
    /// [`shapes::THE_CALL`] is the one id it does carry.
    async fn get_call(&self, job_id: JobId, call_id: String) -> Result<CallArguments, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        if call_id != shapes::THE_CALL {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_call",
                format!("nothing in this Job's transcripts is the call `{call_id}`"),
                run_id(),
            )));
        }
        Ok(shapes::call(call_id))
    }

    /// **Two refusals again, and both are 404s here**: a Job that is not there,
    /// and a call this fake holds no open command for. The second is a 404
    /// rather than the 422 `get_call` answers, because the id is only nameable
    /// while the command is open — `crates/ipc/operations.toml`. No model is
    /// reached; what the route has to prove is the shape and the two answers.
    async fn explain_command(
        &self,
        job_id: JobId,
        call_id: String,
    ) -> Result<CommandExplained, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        if call_id != shapes::THE_CALL {
            return Err(Refusal::NoSuchJob(ipc::WireError::raised(
                "fleet.nothing_to_explain",
                format!("this Job is neither waiting on nor refused the call `{call_id}`"),
                run_id(),
            )));
        }
        Ok(shapes::explained())
    }

    /// **The same two refusals one record over**: a Job that is not there, and
    /// a name no row of the Job kept. [`shapes::THE_OUTPUT`] is the one name it
    /// does hold, and nothing here opens a file — what the route has to prove
    /// is the shape and the two answers, not the reading.
    async fn get_check_output(&self, job_id: JobId, kept: String) -> Result<CheckOutput, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        if kept != shapes::THE_OUTPUT {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_check_output",
                format!("no Check of this Job kept an output named `{kept}`"),
                run_id(),
            )));
        }
        Ok(shapes::check_output(kept))
    }

    /// **The planted log, under its own name, and a refusal for any other.**
    /// A route test plants a [`crate::LiveOutput`] whose reader it drives, so
    /// what the socket does with a growing file is proved without a file.
    async fn observe_check_output(
        &self,
        job_id: JobId,
        kept: String,
    ) -> Result<crate::LiveOutput, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        let planted = self.live.lock().expect("not poisoned").clone();
        match planted {
            Some((name, live)) if name == kept => Ok(live),
            _ => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_check_output",
                format!("no running Check of this Job is writing a log named `{kept}`"),
                run_id(),
            ))),
        }
    }

    async fn get_run_sheet(&self, job_id: JobId) -> Result<ipc::RunSheet, Refusal> {
        self.runs_nothing(&job_id)
    }

    async fn list_runs(&self, job_id: JobId) -> Result<ipc::RunList, Refusal> {
        self.runs_nothing(&job_id)
    }

    async fn get_run_output(
        &self,
        job_id: JobId,
        _run_id: String,
    ) -> Result<ipc::RunOutput, Refusal> {
        self.runs_nothing(&job_id)
    }

    async fn observe_run(
        &self,
        job_id: JobId,
        _run_id: String,
    ) -> Result<crate::ObservedRun, Refusal> {
        self.runs_nothing(&job_id)
    }

    /// **The fake declares nothing**, so the sheet is empty rather than
    /// refused: `get_checkout_run_sheet` answers on a repository that has
    /// never had a Job in it, and an empty Manifest is that answer.
    async fn get_checkout_run_sheet(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunSheet, Refusal> {
        Ok(ipc::CheckoutRunSheet {
            setup: Vec::new(),
            checks: Vec::new(),
            commands: Vec::new(),
            manifest_edited_at: None,
            running: None,
            servers: Vec::new(),
            verify: None,
            workspaces: Vec::new(),
            seed: None,
        })
    }

    async fn list_checkout_runs(
        &self,
        manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunList, Refusal> {
        self.checkout_runs_named
            .lock()
            .expect("not poisoned")
            .push(manifest_id);
        Ok(ipc::CheckoutRunList {
            runs: Vec::new(),
            unreadable: Vec::new(),
        })
    }

    async fn get_checkout_run_output(
        &self,
        _run_id: String,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::RunOutput, Refusal> {
        self.runs_nothing_here()
    }

    async fn get_checkout_run_diff(
        &self,
        _run_id: String,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunDiff, Refusal> {
        self.runs_nothing_here()
    }

    async fn observe_checkout_run(
        &self,
        _run_id: String,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<crate::ObservedCheckoutRun, Refusal> {
        self.runs_nothing_here()
    }

    /// **The fake holds no server**, so the list is empty and every id names
    /// nothing. What holding one means is `fleet::servers`', tested there.
    async fn list_servers(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<ipc::ServerList, Refusal> {
        let servers = self.servers.lock().expect("not poisoned").clone();
        Ok(ipc::ServerList {
            servers: servers
                .into_iter()
                .filter(|one| manifest_id.is_none() || one.manifest_id == manifest_id)
                .collect(),
        })
    }

    async fn observe_server(&self, server_id: String) -> Result<crate::ObservedServer, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.no_such_server",
            format!("the fake daemon holds no server called `{server_id}`"),
            run_id(),
        )))
    }

    /// **The same two refusals again**, and nothing here opens a file: what the
    /// route has to prove is that the bytes come back as themselves, under a
    /// media type read off the name, and that a name the record does not hold
    /// reaches nothing.
    async fn get_frame(
        &self,
        job_id: JobId,
        kept: String,
    ) -> Result<(KeptFrame, Vec<u8>), Refusal> {
        let (held, bytes) = self.frame_held(&job_id, kept)?;
        Ok((held, bytes.to_vec()))
    }

    /// **The span is arithmetic here rather than a seek**, because the fake
    /// holds bytes and not files. What the route has to prove is the three
    /// answers — a span, a span past the end, and a name no row holds — and
    /// every one of those is a property of the response.
    async fn get_frame_part(
        &self,
        job_id: JobId,
        kept: String,
        span: FrameSpan,
    ) -> Result<(KeptFrame, FramePart), Refusal> {
        let (held, bytes) = self.frame_held(&job_id, kept)?;
        let total = bytes.len() as u64;
        let (first, last) = match span {
            FrameSpan::From { first, last } => (first, last),
            FrameSpan::Last(back) => (total.saturating_sub(back), None),
        };
        if first >= total {
            return Ok((held, FramePart::Beyond { total }));
        }
        let upto = last.map_or(total - 1, |last| last.min(total - 1));
        Ok((
            held,
            FramePart::Span {
                first,
                bytes: bytes[first as usize..=upto as usize].to_vec(),
                total,
            },
        ))
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::workflows())
    }

    async fn list_left_out_workflows(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<Vec<ipc::LeftOutWorkflow>, Refusal> {
        Ok(Vec::new())
    }

    async fn list_manifests(&self) -> Result<Vec<ManifestSummary>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::manifests())
    }

    async fn list_models(&self) -> Result<ModelChoices, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::models())
    }

    async fn list_worktrees(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<WorktreesHeld, Refusal> {
        let held = self.held.lock().expect("not poisoned").clone();
        Ok(WorktreesHeld {
            worktrees: held
                .into_iter()
                .filter(|one| self.owns(manifest_id.as_ref(), &one.job_id))
                .collect(),
        })
    }

    /// Newest first, and the two counts a fake can honestly answer. It knows
    /// nothing about recorded refusals, which are rows in a store this has
    /// none of, so that count is zero rather than invented.
    async fn list_reports(&self) -> Result<ipc::ReportList, Refusal> {
        let reports = self.reports.lock().expect("not poisoned");
        let disputed = |claim: ipc::Claim| {
            reports
                .iter()
                .filter(|report| report.claim == claim)
                .count() as u32
        };
        Ok(ipc::ReportList {
            calibration: ipc::Calibration {
                refusals_recorded: 0,
                refusals_disputed: disputed(ipc::Claim::WronglyRefused),
                passes_disputed: disputed(ipc::Claim::WronglyPassed),
                reports_filed: reports.len() as u32,
            },
            reports: reports.iter().rev().cloned().collect(),
        })
    }

    /// Subscribe, then read the history. The same order Fleet takes, so a test
    /// of the socket exercises the order rather than assuming it.
    async fn observe_job(&self, job_id: JobId) -> Result<Observed, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        drop(jobs);
        let live = self.turns.watching(&job_id);
        Ok(Observed {
            job_id,
            live,
            history: self.history.lock().expect("not poisoned").clone(),
            skipped: *self.skipped.lock().expect("not poisoned"),
        })
    }

    async fn list_repositories(&self) -> Result<ipc::RepositoryList, Refusal> {
        Ok(ipc::RepositoryList {
            repositories: Vec::new(),
        })
    }

    async fn search_files(
        &self,
        query: String,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<FilesFound, Refusal> {
        Ok(FilesFound {
            paths: shapes::files_found(&query),
        })
    }
}
