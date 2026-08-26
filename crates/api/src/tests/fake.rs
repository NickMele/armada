//! A daemon that is not Fleet.
//!
//! It holds Jobs in a `Vec` and moves them on request. **It asserts nothing
//! about the status machine** — that machine is `core-model`'s and is tested
//! there, against the edge table. This exists to answer the four operations so
//! the transport can be exercised, and it is written without `core-model` on
//! purpose: if a fake daemon can be built out of `ipc` alone, so can a Bridge.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use ipc::{
    Actor, Event, Instant, JobId, JobList, JobStateChanged, JobStatus, JobSummary,
    ManifestId, Origin, ProposeJob, RunId, UnreadableJob, Urgency, WorkflowId,
};

use crate::{Broadcaster, Daemon, Refusal};

/// A spelling the registry has. Panics in a test rather than returning an
/// `Option` nobody would handle.
pub fn status(spelling: &str) -> JobStatus {
    JobStatus::from_wire(spelling).expect("a status the registry has")
}

pub fn run_id() -> RunId {
    RunId::carried("01RUN")
}

pub struct FakeDaemon {
    jobs: Mutex<Vec<JobSummary>>,
    unreadable: Mutex<Vec<UnreadableJob>>,
    events: Broadcaster,
    minted: AtomicU64,
    /// When set, every call answers with a fault. The stream closing on a
    /// daemon that cannot answer is a behaviour worth a test.
    pub mute: Mutex<bool>,
}

impl FakeDaemon {
    pub fn new(events: Broadcaster) -> FakeDaemon {
        FakeDaemon {
            jobs: Mutex::new(Vec::new()),
            unreadable: Mutex::new(Vec::new()),
            events,
            minted: AtomicU64::new(0),
            mute: Mutex::new(false),
        }
    }

    /// A row the store could not read back, which the list must still carry.
    pub fn with_unreadable(self, fault: &str) -> FakeDaemon {
        self.unreadable.lock().expect("not poisoned").push(UnreadableJob {
            job_id: None,
            fault: fault.to_string(),
        });
        self
    }

    fn fault(&self, message: &str) -> Refusal {
        Refusal::Fault(ipc::WireError::raised("fake.mute", message, run_id()))
    }

    fn no_such_job(&self, job_id: &JobId) -> Refusal {
        Refusal::NoSuchJob(
            ipc::WireError::raised("fake.no_such_job", "no Job by that id", run_id())
                .about_job(job_id.clone()),
        )
    }

    /// Move a Job, publish the transition, and answer with where it now is.
    fn move_to(&self, job_id: &JobId, from: &str, to: &str, actor: &str) -> Result<JobSummary, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter_mut().find(|job| job.id == *job_id) else {
            return Err(self.no_such_job(job_id));
        };
        if job.status.as_wire() != from {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.illegal_move",
                format!("a Job at {} does not go to {to}", job.status.as_wire()),
                run_id(),
            )));
        }
        job.status = status(to);
        job.reason = None;
        let moved = job.clone();
        self.events.publish(Event::JobStateChanged(JobStateChanged {
            job_id: job_id.clone(),
            from: status(from),
            to: status(to),
            reason: None,
            actor: Actor::from_wire(actor).expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }));
        Ok(moved)
    }
}

impl Daemon for FakeDaemon {
    async fn list_jobs(&self) -> Result<JobList, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(JobList {
            jobs: self.jobs.lock().expect("not poisoned").clone(),
            unreadable: self.unreadable.lock().expect("not poisoned").clone(),
        })
    }

    async fn propose_job(&self, proposal: ProposeJob) -> Result<JobSummary, Refusal> {
        let minted = self.minted.fetch_add(1, Ordering::SeqCst);
        let job = JobSummary {
            id: JobId::carried(format!("01JOB{minted}")),
            // The entry status of a top-level Job. Creation is not a
            // transition, so nothing is published for it.
            status: status("awaiting_approval"),
            reason: None,
            workflow_id: proposal.workflow_id,
            owner_manifest_id: proposal.owner_manifest_id,
            origin: Origin::from_wire(proposal.origin.as_wire()).expect("a top-level origin"),
            urgency: proposal.urgency,
            atomic: proposal.atomic,
            model: proposal.model,
            current_step_id: None,
            assigned_drone: None,
        };
        self.jobs.lock().expect("not poisoned").push(job.clone());
        Ok(job)
    }

    async fn approve_dispatch(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_approval", "queued", "human")
    }

    async fn kill_drone(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "running", "killed", "human")
    }
}

/// A Job already running, so the kill path has something to kill.
pub fn running(daemon: &FakeDaemon, id: &str) {
    daemon.jobs.lock().expect("not poisoned").push(JobSummary {
        id: JobId::carried(id),
        status: status("running"),
        reason: None,
        workflow_id: WorkflowId::carried("01WF"),
        owner_manifest_id: ManifestId::carried("01MF"),
        origin: Origin::from_wire("manual").expect("an origin"),
        urgency: Urgency::from_wire("normal").expect("an urgency"),
        atomic: false,
        model: "a-model".to_string(),
        current_step_id: None,
        assigned_drone: None,
    });
}

/// A proposal body, as Bridge would send it.
pub const A_PROPOSAL: &str = r#"{
    "workflow_id": "01WF",
    "owner_manifest_id": "01MF",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false,
    "model": "a-model",
    "acceptance_criteria": [{"text": "the symptom is gone", "source": "check"}]
}"#;
