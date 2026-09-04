//! The tools a working Drone calls, which are not operations and not Bridge's.
//!
//! **One of the three surfaces `Daemon` is composed of, and the one whose
//! caller is a different peer.** `crates/ipc/operations.toml` holds no row for
//! any of these: they come off the tool roster, they refuse through
//! [`NotRecorded`] rather than a status code, and every one is bound to a Job
//! the caller never names — [`Caller`] is the transport's word about the
//! connection and never the body's.
//!
//! A separate trait rather than a section of one, because it is a separate
//! seam. `crate::mcp` serves it, on the listener that was already there, and is
//! deliberately absent from [`SERVED`](crate::SERVED); a bound naming this
//! trait is a bound naming the Drone endpoint and nothing on the Fleet/Bridge
//! wire.

use std::future::Future;

use crate::mcp::Caller;
use ipc::mcp::{
    AskQuestion, CheckReport, DeclareScope, DispatchJob, NotRecorded, Receipt, RequestScope,
    SubmitEvidence,
};

/// Everything a Drone Fleet spawned may call.
pub trait Tools: Send + Sync + 'static {
    /// `ask_question` — the working Drone asks the person who approved the Job
    /// something it cannot answer from the repository, and offers the answers it
    /// will accept.
    ///
    /// **Not an inventory operation and not Bridge's**, exactly as
    /// [`Tools::submit_evidence`] is not: its caller is a Drone, which is why
    /// its refusal is [`NotRecorded`]. The Bridge half is
    /// [`Commands::answer_question`].
    ///
    /// # The receipt is not the answer
    ///
    /// It returns as soon as Fleet has taken the question, like submitting and
    /// unlike [`Tools::run_checks`]. Holding it open was rejected twice over: a
    /// person's wait has no budget that could bound an HTTP call, and an injected
    /// turn is consumed when the current tool call returns — so a Drone blocked
    /// inside this would swallow every redirect sent to unstick it. The answer
    /// arrives as a later turn in the Drone's own session.
    ///
    /// **One at a time**, because a Drone that could stack questions would be
    /// holding a conversation. Bound to a Job and a step the caller never names,
    /// for [`submit_evidence`](Tools::submit_evidence)'s reason.
    fn ask_question(
        &self,
        caller: Caller,
        asking: AskQuestion,
    ) -> impl Future<Output = Result<Receipt, NotRecorded>> + Send;

    /// `submit_evidence` — the Evidence tool, called by the Drone that is
    /// working. **Not an inventory operation and not Bridge's**: its caller is
    /// a Drone rather than Bridge, which is why its refusal is [`NotRecorded`]
    /// rather than [`Refusal`]. [`Tools::declare_scope`],
    /// [`Tools::run_checks`] and [`Tools::request_scope`] are the others.
    ///
    /// **The submission is bound to a Job the caller never names.** There is no
    /// `job_id` parameter and no `step_id`, so there is nothing to forge: the
    /// implementation attributes the submission to the Job whose Drone holds
    /// the connection it arrived on, which it knows and the caller cannot
    /// influence. A call arriving while that Job is not working is refused
    /// rather than queued, and [`Caller`] is the transport's word and never the
    /// body's — it carries the peer of the connection and nothing a caller
    /// wrote, so no arrangement of a request puts a Job id into it.
    ///
    /// **That is binding by construction and not authentication.** Any process
    /// that can reach the listener can make this call; what it cannot do is
    /// choose which Job the evidence lands against, and a caller the
    /// implementation cannot place is refused rather than guessed at.
    ///
    /// **It decides nothing.** The receipt says the submission was taken, not
    /// that it passed: a call that blocked while a repository's Checks ran
    /// would time out, so the gate runs afterwards and the outcome reaches the
    /// Drone as a later turn.
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
    /// [`submit_evidence`](Tools::submit_evidence)'s reason: there is no
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
    /// [`submit_evidence`](Tools::submit_evidence)'s reason. It moves nothing:
    /// what comes back is a receipt, and what the declaration does is give
    /// Fleet something to compare the worktree against — while the step runs,
    /// and again at the gate.
    fn declare_scope(
        &self,
        caller: Caller,
        declaration: DeclareScope,
    ) -> impl Future<Output = Result<Receipt, NotRecorded>> + Send;

    /// `request_scope` — the working Drone asks the **task's** stated scope to
    /// grow, because the work it was given needs a path the task does not name.
    ///
    /// # The other scope call, and what makes it a different call
    ///
    /// [`declare_scope`](Tools::declare_scope) states where a part's work will
    /// be, is replaced by calling it again, and decides nothing. This asks for
    /// the Job's own scope to change, which is a claim about the whole task —
    /// so it is answered rather than taken, and the answer is a Judge's.
    ///
    /// # It blocks, and what bounds the wait is a Judge budget
    ///
    /// [`run_checks`](Tools::run_checks) is the other call held open, and this
    /// is held open for the same reason: the outcome *is* the answer. What it
    /// is **not** is [`ask_question`](Tools::ask_question) — that one cannot
    /// be waited for because a person's wait has no budget, and this one is a
    /// model call with one on it.
    ///
    /// **Nothing moves while the call is out.** The Job is `running` when it is
    /// made and `running` when it returns, so the Drone keeps its session and
    /// its working slot and carries on the moment it answers. A refusal is the
    /// exception and escalates.
    ///
    /// Bound to a Job and a step the caller never names, for
    /// [`submit_evidence`](Tools::submit_evidence)'s reason.
    fn request_scope(
        &self,
        caller: Caller,
        request: RequestScope,
    ) -> impl Future<Output = Result<Receipt, NotRecorded>> + Send;

    /// `dispatch_job` — the working Drone asks for one more Job to exist, as a
    /// child of its own.
    ///
    /// **The one method on this trait whose effect is another Job.** Every
    /// other call answers a question or moves the Job it was made on; this one
    /// creates a record that will get a worktree, a Drone and a bill. So the
    /// receipt carries the id that was minted, and the refusals are about
    /// whether that Job was allowed to exist at all rather than about whether
    /// a field parsed.
    ///
    /// # It takes no parent id, for the reason none of the other three does
    ///
    /// [`Caller`] is the transport's word. There is no parameter through which
    /// a Drone could name the Job its children hang from, so one Drone minting
    /// work under another Drone's Job is not an arrangement of this call that
    /// exists.
    ///
    /// # The child is created approved, and this does not widen that
    ///
    /// `core_model::Job::create_sub_dispatched` enters at `queued` — already
    /// approved as part of its parent — and the implementation may reach it
    /// only from a step whose frozen workflow gave it the dispatching role,
    /// which is a step a person cleared the plan of. A Job that reached
    /// `queued` any other way is the approval gate weakened, and there is no
    /// path here that does it.
    fn dispatch_job(
        &self,
        caller: Caller,
        dispatch: DispatchJob,
    ) -> impl Future<Output = Result<Receipt, NotRecorded>> + Send;
}
