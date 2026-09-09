//! Which of the four refusals a failure is, and the code it carries.
//!
//! Split from [`serving`](mod@crate::serving) at the 900-line refusal, and
//! along the seam that file's own header already names: the trait impl decides
//! *what* to answer, and this decides *how a failure is spelled on the wire*.
//! Every `WireError` Fleet raises is raised here.
//!
//! **The choice is made from the typed leaf error and never from a message.**
//! `api::Refusal` has four variants because a caller has four things to do
//! about them — retry, fix the request, look somewhere else, or report a fault
//! — and a string match would make that mapping depend on wording.
//!
//! **The codes are declared here rather than in a registry**, beside the thing
//! that raises them: the set is closed by collection, and a central list would
//! put every code far from the failure it names.

use adapter_traits::{AgentHarness, Delivery, NotMerged, Vcs, WorkProduct};
use api::Refusal;
use ipc::{RunId, WireError, WireValue};
use store::{LoadJobError, ResolveJobError, WriteError};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::judging::CallFailed;
use crate::proposing::NotProposed;

/// The codes this boundary raises, declared beside the thing that raises them.
///
/// The set is closed by collection rather than by authorship — a central
/// registry would put every code far from the failure it names.
const NO_SUCH_JOB: &str = "fleet.no_such_job";
const ILLEGAL_MOVE: &str = "fleet.illegal_move";
const FAULT: &str = "fleet.fault";
/// A proposal that decoded and names something that cannot produce a Drone.
const UNACCEPTABLE: &str = "fleet.unacceptable_proposal";
/// The request was read and no workflow fits. **A refusal about the request**,
/// and the reason it has a code of its own: a caller reading `UNACCEPTABLE`
/// cannot tell it from a proposal naming a workflow that does not exist.
const NO_WORKFLOW_FITS: &str = "fleet.no_workflow_fits";
/// The proposer call could not be made. **Never the code above** — a client
/// that rendered an outage as "nothing fits" would tell a person their request
/// was refused when it was never read.
const PROPOSER_UNREACHABLE: &str = "fleet.proposer_unreachable";
/// A person watching the proposer decided not to wait, and stopped it.
///
/// **Its own code because it is not a failure**, which is the same argument
/// `NO_WORKFLOW_FITS` makes against `UNACCEPTABLE` one step along: a client
/// that rendered this as `PROPOSER_UNREACHABLE` would tell somebody Armada
/// broke when what happened is that they pressed a control Armada offered them.
/// Nothing was created and what they typed comes back, so the surface returns
/// them to the form rather than to an error.
const PROPOSER_STOPPED: &str = "fleet.proposer_stopped";
/// A redispatch asked for on a Job that is not waiting for a person. A 409 like
/// a refused move, and a code of its own because the machine was never asked.
const NOT_REDISPATCHABLE: &str = "fleet.not_redispatchable";
/// A review act asked for on a Job that is not standing at a human gate. Its
/// own code because a caller reading `ILLEGAL_MOVE` would look for an edge that
/// exists — the machine was never asked.
const NOT_UNDER_REVIEW: &str = "fleet.not_under_review";
/// A note left for the next Drone on a Job already holding one nobody has
/// opened with. A 409 like the conflicts above, and its own code because two
/// acts write that note now — a caller reading [`NOT_UNDER_REVIEW`] off a
/// restart would go looking at the Job's status and find nothing wrong with it.
/// The message carries the held note, so the person keeps both sets of words.
const NOTE_ALREADY_WAITING: &str = "fleet.note_already_waiting";
/// An act on a stopped step asked for on a Job that has no stopped step to act
/// on, or one whose step stopped for a reason the act does not answer. A 409
/// for [`NOT_UNDER_REVIEW`]'s reason — the machine was never asked, so a caller
/// reading `ILLEGAL_MOVE` would go looking for an edge that is there.
///
/// **These reached a caller as 500s.** A resume on a Job that is not escalated
/// is the caller asking for the wrong act, not Fleet breaking, and a 500 sends
/// them to retry something that will fail identically for ever.
const NOT_RESUMABLE: &str = "fleet.not_resumable";
/// A forget asked for on a Job that has not reached a terminal status. A 409
/// like the other status conflicts — the machine was never asked, only the
/// row itself, and `kill_job` is the act on a Job still in flight.
const NOT_FORGETTABLE: &str = "fleet.not_forgettable";
/// A reclaim asked for on a Job that has not reached a terminal status. A 409
/// beside [`NOT_FORGETTABLE`] and not the same code: the two acts refuse on the
/// same predicate and a client telling a person which one to try next has to be
/// able to tell them apart.
const NOT_RECLAIMABLE: &str = "fleet.not_reclaimable";
/// A raise on a Job that is over. Its own code beside the two above, because
/// the act a person is told to try instead is a redispatch and not a kill.
const NOT_CAPPABLE: &str = "fleet.not_cappable";
/// The repository a Job's worktree is in would not open. **A 500 and never a
/// 200 with both halves absent** — a client reading "nothing to reclaim" would
/// draw a Job whose disk is back when the disk is still there.
const NOT_RECLAIMED: &str = "fleet.not_reclaimed";
/// Nothing on this machine could say what is on a pull request. **A 500**, for
/// [`MERGE_NO_TOOL`]'s reason: nothing about the request is wrong and asking
/// again is reasonable. It is never an empty list — a pull request nobody has
/// commented on and a forge nobody could reach are opposite answers.
const REVIEW_UNREADABLE: &str = "fleet.review_unreadable";
/// A press that named no comment at all. **A 422**, like a blank note: the
/// request decoded and names something that cannot work.
const NO_REMARKS_CHOSEN: &str = "fleet.no_remarks_chosen";
/// A press naming comments the pull request no longer has. A 409: the whole
/// press is refused, and what a person does about it is read the pull request
/// again and choose again.
const REMARKS_GONE: &str = "fleet.remarks_gone";
/// A press naming comments a Drone on this Job has already been handed. A 409
/// beside [`NOTE_ALREADY_WAITING`] and its own code for that pair's reason: one
/// is a note nothing has collected yet and this is one already delivered, and a
/// client telling a person what to do next has to tell them apart.
const REMARKS_ALREADY_TAKEN_UP: &str = "fleet.remarks_already_taken_up";
/// A merge asked for on a Job whose record holds no pull request. A 409 like
/// the status conflicts above: nothing was ever opened, so the act a person
/// wants is an approval and the message says so.
const NOTHING_TO_MERGE: &str = "fleet.nothing_to_merge";
/// The base branch is protected. **Its own code, and the whole reason the
/// refusal kinds are not one**: what answers this is an administrator or a
/// rule, and nothing a person does to the Job changes it.
const MERGE_BRANCH_PROTECTED: &str = "fleet.merge_branch_protected";
/// The branch and its base disagree. Its own code because the answer is on the
/// branch — a `request_changes` that asks for a rebase, or a person resolving
/// it — and never on the forge.
const MERGE_CONFLICTED: &str = "fleet.merge_conflicted";
/// A check the **forge** requires has not passed. Its own code because the
/// answer is to wait or to fix that check, and because a client reading
/// `CHECK_DID_NOT_PASS` would go looking at Armada's own Checks, which all
/// passed or the Job would not be at a gate.
const MERGE_CHECKS_NOT_PASSED: &str = "fleet.merge_checks_not_passed";
/// The pull request is closed, gone, or was never there. Its own code because
/// the Job is answerable and the pull request is not: what is left is an
/// approval or a redispatch.
const MERGE_NOT_OPEN: &str = "fleet.merge_not_open";
/// Nothing on this machine could ask the forge. **A 500**, unlike the four
/// above: nothing about the request is wrong and asking again is reasonable
/// once whoever runs Fleet has signed in.
const MERGE_NO_TOOL: &str = "fleet.merge_no_tool";
/// The forge refused and said something this vocabulary has no name for. A 500
/// for [`MERGE_NO_TOOL`]'s reason, and the sentence the forge printed is in the
/// message — which is the honest answer where a guess would send a person to
/// fix the wrong thing.
const MERGE_REFUSED: &str = "fleet.merge_refused";

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Which of the three refusals a failure is, decided from its type.
    ///
    /// **`run_id` names this process**, not Fleet's-by-assumption: it is minted
    /// where the emitter is, which is here.
    pub(crate) fn refusal(&self, why: Adrift) -> Refusal {
        let said = why.to_string();
        match &why {
            Adrift::Reading(LoadJobError::NoSuchJob { job_id })
            | Adrift::Writing(WriteError::NoSuchJob { job_id }) => Refusal::NoSuchJob(
                WireError::raised(NO_SUCH_JOB, said, self.run_id())
                    .about_job(ipc::JobId::from(job_id)),
            ),
            // The same 404 one segment earlier, and with no Job on it: what the
            // caller said resolved to nothing, so there is no id to name. A
            // database that would not answer is the fault it always was.
            Adrift::Unresolvable(ResolveJobError::Database(_)) => {
                Refusal::Fault(WireError::raised(FAULT, said, self.run_id()))
            }
            Adrift::Unresolvable(_) => {
                Refusal::NoSuchJob(WireError::raised(NO_SUCH_JOB, said, self.run_id()))
            }
            // The machine refused the move. The caller asked for something the
            // edge table does not have — approving a Job already running,
            // killing one already over — and 409 is the answer to that.
            Adrift::IllegalMove(_) | Adrift::IllegalStepMove(_) => {
                Refusal::IllegalMove(WireError::raised(ILLEGAL_MOVE, said, self.run_id()))
            }
            // The same conflict, from a request the machine never saw: the Job
            // is somewhere a replacement would mean nothing.
            // The Job is not at a human gate. A 409 like the conflicts above,
            // and never a 500: the machine was never asked, so there is no edge
            // for a caller to go looking for.
            Adrift::NotUnderReview { job, .. } | Adrift::NoDroneToTell { job } => {
                Refusal::IllegalMove(
                    WireError::raised(NOT_UNDER_REVIEW, said, self.run_id())
                        .about_job(ipc::JobId::from(job)),
                )
            }
            // A note over a note. Its own code since `restart_step` became the
            // second act that writes one: a caller reading `NOT_UNDER_REVIEW`
            // off a restart would go and look at the Job's status, and find
            // nothing wrong with it. The message carries both sets of words.
            Adrift::NoteAlreadyWaiting { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOTE_ALREADY_WAITING, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // A forget on a Job that is not yet terminal. The machine was
            // never asked — there is no move to refuse, only a row that is
            // still live.
            Adrift::NotForgettable { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOT_FORGETTABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // A reclaim on a Job that is not yet terminal. The same shape as
            // the forget above and a code of its own, because the act a person
            // is told to try instead is not the same one.
            Adrift::NotReclaimable { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOT_RECLAIMABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // A raise on a Job that is over — the same shape as the two above
            // and the opposite half of their predicate. A code of its own for
            // their reason: what a person does instead is a redispatch.
            Adrift::NotCappable { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOT_CAPPABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // The repository would not open, so neither half was attempted.
            // Fleet's own ground failed and nothing about the request is
            // wrong, which is the 500 this variant is for.
            Adrift::NotReclaimed { job, .. } => Refusal::Fault(
                WireError::raised(NOT_RECLAIMED, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // What an act on a stopped step refuses with — plus a redirect
            // asked for with no Drone, or a restart asked for with one still
            // there or its worktree gone, the same two acts refusing the
            // other's precondition. `NotTheJudges` and `CheckDidNotPass` are
            // an override's; the rest are a gate re-run's.
            Adrift::NotResumable { job, .. }
            | Adrift::NoStepStopped { job }
            | Adrift::NoDroneToRedirect { job }
            | Adrift::NotAnswerable { job, .. }
            | Adrift::DroneStillThere { job }
            | Adrift::WorktreeGone { job, .. }
            | Adrift::NotTheJudges { job, .. }
            | Adrift::CheckDidNotPass { job, .. }
            | Adrift::NotUndecided { job, .. }
            | Adrift::NotStandingThere { job }
            | Adrift::NothingToRuleOn { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOT_RESUMABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            Adrift::NotRedispatchable { job, .. }
            | Adrift::NeverRan { job }
            | Adrift::NotReplaceable { job }
            | Adrift::WorkflowWithdrawn { job, .. } => Refusal::IllegalMove(
                WireError::raised(NOT_REDISPATCHABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // The request is well-formed and the values in it cannot work. Not
            // a 500: retrying it will fail identically forever, and the message
            // names what to send instead.
            // A cap that raises nothing and a ceiling Helm went past are both
            // well-formed requests carrying a value that cannot work, and the
            // message names the figure to send instead. **Not a 409**: neither
            // is about where the Job stands, and a caller reading a conflict
            // off one would go and look at a status with nothing wrong with it.
            Adrift::Unnameable
            | Adrift::CapNotRaised { .. }
            | Adrift::CapAboveCeiling { .. }
            | Adrift::Unreasoned { .. }
            | Adrift::NotFileable { .. }
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::NoSuchCall { .. }
            | Adrift::NoSuchCheckOutput { .. }
            | Adrift::NoSuchFrame { .. }
            | Adrift::Modelless
            | Adrift::NothingToPropose
            | Adrift::AttachmentUnreadable { .. } => {
                Refusal::Unacceptable(WireError::raised(UNACCEPTABLE, said, self.run_id()))
            }
            // The request was read and declined, and it goes back on the field
            // rather than being echoed in the message: what the person retypes
            // or hands to `propose_job` is what they wrote, character for
            // character. No Job exists.
            Adrift::NoWorkflowFits { request, .. } => Refusal::Unacceptable(
                WireError::raised(NO_WORKFLOW_FITS, said, self.run_id())
                    .with_field("request", WireValue::Str(request.clone())),
            ),
            // A call that could not be made, which is not that refusal — 500,
            // because nothing about the request is wrong and asking again is
            // reasonable. It comes back on the same field either way.
            // `NotProposable` falls to the catch-all below: a proposer that
            // could not be configured is Fleet's own fault and carries no
            // request to return.
            // Somebody stopped it. **Told apart before the outage arm**, and
            // still a `Fault` on the transport — 500 is what says no Job
            // exists, and the code is what says whose doing that was.
            Adrift::NotProposed {
                request,
                cause: NotProposed::Call(CallFailed::Stopped),
            } => Refusal::Fault(
                WireError::raised(PROPOSER_STOPPED, said, self.run_id())
                    .with_field("request", WireValue::Str(request.clone())),
            ),
            Adrift::NotProposed { request, .. } => Refusal::Fault(
                WireError::raised(PROPOSER_UNREACHABLE, said, self.run_id())
                    .with_field("request", WireValue::Str(request.clone())),
            ),
            // The forge would not answer. A 500 and never an empty answer.
            Adrift::ReviewUnreadable { job, pull_request } => Refusal::Fault(
                WireError::raised(REVIEW_UNREADABLE, said, self.run_id())
                    .about_job(ipc::JobId::from(job))
                    .with_field("pull_request", WireValue::Str(pull_request.clone())),
            ),
            // A press that picked nothing. A 422, like a blank note.
            Adrift::NoRemarksChosen { job } => Refusal::Unacceptable(
                WireError::raised(NO_REMARKS_CHOSEN, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // **The handles come back on both of these**, for
            // `NOTE_ALREADY_WAITING`'s reason: a refusal that named only a
            // count would leave a person to work out which of the comments
            // they picked was the problem.
            Adrift::RemarksGone { job, gone } => Refusal::IllegalMove(
                WireError::raised(REMARKS_GONE, said, self.run_id())
                    .about_job(ipc::JobId::from(job))
                    .with_field("remarks", WireValue::Str(gone.join(", "))),
            ),
            Adrift::RemarksAlreadyTakenUp { job, already } => Refusal::IllegalMove(
                WireError::raised(REMARKS_ALREADY_TAKEN_UP, said, self.run_id())
                    .about_job(ipc::JobId::from(job))
                    .with_field("remarks", WireValue::Str(already.join(", "))),
            ),
            // Nothing was ever opened, so there is no forge answer to name.
            Adrift::NothingToMerge { job } => Refusal::IllegalMove(
                WireError::raised(NOTHING_TO_MERGE, said, self.run_id())
                    .about_job(ipc::JobId::from(job)),
            ),
            // **One code per kind, decided from the typed kind and never from
            // the sentence.** This is the arm the decision "a refused merge
            // says which of the reasons it was" lands on: `Adrift` carries one
            // variant because no arm in this crate does anything different
            // about them, and here is where they stop being one thing —
            // because a client does.
            //
            // The four that are the person's to answer are 409s; the two that
            // are this machine's are 500s, for `NOT_RECLAIMED`'s reason.
            Adrift::NotMerged { job, why } => {
                let job = ipc::JobId::from(job);
                let raised = |code: &'static str| {
                    WireError::raised(code, said.clone(), self.run_id())
                        .about_job(job.clone())
                        .with_field("refused", WireValue::Str(why.kind().to_string()))
                };
                match why {
                    NotMerged::Protected { .. } => {
                        Refusal::IllegalMove(raised(MERGE_BRANCH_PROTECTED))
                    }
                    NotMerged::Conflicted { .. } => Refusal::IllegalMove(raised(MERGE_CONFLICTED)),
                    NotMerged::ChecksNotPassed { .. } => {
                        Refusal::IllegalMove(raised(MERGE_CHECKS_NOT_PASSED))
                    }
                    NotMerged::NotOpen { .. } => Refusal::IllegalMove(raised(MERGE_NOT_OPEN)),
                    NotMerged::NoTool { .. } => Refusal::Fault(raised(MERGE_NO_TOOL)),
                    NotMerged::Refused { .. } => Refusal::Fault(raised(MERGE_REFUSED)),
                }
            }
            _ => Refusal::Fault(WireError::raised(FAULT, said, self.run_id())),
        }
    }

    /// A worktree that would not be read, named against the Job it was for.
    ///
    /// **A 500 and never an empty diff.** A repository that will not open and a
    /// Drone that changed nothing are opposite answers, and a reviewer handed
    /// the second when the first happened would take work nobody read.
    pub(crate) fn unreadable<E: std::error::Error + Send + Sync + 'static>(
        &self,
        job: &core_model::JobId,
        cause: E,
    ) -> Refusal {
        self.refusal(Adrift::WorkUnreadable {
            job: job.clone(),
            cause: Box::new(cause),
        })
    }

    /// This process's run id.
    ///
    /// **Not minted here per call.** It is the id of the emitter, and the
    /// emitter is one process — so it is derived from the mint once and held,
    /// which is what `run_id` names.
    pub(crate) fn run_id(&self) -> RunId {
        RunId::carried(self.run().as_str())
    }
}
