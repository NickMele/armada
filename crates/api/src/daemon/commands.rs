//! The writes: what a person tells Fleet to do, and what the Job looks like
//! afterwards.
//!
//! **One of the three surfaces `Daemon` is composed of.** What separates these
//! from [`Queries`](super::Queries) is not size: a command decodes a body, may
//! answer 201, and has refusals meaning the machine would not admit the move.
//! `crate::commands` drew that line through the transport already; this is the
//! same line through the trait.
//!
//! # Killing a Drone and killing a Job are two methods, not one
//!
//! They are different acts on different things, and the registry is what says
//! so: `awaiting_approval -> killed` and `queued -> killed` leave statuses no
//! Drone has been spawned under, so a Job ends there with no process to
//! terminate. One signature covering both would have to mean whichever the
//! caller happened to be looking at.

use std::future::Future;

use crate::daemon::{Redirector, Refusal};
use ipc::{
    AddTask, AnswerCommand, CapRaise, ChangesRequested, CheckoutRunRecord, CheckoutRunUnderway,
    ChosenAnswer, DropTask, FileReport, FindingDismissed, FindingQueued, IssueFiled, JobExamined,
    JobForgotten, JobId, JobSummary, JudgeAnswered, ManifestSaved, NamedRun, ProposeJob,
    Redirection, Redispatched, RemarksTakenUp, Report, RestartRequested, RunRecord, RunUnderway,
    SaveManifestFile, SetWhenBlocked, SetWhenRefused, StartCheckoutRun, StartRun, TurnRaise,
    WorktreeReclaimed,
};

/// Everything a client asks Fleet to do.
pub trait Commands: Send + Sync + 'static {
    /// `propose_job` — drafts a Job onto the approval gate. **The gate is
    /// unchanged:** what comes back is a Job at `awaiting_approval`, not a
    /// running one.
    ///
    /// `by` is the transport's word, never the body's — `redirect_drone`'s
    /// reason: [`Redirector::Helm`] only where the door placed the call in a
    /// Helm session, which is one of the acts `#941` lets it draft. `#943`.
    fn propose_job(
        self: std::sync::Arc<Self>,
        proposal: ProposeJob,
        by: Redirector,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `propose_from_request` — reads a request and drafts the Job it proposes
    /// onto the approval gate.
    ///
    /// **The dispatch path for a person who describes work rather than filling
    /// in a form**, and the only caller of the Job proposer. What comes back is
    /// the same thing [`Commands::propose_job`] answers: a Job at
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
    ///
    /// `by` is [`Commands::propose_job`]'s own word, for the same reason: the
    /// door lets a Helm session read a request too.
    fn propose_from_request(
        &self,
        request: ipc::JobRequest,
        manifest_id: Option<ipc::ManifestId>,
        by: Redirector,
    ) -> impl Future<Output = Result<ipc::ProposedPlan, Refusal>> + Send;

    /// `stop_proposal` — stops a Job proposer call that is still out.
    ///
    /// **The only thing anybody may do to a proposal**, and it exists because
    /// a client that merely stopped waiting would leave the call running inside
    /// Fleet, spending, until its budget expired. The control a person is
    /// offered has to reach the process.
    ///
    /// # Both arms are a success
    ///
    /// A proposal that has already finished answers
    /// [`ProposalStopped::stopped`](ipc::ProposalStopped::stopped) false rather
    /// than refusing. By the time somebody presses this the call may have just
    /// landed and the Jobs may already be on the board — reporting a failure
    /// there would say something untrue about the only thing they care about.
    ///
    /// **It ends no Job, because there is no Job.** Nothing is created and
    /// nothing moves. The request the stop interrupted answers as a fault
    /// carrying `proposer_stopped`, which is not an error: somebody decided.
    fn stop_proposal(
        &self,
        proposal_id: ipc::ProposalId,
    ) -> impl Future<Output = Result<ipc::ProposalStopped, Refusal>> + Send;

    /// `approve_dispatch` — releases a Job to spawn. The primary autonomy
    /// control, and a human act: `agent_access` on this row is `No`.
    ///
    /// **What comes back is `queued`, not `running`.** The dispatch is a
    /// turn's, because one inside this request died whenever a client stopped
    /// waiting for it — `fleet::daemon::Fleet::approve` and `#428`.
    fn approve_dispatch(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `kill_drone` — kills a Drone, captures learnings, holds the worktree.
    /// Intervention Ladder rung 2. **The Job survives**: what comes back is the
    /// Job the killed Drone was on, still open, with its worktree held for a
    /// redispatch. Nothing here ends a Job.
    fn kill_drone(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `kill_job` — ends the Job at `killed`, terminal, carrying no verdict.
    /// Not on the Intervention Ladder, and the only thing that ends a Job by
    /// hand: **Fleet stops a Drone only at a cap**, and a cap ends the
    /// spending rather than the work — the worktree survives it.
    ///
    /// Legal from every non-terminal status, including those with no Drone
    /// under them, which is why it cannot be spelled as [`Commands::kill_drone`].
    fn kill_job(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `examine_job` — go and look at this Job now, and say what was found.
    ///
    /// **The rung below intervene.** Every other act here changes the Job, so a
    /// person who suspected one was wedged had one move — end it — and no way
    /// to find out first whether ending it was warranted.
    ///
    /// **A command rather than a query, and the split is worth stating.** It
    /// decodes no body and answers no 201, which are two of the three marks of
    /// a read. What makes it an act is the third thing it does: it writes what
    /// it found into the Job's own log, so the answer is on the record beside
    /// everything else Fleet did rather than in one person's terminal.
    ///
    /// **It costs no model call and is bounded.** The one thing a person
    /// presses when they already suspect a hang must not be the next thing that
    /// stops answering.
    ///
    /// **`cannot_tell` is a real answer and is never rounded up.** A look that
    /// cannot separate working from not says so, and one such look keeps the
    /// whole examination off `working` — "everything looks fine" on a plainly
    /// hung Job spends a person's suspicion and returns nothing.
    ///
    /// [`Refusal::NoSuchJob`] where the id names nothing. There is no
    /// [`Refusal::IllegalMove`]: every status is examinable, including the
    /// terminal ones, because *check this Job* is not *check preparation*.
    fn examine_job(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobExamined, Refusal>> + Send;

    /// `forget_job` — deletes the Job's whole record. **Real deletion, not a
    /// further status**: the row and everything beneath it are gone, through
    /// `Store::forget_job`, and there is no undo.
    ///
    /// **Terminal only.** Refused with a 409 on a Job still in flight — there
    /// is no record to erase while a Drone might still write to it, only a
    /// status to move, and [`Commands::kill_job`] is the act that ends one that
    /// is not there yet.
    ///
    /// **It does not reclaim the worktree or the branch.** `armada clean`
    /// already owns that, on its own retention schedule; a person clearing a
    /// finished Job off the Board is not also being asked to think about disk.
    ///
    /// What comes back is the id and nothing else — there is no Job left for a
    /// summary to describe.
    fn forget_job(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobForgotten, Refusal>> + Send;

    /// `reclaim_worktree` — removes one terminal Job's checkout and deletes the
    /// branch it derived. **The other half of [`Commands::forget_job`]**, and a
    /// separate method for the reason that one gives for not doing it: two
    /// unrelated things to fail at in one call is worse than two calls, and
    /// neither one's outcome depends on the other.
    ///
    /// **Terminal only**, with a 409 otherwise: there is no disk to reclaim
    /// while a Drone might still write to it.
    ///
    /// **A branch the base cannot reach is kept**, always. There is no force
    /// on this seam — a live Fleet must not be the thing that deletes commits
    /// nobody has taken — so a caller asking for the disk back may get the
    /// checkout and not the branch, and [`ipc::WorktreeReclaimed`] says which
    /// half happened rather than reporting one number for both.
    ///
    /// The record is untouched. A reclaimed Job is still on the Board, and
    /// [`Commands::forget_job`] is what takes the row.
    fn reclaim_worktree(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<WorktreeReclaimed, Refusal>> + Send;

    /// `delete_branch` — deletes a terminal Job's branch, unmerged or not, once
    /// its checkout is gone and only while it stands at the `tip` a person was
    /// shown. [`Refusal::IllegalMove`] otherwise, naming which.
    fn delete_branch(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        asked: ipc::DeleteBranch,
    ) -> impl Future<Output = Result<ipc::BranchDeleted, Refusal>> + Send;

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
        self: std::sync::Arc<Self>,
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
    /// applies there is [`Commands::restart_step`]. It does not respawn — a
    /// redirect that spawned is a restart that lost the session for nothing,
    /// and the two are separate methods so that neither can quietly become the
    /// other.
    ///
    /// `by` is the transport's word, never the body's: [`Redirector::Helm`]
    /// only where the door placed the call in a Helm session.
    fn redirect_drone(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        instruction: Redirection,
        by: Redirector,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `restart_step` — a fresh Drone on the worktree the last one left, at the
    /// step that stopped.
    ///
    /// **The worktree, the branch and every earlier step's work survive**, and
    /// nothing else does: there is no session to resume, which is what makes
    /// this a different act from [`Commands::redirect_drone`] rather than a
    /// slower one. The toolset is resolved again from scratch.
    ///
    /// **Refused where the Drone is alive**, and refused where the worktree has
    /// been reclaimed — the second says the act being asked for is a
    /// redispatch rather than becoming one.
    ///
    /// **What comes back is `queued`.** The act asks for a Drone; the turn
    /// starts one, because a spawn inside this request died whenever a client
    /// stopped waiting for it — `#428` and `#456`.
    ///
    /// **The note is optional and is not a reason for the restart.** `None` is
    /// the plain restart this act has always been. `Some` is a person saying
    /// what to do differently in the same breath as asking for another
    /// attempt — held on the Job and delivered into the opening brief of the
    /// Drone this asks for, which is the road [`Commands::request_changes`]
    /// writes down and not a second one. Refused where a note is already
    /// waiting, and refused blank for [`Commands::redirect_drone`]'s reason.
    fn restart_step(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        note: Option<RestartRequested>,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `approve_review` — the person takes the work, and the Job goes on.
    ///
    /// **The counterpart to [`Commands::approve_dispatch`]**, at the other end of
    /// the Job: that one is the gate before anything runs and this is the
    /// decision after it has. Both are human acts.
    ///
    /// It moves the machine and never writes a status: the step advances on the
    /// inner machine, which is legal beneath `awaiting_review`, and then the
    /// Job goes back in the **queue** at the next step — or, where the step
    /// that passed was the workflow's last, is committed, delivered and
    /// recorded `completed_success`.
    ///
    /// **What comes back is `queued`, not `running`**, for
    /// [`Commands::approve_dispatch`]'s reason and `#456`. The Drone on the
    /// next step is a turn's.
    ///
    /// **Refused with a 409 anywhere but `awaiting_review`**, and with a 409
    /// where the Job's worktree has been reclaimed and a step is left to work
    /// in it. The first is what stops this from quietly becoming the dispatch
    /// gate: `awaiting_approval` has its own approval and its own denial. The
    /// second arrives while the person's hand is still on the control, which is
    /// the point of it — deferring the dispatch is `#456`, deferring the
    /// refusal would have been collateral.
    fn approve_review(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `merge_pull_request` — a person merges the pull request their Job
    /// opened, and Fleet runs what the merge implies.
    ///
    /// **[`Commands::approve_review`] with a write to the forge in front of
    /// it.** What it adds is the merge, the record of it and the repository's
    /// `after_merge` Checks against the tree the merge left; what it does to
    /// the Job is that act's, so the same statuses come back for the same
    /// reasons.
    ///
    /// **The only act on this trait that writes into a repository Fleet did
    /// not make**, which is why it exists under `auto_merge: never`: the policy
    /// says no machine decides that work lands, and a person merging on the
    /// forge instead skips the Checks Armada would have run.
    ///
    /// **Refused with a 409 anywhere but `awaiting_review`**, sharing that with
    /// the three acts beside it, and with a 409 where the record holds no pull
    /// request — a workflow declaring no delivering step opened none. A forge
    /// that would not merge answers with the kind it refused on: a protected
    /// base, a conflict and a required check are 409s the person acts on, and a
    /// missing tool or an unreadable refusal are 500s. **Nothing retries.**
    fn merge_pull_request(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `resolve_pull_request_conflict` — a person at the review gate sends the
    /// Job's branch back for a Drone that can edit files to bring it current
    /// with main. `#663`.
    ///
    /// **Fleet runs the rebase, never a Drone — a Drone has no git.** A clean
    /// result is pushed and nothing else moves. A conflicted one sends the Job
    /// back to the step before the one that delivers, the same
    /// `StepTarget::Returned` edge a workflow's own `verdict_routing` already
    /// uses to redo a step on purpose, so a Drone that can edit files reads the
    /// conflict as its opening brief — never the gate's own Drone, which is
    /// very often exactly the one that cannot.
    ///
    /// **Refused with a 409 anywhere but `awaiting_review`**, sharing that with
    /// the acts beside it, with a 409 where the record holds no open pull
    /// request, and with a 409 where the frozen workflow has no step before the
    /// one that delivers — a single step doing both, which this cannot redo
    /// without redelivering onto its own conflict.
    fn resolve_pull_request_conflict(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `rerun_failed_checks` — ask the forge to start the pull request's failed CI runs again.
    /// #905. A write to the forge, only from a press; nothing is posted and the Job does not move.
    fn rerun_failed_checks(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `investigate_failed_checks` — send the branch back for a Drone to find out why the
    /// pull request's CI failed. #905. Refused off the gate and where nothing failed.
    fn investigate_failed_checks(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `request_changes` — the work is not right yet, and here is what to fix.
    ///
    /// **It keeps everything**: the worktree, the branch and every step so far.
    /// The step does not advance, which is the whole difference between this
    /// and [`Commands::approve_review`].
    ///
    /// **Which status comes back depends on whether a Drone is there.** A live
    /// session is told — the note is a turn injected into it and the Job is
    /// `running`. A gate that stood its Drone down has nobody to tell, so the
    /// note is written onto the Job and `queued` comes back; the fresh Drone
    /// re-admission puts on the same step opens with it.
    ///
    /// **Refused where the *worktree* is gone**, not where the Drone is. There
    /// is then nowhere for the next pass to happen and no Drone the note could
    /// ever reach, and what is being asked for is a redispatch — the reading
    /// `job-statuses.toml`'s `awaiting_review` row gives. A blank note is
    /// refused as well: a Drone told nothing resumes with exactly the
    /// information that was not enough.
    fn request_changes(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        note: ChangesRequested,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `take_up_remarks` — the comments a person picked off the pull request
    /// reach a Drone. Nothing is written back onto the pull request.
    ///
    /// **A second entrance onto [`Commands::request_changes`]'s road, not a
    /// second road.** What it does to the Job is that act, called rather than
    /// restated, so what comes back is the same `queued` and the fresh Drone
    /// opens with the same block. What is different is where the words came
    /// from: the forge, read again on the press, rather than a person's
    /// keyboard.
    ///
    /// **The body carries handles and never words.** A client names the
    /// comments it picked and Fleet takes the text from the forge, so nothing
    /// on the far side of this seam decides what a Drone is told.
    ///
    /// **Refused, by name, on a comment already handed to a Drone on this Job**
    /// — a comment reads the same on a forge forever, and only Armada's own
    /// record can tell one that was worked from one that was not. Refused too
    /// on a comment the pull request no longer has, on a press naming none, and
    /// on everything `request_changes` refuses, a note already waiting
    /// included.
    fn take_up_remarks(
        &self,
        job_id: JobId,
        picked: RemarksTakenUp,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `dismiss_finding` — a person dismisses a finding the review raised, and says why. #907.
    ///
    /// **It moves nothing.** The finding leaves the review a person reads, the reason stays on
    /// the record, and the next review pass is handed both. Refused on a blank reason, and on
    /// a finding the Job's latest review did not raise.
    fn dismiss_finding(
        &self,
        job_id: JobId,
        dismissed: FindingDismissed,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `queue_after_finding` — a person turns a For context finding into a Job that waits on
    /// this one, so it starts when this one lands. #906. The Job itself moves nothing.
    fn queue_after_finding(
        &self,
        job_id: JobId,
        queued: FindingQueued,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `file_finding_issue` — a person confirms an issue drafted from a For context finding,
    /// and the forge files it as them. #906. Refused on a blank title.
    fn file_finding_issue(
        &self,
        job_id: JobId,
        filed: IssueFiled,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `override_verdict` — the Judge refused, a person disagrees, and the step
    /// advances anyway. **The fifth act on an escalated Job, and the only one
    /// that keeps the work.** [`Commands::approve_review`] cannot be it: a Job a
    /// gate refused is `escalated`, not `awaiting_review`.
    /// [`Commands::restart_step`] cannot either — it re-runs the step, discarding
    /// work that was right and possibly drawing the same refusal. A verdict with
    /// no appeal is worse than no verdict, because a verifier a person cannot
    /// overrule is one they route around.
    ///
    /// **It is not an approve-anything.** Only `gate_failure` is liftable — the
    /// Judge refusing a criterion, which is a matter of opinion. A step stopped
    /// on `gate_undecided` was never weighed and one stopped on
    /// `evidence_suspect` is a claim about the Drone's honesty; both are
    /// [`Refusal::IllegalMove`]. A failed mechanical Check is out of reach twice
    /// over: it ends the Job at `completed_failed`, which is terminal and stops
    /// no step, and the recorded Check runs are read again before anything moves.
    ///
    /// **It answers `queued`**, or `completed_success` on the last step: the
    /// verdict is not deferred and the Drone is — `#456`. **And it is recorded as
    /// an override** — the step move is `stopped -> advanced` carrying the
    /// trigger it overruled, so the row still says `failed` beside a state that
    /// says `advanced`, and [`ipc::StepDetail::overridden`] is that pair read
    /// once here rather than by every surface. A blank reason is
    /// [`Refusal::Unacceptable`]: an override that says nothing is how this
    /// becomes the way somebody quiets a gate.
    fn override_verdict(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        overruling: ipc::Overruled,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `rerun_gate` — the gate could not decide, and a person asks it again on
    /// the evidence the step already submitted.
    ///
    /// **The act [`Commands::override_verdict`] is deliberately not.** A step
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
    /// and [`Commands::restart_step`] is what applies.
    fn rerun_gate(&self, job_id: JobId)
        -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `show_again` — run the repository's harness against this Job's worktree
    /// now, and keep what it captured as a set of its own.
    ///
    /// **The one command here that takes the daemon by `Arc`**, and that is the
    /// whole of how it stays off the turn loop. A press runs a server and a
    /// spec, which takes as long as the app takes to start; the daemon spawns
    /// the run as a task of its own so nothing that turns Jobs waits on it, and
    /// a spawned task has to own what it runs on. The request waits for the
    /// task, and a client that stops waiting does not stop the run.
    ///
    /// **`picked` is one of the specs this Job's Drones named**, or `None` for
    /// the last one they named — which is what every press ran before a person
    /// could choose. Nothing outside that list reaches `evidence.run`.
    ///
    /// [`Refusal::IllegalMove`] before anything runs, naming what is missing: no
    /// harness declared, no worktree, no spec named, a spec no Drone named, the
    /// spec gone from the worktree, a Drone working in it, or a press already
    /// out. A harness that ran and captured nothing is a 200 carrying why.
    fn show_again(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        picked: Option<String>,
    ) -> impl Future<Output = Result<ipc::ShownAgain, Refusal>> + Send;

    /// `reject_job` — the work is not wanted, and the Job is over.
    ///
    /// **Terminal, which is what makes it the hard stop.** `rejected` is a
    /// verdict on the work rather than an operator clearing the Board, which is
    /// [`Commands::kill_job`]. The act for work that is nearly right is
    /// [`Commands::request_changes`], which keeps the Job.
    ///
    /// Refused anywhere but `awaiting_review`. `awaiting_approval -> rejected`
    /// is a legal edge and it belongs to `deny_dispatch`, which is a different
    /// act on a Job that has never run.
    fn reject_job(
        self: std::sync::Arc<Self>,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

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
        self: std::sync::Arc<Self>,
        job_id: JobId,
        filing: FileReport,
    ) -> impl Future<Output = Result<Report, Refusal>> + Send;

    /// `raise_cost_cap` — give this Job a higher cost ceiling than the tier
    /// above it allows.
    ///
    /// **The act the `over_budget` label has always pointed at**, and the one
    /// thing here that changes what a Job may spend. Until it the remedy was a
    /// machine-wide setting, raised for every Job at once and taken only on a
    /// restart. The dollar cap only, and
    /// [`raise_turn_cap`](Commands::raise_turn_cap) is the other: one signature
    /// moving either would be a body meaning two things.
    ///
    /// **It moves nothing** — no status, no step, no Drone. Admission was going
    /// to start one and was refused for money, so the next turn starts it, and
    /// the field a caller reads the summary for is `queued_reason`.
    ///
    /// [`Refusal::IllegalMove`] on a terminal Job. [`Refusal::Unacceptable`] on
    /// a figure at or under the cap in force — answering 200 while leaving the
    /// work stopped is what this route was built against — and again where the
    /// surface asked past its ceiling, which a person has none of.
    /// `crates/fleet/src/raising.rs` carries that ceiling and defends it.
    fn raise_cost_cap(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        raise: CapRaise,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `raise_turn_cap` — let this Job take more turns than the tier above it
    /// allows.
    ///
    /// **The other half of the label `over_budget` folds.** A `queued` Job
    /// carries that reason for either ceiling and `budget_hold` on the summary
    /// says which; this is the act for the turns, and until it there was none —
    /// no key, no route, and no number anywhere below a compile-time constant.
    ///
    /// **It moves nothing**, refuses in the same three ways as
    /// [`raise_cost_cap`](Commands::raise_cost_cap) and for the same reasons,
    /// and comes back as the summary a caller reads `queued_reason` off.
    fn raise_turn_cap(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        raise: TurnRaise,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `answer_question` — a person picks one of the answers a waiting Drone
    /// offered.
    ///
    /// **The Bridge half of the Drone's [`ask_question`](Tools::ask_question)**,
    /// and one act seen from its two ends: a Drone asked and stopped, and this
    /// starts it again. The answer goes into the live session as a turn, the
    /// delivery half `redirect_drone` already uses.
    ///
    /// **It moves nothing.** The Job is `running` before and after, and it comes
    /// back for the reason every other command answers with one — a caller folds
    /// the row rather than re-reading the board.
    ///
    /// **There is no field for prose.** [`ChosenAnswer::chose`] is one of the labels
    /// the Drone offered and a label matching none is refused, which keeps this from
    /// becoming the conversation `docs/scope.md` rejected. Words reach a Drone by
    /// redirect, and since 11.5 by a reject's note on [`Commands::answer_command`]
    /// — about the one command it refuses. Never through this one.
    ///
    /// [`Refusal::IllegalMove`] where nothing is outstanding, where the id names
    /// a question already answered, and where the label was not offered.
    fn answer_question(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        answer: ChosenAnswer,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `answer_command` — a person answers a command a Drone reached for and
    /// was not granted: allow it for this Job, allow it in the repository, or
    /// reject it.
    ///
    /// **Two moments, one call id.** A command a Drone is waiting on is
    /// answered in place and the Drone carries on in its session; a refused
    /// row on a Job stopped at `blocked_by_policy` is answered after the fact,
    /// and the Drone standing there is told or the step restarts.
    ///
    /// [`Refusal::IllegalMove`] where the call names nothing waiting or
    /// refused, and where the answer is not among the ones the command offers.
    fn answer_command(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        answer: AnswerCommand,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `set_when_blocked` — how this Job meets a command its Drone was not
    /// granted, changed while it runs. **Read by the next permission
    /// question**, so nothing respawns and nothing moves.
    fn set_when_blocked(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        setting: SetWhenBlocked,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `answer_judge` — a person answers the question a Judge refusal opened:
    /// agree with it, disagree for this step, or disagree and stand the
    /// criterion down for the repository. **The Job comes back moved**: the
    /// step fails exactly as it would have without this design, or advances.
    ///
    /// [`Refusal::IllegalMove`] where the Job is not holding a question open.
    fn answer_judge(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        answered: JudgeAnswered,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `set_when_refused` — how this Job meets a Judge criterion that
    /// refuses, changed while it runs. **Read by the next gate**, so nothing
    /// respawns and nothing moves.
    fn set_when_refused(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        setting: SetWhenRefused,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `set_model` — the model this Job's later steps spawn on, chosen by a
    /// person, or cleared back to each step's own. **Read by the next spawn**:
    /// the step running now keeps the model its Drone started with.
    ///
    /// [`Refusal::IllegalMove`] on a model this Fleet does not offer.
    fn set_model(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        choice: ipc::SetModel,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `set_review_model` — the model this Job's review step spawns on, chosen by a person,
    /// or cleared. #903. On the review step it beats `set_model`'s choice.
    ///
    /// [`Refusal::IllegalMove`] on a model this Fleet does not offer.
    fn set_review_model(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        choice: ipc::SetModel,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `remove_allowed_command` — take back a command a person allowed for
    /// this Job. The next reach for it is answered by the Job's setting again;
    /// one already written into armada.yml stays there.
    ///
    /// [`Refusal::IllegalMove`] where the Job has no such allow.
    fn remove_allowed_command(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        removing: ipc::RemoveAllowedCommand,
    ) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

    /// `start_run` — run one Check or Command in this Job's worktree, as a
    /// rehearsal: no Evidence, no Check row, nothing on the Job moves.
    ///
    /// **By `Arc`, for [`Commands::show_again`]'s reason**: the run is a task of
    /// its own and outlives the request. It answers once the run is underway;
    /// the output and the end arrive as events.
    ///
    /// [`Refusal::IllegalMove`] where the worktree is gone or a run is already
    /// out on this Job; [`Refusal::Unacceptable`] where nothing declares the
    /// name, or a narrowed run has nothing to narrow to.
    fn start_run(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        run: StartRun,
    ) -> impl Future<Output = Result<RunUnderway, Refusal>> + Send;

    /// `stop_run` — end the run's process group, and answer with its record
    /// once it is written. [`Refusal::IllegalMove`] on a run not in flight.
    fn stop_run(
        &self,
        job_id: JobId,
        run: NamedRun,
    ) -> impl Future<Output = Result<RunRecord, Refusal>> + Send;

    /// `undo_run` — put back what one run changed, from the snapshot taken
    /// before it. **Never a discard**: a Drone's work is uncommitted until
    /// delivery. [`Refusal::IllegalMove`] while a Drone works in the tree, while
    /// a run is in flight, or where a path the run changed has moved since.
    fn undo_run(
        &self,
        job_id: JobId,
        run: NamedRun,
    ) -> impl Future<Output = Result<RunRecord, Refusal>> + Send;

    /// `start_checkout_run` — run one Check or Command in the main checkout,
    /// as a rehearsal: no Evidence, no Check row, and no Job to move.
    ///
    /// **By `Arc`, for [`Commands::start_run`]'s reason.** It runs in the
    /// working tree as it is on disk — there is no throwaway copy and no Where
    /// control, Journey 9, *Running one*.
    ///
    /// **A checkout run and a Job's run do not lock each other out.** One run
    /// at a time is per owner: two runs in one tree fight over one build
    /// directory, and these are two trees.
    ///
    /// [`Refusal::IllegalMove`] where a run is already out in the checkout;
    /// [`Refusal::Unacceptable`] where nothing declares the name.
    fn start_checkout_run(
        self: std::sync::Arc<Self>,
        run: StartCheckoutRun,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<CheckoutRunUnderway, Refusal>> + Send;

    /// `stop_checkout_run` — end the run's process group, and answer with its
    /// record once it is written. [`Refusal::IllegalMove`] on a run not in
    /// flight.
    fn stop_checkout_run(
        &self,
        run: NamedRun,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<CheckoutRunRecord, Refusal>> + Send;

    /// `undo_checkout_run` — put back what one run changed, from the snapshot
    /// taken before it.
    ///
    /// **Never offered where no snapshot was taken.** This tree holds a
    /// person's own uncommitted work, which no Job's worktree does, so a run
    /// with nothing kept behind it is refused rather than discarded from.
    ///
    /// [`Refusal::IllegalMove`] while a run is in flight, on a run already
    /// undone or with no snapshot, and where a path the run changed has moved
    /// since.
    fn undo_checkout_run(
        &self,
        run: NamedRun,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<CheckoutRunRecord, Refusal>> + Send;

    /// `start_checkout_verify` — run setup and every Check once in the main
    /// checkout, one after another, each an ordinary checkout run: Journey 9,
    /// *Verify*. Answers once the first step is out.
    ///
    /// **By `Arc`, for [`Commands::start_run`]'s reason**: the steps outlive
    /// the call.
    ///
    /// `asked.workspace` names a directory below the root whose own
    /// `armada.yml` runs, in that directory, instead of the root's.
    ///
    /// [`Refusal::IllegalMove`] where a run or a Verify is already out in the
    /// checkout; [`Refusal::Unacceptable`] where there is nothing to run, or
    /// the workspace leaves the repository or its file will not load.
    fn start_checkout_verify(
        self: std::sync::Arc<Self>,
        asked: ipc::StartCheckoutVerify,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::CheckoutVerify, Refusal>> + Send;

    /// `save_manifest_file` — write a corrected `armada.yml` to disk, and stop
    /// there.
    ///
    /// **No staging, no commit, no formatting and no reserialisation.** The
    /// bytes the caller sends are the bytes on disk — Journey 9, *Editing*.
    ///
    /// **A write that does not parse is still a write.** Refusing invalid YAML
    /// would mean work in progress cannot be saved. What Fleet could not adopt
    /// arrives as `manifest.reread`, previous values still in force.
    ///
    /// **What comes back is not a reading.** The watch settles before it
    /// re-reads, so what the save moved is not known when this answers.
    ///
    /// [`Refusal::Fault`] where the file will not be written. Nothing about
    /// the text can refuse it.
    fn save_manifest_file(
        &self,
        save: SaveManifestFile,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ManifestSaved, Refusal>> + Send;

    /// `edit_manifest` — apply a form's edits to `armada.yml`, **changing only
    /// the lines they name**, and write the result.
    ///
    /// **What a form produces always loads**, unlike a save: the result is
    /// parsed before anything is written. [`Refusal::Unacceptable`] where it
    /// would not load, with every fault; where an edit names what the file
    /// does not hold; and where the file is written in a shape a form does not
    /// edit. [`Refusal::IllegalMove`] where the file moved under the edit, as
    /// for a save.
    fn edit_manifest(
        &self,
        edit: ipc::EditManifest,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::ManifestEdited, Refusal>> + Send;

    /// `edit_manifest_proposal` — one edit to one proposal. [`Refusal::Unacceptable`] for an
    /// unknown workspace or an edit that cannot apply; a bad value is Write's to refuse.
    fn edit_manifest_proposal(
        &self,
        asked: ipc::EditManifestProposal,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::ManifestProposal, Refusal>> + Send;

    /// `write_manifest_proposal` — create `armada.yml` from a proposal. Refused where it would
    /// not load or a file is already there, which is never written over.
    fn write_manifest_proposal(
        &self,
        asked: ipc::WriteManifestProposal,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::ManifestProposal, Refusal>> + Send;

    /// `start_server` — start a server the Manifest declares, in a Job's
    /// worktree on its span or in the main checkout on its own, **or answer
    /// with the instance already up**: one per Job per server.
    ///
    /// **By `Arc`, for [`Commands::show_again`]'s reason**: the server is a task
    /// of its own and outlives the request. It answers `starting`; the rest
    /// arrives as `server.*` events.
    fn start_server(
        self: std::sync::Arc<Self>,
        asked: ipc::StartServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::ServerState, Refusal>> + Send;

    /// `stop_server` — end a server's process group, and answer with the
    /// instance once it has ended. [`Refusal::IllegalMove`] on one not running.
    fn stop_server(
        &self,
        named: ipc::NamedServer,
    ) -> impl Future<Output = Result<ipc::ServerState, Refusal>> + Send;

    /// `save_limits` — save any of the three limits and answer with what is
    /// now in force. **An omitted field keeps its value.**
    ///
    /// **Nothing out of range reaches this.** `ipc::SaveLimits` cannot hold
    /// one, so the route refuses it as undecodable. It changes the next
    /// admission and stops nothing already running; [`Refusal::Fault`] where
    /// the save would not be written, and then nothing changed.
    fn save_limits(
        &self,
        save: ipc::SaveLimits,
    ) -> impl Future<Output = Result<ipc::FleetLimits, Refusal>> + Send;

    /// `save_preferences` — save one preference by name, and answer with what
    /// is now in force. **`limits`' shape one table over, one field at a
    /// time**: a save names a preference rather than the whole set, and every
    /// other preference is untouched.
    ///
    /// [`Refusal::Unacceptable`] where `name` is outside the closed set —
    /// refused by name, since `SavePreference.name` is a plain string and
    /// always decodes.
    fn save_preferences(
        &self,
        save: ipc::SavePreference,
    ) -> impl Future<Output = Result<ipc::Preferences, Refusal>> + Send;

    /// `remove_repository_allowed_command` — take back a rule a person
    /// always-allowed for this Manifest's repository. **Since `#836`.**
    ///
    /// **Fleet-wide, and touches no running Job**: read by the next permission
    /// question and the next spawn, so a Drone already granted the rule keeps
    /// it for the step it is on. Refused where nothing is spelled `run`.
    /// `add_repository` — serve one more repository, from a folder.
    ///
    /// [`Refusal::Unacceptable`] where the folder is not a git repository's
    /// root or its `armada.yml` will not load; [`Refusal::IllegalMove`] where
    /// it, or its Manifest id, is already served. A folder with no `armada.yml`
    /// is served, for Scan to read.
    fn add_repository(
        &self,
        asked: ipc::AddRepository,
    ) -> impl Future<Output = Result<ipc::RepositorySummary, Refusal>> + Send;

    /// `clone_repository` — clone from a URL into a new folder under `parent`,
    /// then serve it as [`add_repository`](Commands::add_repository) does.
    /// Answers when git finishes; its refusals are `add_repository`'s and three
    /// more, in `crates/ipc/operations.toml`.
    fn clone_repository(
        self: std::sync::Arc<Self>,
        asked: ipc::CloneRepository,
    ) -> impl Future<Output = Result<ipc::RepositorySummary, Refusal>> + Send;

    fn remove_repository_allowed_command(
        &self,
        removing: ipc::RemoveRepositoryAllowedCommand,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::RepositoryAllowedCommands, Refusal>> + Send;

    /// `add_task` — a person adds a task to the Job's plan, and the plan it
    /// leaves comes back. `#897`. Refused where the Job has no plan, or
    /// `after` names no task the plan holds — `crates/ipc/operations.toml`.
    /// **A working Drone is told**, mid-step; at a step boundary the next
    /// brief's THE PLAN carries it, and nothing respawns to deliver it.
    fn add_task(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        add: AddTask,
    ) -> impl Future<Output = Result<ipc::WorkPlan, Refusal>> + Send;

    /// `drop_task` — a person drops a task with a reason, and the plan it
    /// leaves comes back. Refused where the task is already `done` or
    /// already `dropped`, on `crates/ipc/operations.toml`'s terms. Delivery
    /// is [`Commands::add_task`]'s.
    fn drop_task(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        drop: DropTask,
    ) -> impl Future<Output = Result<ipc::WorkPlan, Refusal>> + Send;
}
