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

use crate::observing::Observed;
use ipc::mcp::{NotRecorded, Receipt, SubmitEvidence};
use ipc::{
    JobDetail, JobId, JobList, JobSummary, ManifestSummary, ModelChoices, ProposeJob, Redispatched,
    WireError, WorkflowSummary,
};

/// The request-response operations M1 serves.
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

    /// `get_job` — one Job in full: its steps and where each got to, the
    /// criteria it is held to, the branch its worktree is on, and the brief it
    /// was given.
    ///
    /// **What `list_jobs` deliberately leaves behind.** A Board row is a row; a
    /// detail view is one Job somebody opened, and it cannot be assembled from
    /// the list without Fleet answering for the fields the list redacts.
    fn get_job(&self, job_id: JobId) -> impl Future<Output = Result<JobDetail, Refusal>> + Send;

    /// `list_workflows` — the workflows Fleet holds, with their steps.
    ///
    /// **What makes a `workflow_id` checkable.** Nothing joined one to a
    /// workflow before this, so a proposal naming an invented id was stored and
    /// the Job sat on the board claiming a workflow Fleet had never heard of.
    /// The refusal at creation is the fix; this is what lets a caller name one
    /// that will not be refused.
    fn list_workflows(&self) -> impl Future<Output = Result<Vec<WorkflowSummary>, Refusal>> + Send;

    /// `list_manifests` — the Manifests Fleet holds, and the repository each
    /// was read from. The counterpart to [`Daemon::list_workflows`], for the
    /// other id a proposal names.
    fn list_manifests(&self) -> impl Future<Output = Result<Vec<ManifestSummary>, Refusal>> + Send;

    /// `list_models` — what a Job may be spawned as, and what it gets when it
    /// names nothing.
    fn list_models(&self) -> impl Future<Output = Result<ModelChoices, Refusal>> + Send;

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

    /// `redispatch_job` — mints a replacement for a Job that ran and stopped,
    /// and kills the original where it is still killable. Intervention Ladder
    /// rung 2, and the answer to a Job with no way to be tried again.
    ///
    /// **`escalated`, `completed_failed` and `killed`**; a `rejected` Job never
    /// ran, so there is nothing to carry forward and the act is `propose_job`.
    ///
    /// **It does not reopen the failed Job**, which is why it answers with
    /// [`Redispatched`] rather than a `JobSummary`: the registry's
    /// `redispatched_from` row says a redispatch is always a new Job carrying
    /// a reference back, and the replacement's id is what the caller needs
    /// next.
    fn redispatch_job(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<Redispatched, Refusal>> + Send;

    /// `observe_job` — one Job's turns, the history and then the live ones.
    ///
    /// # It answers before the socket opens
    ///
    /// A Job that does not exist is a 404 the caller reads at the moment they
    /// asked, rather than a connection that opens and immediately says nothing.
    /// That is why this returns a value and not a socket.
    ///
    /// # The order inside is the whole guarantee
    ///
    /// An implementation subscribes **first** and reads the history after, so
    /// no row can fall between the two. A caller holding an [`Observed`] cannot
    /// get that order wrong because both halves are already in it.
    ///
    /// A Job nothing is writing is not a refusal. [`Observed::live`] is `None`
    /// and the history is served on its own — a Job never dispatched, one
    /// already finished, and one whose Drone went with the Fleet that spawned
    /// it all reach a viewer that way.
    fn observe_job(&self, job_id: JobId) -> impl Future<Output = Result<Observed, Refusal>> + Send;

    /// `submit_evidence` — the Evidence tool, called by the Drone that is
    /// working. **Not an inventory operation and not Bridge's**: it is the one
    /// method on this trait whose caller is a Drone, which is why its refusal
    /// is [`NotRecorded`] rather than [`Refusal`].
    ///
    /// # The submission is bound to a Job the caller never names
    ///
    /// There is no `job_id` parameter and no `step_id`, so there is nothing to
    /// forge: the implementation attributes the submission to the Job it is
    /// itself working, which it knows and the caller cannot influence. A call
    /// that arrives while nothing is working is refused rather than queued.
    ///
    /// **That is binding by construction and not authentication.** Any process
    /// that can reach the listener can make this call; what it cannot do is
    /// choose which Job the evidence lands against.
    ///
    /// # It decides nothing
    ///
    /// The receipt says the submission was taken, not that it passed. A call
    /// that blocked while a repository's Checks ran would time out, so the
    /// gate runs afterwards and the outcome reaches the Drone as a later turn.
    fn submit_evidence(
        &self,
        submission: SubmitEvidence,
    ) -> impl Future<Output = Result<Receipt, NotRecorded>> + Send;
}

/// A request the daemon would not serve.
///
/// Four variants because a caller has four different things to do about them,
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
    /// **The request decoded and names something that cannot work.** A proposal
    /// naming a workflow or a Manifest Fleet does not hold, or carrying a blank
    /// model where nothing configured supplies one.
    ///
    /// The fourth variant, added because those three had nowhere honest to go.
    /// A 400 belongs to the transport and means the bytes did not become a
    /// request; a 500 says the daemon broke, which sends the caller to retry
    /// something that will fail identically forever. This says: the request is
    /// well-formed, the values in it are not, and the message names them.
    Unacceptable(WireError),
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
            Refusal::Unacceptable(_) => 422,
            Refusal::Fault(_) => 500,
        }
    }

    pub fn error(&self) -> &WireError {
        match self {
            Refusal::NoSuchJob(error)
            | Refusal::IllegalMove(error)
            | Refusal::Unacceptable(error)
            | Refusal::Fault(error) => error,
        }
    }
}
