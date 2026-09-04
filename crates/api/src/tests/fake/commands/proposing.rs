//! Minting a Job, and looking at one without moving it.
//!
//! **Nothing here is a transition.** A proposal creates, and creation is not a
//! move — `job.created` is what carries it, and a `job.state_changed` beside it
//! would name a move from a status the Job was never in. `examine_job` reads.
//! What the route is under test for is the status code and the body.

use ipc::{
    Actor, Event, Instant, JobCreated, JobExamined, JobId, JobSummary, ManifestId, Origin,
    ProposeJob, Urgency, WorkflowId,
};
use std::sync::atomic::Ordering;

use super::super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::{run_id, status};
use crate::Refusal;

impl FakeDaemon {
    /// The proposer's own path, faked at the seam above it: the fake names the
    /// workflow the request asked for by naming none of its own reasoning. What
    /// `api` is under test for is the route, the status and the body.
    pub(super) async fn fake_propose_from_request(
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
        self.fake_propose_job(ProposeJob {
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
    pub(super) async fn fake_propose_job(
        &self,
        proposal: ProposeJob,
    ) -> Result<JobSummary, Refusal> {
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
            resumption: None,
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
            // No slot, so nothing is waiting. See `Tools::ask_question`.
            asking: false,
            landed: None,
        };
        self.jobs.lock().expect("not poisoned").push(job.clone());
        self.events.publish(Event::JobCreated(JobCreated {
            job: job.clone(),
            actor: Actor::from_wire("human").expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }));
        Ok(job)
    }
    /// Stop a proposal. **The fake never has one out** — it answers a proposal
    /// without making a call — so this always reports that there was nothing to
    /// stop, which is the arm a route test needs: it is the one that must be a
    /// 200 and not a 404.
    pub(super) async fn fake_stop_proposal(
        &self,
        _proposal_id: ipc::ProposalId,
    ) -> Result<ipc::ProposalStopped, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(ipc::ProposalStopped { stopped: false })
    }
    /// Look at the Job. **The one command that moves nothing**, so the fake
    /// answers the shape and leaves its Job list alone — and a 404 on an id
    /// naming nothing, which is the only refusal this act has.
    pub(super) async fn fake_examine_job(&self, job_id: JobId) -> Result<JobExamined, Refusal> {
        let jobs = self.jobs.lock().expect("not poisoned");
        if !jobs.iter().any(|job| job.id == job_id) {
            return Err(self.no_such_job(&job_id));
        }
        Ok(shapes::examined(job_id))
    }
}
