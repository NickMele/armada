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

use crate::daemon::Refusal;
use ipc::{
    ChangesRequested, ChosenAnswer, FileReport, JobExamined, JobForgotten, JobId, JobSummary,
    ProposeJob, Redirection, Redispatched, Report, RestartRequested, WorktreeReclaimed,
};

/// Everything a client asks Fleet to do.
pub trait Commands: Send + Sync + 'static {
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
    fn propose_from_request(
        &self,
        request: ipc::JobRequest,
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
    /// control, and a human act: `helm_access` on this row is `No`.
    ///
    /// **What comes back is `queued`, not `running`.** The dispatch is a
    /// turn's, because one inside this request died whenever a client stopped
    /// waiting for it — `fleet::daemon::Fleet::approve` and `#428`.
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
    /// under them, which is why it cannot be spelled as [`Commands::kill_drone`].
    fn kill_job(&self, job_id: JobId) -> impl Future<Output = Result<JobSummary, Refusal>> + Send;

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
        &self,
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
        &self,
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
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<WorktreeReclaimed, Refusal>> + Send;

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
    /// applies there is [`Commands::restart_step`]. It does not respawn — a
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
        &self,
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
        &self,
        job_id: JobId,
        note: ChangesRequested,
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
        &self,
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
    /// the Drone offered and a label matching none is refused rather than passed
    /// through; words reach a Drone through [`Commands::redirect_drone`] and
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
}
