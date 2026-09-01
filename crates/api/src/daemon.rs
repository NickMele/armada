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

use crate::mcp::Caller;
use crate::observing::Observed;
use ipc::mcp::{AskQuestion, CheckReport, DeclareScope, NotRecorded, Receipt, SubmitEvidence};
use ipc::{
    ChangesRequested, ChosenAnswer, FileReport, FleetCapacity, JobDetail, JobDiff, JobEvidence,
    JobForgotten, JobHistory, JobId, JobList, JobSummary, ManifestSummary, ModelChoices,
    ProposeJob, Redirection, Redispatched, Report, ReportList, WireError, WorkflowSummary,
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
    /// **The path taken, which [`Daemon::get_job`] deliberately does not
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
    /// Its own operation beside [`Daemon::get_diff`] rather than folded into
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
    /// route of its own so that [`Daemon::get_job`], read on every open, never
    /// pays for them.
    ///
    /// A Job with no worktree answers with [`ipc::JobDiff::work`] absent rather
    /// than an empty reading: a Drone that changed nothing is a real and
    /// different answer, and one shape for both would draw a Job that never ran
    /// as one that did nothing. A worktree that will not open is
    /// [`Refusal::Fault`], never an empty patch.
    fn get_diff(&self, job_id: JobId) -> impl Future<Output = Result<JobDiff, Refusal>> + Send;

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

    /// `propose_from_request` — reads a request and drafts the Job it proposes
    /// onto the approval gate.
    ///
    /// **The dispatch path for a person who describes work rather than filling
    /// in a form**, and the only caller of the Job proposer. What comes back is
    /// the same thing [`Daemon::propose_job`] answers: a Job at
    /// `awaiting_approval`. This adds no gate and removes none.
    ///
    /// # Two failures, and they are not the same failure
    ///
    /// A request no workflow fits is [`Refusal::Unacceptable`], and the request
    /// comes back on the error's `request` field with no Job created — nothing
    /// is assigned by default, because the resolved definition is frozen into
    /// the Job and becomes the yardstick the work is judged against.
    ///
    /// A call that could not be made — the network, the quota, the budget — is
    /// [`Refusal::Fault`]. It says nothing about the request, and a caller that
    /// could not tell the two apart would read an outage as a refusal.
    ///
    /// # It answers with a plan, not a Job
    ///
    /// One request can be several Jobs, and approving is a different act
    /// depending on how many: one is dispatched by its approval, several are a
    /// plan whose members each take their own. A signature answering one Job
    /// would make the second case unrepresentable rather than merely unbuilt.
    fn propose_from_request(
        &self,
        request: ipc::JobRequest,
    ) -> impl Future<Output = Result<ipc::ProposedPlan, Refusal>> + Send;

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
    /// hand: **Fleet stops a Drone only at a cap**, and a cap ends the
    /// spending rather than the work — the worktree survives it.
    ///
    /// Legal from every non-terminal status, including those with no Drone
    /// under them, which is why it cannot be spelled as [`Daemon::kill_drone`].
    fn kill_job(&self, job_id: JobId) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `forget_job` — deletes the Job's whole record. **Real deletion, not a
    /// further status**: the row and everything beneath it are gone, through
    /// `Store::forget_job`, and there is no undo.
    ///
    /// **Terminal only.** Refused with a 409 on a Job still in flight — there
    /// is no record to erase while a Drone might still write to it, only a
    /// status to move, and [`Daemon::kill_job`] is the act that ends one that
    /// is not there yet.
    ///
    /// **It does not reclaim the worktree or the branch.** `armada clean`
    /// already owns that, on its own retention schedule; a person clearing a
    /// finished Job off the Board is not also being asked to think about disk.
    ///
    /// What comes back is the id and nothing else — there is no Job left for a
    /// summary to describe.
    fn forget_job(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobForgotten, Refusal>> + Send;

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

    /// `redirect_drone` — a person's instruction to the Drone that is there.
    /// Intervention Ladder rung 1, and the one command Helm reaches directly.
    ///
    /// **It keeps everything.** The session, the worktree and every step so
    /// far: the Job goes back to `running` at the step it stopped on and the
    /// instruction is a turn injected into a process that never went away.
    ///
    /// **Refused where the Drone is gone**, with a 409, because the act that
    /// applies there is [`Daemon::restart_step`]. It does not respawn — a
    /// redirect that spawned is a restart that lost the session for nothing,
    /// and the two are separate methods so that neither can quietly become the
    /// other.
    fn redirect_drone(
        &self,
        job_id: JobId,
        instruction: Redirection,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `restart_step` — a fresh Drone on the worktree the last one left, at the
    /// step that stopped.
    ///
    /// **The worktree, the branch and every earlier step's work survive**, and
    /// nothing else does: there is no session to resume, which is what makes
    /// this a different act from [`Daemon::redirect_drone`] rather than a
    /// slower one. The toolset is resolved again from scratch.
    ///
    /// **Refused where the Drone is alive**, and refused where the worktree has
    /// been reclaimed — the second says the act being asked for is a
    /// redispatch rather than becoming one.
    fn restart_step(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `approve_review` — the person takes the work, and the Job goes on.
    ///
    /// **The counterpart to [`Daemon::approve_dispatch`]**, at the other end of
    /// the Job: that one is the gate before anything runs and this is the
    /// decision after it has. Both are human acts.
    ///
    /// It moves the machine and never writes a status: the step advances on the
    /// inner machine, which is legal beneath `awaiting_review`, and then the
    /// Job goes back to `running` at the next step — or, where the step that
    /// passed was the workflow's last, is committed, delivered and recorded
    /// `completed_success`.
    ///
    /// **Refused with a 409 anywhere but `awaiting_review`.** All three review
    /// acts share that refusal, and it is what stops this from quietly becoming
    /// the dispatch gate: `awaiting_approval` has its own approval and its own
    /// denial.
    fn approve_review(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `request_changes` — the work is not right yet, and here is what to fix.
    ///
    /// **It keeps everything.** The Job goes back to `running` at the same
    /// step, and the note is a turn injected into the session that was waiting
    /// at the gate — so the Drone, the worktree and every step so far survive.
    ///
    /// **Refused where the Drone is gone**, for [`Daemon::redirect_drone`]'s
    /// reason: there is nobody to tell, and a Job put back to `running` with no
    /// process on it escalates as `interrupted` a moment later having lost the
    /// note. A blank note is refused as well — a Drone told nothing resumes
    /// with exactly the information that was not enough.
    fn request_changes(
        &self,
        job_id: JobId,
        note: ChangesRequested,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `override_verdict` — the Judge refused, a person disagrees, and the step
    /// advances anyway.
    ///
    /// **The fifth act on an escalated Job, and the only one that keeps the
    /// work.** [`Daemon::approve_review`] cannot be it: a Job a gate refused is
    /// `escalated`, not `awaiting_review`. [`Daemon::restart_step`] cannot be
    /// it either — it re-runs the step, which discards work that was right and
    /// may draw the same refusal. A verdict with no appeal is worse than no
    /// verdict, because a verifier a person cannot overrule is one they route
    /// around.
    ///
    /// # It is not an approve-anything
    ///
    /// Only `gate_failure` is liftable — the Judge refusing a criterion, which
    /// is a matter of opinion. A step stopped on `gate_undecided` was never
    /// weighed and one stopped on `evidence_suspect` is a claim about the
    /// Drone's honesty; both are [`Refusal::IllegalMove`]. A failed mechanical
    /// Check is out of reach twice over: it ends the Job at `completed_failed`,
    /// which is terminal and stops no step, and the recorded Check runs are
    /// read again before anything moves.
    ///
    /// # It is recorded as an override
    ///
    /// The step move is `stopped -> advanced` carrying the trigger it
    /// overruled, so the row still says `failed` beside a state that says
    /// `advanced`, and [`ipc::StepDetail::overridden`] is that pair read once
    /// here rather than by every surface. A blank reason is
    /// [`Refusal::Unacceptable`]: an override that says nothing is how this
    /// becomes the way somebody quiets a gate.
    fn override_verdict(
        &self,
        job_id: JobId,
        overruling: ipc::Overruled,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `rerun_gate` — the gate could not decide, and a person asks it again on
    /// the evidence the step already submitted.
    ///
    /// **The act [`Daemon::override_verdict`] is deliberately not.** A step
    /// stopped on `gate_undecided` was never weighed: `Ruling::CouldNotDecide`
    /// exists so that a machine unable to answer produces no verdict in either
    /// direction, and advancing on one would pass work nothing ruled on. There
    /// is no decision to disagree with, so what is owed is the question, asked
    /// again.
    ///
    /// # It re-runs once, takes no body, and spends no retry budget
    ///
    /// A transient cause and a permanent one arrive as the same value and are
    /// not told apart, so nothing loops: where the cause was permanent the gate
    /// fails again and says so. There is no reason to carry, because nothing is
    /// being disagreed with. And a gate re-run is not a run of the step, so the
    /// budget a failed Check hands the step back inside is untouched.
    ///
    /// # What it refuses
    ///
    /// [`Refusal::IllegalMove`] on a Job that is not `escalated`, on an
    /// escalation that stopped no step, on a step stopped on any other trigger
    /// — that one is an override, or nothing — and on a Job the daemon is no
    /// longer standing at, where the baseline the first reading used is gone
    /// and [`Daemon::restart_step`] is what applies.
    fn rerun_gate(&self, job_id: JobId)
        -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `reject_job` — the work is not wanted, and the Job is over.
    ///
    /// **Terminal, which is what makes it the hard stop.** `rejected` is a
    /// verdict on the work rather than an operator clearing the Board, which is
    /// [`Daemon::kill_job`]. The act for work that is nearly right is
    /// [`Daemon::request_changes`], which keeps the Job.
    ///
    /// Refused anywhere but `awaiting_review`. `awaiting_approval -> rejected`
    /// is a legal edge and it belongs to `deny_dispatch`, which is a different
    /// act on a Job that has never run.
    fn reject_job(&self, job_id: JobId)
        -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `file_report` — a person says this Job failed in error, and the Job's
    /// own record is filed with what they said.
    ///
    /// **The one operation here that records a person disagreeing with the
    /// machine at all.** Every other signature on this trait answers *did this
    /// fail*; none of them answers *did this fail correctly*, and a Job that
    /// failed perfectly by its own lights and was wrong is indistinguishable
    /// through any of them from one that failed rightly.
    ///
    /// # It creates nothing and dispatches nothing
    ///
    /// What comes back is the report. No Job is proposed, no Drone is spawned,
    /// and nothing about the Job it names changes — filing is its own act, and
    /// dispatching against what was filed is the existing flow pointed at it.
    ///
    /// # A blank sentence is [`Refusal::Unacceptable`]
    ///
    /// The record was already there before anybody pressed anything, so a
    /// report with the bundle and no sentence has added exactly nothing. This
    /// is `override_verdict`'s refusal for the same reason and it is not
    /// enough on its own: a required non-blank field does not make a reason
    /// meaningful, which is why [`ipc::FileReport::claim`] is a closed set and
    /// is the field anything counting reads.
    fn file_report(
        &self,
        job_id: JobId,
        filing: FileReport,
    ) -> impl Future<Output = Result<Report, Refusal>> + Send;

    /// `list_reports` — every report filed, newest first, with what is known
    /// about whether the Judge has been right.
    ///
    /// **Not scoped to a Job, because a report outlives one.** `armada clean`
    /// forgets a Job and its report stays; a listing reachable only through a
    /// Job would lose exactly the reports that most need reading.
    fn list_reports(&self) -> impl Future<Output = Result<ReportList, Refusal>> + Send;

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

    /// `answer_question` — a person picks one of the answers a waiting Drone
    /// offered.
    ///
    /// **The Bridge half of the Drone's [`ask_question`](Daemon::ask_question)**,
    /// and one act seen from its two ends: a Drone asked and stopped, and this
    /// starts it again. The answer goes into the live session as a turn, the
    /// delivery half `redirect_drone` already uses.
    ///
    /// **It moves nothing.** The Job is `running` before and after, and it comes
    /// back for the reason every other command answers with one — a caller folds
    /// the row rather than re-reading the board.
    ///
    /// **There is no field for prose.** [`ChosenAnswer::chose`] is one of the labels
    /// the Drone offered and a label matching none is refused rather than passed
    /// through; words reach a Drone through [`Daemon::redirect_drone`] and
    /// nothing else, which is what keeps this from becoming the conversation
    /// `docs/scope.md` rejected.
    ///
    /// [`Refusal::IllegalMove`] where nothing is outstanding, where the id names
    /// a question already answered, and where the label was not offered.
    fn answer_question(
        &self,
        job_id: JobId,
        answer: ChosenAnswer,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `ask_question` — the working Drone asks the person who approved the Job
    /// something it cannot answer from the repository, and offers the answers it
    /// will accept.
    ///
    /// **Not an inventory operation and not Bridge's**, exactly as
    /// [`Daemon::submit_evidence`] is not: its caller is a Drone, which is why
    /// its refusal is [`NotRecorded`]. The Bridge half is
    /// [`Daemon::answer_question`].
    ///
    /// # The receipt is not the answer
    ///
    /// It returns as soon as Fleet has taken the question, like submitting and
    /// unlike [`Daemon::run_checks`]. Holding it open was rejected twice over: a
    /// person's wait has no budget that could bound an HTTP call, and an injected
    /// turn is consumed when the current tool call returns — so a Drone blocked
    /// inside this would swallow every redirect sent to unstick it. The answer
    /// arrives as a later turn in the Drone's own session.
    ///
    /// **One at a time**, because a Drone that could stack questions would be
    /// holding a conversation. Bound to a Job and a step the caller never names,
    /// for [`submit_evidence`](Daemon::submit_evidence)'s reason.
    fn ask_question(
        &self,
        caller: Caller,
        asking: AskQuestion,
    ) -> impl Future<Output = Result<Receipt, NotRecorded>> + Send;

    /// `submit_evidence` — the Evidence tool, called by the Drone that is
    /// working. **Not an inventory operation and not Bridge's**: its caller is
    /// a Drone rather than Bridge, which is why its refusal is [`NotRecorded`]
    /// rather than [`Refusal`]. [`Daemon::declare_scope`] and
    /// [`Daemon::run_checks`] are the other two.
    ///
    /// # The submission is bound to a Job the caller never names
    ///
    /// There is no `job_id` parameter and no `step_id`, so there is nothing to
    /// forge: the implementation attributes the submission to the Job whose
    /// Drone holds the connection it arrived on, which it knows and the caller
    /// cannot influence. A call that arrives while that Job is not working is
    /// refused rather than queued.
    ///
    /// **[`Caller`] is the transport's word and never the body's.** It carries
    /// the peer of the connection and nothing a caller wrote, so there is no
    /// arrangement of a request that could put a Job id into it.
    ///
    /// **That is binding by construction and not authentication.** Any process
    /// that can reach the listener can make this call; what it cannot do is
    /// choose which Job the evidence lands against, and a caller the
    /// implementation cannot place is refused rather than guessed at.
    ///
    /// # It decides nothing
    ///
    /// The receipt says the submission was taken, not that it passed. A call
    /// that blocked while a repository's Checks ran would time out, so the
    /// gate runs afterwards and the outcome reaches the Drone as a later turn.
    fn submit_evidence(
        &self,
        caller: Caller,
        submission: SubmitEvidence,
    ) -> impl Future<Output = Result<Receipt, NotRecorded>> + Send;

    /// `run_checks` — the working Drone asks whether the work it has so far
    /// passes the Checks that gate its step, and is told.
    ///
    /// # It is not the gate, and the signature is what says so
    ///
    /// What comes back is [`CheckReport`], which has no verdict on it and no
    /// method that could become one. Nothing here advances a step, records
    /// evidence, or writes a Check row — the gate runs the same Checks again
    /// for itself when evidence is submitted, and only that run decides
    /// anything.
    ///
    /// # It blocks, where submitting does not
    ///
    /// A receipt is returned before the Checks run because the outcome is not
    /// known yet; here the outcome *is* the answer, so the call is held open
    /// while they run. What that costs is bounded by the implementation — a cap
    /// per step, and a refusal while one is already running — because the
    /// convergence clocks are suspended for the duration and cannot bound it.
    ///
    /// Bound to a Job and a step the caller never names, for
    /// [`submit_evidence`](Daemon::submit_evidence)'s reason: there is no
    /// parameter at all.
    fn run_checks(
        &self,
        caller: Caller,
    ) -> impl Future<Output = Result<CheckReport, NotRecorded>> + Send;

    /// `declare_scope` — where the working Drone says its work for this step
    /// will be. **The one call that arrives before the work rather than
    /// after it.**
    ///
    /// Bound to a Job and a step the caller never names, for
    /// [`submit_evidence`](Daemon::submit_evidence)'s reason. It moves nothing:
    /// what comes back is a receipt, and what the declaration does is give
    /// Fleet something to compare the worktree against — while the step runs,
    /// and again at the gate.
    fn declare_scope(
        &self,
        caller: Caller,
        declaration: DeclareScope,
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
