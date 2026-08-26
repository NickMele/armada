//! What `api` needs from the daemon, and the refusals it may answer with.
//!
//! # This trait is the reason `api` does not depend on `fleet`
//!
//! It is stated here, where the transport is, and implemented over there, where
//! the Jobs are. The dependency therefore points from `fleet` to `api` and never
//! back, `cargo tree -p api` names no `fleet`, and the daemon core stays
//! drivable in a test with **no socket, no port and no process** — a fake
//! implements the trait and the same router serves it.
//!
//! # It speaks DTOs, not Jobs
//!
//! Every signature below is `ipc` vocabulary. That is where the redaction sits:
//! Fleet converts at this boundary, so a field added to `core_model::Job`
//! reaches the wire only when somebody writes the line that puts it there.
//! `api` never sees a domain type, and nothing in this crate can leak one.
//!
//! # No `list_jobs` variant that returns a bare list
//!
//! [`ipc::JobList`] carries the Jobs that loaded *and* the rows that would not.
//! A method returning `Vec<JobSummary>` would let the transport layer be the
//! place a partial failure is quietly completed — the v1 bug that lost
//! twenty-one Jobs, one layer further out.

use std::future::Future;

use ipc::{JobId, JobList, JobSummary, ProposeJob, WireError};

/// The five request-response operations M1 serves.
///
/// **What M1 needs, not what the seam carries.** The inventory in
/// `crates/ipc/operations.toml` is
/// the authority on what exists; this is the subset M1 needs, named with that
/// file's own keys so a rule reading both needs no mapping in between.
///
/// # Killing a Drone and killing a Job are two methods, not one
///
/// They are different acts on different things, and the registry is what says
/// so: `awaiting_approval -> killed` and `queued -> killed` leave statuses no
/// Drone has been spawned under, so a Job ends there with no process to
/// terminate. One signature covering both would have to mean whichever the
/// caller happened to be looking at.
pub trait Daemon: Send + Sync + 'static {
    /// `list_jobs` — Jobs with state and reason.
    fn list_jobs(&self) -> impl Future<Output = Result<JobList, Refusal>> + Send;

    /// `propose_job` — drafts a Job onto the approval gate. **The gate is
    /// unchanged:** what comes back is a Job at `awaiting_approval`, not a
    /// running one.
    fn propose_job(
        &self,
        proposal: ProposeJob,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `approve_dispatch` — releases a Job to spawn. The primary autonomy
    /// control, and a human act: `helm_access` on this row is `No`.
    fn approve_dispatch(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `kill_drone` — kills a Drone, captures learnings, holds the worktree.
    /// Intervention Ladder rung 2. **The Job survives**: what comes back is the
    /// Job the killed Drone was on, still open, with its worktree held for a
    /// redispatch. Nothing here ends a Job.
    fn kill_drone(&self, job_id: JobId)
        -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `kill_job` — ends the Job at `killed`, terminal, carrying no verdict.
    /// Not on the Intervention Ladder, and the only thing that ends a Job by
    /// hand: **Fleet never auto-kills.**
    ///
    /// Legal from every non-terminal status, including those with no Drone
    /// under them, which is why it cannot be spelled as [`Daemon::kill_drone`].
    fn kill_job(&self, job_id: JobId) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;
}

/// A request the daemon would not serve.
///
/// Three variants because a caller has three different things to do about them,
/// and "the request failed" is not an answer anybody can act on. The status is
/// derived here rather than carried on [`WireError`]: an HTTP code is the
/// transport's, and putting one on the wire type would give the lifeboat's job
/// to the protocol.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// No Job by that id. The id was invented, or the Job was retained out.
    NoSuchJob(WireError),
    /// The Job exists and the machine does not admit the move — approving one
    /// already running, killing one already terminal.
    IllegalMove(WireError),
    /// Something under the daemon failed. Not the caller's doing, and retrying
    /// the same request is reasonable.
    Fault(WireError),
}

impl Refusal {
    /// The HTTP status this answers with.
    pub fn status(&self) -> u16 {
        match self {
            Refusal::NoSuchJob(_) => 404,
            Refusal::IllegalMove(_) => 409,
            Refusal::Fault(_) => 500,
        }
    }

    pub fn error(&self) -> &WireError {
        match self {
            Refusal::NoSuchJob(error) | Refusal::IllegalMove(error) | Refusal::Fault(error) => {
                error
            }
        }
    }
}
