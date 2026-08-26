//! `api::Daemon`, implemented over a real Fleet.
//!
//! # The dependency points this way and never back
//!
//! The trait is stated in `api`, where the transport is, and implemented here,
//! where the Jobs are. `cargo tree -p api` names no `fleet`, which is what
//! keeps the daemon core drivable in a test with no socket, no port and no
//! process — and what let `api`'s own tests be written against a fake before
//! this existed.
//!
//! # This is the redaction, and it is a visible step
//!
//! Every signature below speaks `ipc` DTOs. `JobSummary::of` is called here,
//! by hand, and a field added to `core_model::Job` reaches the wire only when
//! somebody writes the line that puts it there. `api` never sees a domain type.
//!
//! # Where the refusals come from
//!
//! [`Refusal`] has four variants because a caller has four different things to
//! do about them, and every mapping below is decided by which *typed* leaf
//! error came back rather than by reading a message. `Adrift::IllegalMove` is a
//! 409 because the machine refused; a store fault is a 500 because it was not
//! the caller's doing; a Job that is not there is a 404; and a proposal naming
//! a workflow, a Manifest or a model that cannot work is a 422, because the
//! request is well-formed and the values in it are not. Nothing here parses
//! prose to decide.
//!
//! # The reason costs a second read, and is not derived
//!
//! `JobSummary` carries the qualifying reason its last transition stored, and
//! the `jobs` row does not have it — `job_events` does. So each summary below
//! reads the Job's log for it. That is N reads for N Jobs and it is the honest
//! shape at M1: the alternative is a status-to-reason mapping in this file,
//! which would be a second vocabulary that agrees with the log only until
//! something changes.

use adapter_traits::{AgentHarness, Vcs, WorkProduct};
use api::{Daemon, Refusal};
use core_model::Job;
use ipc::{
    JobId, JobList, JobSummary, ManifestId, ManifestSummary, ModelChoices, ProposeJob, RunId,
    StepId, WireError, WorkflowId, WorkflowSummary,
};
use store::{LoadJobError, WriteError};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The codes this boundary raises, declared beside the thing that raises them.
///
/// The set is closed by collection rather than by authorship — a central
/// registry would put every code far from the failure it names.
const NO_SUCH_JOB: &str = "fleet.no_such_job";
const ILLEGAL_MOVE: &str = "fleet.illegal_move";
const FAULT: &str = "fleet.fault";
/// A proposal that decoded and names something that cannot produce a Drone.
const UNACCEPTABLE: &str = "fleet.unacceptable_proposal";

impl<H, V, W> Daemon for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
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

    /// The one workflow this Fleet holds, so a caller can name one that will
    /// not be refused.
    ///
    /// **A list over a set of one.** The reason there was one is that nothing
    /// could look a second up — `ResolvedWorkflow` carried no id — and that
    /// reason has gone, so a query that answered with a single object would
    /// have to be replaced rather than extended.
    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        let workflow = self.workflow();
        Ok(vec![WorkflowSummary {
            id: WorkflowId::from(workflow.id()),
            name: workflow.name().to_string(),
            version: workflow.version(),
            steps: workflow
                .steps()
                .iter()
                .map(|s| StepId::from(s.id()))
                .collect(),
            manifest_id: ManifestId::from(self.manifest().id()),
        }])
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
            path: path.to_string_lossy().to_string(),
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

    async fn approve_dispatch(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = self
            .approve(&job_id.to_domain())
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
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A Job as a Board row, with the reason its last transition stored.
    async fn summarised(&self, job: &Job) -> Result<JobSummary, Refusal> {
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(JobSummary::of(job, reason.as_ref()))
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
            // The request is well-formed and the values in it cannot work. Not
            // a 500: retrying it will fail identically forever, and the message
            // names what to send instead.
            Adrift::Unnameable
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::Modelless => {
                Refusal::Unacceptable(WireError::raised(UNACCEPTABLE, said, self.run_id()))
            }
            _ => Refusal::Fault(WireError::raised(FAULT, said, self.run_id())),
        }
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
