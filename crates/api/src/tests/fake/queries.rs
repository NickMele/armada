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
    CallArguments, CheckOutput, FilesFound, FleetCapacity, JobDetail, JobDiff, JobEvidence,
    JobHistory, JobId, JobList, JobRemarks, JobResources, KeptFrame, ManifestReading,
    ManifestSummary, ModelChoices, WorkflowSummary, WorktreesHeld,
};

use super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::run_id;
use crate::{Observed, Queries, Refusal, Resolved};

impl FakeDaemon {
    /// The run sheet's six operations, which the fake does not run: a 404 for
    /// a Job it does not hold, and a 422 naming that for one it does. What the
    /// routes prove is that each is wired, and `fleet` proves what they do.
    pub(super) fn runs_nothing<T>(&self, job_id: &JobId) -> Result<T, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| &job.id == job_id) {
            return Err(self.no_such_job(job_id));
        }
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.cannot_run_here",
            "the fake daemon runs nothing in a worktree",
            run_id(),
        )))
    }
}

impl Queries for FakeDaemon {
    /// **The three forms, against the summaries the fake is holding.** A real
    /// daemon asks the store; this asks the list, which is the same three
    /// answers and lets a route test drive a handle through the extractor
    /// without a store behind it.
    async fn resolve_job(&self, named: String) -> Result<Resolved, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let found = jobs.iter().find(|job| {
            job.id.as_str() == named
                || job.handle == named
                || job.handle.split('-').next() == Some(named.as_str())
        });
        match found {
            Some(job) => Ok(Resolved::of(job.id.clone(), job.handle.clone())),
            None => Err(self.no_such_job(&JobId::carried(named))),
        }
    }

    async fn list_jobs(&self) -> Result<JobList, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(JobList {
            jobs: self.jobs.lock().expect("not poisoned").clone(),
            unreadable: self.unreadable.lock().expect("not poisoned").clone(),
        })
    }

    async fn get_capacity(&self) -> Result<FleetCapacity, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::capacity())
    }

    /// **Always a reading, and always one worth saying.** The fake exists so a
    /// route test has a shape to assert on; a `None` here would make the
    /// ordinary case a test of the empty answer.
    async fn get_manifest_reading(&self) -> Result<Option<ManifestReading>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(Some(shapes::manifest_reading()))
    }

    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        Ok(shapes::detail(job.clone()))
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

    /// **The fake holds no server**, so the list is empty and every id names
    /// nothing. What holding one means is `fleet::servers`', tested there.
    async fn list_servers(&self) -> Result<ipc::ServerList, Refusal> {
        Ok(ipc::ServerList {
            servers: Vec::new(),
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
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        if kept != shapes::THE_FRAME {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_frame",
                format!("no step of this Job kept a frame named `{kept}`"),
                run_id(),
            )));
        }
        Ok((shapes::frame(kept), shapes::THE_FRAME_BYTES.to_vec()))
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::workflows())
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

    async fn list_worktrees(&self) -> Result<WorktreesHeld, Refusal> {
        Ok(WorktreesHeld {
            worktrees: self.held.lock().expect("not poisoned").clone(),
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

    async fn search_files(&self, query: String) -> Result<FilesFound, Refusal> {
        Ok(FilesFound {
            paths: shapes::files_found(&query),
        })
    }
}
