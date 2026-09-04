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
    CallArguments, FleetCapacity, JobDetail, JobDiff, JobEvidence, JobHistory, JobId, JobList,
    JobResources, ManifestReading, ManifestSummary, ModelChoices, WorkflowSummary, WorktreesHeld,
};

use super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::run_id;
use crate::{Observed, Queries, Refusal};

impl Queries for FakeDaemon {
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
}
