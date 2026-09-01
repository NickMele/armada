//! Why the loop could not carry a Job forward.
//!
//! # An illegal transition is in here, and that is the point
//!
//! [`Adrift::IllegalMove`] and [`Adrift::IllegalStepMove`] exist so that a move
//! the machine refuses **surfaces as a bug in Fleet** rather than being logged
//! and stepped over. There is no arm anywhere in this crate that turns one into
//! a warning and continues: `Job::transition` returning `Err` means Fleet asked
//! for something the registry says cannot happen, and a fallback for it would
//! be Fleet quietly disagreeing with the edge table.
//!
//! # The adapter failures are boxed and the domain failures are not
//!
//! `docs/contracts/error-contract.md` puts a typed leaf enum beside the code
//! that raised it and a real cause chain in the wrapper. The typed halves below
//! are the ones this crate can name: the store's, the machine's, the spawn
//! configuration's. The adapter halves arrive through a generic seam whose
//! error type is the implementation's — a `V::Error` cannot be a variant here
//! without making this enum generic over three parameters, which would put the
//! adapter's vocabulary into every signature in the crate. They are boxed, and
//! reachable through [`Error::source`].
//!
//! This is not `Box<dyn Error>` collapsing everything: every variant still says
//! *what Fleet was doing*, which is the thing a caller matches on, and the box
//! carries only the leaf underneath it.

use std::error::Error;
use std::io;

use adapter_traits::{NotDelivered, SpawnConfigRefused, WorktreeSpecRefused};
use core_model::{
    EscalationTrigger, IllegalDroneMove, IllegalStepTransition, IllegalTransition, JobId,
    JobStatus, RedirectAlreadyWaiting, StepId,
};
use store::{LoadAllError, LoadJobError, WriteError};

use crate::preparing::NotPrepared;
use crate::proposing::{NotProposed, Unresolved};
use crate::reporting::NotFiled;

/// A Job that could not be carried forward, and what stopped it.
#[derive(Debug)]
pub enum Adrift {
    /// The boot read failed outright. Not the partial-failure case — that one
    /// is carried alongside the Jobs that did load, because a caller must never
    /// be handed a short list with nothing saying so.
    BootRead(LoadAllError),
    /// One Job would not load.
    Reading(LoadJobError),
    /// A write did not land. The Job is where the log says it is, which is
    /// wherever it was before this.
    Writing(WriteError),
    /// **A bug in Fleet.** The machine refused a move Fleet asked for.
    IllegalMove(IllegalTransition),
    /// **A bug in Fleet**, for the inner machine.
    IllegalStepMove(IllegalStepTransition),
    /// **A bug in Fleet**, for the pointer a Drone's presence writes: a spawn
    /// onto a Job that already holds one.
    IllegalDroneMove(IllegalDroneMove),
    /// The Job's id or the configured repository root could not name a
    /// worktree.
    Unworkable {
        job: JobId,
        cause: WorktreeSpecRefused,
    },
    /// Version control would not create the worktree. Nothing was spawned.
    NoWorktree {
        job: JobId,
        cause: Box<dyn Error + Send + Sync>,
    },
    /// The worktree was cut and what the Manifest requires in one would not
    /// run. **Nothing was spawned, and no step was entered** — the Job is
    /// escalated before the first step moves, so this can never be read as a
    /// step that failed.
    ///
    /// **Boxed, and for the size rather than for the seam.** The leaf is this
    /// crate's own and could be inlined the way its neighbours are; what stops
    /// it is that it carries a command's captured output, which is 136 bytes
    /// against this enum's 104 and would grow every `Result<_, Adrift>` in the
    /// crate — which is to say every signature in it — to pay for the rarest
    /// variant in it.
    NotPrepared { job: JobId, cause: Box<NotPrepared> },
    /// The Job's own values would not make a confined spawn — a blank model, an
    /// unusable MCP config path, an environment variable that is not a name.
    NotConfigurable {
        job: JobId,
        cause: SpawnConfigRefused,
    },
    /// No Drone is running. Every reason is in the chain beneath.
    NoDrone {
        job: JobId,
        cause: Box<dyn Error + Send + Sync>,
    },
    /// The Drone's transcript, or the Job's log, could not be opened.
    ///
    /// **Nothing was spawned.** The record is opened before the Drone for that
    /// reason: a Job whose work cannot be written down is one a person should
    /// see, and a disk that refuses a file will refuse the worktree next.
    NoTranscript { job: JobId, cause: io::Error },
    /// The gate ruled and the Drone could not be told. **The step still
    /// advanced** — the ruling is Fleet's and does not depend on the Drone
    /// hearing it — so this says a session went deaf, not that a verdict was
    /// lost.
    NotTold { job: JobId, cause: io::Error },
    /// The last step advanced and the work would not commit.
    ///
    /// **The Job is `completed_success` anyway**, because its Checks passed and
    /// that is a fact about the work rather than about git. **Nothing is
    /// lost**: the worktree holds the change exactly as the Drone left it, and
    /// the cause below says how to take it by hand.
    ///
    /// No trigger names an infrastructure failure at the gate, so this
    /// escalates nothing rather than borrowing a trigger that means something
    /// else. `dispatch` used to name the same gap about its own failures and no
    /// longer does — `no_worktree`, `not_configurable` and `would_not_start`
    /// closed it upstream of a spawn. This one is still open because the
    /// failure is downstream of every verdict: the work passed, and a commit
    /// that would not run is not a Job that did not do it.
    NotCommitted {
        job: JobId,
        cause: Box<dyn Error + Send + Sync>,
    },
    /// The work is committed and getting it to the branch it merges into did
    /// not finish — the base would not read, the rebase would not run, the push
    /// or the pull request was refused.
    ///
    /// **Nothing is lost and no verdict changed.** A Job that reaches this at
    /// its last step is still `completed_success`, because that is a fact about
    /// its Checks; a Job that reaches it mid-workflow is still running and its
    /// Drone was still told the step advanced.
    ///
    /// Not boxed, unlike its neighbours: the seam answers one shape rather than
    /// an implementation's own enum, so there is nothing here to downcast to.
    NotDelivered {
        job: JobId,
        doing: &'static str,
        said: String,
    },
    /// The frozen workflow has no step by this name. A Job dispatched against a
    /// workflow that has since been edited, or a workflow with no steps at all.
    NoSuchStep { job: JobId, step: Option<StepId> },
    /// Whether a Drone's process had exited could not be established.
    /// **Not the same as "it is gone"** — folding a failed reading into absence
    /// is how a live Drone gets declared interrupted.
    NotReaped { job: JobId, cause: io::Error },
    /// A forget was asked for on a Job that has not reached a terminal
    /// status.
    ///
    /// **Not an illegal transition.** `forget_job` does not move the machine
    /// at all — it deletes the row — so there is no edge for a refusal to
    /// point at, only the status the Job is still standing in.
    NotForgettable { job: JobId, status: JobStatus },
    /// A redispatch was asked for on a Job that has not stopped.
    ///
    /// **Not an illegal transition**, which is why it is its own variant: the
    /// machine was never asked. A redispatch mints a replacement, so it is a
    /// refusal to *create* rather than to move, and there is no edge whose
    /// absence could have said so.
    NotRedispatchable { job: JobId, status: JobStatus },
    /// A redispatch was asked for on a Job that was rejected before it ran.
    ///
    /// **Its own variant because the reason differs in kind.** Every other
    /// refusal here is about where the Job is; this one is that nothing was
    /// ever produced to carry into a replacement.
    NeverRan { job: JobId },
    /// A redispatch was asked for on a Job that is a step of another Job.
    /// Replacing one is the parent's act, and nothing dispatches a sub-Job yet.
    NotReplaceable { job: JobId },
    /// A redispatch was asked for on a Job whose own `workflow_id` this Fleet
    /// no longer holds. The file it named was renamed or deleted after the
    /// Job was created — not the case `NoSuchWorkflow` names, which is a
    /// proposal, at creation, naming an id that never existed.
    WorkflowWithdrawn { job: JobId, workflow_id: String },
    /// A resume was asked for on a Job that is not waiting for a person.
    ///
    /// **Not an illegal transition.** The machine has `escalated -> running`
    /// and would have taken the move; what is missing is the escalation, so
    /// there is nothing for either act to hand back.
    NotResumable { job: JobId, status: JobStatus },
    /// A **restart** was asked for on a Job no step of which stopped.
    ///
    /// A Job-level escalation — `interrupted`, `resource_exhausted`,
    /// `dependency_failed`, `stalled` — names no step, so a restart has nowhere
    /// to land. For the first three the Drone is gone too, which leaves
    /// redispatch and Pilot; `stalled` holds a live session, and a redirect is
    /// what answers that one.
    ///
    /// **A redirect never reaches this.** It needs a session, not a step —
    /// which is why the two acts stopped sharing one predicate.
    NoStepStopped { job: JobId },
    /// A redirect was asked for on a Job whose Drone is gone.
    ///
    /// **The other act applies**, and this says so rather than respawning: a
    /// redirect that spawned would be a restart that lost nothing it could
    /// have kept, and the record would say a person injected context into a
    /// session that did not exist.
    NoDroneToRedirect { job: JobId },
    /// An answer that does not apply: nothing outstanding, the wrong question,
    /// a label never offered, or a pipe that would not take it. **One variant
    /// carrying a sentence rather than four** — the recourse is identical in all
    /// four, and `crate::questioning::NotAnswered` says which this is.
    NotAnswerable { job: JobId, because: String },
    /// A review act was asked for on a Job that is not at a human gate.
    ///
    /// **Not an illegal transition.** Two of the three moves a review makes are
    /// edges the machine has from elsewhere — `awaiting_approval -> rejected`
    /// is the dispatch gate's denial, and plenty of statuses reach `running` —
    /// so the machine would take them and the record would say a person
    /// reviewed work nobody was ever shown.
    NotUnderReview { job: JobId, status: JobStatus },
    /// Changes were asked for on a Job with no Drone and nowhere to put one.
    ///
    /// **Narrowed by `#207`, not deleted.** It used to mean "no process right
    /// now", which after `#140` was every gate: a Drone ends when its step
    /// passes the machine gates, so `awaiting_review` holds none and every
    /// request refused. A note has somewhere to wait now — the Job's own record
    /// — so the absence of a process is no longer a refusal.
    ///
    /// **What is left is the absence of a Job to put one on.** A worktree that
    /// has been reclaimed is a Job no Drone can be started against, so a note
    /// written for the next one would wait for a Drone that is never coming.
    /// That is the case where there is genuinely nowhere for the words to go,
    /// and it is refused while the Job is still at the gate rather than
    /// discovered a move later.
    NoDroneToTell { job: JobId },
    /// A second note arrived for a Job already holding an undelivered one.
    ///
    /// **The record refuses it and this carries the refusal out.** Overwriting
    /// drops the first note silently and queueing brings back the expiry the
    /// waiting rule was chosen to avoid — `core_model::RedirectAlreadyWaiting`
    /// has the reasoning. What the person gets back is their own words and the
    /// words already waiting, which is the one answer that loses neither.
    NoteAlreadyWaiting {
        job: JobId,
        held: RedirectAlreadyWaiting,
    },
    /// A Job's work product could not be read out of its worktree.
    ///
    /// **Never an empty diff.** A repository that will not open and a Drone
    /// that changed nothing are opposite answers, and a reviewer handed the
    /// second when the first happened would take work that was never read.
    WorkUnreadable {
        job: JobId,
        cause: Box<dyn Error + Send + Sync>,
    },
    /// An override was asked for on a step stopped by something no machine
    /// ruled on.
    ///
    /// **A person may overrule a decision and not the absence of one.** A step
    /// stopped on `gate_failure` or on `evidence_suspect` is a step a machine
    /// weighed and called wrong, and either is a call a person can disagree
    /// with. A step stopped on `gate_undecided` was never weighed at all, so
    /// advancing it would pass work nothing ruled on; the rest — a Check that
    /// hit its bound, evidence too large to read, a loop that did not
    /// converge — say the same thing in their own way. `overrulable` in
    /// `crate::overruling` is the list, arm by arm.
    NotTheJudges {
        job: JobId,
        step: StepId,
        trigger: EscalationTrigger,
    },
    /// An override was asked for on a step one of whose mechanical Checks did
    /// not pass.
    ///
    /// **`build` failing is not a matter of opinion**, and this is the guard
    /// that says so out of the record rather than out of the tier ordering. A
    /// refusal implies the mechanical tier held, so ordinarily this cannot
    /// fire; a gate that could not decide *after* running the Checks records
    /// what they did and stops the step, and that path can leave a stopped step
    /// with a failing Check on it. The check runs are read again before
    /// anything moves so that no arrangement of triggers turns this route into
    /// an approve-anything.
    CheckDidNotPass {
        job: JobId,
        step: StepId,
        check: String,
    },
    /// The gate was asked again on a step it had already ruled on.
    ///
    /// **[`NotTheJudges`](Adrift::NotTheJudges) from the other side**, and the
    /// two partition the triggers: an override lifts a decision, a re-run
    /// answers the absence of one, and no trigger reaches both. Asking a gate
    /// that ruled the same question draws the same answer.
    NotUndecided {
        job: JobId,
        step: StepId,
        trigger: EscalationTrigger,
    },
    /// The gate was asked again on a Job Fleet is no longer standing at.
    ///
    /// **The baseline lives in the working slot and nowhere else.** A re-run
    /// derives the same artifacts against the same reading of what the worktree
    /// held when the step began; a Fleet restarted since the escalation has
    /// none, so `diff_nonempty` would decide on nothing known to have moved and
    /// the second reading would answer a different question from the first.
    /// `restart_step` puts a fresh Drone on the worktree the last one left.
    NotStandingThere { job: JobId },
    /// The gate was asked again on a step with no evidence to be asked about.
    ///
    /// **Both halves of it**: nothing was recorded, or what was recorded will
    /// not read back as a submission — a row written by something that did not
    /// share `verification`'s constructor. Ordinarily unreachable, because the
    /// gate records the evidence it ruled on whatever it ruled.
    NothingToRuleOn { job: JobId, step: StepId },
    /// A restart was asked for on a Job whose Drone is alive.
    ///
    /// The inverse refusal. Ending a live session to spawn a replacement onto
    /// the same worktree throws away the context that makes a redirect cost
    /// nothing.
    DroneStillThere { job: JobId },
    /// A restart was asked for on a Job whose worktree is no longer on disk.
    ///
    /// **Nothing is spawned.** The earlier steps' work lived in that directory,
    /// so what is being asked for is a Job that starts again from the approval
    /// gate — which is a redispatch, and is named as one rather than silently
    /// performed.
    WorktreeGone { job: JobId, path: String },
    /// A proposal carried a title nothing could be picked out of a list by.
    Unnameable,
    /// An override carried no reason.
    ///
    /// **Nothing else on this route would hold it.** A redirect with a blank
    /// note is refused because a Drone told nothing resumes with exactly the
    /// information that failed; here nothing is told anything, so what an empty
    /// string loses is the only account there will ever be of why a verdict was
    /// overruled — which is the half a rate cannot supply.
    Unreasoned { job: JobId },
    /// A report could not be made from what was sent, in [`NotFiled`]'s words.
    ///
    /// **Not [`Adrift::Unreasoned`]**, which it shared until a filing with a
    /// long reason and an orphan `step_id` was told its reason was missing: one
    /// refusal for two acts could describe only one of them.
    NotFileable { job: JobId, cause: NotFiled },
    /// A proposal named a workflow this Fleet does not hold.
    ///
    /// **Nothing checked this until now.** `ResolvedWorkflow` carried no id, so
    /// the proposal's was written onto the record unverified and the Job sat on
    /// the board claiming a workflow Fleet had never heard of — the same class
    /// as the blank model, a value that cannot work accepted where it enters.
    ///
    /// **`held` names every id this Fleet holds**, not one — `.armada/workflows/`
    /// may declare more than one definition, and a caller needs the whole list
    /// to pick a name that will not be refused a second time.
    NoSuchWorkflow { named: String, held: Vec<String> },
    /// A proposal named a Manifest this Fleet does not hold. The same fault as
    /// [`Adrift::NoSuchWorkflow`], for the other id a proposal carries.
    NoSuchManifest { named: String, held: String },
    /// A proposal named a peer Job this Fleet does not hold.
    ///
    /// The third of the same fault, for the ids on `dependencies` — and the one
    /// that buys more than a good message. An edge may only point at a Job that
    /// already exists, so every edge points backwards in time and a cycle is
    /// unrepresentable rather than merely undetected. Unenforced, two Jobs each
    /// naming the other were both permanently unadmittable behind a Board label
    /// that reads as ordinary waiting.
    NoSuchPeer { named: String },
    /// A request named a tool call this Job's transcripts do not carry.
    ///
    /// **The Job is there and the call is not**, which is why it is this rather
    /// than a missing Job: an id read off a row that is no longer in the record
    /// — transcripts reclaimed, or a Fleet that never wrote them — and a
    /// caller told the Job does not exist would go looking for the wrong thing.
    NoSuchCall { named: String },
    /// A proposal named no model and nothing configured supplies one.
    ///
    /// Refused **at creation**, which is the whole point: the same value used
    /// to be accepted, stored, shown on the board looking approvable, and
    /// refused at spawn as "no model was named". The message names the settings
    /// row to set rather than the layer that failed.
    Modelless,
    /// An attachment's bytes could not be gotten where they needed to be —
    /// at creation, a staged path that does not exist or cannot be read; at
    /// dispatch, Fleet's own stored copy that would not copy into the fresh
    /// worktree.
    ///
    /// **Refused, not dropped, either time.** A person who believed they
    /// attached a screenshot to the brief and got a Job that silently carries
    /// none — or a Drone briefed about a file that is not where the brief
    /// says — is worse off than being told outright. The same argument
    /// [`Adrift::NoSuchWorkflow`] and [`Adrift::NoSuchManifest`] make about a
    /// value that cannot work being accepted where it enters.
    AttachmentUnreadable {
        job: JobId,
        filename: String,
        cause: io::Error,
    },
    /// A request arrived with nothing in it to read.
    NothingToPropose,
    /// The repository root or the minted id could not name a checkout for the
    /// reading. **Unreachable in practice** — both are Fleet's own values —
    /// and carried rather than unwrapped for the reason every other
    /// `WorktreeSpecRefused` is.
    NoReadingWorktree(WorktreeSpecRefused),
    /// The reading's checkout would not be made, so the proposer had nowhere
    /// to look. **Nothing was asked and nothing was created.**
    NoReading {
        request: String,
        cause: Box<dyn Error + Send + Sync>,
    },
    /// The proposal was made and the reading's checkout would not go back.
    ///
    /// **The Jobs are not created and the person is told.** A reading nothing
    /// removes is a directory nothing owns — `armada clean` deletes what a
    /// record derives and a reading has no record — so it would be reported as
    /// unclaimed for ever, and silence here is how a repository fills up with
    /// them.
    ReadingNotDiscarded {
        path: String,
        cause: Box<dyn Error + Send + Sync>,
    },
    /// The proposer call could not be configured — the same environment a Drone
    /// and a Judge are built from would not build.
    NotProposable(SpawnConfigRefused),
    /// **The request was read and no workflow resolved.** It is refused at
    /// dispatch and comes back unchanged, and no Job exists.
    ///
    /// Not a fault and never a default: the resolved definition is frozen into
    /// the Job and becomes the yardstick the work is judged against, so a
    /// nearest fit would be the standard rather than a guess a person could
    /// correct.
    NoWorkflowFits { request: String, why: Unresolved },
    /// **The call could not be made, which is a different thing.** The network,
    /// the quota, the budget, or an answer nothing could be read out of.
    ///
    /// Deliberately not [`Adrift::NoWorkflowFits`]. A proposal that could not
    /// be made says nothing about the request, and turning an outage into "no
    /// workflow fits" refuses a dispatch on the strength of one.
    NotProposed { request: String, cause: NotProposed },
}
impl Adrift {
    /// A refusal from the delivery seam, named against the Job it was for.
    ///
    /// One constructor rather than the same three-field literal at six call
    /// sites, which is where the two halves would come to disagree about which
    /// is `doing` and which is `said`.
    pub(crate) fn from_delivery(job: &JobId, why: NotDelivered) -> Adrift {
        Adrift::NotDelivered {
            job: job.clone(),
            doing: why.doing,
            said: why.said,
        }
    }
}
