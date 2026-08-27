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
use api::{Daemon, Observed, Refusal};
use core_model::Job;
use ipc::mcp::{NotRecorded, Receipt, SubmitEvidence};
use ipc::{
    CheckRun, DeclaredCheck, JobDetail, JobId, JobList, JobSummary, ManifestId, ManifestSummary,
    ModelChoices, ProposeJob, Redispatched, RunId, StepFacts, StepId, WireError, WorkflowId,
    WorkflowStep, WorkflowSummary,
};
use store::{LoadJobError, WriteError};

use crate::adrift::{Adrift, NotSubmitted};
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
/// A redispatch asked for on a Job that is not waiting for a person. A 409 like
/// a refused move, and a code of its own because the machine was never asked.
const NOT_REDISPATCHABLE: &str = "fleet.not_redispatchable";

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

    /// One Job in full, folded from its log like every other read.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let reason = self
            .last_reason(job.id())
            .await
            .map_err(|why| self.refusal(why))?;
        let ran = self
            .store()
            .lock()
            .await
            .step_checks(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        Ok(JobDetail::of(
            &job,
            reason.as_ref(),
            &self.step_facts(&job, ran),
        ))
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
            steps: declared(workflow),
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
}

/// The wire shape of one declared Check.
///
/// A free function rather than a `From` impl: `ipc` already depends on
/// `core-model` and could hold this, but a `DeclaredCheck` is assembled from a
/// step rather than converted from one, and the assembly is Fleet's.
///
/// **The command does not cross.** A `ResolvedCheck` holds the `run` lifted out
/// of the Manifest; what is served is the name, which is what an escalation
/// cites and what `ManifestSummary` already carries.
fn declared_check(check: &core_model::ResolvedCheck) -> DeclaredCheck {
    DeclaredCheck {
        kind: check.kind().to_string(),
        name: check.name().map(str::to_string),
        expect_exit_code: check.expects(),
    }
}

/// A workflow's steps with the Checks each declares, in the workflow's order.
///
/// **This is what the next Job would freeze**, and `get_job` answers from what
/// its Job already froze. The two can now differ, which is the point: a
/// workflow edited under a running Job shows the new declaration here and the
/// approved one there.
fn declared(workflow: &config::ResolvedWorkflow) -> Vec<WorkflowStep> {
    workflow
        .steps()
        .iter()
        .map(|step| WorkflowStep {
            step_id: StepId::from(step.id()),
            checks: step.checks().iter().map(declared_check).collect(),
        })
        .collect()
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
    fn step_facts(
        &self,
        job: &Job,
        ran: Vec<(core_model::StepId, Vec<core_model::StepCheck>)>,
    ) -> Vec<StepFacts> {
        job.steps()
            .iter()
            .map(|step| StepFacts {
                step_id: StepId::from(step.step_id()),
                declares: job
                    .workflow()
                    .step(step.step_id())
                    .map(|declared| declared.checks().iter().map(declared_check).collect()),
                ran: ran
                    .iter()
                    .find(|(at, _)| at == step.step_id())
                    .map(|(_, checks)| checks.iter().map(CheckRun::from).collect())
                    .unwrap_or_default(),
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
            Adrift::NotRedispatchable { job, .. } | Adrift::NotReplaceable { job } => {
                Refusal::IllegalMove(
                    WireError::raised(NOT_REDISPATCHABLE, said, self.run_id())
                        .about_job(ipc::JobId::from(job)),
                )
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
