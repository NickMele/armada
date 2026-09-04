//! The reads: what a client asks Fleet, that moves nothing.
//!
//! **One of the three surfaces `Daemon` is composed of**, and the seam is the
//! one `crate::queries` already draws against `crate::commands`: a read never
//! decodes a body, never answers 201, and has no refusal meaning the machine
//! would not admit a move. The transport half of that split was there first;
//! this is the same line drawn through the trait, so an implementation writes
//! one block per surface and a handler names the one surface it uses.
//!
//! The expensive reads are here on purpose and each says why. `get_diff`
//! spends the patch and `get_call` spends one argument, and both are their own
//! operation so that the reads made on every refresh do not pay for them.

use std::future::Future;

use crate::daemon::Refusal;
use crate::observing::Observed;
use ipc::{
    CallArguments, FleetCapacity, JobDetail, JobDiff, JobEvidence, JobHistory, JobId, JobList,
    JobResources, ManifestReading, ManifestSummary, ModelChoices, ReportList, WorkflowSummary,
    WorktreesHeld,
};

/// Everything a client reads.
///
/// # No `list_jobs` variant that returns a bare list
///
/// [`ipc::JobList`] carries the Jobs that loaded *and* the rows that would not.
/// A method returning `Vec<JobSummary>` would let the transport layer be the
/// place a partial failure is quietly completed — the v1 bug that lost
/// twenty-one Jobs, one layer further out.
pub trait Queries: Send + Sync + 'static {
    /// `list_jobs` — Jobs with state and reason.
    fn list_jobs(&self) -> impl Future<Output = Result<JobList, Refusal>> + Send;

    /// `get_capacity` — the bound, what is occupying it, and the one thing
    /// holding the next Drone back.
    ///
    /// **Fleet-wide, and the answer no per-Job field could give.** A `queued`
    /// Job's reason folds the concurrency bound and all three machine signals
    /// into `waiting_on_resources`, because that is the only label the registry
    /// grants it. This is where the distinction between them lives.
    ///
    /// It answers rather than refuses when the machine will not be read: an
    /// unreadable machine admits, so `held_by` is absent and the two numbers
    /// are still true. The only `Refusal` here is Fleet being unable to read
    /// its own roster, which is not a state that has ever occurred.
    fn get_capacity(&self) -> impl Future<Output = Result<FleetCapacity, Refusal>> + Send;

    /// `get_manifest_reading` — what Fleet's last re-read of `armada.yml` came
    /// to, and whether it took.
    ///
    /// **Fleet-wide, and there is no Job to hang it off.** A Manifest reload
    /// belongs to no Job's transcript, which is why the only place it used to
    /// be said was the daemon's console.
    ///
    /// `None` is a real answer and not a miss: Fleet has not re-read the file
    /// since it started with it. The configuration in force is then simply the
    /// one it booted on, and there is nothing about it to say.
    ///
    /// It cannot refuse. Fleet either holds a reading or does not, and both are
    /// answers — `Result` is here to match the surface, not because a failure
    /// is reachable.
    fn get_manifest_reading(
        &self,
    ) -> impl Future<Output = Result<Option<ManifestReading>, Refusal>> + Send;

    /// `get_job` — one Job in full: its steps and where each got to, the
    /// criteria it is held to, the branch its worktree is on, and the brief it
    /// was given.
    ///
    /// **What `list_jobs` deliberately leaves behind.** A Board row is a row; a
    /// detail view is one Job somebody opened, and it cannot be assembled from
    /// the list without Fleet answering for the fields the list redacts.
    fn get_job(&self, job_id: JobId) -> impl Future<Output = Result<JobDetail, Refusal>> + Send;

    /// `get_job_events` — every move one Job made, oldest first.
    ///
    /// **The path taken, which [`Queries::get_job`] deliberately does not
    /// carry.** A detail view answers where a Job is now; this answers how it
    /// got there — every status transition, every step move, every Drone
    /// arriving and leaving, with the actor and the instant on each.
    ///
    /// # A separate operation because a history has no bound
    ///
    /// `get_job` is read on opening a Job and is the size of a summary. A
    /// history grows for as long as the Job lives, and the surface that draws
    /// it is folded away by default. One read paying for the other, on every
    /// open, is what a field on [`ipc::JobDetail`] would have cost.
    ///
    /// # It replays nothing
    ///
    /// The rows are read and rendered. `crates/store/src/fold.rs` is the only
    /// thing that puts a recorded move back through `Job::transition`, and an
    /// implementation that folded here would be a second machine.
    ///
    /// A Job that is not there is [`Refusal::NoSuchJob`] and never an empty
    /// history: empty is a real answer, and it means a Job that has not moved.
    fn get_job_events(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobHistory, Refusal>> + Send;

    /// `get_evidence` — every claim a Job's Drones have submitted, step by
    /// step.
    ///
    /// **The material a person decides on, and the cheap half of it.** A step
    /// that has submitted nothing is absent rather than present and blank,
    /// which is the shape the store answers in and the only one that tells a
    /// step claiming nothing from a step that has not claimed yet.
    ///
    /// Its own operation beside [`Queries::get_diff`] rather than folded into
    /// it: this is a handful of sentences per step and that is however large
    /// the work is.
    fn get_evidence(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobEvidence, Refusal>> + Send;

    /// `get_diff` — one Job's whole patch, with the file list beside it.
    ///
    /// **The expensive half, and the one call that spends it.**
    /// `WorkProduct` splits the file list from the patch because the bytes are
    /// large and most steps ask no semantic question, so a call returning both
    /// would pay for the patch on every gate. A person reading a diff to decide
    /// whether to take the work is the case those bytes are for — and this is a
    /// route of its own so that [`Queries::get_job`], read on every open, never
    /// pays for them.
    ///
    /// A Job with no worktree answers with [`ipc::JobDiff::work`] absent rather
    /// than an empty reading: a Drone that changed nothing is a real and
    /// different answer, and one shape for both would draw a Job that never ran
    /// as one that did nothing. A worktree that will not open is
    /// [`Refusal::Fault`], never an empty patch.
    fn get_diff(&self, job_id: JobId) -> impl Future<Output = Result<JobDiff, Refusal>> + Send;

    /// `get_job_resources` — what this Job holds on this machine: the
    /// processes it owns, what each is burning, the disk its worktree has
    /// taken, and when anything was last written to its own log.
    ///
    /// **The other axis from `spend`.** That answers what a Job cost in model
    /// terms, and on a wedged Job every one of its four figures read zero — a
    /// Job between phases and a Job hung are the same four zeros.
    ///
    /// **Its own operation and not a field on [`Queries::get_job`].** That read
    /// is made on every open of a Job and on every event naming it; this one
    /// walks a process table and a directory, which is a cost paid when
    /// somebody asks rather than continuously.
    ///
    /// It answers rather than refuses on a reading it could not take. A `ps`
    /// that will not run and a worktree walk that ran long are said, because a
    /// panel that 500s is one a person stops opening.
    fn get_job_resources(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobResources, Refusal>> + Send;

    /// `get_call` — one tool call's arguments, as the record holds them.
    ///
    /// **The other end of a cut row.** `Saw::Called` carries a line and how
    /// many characters the argument had; this carries the argument. The split
    /// is [`Queries::get_diff`]'s against `job.files_changed`, made for the same
    /// reason: `observe_job` is bounded and lossy under backpressure by design,
    /// so an unbounded payload on it would evict the rows a person is reading.
    ///
    /// **A person's gesture is what pays for it.** Opening a row asks about one
    /// call, so an implementation looks for one call id rather than
    /// materialising a history.
    ///
    /// [`Refusal::NoSuchJob`] where the id names no Job.
    /// [`Refusal::Unacceptable`] where the Job is there and nothing in its
    /// transcripts carries that call — a well-formed request naming something
    /// that is not in the record, which is not the same answer as the Job being
    /// absent and must not be drawn as one.
    fn get_call(
        &self,
        job_id: JobId,
        call_id: String,
    ) -> impl Future<Output = Result<CallArguments, Refusal>> + Send;

    /// `list_workflows` — the workflows Fleet holds, with their steps.
    ///
    /// **What makes a `workflow_id` checkable.** Nothing joined one to a
    /// workflow before this, so a proposal naming an invented id was stored and
    /// the Job sat on the board claiming a workflow Fleet had never heard of.
    /// The refusal at creation is the fix; this is what lets a caller name one
    /// that will not be refused.
    fn list_workflows(&self) -> impl Future<Output = Result<Vec<WorkflowSummary>, Refusal>> + Send;

    /// `list_manifests` — the Manifests Fleet holds, and the repository each
    /// was read from. The counterpart to [`Queries::list_workflows`], for the
    /// other id a proposal names.
    fn list_manifests(&self) -> impl Future<Output = Result<Vec<ManifestSummary>, Refusal>> + Send;

    /// `list_models` — what a Job may be spawned as, and what it gets when it
    /// names nothing.
    fn list_models(&self) -> impl Future<Output = Result<ModelChoices, Refusal>> + Send;

    /// `list_reports` — every report filed, newest first, with what is known
    /// about whether the Judge has been right.
    ///
    /// **Not scoped to a Job, because a report outlives one.** `armada clean`
    /// forgets a Job and its report stays; a listing reachable only through a
    /// Job would lose exactly the reports that most need reading.
    fn list_reports(&self) -> impl Future<Output = Result<ReportList, Refusal>> + Send;

    /// `list_worktrees` — every worktree Fleet is holding disk for, and the
    /// test each one did not pass.
    ///
    /// **The read half of the rule [`Commands::reclaim_worktree`] acts on.**
    /// Fleet gives back what passes all five tests without asking; this is what
    /// the rest is chosen from, and the reasons are the whole point of it — an
    /// unmerged branch, uncommitted files and a Job still moving are answered
    /// differently by the person reading them.
    ///
    /// **Not scoped to a Job.** The question is which of these to give back,
    /// which is asked of the set rather than of any one row.
    ///
    /// **A piloted Job's worktree is not in the answer** — `#367`. It is
    /// dropped where the rule lives rather than hidden by a client, because an
    /// act that is drawn is an act somebody eventually clicks.
    fn list_worktrees(&self) -> impl Future<Output = Result<WorktreesHeld, Refusal>> + Send;

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
}
