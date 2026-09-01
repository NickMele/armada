//! A daemon that is not Fleet.
//!
//! It holds Jobs in a `Vec` and moves them on request. **It asserts nothing
//! about the status machine** — that machine is `core-model`'s and is tested
//! there, against the edge table. This exists to answer the operations M1 serves
//! so the transport can be exercised, and it is written without `core-model` on
//! purpose: if a fake daemon can be built out of `ipc` alone, so can a Bridge.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use ipc::mcp::{CheckRan, CheckReport, DeclareScope, NotRecorded, Receipt, SubmitEvidence};
use ipc::{
    Actor, ChangesRequested, Event, EvidenceType, Instant, JobCreated, JobDetail, JobDiff,
    JobEvidence, JobForgotten, JobHistory, JobId, JobList, JobStateChanged, JobStatus, JobSummary,
    ManifestId, ManifestSummary, ModelChoices, Movement, Origin, ProposeJob, Recorded,
    Redispatched, RunId, StatusMoved, StepId, Submitted, UnreadableJob, Urgency, Work, WorkflowId,
    WorkflowSummary,
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

/// One step of the rail, declared and not yet run.
///
/// **Built out of `ipc` and nothing else**, which is the file's whole claim: a
/// gate and a judge declaration a Bridge can construct are a gate and a judge
/// declaration a Bridge can read.
fn step_rail(
    step_id: &str,
    ordinal: u32,
    gate: &str,
    judge_checks: Vec<ipc::DeclaredJudge>,
) -> ipc::StepDetail {
    ipc::StepDetail {
        step_id: StepId::carried(step_id),
        label: step_id.to_string(),
        ordinal,
        state: ipc::StepState::from_wire("not_started").expect("a step state the registry has"),
        checks: Some(Vec::new()),
        check_runs: Vec::new(),
        judge_checks: Some(judge_checks),
        advance_gate: Some(ipc::AdvanceGate::from_wire(gate).expect("a gate the registry has")),
        last_verdict: None,
        overridden: false,
        judged: Vec::new(),
        flagged: Vec::new(),
        judging: None,
        entered_at: Instant::carried("2026-08-26T09:00:00.000Z"),
        updated_at: Instant::carried("2026-08-26T09:00:00.000Z"),
    }
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
    /// Every scope declaration taken, in arrival order.
    pub declared: Mutex<Vec<DeclareScope>>,
    /// Every question taken, in arrival order.
    pub asked: Mutex<Vec<ipc::mcp::AskQuestion>>,
    /// How many dry runs were asked for, so a test can assert that a refused
    /// call ran nothing.
    pub checked: AtomicU64,
    /// Every report filed, in filing order, so a test can assert that a
    /// refused filing left none behind.
    pub reports: Mutex<Vec<ipc::Report>>,
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
            declared: Mutex::new(Vec::new()),
            asked: Mutex::new(Vec::new()),
            checked: AtomicU64::new(0),
            reports: Mutex::new(Vec::new()),
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

    /// One Job, with nothing the fake does not hold — except a step rail,
    /// which it holds because the rail is the shape a client draws a Job from.
    ///
    /// **Two steps, and they are the pair the rail turns on**: one the Judge
    /// gates, one a person does. Every other list is empty and every other
    /// option absent, because this daemon holds `JobSummary` and nothing
    /// beneath it, and the real values are asserted against a real Fleet in
    /// `fleet`'s own suite.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        Ok(JobDetail {
            spend: None,
            delivery: None,
            job: job.clone(),
            created_at: Instant::carried("2026-08-26T09:00:00.000Z"),
            branch: None,
            steps: vec![
                step_rail(
                    "implement",
                    0,
                    "auto_if_judge_passes",
                    vec![ipc::DeclaredJudge {
                        criteria: 2,
                        panel_size: Some(3),
                        gaming_check: true,
                    }],
                ),
                step_rail("handoff", 1, "human_always", Vec::new()),
            ],
            acceptance_criteria: Vec::new(),
            facts: None,
            write_targets: None,
            subject: None,
            dependencies: Vec::new(),
            // Absent, like every other option here: this daemon reads no store
            // and a footprint is a store's answer.
            footprint: None,
            // Absent for the same reason, one layer further in: an outstanding
            // redirect is read from a working slot, and this daemon holds none.
            redirecting: None,
            // Absent for the redirect's reason exactly: an unanswered question
            // lives on a working slot, and this daemon holds none.
            asking: None,
            // Absent again: a waiting note is a column on `jobs`, and this
            // daemon's Jobs are wire summaries rather than records.
            redirect_waiting: None,
            // Absent, not empty: no other Job's record is held, so nothing looked.
            write_scope_overlaps: None,
            // Absent for the same reason again: a classification is read from
            // a slot, a filesystem and a store, and this daemon has none of the
            // three.
            stuck: None,
        })
    }

    /// The one move this fake records: the Job reaching the status it is at.
    ///
    /// **Not a fold and not a machine.** The header comment holds for this too
    /// — the fake asserts nothing about either machine, and what it answers is
    /// the shape rather than a real history. The refusal is what matters here:
    /// an id that names nothing is a 404, never an empty list.
    async fn get_job_events(&self, job_id: JobId) -> Result<JobHistory, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        Ok(JobHistory {
            job_id,
            moves: vec![Recorded {
                seq: 1,
                status: status("awaiting_approval"),
                moved: Movement::Status(StatusMoved {
                    to: job.status,
                    reason: None,
                }),
                actor: Actor::from_wire("human").expect("an actor the envelope has"),
                at: Instant::carried("2026-08-26T09:00:00.000Z"),
            }],
        })
    }

    /// One step's evidence, for a Job that exists. **A step that submitted
    /// nothing is absent** rather than blank, which is the shape the store
    /// answers in and the one this fake is here to prove crosses.
    async fn get_evidence(&self, job_id: JobId) -> Result<JobEvidence, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        Ok(JobEvidence {
            job_id,
            steps: vec![Submitted {
                step_id: StepId::carried("implement"),
                evidence_type: EvidenceType::from_wire("diff").expect("a type the registry has"),
                claimed: "the log reader stops one line early".to_string(),
                shown_by: "a failing test that now passes".to_string(),
                // Absent, not blank. The rule the whole DTO turns on.
                not_claimed: None,
            }],
        })
    }

    /// A reading with a patch in it. **Present because there is something in
    /// it** — the absent case is a Job with no worktree, and this fake has no
    /// worktrees to be absent.
    async fn get_diff(&self, job_id: JobId) -> Result<JobDiff, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        Ok(JobDiff {
            job_id,
            work: Some(Work {
                files: vec![ipc::ChangedFile {
                    path: "crates/store/src/read.rs".to_string(),
                    change: ipc::ChangeKind::Modified,
                    outside_plan: false,
                }],
                plan_declared: false,
                patch: Some(
                    "--- a/crates/store/src/read.rs\n+++ b/crates/store/src/read.rs\n".to_string(),
                ),
            }),
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
                // rail turns on, so the fake carries both rather than one — and
                // the second is the step that stops for a person, which a
                // preview drew as a step with nothing on it.
                ipc::WorkflowStep {
                    step_id: StepId::carried("implement"),
                    label: "Implement the change".to_string(),
                    checks: vec![ipc::DeclaredCheck {
                        kind: "manifest_check".to_string(),
                        name: Some("build".to_string()),
                        run: Some("cargo build --workspace --locked".to_string()),
                        expect_exit_code: Some(0),
                        // Covering everything, which is most Checks. The scoped
                        // case is asserted in `ipc`'s own round-trip.
                        when: None,
                    }],
                    judge_checks: vec![ipc::DeclaredJudge {
                        criteria: 2,
                        panel_size: None,
                        gaming_check: false,
                    }],
                    advance_gate: ipc::AdvanceGate::from_wire("auto_if_judge_passes")
                        .expect("a gate the registry has"),
                },
                ipc::WorkflowStep {
                    step_id: StepId::carried("handoff"),
                    label: "Hand the work back".to_string(),
                    checks: Vec::new(),
                    judge_checks: Vec::new(),
                    advance_gate: ipc::AdvanceGate::from_wire("human_always")
                        .expect("a gate the registry has"),
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

    /// The proposer's own path, faked at the seam above it: the fake names the
    /// workflow the request asked for by naming none of its own reasoning. What
    /// `api` is under test for is the route, the status and the body.
    async fn propose_from_request(
        &self,
        request: ipc::JobRequest,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        if request.request.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.unacceptable_proposal",
                "a request needs something in it to read",
                run_id(),
            )));
        }
        self.propose_job(ProposeJob {
            title: request.request.clone(),
            workflow_id: WorkflowId::carried("bug"),
            owner_manifest_id: ManifestId::carried("01MANIFEST"),
            origin: ipc::TopLevelOrigin::from_wire("auto_detected").expect("an origin"),
            urgency: Urgency::from_wire("normal").expect("an urgency"),
            atomic: false,
            model: None,
            acceptance_criteria: Vec::new(),
            subject: None,
            facts: request.request,
            write_targets: None,
            dependencies: Vec::new(),
            attachments: Vec::new(),
        })
        .await
        .map(|job| ipc::ProposedPlan { jobs: vec![job] })
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
            created_at: Instant::carried("2026-01-01T00:00:00.000Z"),
            // No worktree exists at the approval gate, so no branch is claimed.
            branch: None,
            reason: None,
            queued_reason: None,
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
            // No slot, so nothing is waiting. See `Daemon::ask_question`.
            asking: false,
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

    /// The person takes the work. The gate this answers is the one after the
    /// Job ran, which is why the status it moves from is not the one
    /// `approve_dispatch` moves from.
    async fn approve_review(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_review", "running", "human")
    }

    /// The work goes back with a note. The same destination as an approval on a
    /// Job with steps left, and **a different act** — what separates them here
    /// is the note, and in Fleet it is the step that does or does not advance.
    async fn request_changes(
        &self,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        if note.note.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.blank_note",
                "a review note with nothing in it says nothing to change",
                run_id(),
            )));
        }
        self.move_to(&job_id, "awaiting_review", "running", "human")
    }

    /// A verdict on the work. Terminal, and from the review gate alone: the
    /// other edge into `rejected` is the dispatch gate's and belongs to
    /// `deny_dispatch`.
    async fn reject_job(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        self.move_to(&job_id, "awaiting_review", "rejected", "human")
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

    /// The record, gone. **The fake actually removes it**, the same as
    /// `Store::forget_job` does on the real one, so a caller that forgets a
    /// Job and then asks for it again sees exactly what asking for an id that
    /// never existed sees.
    async fn forget_job(&self, job_id: JobId) -> Result<JobForgotten, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        // Terminality is the domain's, read through the DTO rather than
        // restated, for `kill_job`'s reason.
        if !job.status.domain().is_terminal() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.not_forgettable",
                format!("a Job at {} cannot be forgotten", job.status.as_wire()),
                run_id(),
            )));
        }
        jobs.retain(|job| job.id != job_id);
        Ok(JobForgotten { job_id })
    }

    /// Mint a replacement for a stopped Job. **The fake asserts one thing about
    /// the domain and no more**: a Job that ran and stopped is replaceable and
    /// anything else is refused, because that is the whole of what the route
    /// refuses on.
    /// Both resume acts, faked the way the real ones are told apart: **which
    /// one applies is decided by the Drone, not by the caller.** A redirect
    /// needs one alive; a restart is what exists when it is gone. A fake that
    /// let either work on any Job would let a test pass against a rule the
    /// real Fleet enforces.
    async fn redirect_drone(
        &self,
        job_id: JobId,
        _instruction: ipc::Redirection,
    ) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.assigned_drone.is_none() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.no_drone_to_redirect",
                "this Job has no Drone to redirect".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }

    /// The answer to a question, faked on the one thing a `JobSummary` shows:
    /// **there is no Drone waiting.** Which question is outstanding and which
    /// labels it offered come off a working slot this daemon has none of.
    async fn answer_question(
        &self,
        job_id: JobId,
        _answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.assigned_drone.is_none() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.nothing_is_asking",
                "this Job has no Drone waiting on an answer".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }

    async fn restart_step(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.assigned_drone.is_some() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.drone_still_there",
                "this Job still has a Drone — redirect it rather than restarting".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }

    /// The one act on a refused step, faked on the one thing the route refuses
    /// on that the transport can see: **a blank reason is not an override.**
    /// Which trigger stopped the step is Fleet's to read off the record and
    /// nothing a `JobSummary` carries, so the fake does not pretend to know it.
    async fn override_verdict(
        &self,
        job_id: JobId,
        overruling: ipc::Overruled,
    ) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if overruling.reason.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.unreasoned_override",
                "overruling a verdict needs a reason".to_string(),
                run_id(),
            )));
        }
        Ok(job.clone())
    }

    /// The act on a step nothing ruled on, faked on the one thing the route
    /// refuses on that the transport can see: **a Job that is not escalated
    /// reaches no act on a stopped step.** Which trigger stopped the step, and
    /// whether the daemon is still standing at it, are Fleet's to read off the
    /// record and the slot; a `JobSummary` carries neither, so the fake does
    /// not pretend to know them.
    async fn rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
            return Err(self.no_such_job(&job_id));
        };
        if job.status.as_wire() != "escalated" {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.not_resumable",
                format!("a Job at {} has no stopped step", job.status.as_wire()),
                run_id(),
            )));
        }
        Ok(job.clone())
    }

    /// Filing, faked on the two things the transport can see: the Job has to
    /// exist, and **a report with no sentence is not a report.** What Fleet
    /// attaches around the sentence is Fleet's — three reads and a render —
    /// and a fake that produced a record would be asserting about a bundle it
    /// invented.
    async fn file_report(
        &self,
        job_id: JobId,
        filing: ipc::FileReport,
    ) -> Result<ipc::Report, Refusal> {
        let job = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            job.clone()
        };
        if filing.said.trim().is_empty() {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.unsaid_report",
                "a report needs the sentence the record is context for".to_string(),
                run_id(),
            )));
        }
        let report = ipc::Report {
            id: ipc::ReportId::carried("01REPORT"),
            filed_at: Instant::carried("2026-08-28T21:00:00.000Z"),
            origin: ipc::ReportOrigin::Human,
            claim: filing.claim,
            job_id,
            job_title: job.title.clone(),
            step_id: filing.step_id,
            criterion_id: filing.criterion_id,
            said: filing.said,
            record: "## Every move it made".to_string(),
        };
        self.reports
            .lock()
            .expect("not poisoned")
            .push(report.clone());
        Ok(report)
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

    async fn redispatch_job(&self, job_id: JobId) -> Result<Redispatched, Refusal> {
        let failed = {
            let jobs = self.jobs.lock().expect("not poisoned");
            let Some(job) = jobs.iter().find(|job| job.id == job_id) else {
                return Err(self.no_such_job(&job_id));
            };
            if !matches!(
                job.status.as_wire(),
                "escalated" | "completed_failed" | "killed"
            ) {
                return Err(Refusal::IllegalMove(ipc::WireError::raised(
                    "fake.not_redispatchable",
                    format!("a Job at {} has not run and stopped", job.status.as_wire()),
                    run_id(),
                )));
            }
            job.clone()
        };
        let minted = self.minted.fetch_add(1, Ordering::SeqCst);
        let dispatched = JobSummary {
            id: JobId::carried(format!("01JOB{minted}")),
            status: status("awaiting_approval"),
            created_at: Instant::carried("2026-01-01T00:00:00.000Z"),
            // No worktree exists at the approval gate, so no branch is claimed.
            branch: None,
            reason: None,
            queued_reason: None,
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
        // Only an escalated original moves. A terminal one has no outbound
        // edge, and the fake does not invent one.
        let replaced = if failed.status.as_wire() == "escalated" {
            self.move_to(&job_id, "escalated", "killed", "human")?
        } else {
            failed
        };
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

    /// The caller is taken and not read. **A fake daemon has no processes to
    /// place one against** — which Job a connection belongs to is
    /// `fleet::peer`'s answer and is tested there, and a router test is about
    /// the route rather than about the attribution.
    async fn submit_evidence(
        &self,
        _caller: crate::Caller,
        submission: SubmitEvidence,
    ) -> Result<Receipt, NotRecorded> {
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

    async fn declare_scope(
        &self,
        _caller: crate::Caller,
        declaration: DeclareScope,
    ) -> Result<Receipt, NotRecorded> {
        let running = self
            .jobs
            .lock()
            .expect("not poisoned")
            .iter()
            .any(|job| job.status.as_wire() == "running");
        if !running {
            return Err(NotRecorded {
                because: "no Job is being worked, so there is no step for this \
                          declaration to be about"
                    .to_string(),
            });
        }
        self.declared
            .lock()
            .expect("not poisoned")
            .push(declaration);
        Ok(Receipt {
            word: "declared".to_string(),
        })
    }

    /// A question taken, refused on the one thing this daemon sees: nothing is
    /// being worked. **The receipt says taken, never answered** — what a person
    /// chose arrives in the Drone's session, which no fake has.
    async fn ask_question(
        &self,
        _caller: crate::Caller,
        asking: ipc::mcp::AskQuestion,
    ) -> Result<Receipt, NotRecorded> {
        let running = self
            .jobs
            .lock()
            .expect("not poisoned")
            .iter()
            .any(|job| job.status.as_wire() == "running");
        if !running {
            return Err(NotRecorded {
                because: "no Job is being worked, so there is no step for this \
                          question to be about"
                    .to_string(),
            });
        }
        self.asked.lock().expect("not poisoned").push(asking);
        Ok(Receipt {
            word: "asked".to_string(),
        })
    }

    /// One passing Check and one failing one, so a test over this router can
    /// tell that a report reaches the Drone **and** that a failure in it is
    /// still a successful tool call.
    async fn run_checks(&self, _caller: crate::Caller) -> Result<CheckReport, NotRecorded> {
        let running = self
            .jobs
            .lock()
            .expect("not poisoned")
            .iter()
            .any(|job| job.status.as_wire() == "running");
        if !running {
            return Err(NotRecorded {
                because: "no Job is being worked, so there are no checks to run".to_string(),
            });
        }
        self.checked.fetch_add(1, Ordering::SeqCst);
        Ok(CheckReport {
            ran: vec![
                CheckRan {
                    name: "fmt".to_string(),
                    outcome: outcome("passed"),
                    detail: None,
                    took: std::time::Duration::from_millis(300),
                    log: Some(".armada/checks/a-job/implement.dry.0.log".to_string()),
                },
                CheckRan {
                    name: "tests".to_string(),
                    outcome: outcome("failed"),
                    detail: Some("exit code 101, expected 0".to_string()),
                    took: std::time::Duration::from_secs(12),
                    log: Some(".armada/checks/a-job/implement.dry.1.log".to_string()),
                },
            ],
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
        created_at: Instant::carried("2026-01-01T00:00:00.000Z"),
        branch: Some(format!("armada/{id}")),
        reason: None,
        queued_reason: None,
        workflow_id: WorkflowId::carried("01WF"),
        owner_manifest_id: ManifestId::carried("01MF"),
        origin: Origin::from_wire("manual").expect("an origin"),
        urgency: Urgency::from_wire("normal").expect("an urgency"),
        atomic: false,
        model: "a-model".to_string(),
        current_step_id: None,
        assigned_drone: None,
        redispatched_from: None,
        asking: false,
    });
}

/// A check outcome by its registry spelling. **`from_wire` rather than a
/// variant**, because this crate names no domain type — the same reason
/// `origin` and `urgency` above are built this way.
fn outcome(spelling: &str) -> ipc::CheckOutcome {
    ipc::CheckOutcome::from_wire(spelling).expect("a check outcome")
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
