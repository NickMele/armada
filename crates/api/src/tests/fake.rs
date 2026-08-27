//! A daemon that is not Fleet.
//!
//! It holds Jobs in a `Vec` and moves them on request. **It asserts nothing
//! about the status machine** — that machine is `core-model`'s and is tested
//! there, against the edge table. This exists to answer the operations M1 serves
//! so the transport can be exercised, and it is written without `core-model` on
//! purpose: if a fake daemon can be built out of `ipc` alone, so can a Bridge.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use ipc::mcp::{NotRecorded, Receipt, SubmitEvidence};
use ipc::{
    Actor, Event, Instant, JobCreated, JobDetail, JobId, JobList, JobStateChanged, JobStatus,
    JobSummary, ManifestId, ManifestSummary, ModelChoices, Origin, ProposeJob, Redispatched, RunId,
    StepId, UnreadableJob, Urgency, WorkflowId, WorkflowSummary,
};

use crate::{Broadcaster, Daemon, Feed, Observed, Refusal, Turns};

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
    /// The per-Job transcript channels, so a test can watch one Job while a
    /// Board client is on `/events` and prove neither reaches the other.
    pub turns: Turns,
    /// What `observe_job` answers with as the history, for whichever Job is
    /// asked. A Job with none is the ordinary case, not an error.
    pub history: Mutex<Vec<ipc::TranscriptRow>>,
    /// Older rows the history left out.
    pub skipped: Mutex<u64>,
    minted: AtomicU64,
    /// Every submission taken, so a test can assert that a refused call left
    /// nothing behind.
    pub submitted: Mutex<Vec<SubmitEvidence>>,
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
            turns: Turns::new(),
            history: Mutex::new(Vec::new()),
            skipped: Mutex::new(0),
            minted: AtomicU64::new(0),
            submitted: Mutex::new(Vec::new()),
            mute: Mutex::new(false),
        }
    }

    /// A row the store could not read back, which the list must still carry.
    pub fn with_unreadable(self, fault: &str) -> FakeDaemon {
        self.unreadable
            .lock()
            .expect("not poisoned")
            .push(UnreadableJob {
                job_id: None,
                fault: fault.to_string(),
            });
        self
    }

    /// A Drone writing this Job's rows. Dropping what comes back ends it, the
    /// same as a Drone exiting under Fleet.
    pub fn dispatching(&self, job_id: &JobId) -> Feed {
        self.turns.feeding(job_id)
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
    fn move_to(
        &self,
        job_id: &JobId,
        from: &str,
        to: &str,
        actor: &str,
    ) -> Result<JobSummary, Refusal> {
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

    /// One Job, with nothing the fake does not hold. **Every list is empty and
    /// every option absent**, because this daemon holds `JobSummary` and
    /// nothing beneath it — the shape is what is under test here, and the real
    /// fields are asserted against a real Fleet in `fleet`'s own suite.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        Ok(JobDetail {
            job: job.clone(),
            created_at: Instant::carried("2026-08-26T09:00:00.000Z"),
            branch: None,
            steps: Vec::new(),
            acceptance_criteria: Vec::new(),
            facts: None,
            write_targets: None,
            subject: None,
            dependencies: Vec::new(),
        })
    }

    /// The one workflow this fake holds. A list, because the operation is one.
    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(vec![WorkflowSummary {
            id: WorkflowId::carried("01WF"),
            name: "a-workflow".to_string(),
            version: 1,
            steps: vec![
                // Gated, and ungated. The pair is the distinction `get_job`'s
                // rail turns on, so the fake carries both rather than one.
                ipc::WorkflowStep {
                    step_id: StepId::carried("implement"),
                    checks: vec![ipc::DeclaredCheck {
                        kind: "manifest_check".to_string(),
                        name: Some("build".to_string()),
                        expect_exit_code: Some(0),
                    }],
                },
                ipc::WorkflowStep {
                    step_id: StepId::carried("summarise"),
                    checks: Vec::new(),
                },
            ],
            manifest_id: ManifestId::carried("01MF"),
        }])
    }

    async fn list_manifests(&self) -> Result<Vec<ManifestSummary>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(vec![ManifestSummary {
            id: ManifestId::carried("01MF"),
            repository: "a-repository".to_string(),
            path: "/a-repository/armada.yml".to_string(),
            version: 1,
            checks: vec!["build".to_string()],
        }])
    }

    async fn list_models(&self) -> Result<ModelChoices, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(ModelChoices {
            models: vec!["a-model".to_string(), "another-model".to_string()],
            default: "a-model".to_string(),
        })
    }

    async fn propose_job(&self, proposal: ProposeJob) -> Result<JobSummary, Refusal> {
        let minted = self.minted.fetch_add(1, Ordering::SeqCst);
        let job = JobSummary {
            id: JobId::carried(format!("01JOB{minted}")),
            title: proposal.title,
            // The entry status of a top-level Job. Creation is not a
            // transition, and `job.created` is what carries it — a
            // `job.state_changed` here would name a move from a status the Job
            // was never in.
            status: status("awaiting_approval"),
            reason: None,
            workflow_id: proposal.workflow_id,
            owner_manifest_id: proposal.owner_manifest_id,
            origin: Origin::from_wire(proposal.origin.as_wire()).expect("a top-level origin"),
            urgency: proposal.urgency,
            atomic: proposal.atomic,
            // Absent is the ordinary case: Fleet fills it from configuration.
            // The fake has none, so it names what the fixture names.
            model: proposal.model.unwrap_or_else(|| "a-model".to_string()),
            current_step_id: None,
            assigned_drone: None,
            redispatched_from: None,
        };
        self.jobs.lock().expect("not poisoned").push(job.clone());
        self.events.publish(Event::JobCreated(JobCreated {
            job: job.clone(),
            actor: Actor::from_wire("human").expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }));
        Ok(job)
    }

    async fn approve_dispatch(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_approval", "queued", "human")
    }

    /// The process, not the unit of work. The Job is handed back where it
    /// stood, with no Drone on it: `assigned_drone` is presence rather than
    /// state, and **the registry names no edge a killed Drone fires**, so a
    /// transition invented here would be the fake asserting something about a
    /// machine it does not own.
    async fn kill_drone(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter_mut().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        // The one status this fake knows carries a live Drone. Nothing else
        // does: a Job at the approval gate or in the queue has no process.
        if job.status.as_wire() != "running" {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.no_drone",
                format!("a Job at {} has no Drone to kill", job.status.as_wire()),
                run_id(),
            )));
        }
        job.assigned_drone = None;
        Ok(job.clone())
    }

    /// The unit of work, not the process. Legal from wherever the Job is, so
    /// long as it is not already over — including the statuses `kill_drone`
    /// refuses, which is the whole reason the two are separate operations.
    async fn kill_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let from = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            // Terminality is the domain's, read through the DTO rather than
            // restated: a second list of which statuses are over is a second
            // vocabulary.
            if job.status.domain().is_terminal() {
                return Err(Refusal::IllegalMove(ipc::WireError::raised(
                    "fake.illegal_move",
                    format!("a Job at {} is already over", job.status.as_wire()),
                    run_id(),
                )));
            }
            job.status.as_wire()
        };
        self.move_to(&job_id, from, "killed", "human")
    }

    /// Kill the failed Job and mint its replacement. **The fake asserts one
    /// thing about the domain and no more**: only `escalated` is replaceable,
    /// because that is the whole of what the route refuses on.
    async fn redispatch_job(&self, job_id: JobId) -> Result<Redispatched, Refusal> {
        let failed = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            if job.status.as_wire() != "escalated" {
                return Err(Refusal::IllegalMove(ipc::WireError::raised(
                    "fake.not_redispatchable",
                    format!(
                        "a Job at {} is not waiting for a person",
                        job.status.as_wire()
                    ),
                    run_id(),
                )));
            }
            job.clone()
        };
        let minted = self.minted.fetch_add(1, Ordering::SeqCst);
        let dispatched = JobSummary {
            id: JobId::carried(format!("01JOB{minted}")),
            status: status("awaiting_approval"),
            reason: None,
            redispatched_from: Some(failed.id.clone()),
            ..failed.clone()
        };
        self.jobs
            .lock()
            .expect("not poisoned")
            .push(dispatched.clone());
        self.events.publish(Event::JobCreated(JobCreated {
            job: dispatched.clone(),
            actor: Actor::from_wire("human").expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }));
        let replaced = self.move_to(&job_id, "escalated", "killed", "human")?;
        Ok(Redispatched {
            replaced,
            dispatched,
        })
    }

    /// The Evidence tool, faked down to what the transport is under test for:
    /// a submission is taken while a Job is running and refused otherwise.
    ///
    /// **The fake names no Job either.** The trait has no parameter for one, so
    /// a fake that wanted to accept evidence for a Job of the caller's choosing
    /// could not express it — which is the binding this crate is able to assert
    /// about, the rest being Fleet's working slot and asserted there.
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

    async fn submit_evidence(&self, submission: SubmitEvidence) -> Result<Receipt, NotRecorded> {
        let running = self
            .jobs
            .lock()
            .expect("not poisoned")
            .iter()
            .any(|job| job.status.as_wire() == "running");
        if !running {
            return Err(NotRecorded {
                because: "no Job is being worked, so there is no step for this \
                          submission to be against"
                    .to_string(),
            });
        }
        self.submitted
            .lock()
            .expect("not poisoned")
            .push(submission);
        Ok(Receipt {
            word: "recorded".to_string(),
        })
    }
}

/// A Job already running, so the Drone kill has something to kill.
pub fn running(daemon: &FakeDaemon, id: &str) {
    at(daemon, id, "running");
}

/// A Job parked at any status the registry has, without a transition to get it
/// there. What a fake is for: the transport is the thing under test.
pub fn at(daemon: &FakeDaemon, id: &str, spelling: &str) {
    daemon.jobs.lock().expect("not poisoned").push(JobSummary {
        id: JobId::carried(id),
        title: format!("a Job called {id}"),
        status: status(spelling),
        reason: None,
        workflow_id: WorkflowId::carried("01WF"),
        owner_manifest_id: ManifestId::carried("01MF"),
        origin: Origin::from_wire("manual").expect("an origin"),
        urgency: Urgency::from_wire("normal").expect("an urgency"),
        atomic: false,
        model: "a-model".to_string(),
        current_step_id: None,
        assigned_drone: None,
        redispatched_from: None,
    });
}

/// A proposal body, as Bridge would send it.
pub const A_PROPOSAL: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "01WF",
    "owner_manifest_id": "01MF",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false,
    "model": "a-model",
    "acceptance_criteria": [{"text": "the symptom is gone", "source": "check"}]
}"#;
