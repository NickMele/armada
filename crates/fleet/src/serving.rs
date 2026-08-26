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
//! [`Refusal`] has three variants because a caller has three different things
//! to do about them, and every mapping below is decided by which *typed* leaf
//! error came back rather than by reading a message. `Adrift::IllegalMove` is a
//! 409 because the machine refused; a store fault is a 500 because it was not
//! the caller's doing; a Job that is not there is a 404. Nothing here parses
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
use ipc::{JobId, JobList, JobSummary, ProposeJob, RunId, WireError};
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

    /// Draft a Job onto the approval gate. Creation is not a transition, so
    /// nothing is published for it.
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
