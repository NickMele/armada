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
use std::fmt;
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
    /// else — the same gap `dispatch` names about its own failures.
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
    /// An answer to a question that is not outstanding, is not the one
    /// outstanding, names a label the Drone never offered, or would not go down
    /// the pipe. **One variant carrying a sentence rather than four**: all four
    /// are the same act refused for a reason the Job's state does not name, and
    /// the recourse is identical — `crate::asking::NotAnswered` says which.
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

impl fmt::Display for Adrift {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Adrift::BootRead(cause) => write!(out, "the boot read failed: {cause}"),
            Adrift::Reading(cause) => write!(out, "a Job could not be read: {cause}"),
            Adrift::Writing(cause) => write!(out, "a write did not land: {cause}"),
            Adrift::IllegalMove(cause) => {
                write!(out, "Fleet asked for a move the machine refuses: {cause}")
            }
            Adrift::IllegalStepMove(cause) => write!(
                out,
                "Fleet asked for a step move the machine refuses: {cause}"
            ),
            Adrift::IllegalDroneMove(cause) => {
                write!(
                    out,
                    "Fleet asked to put a drone where it cannot go: {cause}"
                )
            }
            Adrift::Unworkable { job, cause } => {
                write!(
                    out,
                    "no worktree can be named for {}: {}",
                    job.as_str(),
                    cause.said()
                )
            }
            Adrift::NoTranscript { job, cause } => write!(
                out,
                "{}'s work could not be written down and nothing was spawned: {cause}",
                job.as_str()
            ),
            Adrift::NoWorktree { job, cause } => write!(
                out,
                "{} has no worktree and nothing was spawned: {cause}",
                job.as_str()
            ),
            Adrift::NotPrepared { job, cause } => write!(
                out,
                "{} has a worktree and no step ran in it: {cause}",
                job.as_str()
            ),
            Adrift::NotConfigurable { job, cause } => write!(
                out,
                "{} could not be turned into a confined spawn: {}",
                job.as_str(),
                cause.said()
            ),
            Adrift::NoDrone { job, cause } => {
                write!(out, "{} has no Drone: {cause}", job.as_str())
            }
            Adrift::NotTold { job, cause } => write!(
                out,
                "the gate ruled on {} and the Drone could not be told: {cause}",
                job.as_str()
            ),
            Adrift::NotDelivered { job, doing, said } => write!(
                out,
                "{}'s work is committed and {doing} did not happen: {said}. The branch holds \
                 the change and every Check passed",
                job.as_str()
            ),
            Adrift::NotCommitted { job, cause } => write!(
                out,
                "{} finished and its work would not commit: {cause}. The Job succeeded and the \
                 worktree still holds the change",
                job.as_str()
            ),
            Adrift::NoSuchStep { job, step } => match step {
                Some(step) => write!(
                    out,
                    "{} froze a workflow with no step `{}`",
                    job.as_str(),
                    step.as_str()
                ),
                None => write!(out, "{} froze a workflow with no steps", job.as_str()),
            },
            Adrift::NotReaped { job, cause } => write!(
                out,
                "whether {}'s Drone had exited could not be established: {cause}",
                job.as_str()
            ),
            Adrift::NotForgettable { job, status } => write!(
                out,
                "{} is {} and cannot be forgotten. Forgetting is a real deletion and only \
                 applies to a terminal Job — `kill_job` is how one still in flight is ended",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NotRedispatchable { job, status } => write!(
                out,
                "{} is {} and cannot be redispatched. Redispatch replaces a Job that ran and \
                 stopped, which is `escalated`, `completed_failed` or `killed`",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NeverRan { job } => write!(
                out,
                "{} was rejected and never ran, so a redispatch would carry nothing forward — no \
                 Facts, no Evidence, no worktree. Asking for the work again is proposing a new \
                 Job, which is what `propose_job` does",
                job.as_str()
            ),
            Adrift::NotReplaceable { job } => write!(
                out,
                "{} was dispatched by a step of another Job, and replacing it is that Job's act \
                 rather than this one's",
                job.as_str()
            ),
            Adrift::WorkflowWithdrawn { job, workflow_id } => write!(
                out,
                "{} froze workflow `{workflow_id}`, which this Fleet no longer holds — the file \
                 was renamed or deleted since. Nothing was replaced",
                job.as_str()
            ),
            Adrift::NotResumable { job, status } => write!(
                out,
                "{} is {}. Redirect and restart both take a Job a person is holding, which is \
                 `escalated`",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NoStepStopped { job } => write!(
                out,
                "{} escalated without stopping a step, so there is none to restart. Only a \
                 step-level trigger names a step — redirect the Drone if it is still there, and \
                 a redispatch or Pilot is what answers this otherwise",
                job.as_str()
            ),
            Adrift::NoDroneToRedirect { job } => write!(
                out,
                "{} has no Drone to redirect — it is gone, and what is left is a restart onto \
                 the worktree it left behind",
                job.as_str()
            ),
            // `crate::asking::NotAnswered`'s sentence, carried whole: a second
            // wording here would be a second authority for one refusal.
            Adrift::NotAnswerable { because, .. } => out.write_str(because),
            Adrift::NotUnderReview { job, status } => write!(
                out,
                "{} is {} and is not standing at a human gate. Approve, request changes and \
                 reject all answer `awaiting_review` and nothing else",
                job.as_str(),
                status.as_wire()
            ),
            Adrift::NoDroneToTell { job } => write!(
                out,
                "{} has no Drone to tell and no worktree to put one on, so there is nowhere \
                 for the note to wait. What is being asked for is a redispatch",
                job.as_str()
            ),
            Adrift::NoteAlreadyWaiting { job, held } => write!(
                out,
                "{} is already holding a note for the next Drone, and a second would lose \
                 one of the two: {held}",
                job.as_str()
            ),
            Adrift::WorkUnreadable { job, cause } => write!(
                out,
                "{}'s worktree would not be read, so there is no diff to show: {cause}",
                job.as_str()
            ),
            Adrift::NotTheJudges { job, step, trigger } => write!(
                out,
                "{}'s step `{}` stopped on {}, which is not a verdict there is anything to \
                 disagree with. gate_failure and evidence_suspect are calls a machine made and a \
                 person can overrule; the others say the gate never weighed the work at all",
                job.as_str(),
                step.as_str(),
                trigger.as_wire()
            ),
            Adrift::CheckDidNotPass { job, step, check } => write!(
                out,
                "{}'s step `{}` has a Check that did not pass — `{check}`. A mechanical Check is \
                 not a matter of opinion and no override lifts one",
                job.as_str(),
                step.as_str()
            ),
            Adrift::NotUndecided { job, step, trigger } => write!(
                out,
                "{}'s step `{}` stopped on {}, which is a decision. Running the gate again would \
                 ask a question that was answered and draw the same answer; disagreeing with it \
                 is an override, and it takes a reason",
                job.as_str(),
                step.as_str(),
                trigger.as_wire()
            ),
            Adrift::NotStandingThere { job } => write!(
                out,
                "{} is not the Job Fleet is standing at, so the reading the gate failed to make \
                 cannot be made again — the baseline the step began from went with the slot. \
                 Restart the step to put a fresh Drone on the worktree it left",
                job.as_str()
            ),
            Adrift::NothingToRuleOn { job, step } => write!(
                out,
                "{}'s step `{}` has no evidence the gate could be asked about again: none was \
                 recorded, or what was recorded will not read back as a submission",
                job.as_str(),
                step.as_str()
            ),
            Adrift::DroneStillThere { job } => write!(
                out,
                "{}'s Drone is alive and idle, holding its session. Redirect it rather than \
                 restarting the step, which would throw that session away",
                job.as_str()
            ),
            Adrift::WorktreeGone { job, path } => write!(
                out,
                "{} has no worktree at {path}, so the earlier steps' work is not on disk. \
                 Starting the work again from the approval gate is a redispatch",
                job.as_str()
            ),
            Adrift::Unnameable => out.write_str("a Job needs a title somebody can read"),
            Adrift::Unreasoned { job } => write!(
                out,
                "overruling {}'s verdict needs a reason. Nobody is told it and nothing acts on \
                 it; it is the record of why the Judge was wrong, and a count of overrides with \
                 no reasons beside it says the rate and never the cause",
                job.as_str()
            ),
            // The Job is named and then the cause speaks, so the sentence reads
            // as one and the field it is about is in it.
            Adrift::NotFileable { job, cause } => {
                write!(out, "nothing was filed about {}: {cause}", job.as_str())
            }
            Adrift::NothingToPropose => {
                out.write_str("a request needs something in it to read before it can be a Job")
            }
            Adrift::NoReadingWorktree(cause) => write!(
                out,
                "the reading's checkout could not be named: {}",
                cause.said()
            ),
            Adrift::NoReading { cause, .. } => write!(
                out,
                "the proposer had nowhere to read: {cause}. Nothing was created"
            ),
            Adrift::ReadingNotDiscarded { path, cause } => write!(
                out,
                "the reading's checkout at {path} would not go back: {cause}. Remove it by \
                 hand — `armada clean` will report it and leave it, because no Job derives it"
            ),
            Adrift::NotProposable(cause) => write!(
                out,
                "the proposer call could not be configured: {}",
                cause.said()
            ),
            Adrift::NoWorkflowFits { why, .. } => write!(
                out,
                "{why}. Nothing was created and the request is unchanged — say it again                  differently, or name a workflow yourself with `propose_job`"
            ),
            Adrift::NotProposed { cause, .. } => write!(
                out,
                "the request could not be read: {cause}. Nothing was created and the                  request is unchanged — this is the call failing rather than the                  request being refused, so asking again is reasonable"
            ),
            Adrift::NoSuchWorkflow { named, held } => {
                let names: Vec<String> = held.iter().map(|id| format!("`{id}`")).collect();
                write!(
                    out,
                    "no workflow is named `{named}`. This Fleet holds {} — `list_workflows` \
                     says what each one is",
                    names.join(", ")
                )
            }
            Adrift::NoSuchPeer { named } => write!(
                out,
                "no Job is named `{named}`, and a dependency is a pointer rather than a \
                 promise. A peer must already exist when the Job naming it is created, which \
                 is what makes a plan creatable in dependency order and a cycle impossible \
                 to state"
            ),
            Adrift::NoSuchManifest { named, held } => write!(
                out,
                "no Manifest is named `{named}`. This Fleet holds `{held}`, the one declared by \
                 the `armada.yml` it was started against, and `list_manifests` says where"
            ),
            Adrift::Modelless => out.write_str(
                "a Job needs a model, and neither the proposal nor configuration named one. \
                 Set `default-model-per-job-type` in crates/config/settings.toml, or name a \
                 model on the proposal — `list_models` says which are available",
            ),
            Adrift::AttachmentUnreadable {
                job,
                filename,
                cause,
            } => write!(
                out,
                "{}'s attachment `{filename}` could not be made ready: {cause}",
                job.as_str()
            ),
        }
    }
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

    /// Which Job could not be carried forward, where one can be named.
    ///
    /// **What makes a failure reachable from the thing it happened to.** A turn
    /// that fails is reported on Fleet's stdout, which is not where a Job is
    /// read; this is what lets the same failure be written into the Job's own
    /// log, where the person watching it is looking.
    ///
    /// `None` is honest rather than a gap. A boot read, a proposal that named
    /// no workflow this Fleet holds, a request with nothing in it — none of
    /// those has a Job yet, and the three illegal-move variants carry the
    /// machine's own refusal rather than the record it was asked about.
    pub fn job(&self) -> Option<&JobId> {
        match self {
            Adrift::Unworkable { job, .. }
            | Adrift::NoWorktree { job, .. }
            | Adrift::NotPrepared { job, .. }
            | Adrift::NotConfigurable { job, .. }
            | Adrift::NoDrone { job, .. }
            | Adrift::NoTranscript { job, .. }
            | Adrift::NotTold { job, .. }
            | Adrift::NotCommitted { job, .. }
            | Adrift::NotDelivered { job, .. }
            | Adrift::NoSuchStep { job, .. }
            | Adrift::NotReaped { job, .. }
            | Adrift::NotForgettable { job, .. }
            | Adrift::NotRedispatchable { job, .. }
            | Adrift::NeverRan { job }
            | Adrift::NotReplaceable { job }
            | Adrift::WorkflowWithdrawn { job, .. }
            | Adrift::NotResumable { job, .. }
            | Adrift::NoStepStopped { job }
            | Adrift::NoDroneToRedirect { job }
            | Adrift::NotAnswerable { job, .. }
            | Adrift::NotUnderReview { job, .. }
            | Adrift::NoDroneToTell { job }
            | Adrift::NoteAlreadyWaiting { job, .. }
            | Adrift::WorkUnreadable { job, .. }
            | Adrift::NotTheJudges { job, .. }
            | Adrift::CheckDidNotPass { job, .. }
            | Adrift::NotUndecided { job, .. }
            | Adrift::NotStandingThere { job }
            | Adrift::NothingToRuleOn { job, .. }
            | Adrift::Unreasoned { job }
            | Adrift::NotFileable { job, .. }
            | Adrift::DroneStillThere { job }
            | Adrift::WorktreeGone { job, .. }
            | Adrift::AttachmentUnreadable { job, .. } => Some(job),
            Adrift::BootRead(_)
            | Adrift::Reading(_)
            | Adrift::Writing(_)
            | Adrift::IllegalMove(_)
            | Adrift::IllegalStepMove(_)
            | Adrift::IllegalDroneMove(_)
            | Adrift::Unnameable
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::NoSuchPeer { .. }
            | Adrift::Modelless
            | Adrift::NothingToPropose
            | Adrift::NoReadingWorktree(_)
            | Adrift::NoReading { .. }
            | Adrift::ReadingNotDiscarded { .. }
            | Adrift::NotProposable(_)
            | Adrift::NoWorkflowFits { .. }
            | Adrift::NotProposed { .. } => None,
        }
    }
}

impl Error for Adrift {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Adrift::BootRead(cause) => Some(cause),
            Adrift::Reading(cause) => Some(cause),
            Adrift::Writing(cause) => Some(cause),
            Adrift::IllegalMove(cause) => Some(cause),
            Adrift::IllegalStepMove(cause) => Some(cause),
            Adrift::IllegalDroneMove(cause) => Some(cause),
            Adrift::NoWorktree { cause, .. }
            | Adrift::NoDrone { cause, .. }
            | Adrift::NotCommitted { cause, .. } => Some(cause.as_ref()),
            Adrift::NotPrepared { cause, .. } => Some(cause),
            Adrift::NotTold { cause, .. }
            | Adrift::NotReaped { cause, .. }
            | Adrift::NoTranscript { cause, .. }
            | Adrift::AttachmentUnreadable { cause, .. } => Some(cause),
            Adrift::NotDelivered { .. }
            | Adrift::Unworkable { .. }
            | Adrift::NotConfigurable { .. }
            | Adrift::NoSuchStep { .. }
            | Adrift::NotForgettable { .. }
            | Adrift::NotRedispatchable { .. }
            | Adrift::NeverRan { .. }
            | Adrift::NotReplaceable { .. }
            | Adrift::WorkflowWithdrawn { .. }
            | Adrift::Unnameable
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::NoSuchPeer { .. }
            // The five resume refusals are refusals rather than faults: a Job
            // that cannot be redirected has nothing underneath saying why, only
            // the state it is in.
            | Adrift::NotResumable { .. }
            | Adrift::NoStepStopped { .. }
            | Adrift::NoDroneToRedirect { .. }
            // And an answer that does not apply, which is the same shape.
            | Adrift::NotAnswerable { .. }
            // The two review refusals join them: a Job that is not at a gate
            // has nothing underneath saying why, only the state it is in.
            | Adrift::NotUnderReview { .. }
            | Adrift::NoDroneToTell { .. }
            | Adrift::NoteAlreadyWaiting { .. }
            // And the two an override makes, which say what the record holds
            // rather than wrapping something that failed.
            | Adrift::NotTheJudges { .. }
            | Adrift::CheckDidNotPass { .. }
            // And the three a gate re-run makes, which say the same: what the
            // record holds, not what failed underneath.
            | Adrift::NotUndecided { .. }
            | Adrift::NotStandingThere { .. }
            | Adrift::NothingToRuleOn { .. }
            | Adrift::Unreasoned { .. }
            // `NotFiled` joins them: it says what a filing could not be, not
            // what failed underneath it.
            | Adrift::NotFileable { .. }
            | Adrift::DroneStillThere { .. }
            | Adrift::WorktreeGone { .. }
            | Adrift::NothingToPropose
            | Adrift::NoReadingWorktree(_)
            | Adrift::NoWorkflowFits { .. }
            // `SpawnConfigRefused` is not an `Error` — it says what a value
            // cannot be rather than wrapping a failure — so it prints into the
            // message above and has nothing to chain to.
            | Adrift::NotProposable(_)
            | Adrift::Modelless => None,
            Adrift::NotProposed { cause, .. } => Some(cause),
            Adrift::NoReading { cause, .. }
            | Adrift::ReadingNotDiscarded { cause, .. }
            | Adrift::WorkUnreadable { cause, .. } => Some(cause.as_ref()),
        }
    }
}

/// Why a submission was not taken.
///
/// **Neither variant is a gate failure.** Nothing has been verified and nothing
/// has failed: the tool call was malformed, or there is no Job for it to be
/// about.
#[derive(Debug)]
pub enum NotSubmitted {
    /// No Job is being worked, so there is no step the submission could be
    /// against. The Evidence tool is bound to a Job at construction, so this is
    /// a call that arrived after its Drone's Job ended.
    NothingIsWorking,
    /// The Job is standing at a step its frozen workflow does not name. **A
    /// fault in Fleet, not in the call**, and nothing the Drone can do about it.
    NoSuchStep { step: StepId },
    /// The step declares no work product, so there is no type for Fleet to
    /// record the submission under. A Drone cannot supply one — the tool has no
    /// parameter for it, because a Drone is never told what its step declared.
    StepDeclaresNothing { step: StepId },
    /// Evidence for this step is already waiting for the gate.
    ///
    /// **This is the "already advanced" refusal**, arriving one moment earlier
    /// than the phrase suggests: the tool names no step, so a submission that
    /// beats the gate and one that follows it are the same bytes, and the
    /// distinguishable case is the second call rather than the stale step.
    /// Refused rather than queued — a second submission would be ruled on
    /// against whatever step the first one advanced the Job to.
    AlreadyWaiting { step: StepId },
    /// The call itself was not a submission — an empty `claimed`, an empty
    /// `shown_by`.
    Malformed(verification::NotASubmission),
}

impl fmt::Display for NotSubmitted {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotSubmitted::NothingIsWorking => out.write_str(
                "no Job is being worked, so there is no step for this submission \
                 to be against. Stop — the Job this Drone was started for has \
                 already ended",
            ),
            NotSubmitted::NoSuchStep { step } => write!(
                out,
                "the Job is standing at step `{}`, which its workflow does not \
                 name. This is a fault in Fleet and not in the submission",
                step.as_str()
            ),
            NotSubmitted::StepDeclaresNothing { step } => write!(
                out,
                "step `{}` declares no work product, so there is nothing for a \
                 submission to be recorded as. This is a fault in the workflow \
                 and not in the submission",
                step.as_str()
            ),
            NotSubmitted::AlreadyWaiting { step } => write!(
                out,
                "the submission already made for step `{}` has not been checked \
                 yet. Wait for the outcome — it arrives as a later turn, and a \
                 second submission is not read",
                step.as_str()
            ),
            NotSubmitted::Malformed(cause) => write!(out, "{cause}"),
        }
    }
}

/// Why a scope declaration was not taken.
///
/// **No variant is a gate failure.** Nothing has been verified: the call was
/// aimed at nothing, at a step that asks for no scope, or at paths the step's
/// own denylist refuses.
#[derive(Debug)]
pub enum NotDeclared {
    /// No Job is being worked. The tool is bound to a Job at construction, so
    /// this is a call that arrived after its Drone's Job ended.
    NothingIsWorking,
    /// The Job is standing at a step its frozen workflow does not name. **A
    /// fault in Fleet, not in the call.**
    NoSuchStep { step: StepId },
    /// The step declares no evidence scope, so there is nothing a declaration
    /// would be measured against. Refused rather than stored: a plan nothing
    /// reads is a plan the Drone believes is being checked.
    StepHasNoScope { step: StepId },
    /// The declaration names paths the step's `exclude_paths` denies.
    Outside(verification::OutsideScope),
}

impl fmt::Display for NotDeclared {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotDeclared::NothingIsWorking => out.write_str(
                "no Job is being worked, so there is no step for this declaration \
                 to be about. Stop — the Job this Drone was started for has \
                 already ended",
            ),
            NotDeclared::NoSuchStep { step } => write!(
                out,
                "the Job is standing at step `{}`, which its workflow does not \
                 name. This is a fault in Fleet and not in the declaration",
                step.as_str()
            ),
            NotDeclared::StepHasNoScope { step } => write!(
                out,
                "step `{}` declares no evidence scope, so a declaration would be \
                 measured against nothing. Get on with the work and submit when \
                 it is done",
                step.as_str()
            ),
            NotDeclared::Outside(why) => write!(out, "{why}. Declare again without them"),
        }
    }
}

impl Error for NotDeclared {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NotDeclared::NothingIsWorking
            | NotDeclared::NoSuchStep { .. }
            | NotDeclared::StepHasNoScope { .. } => None,
            NotDeclared::Outside(why) => Some(why),
        }
    }
}

impl Error for NotSubmitted {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NotSubmitted::NothingIsWorking
            | NotSubmitted::NoSuchStep { .. }
            | NotSubmitted::StepDeclaresNothing { .. }
            | NotSubmitted::AlreadyWaiting { .. } => None,
            NotSubmitted::Malformed(cause) => Some(cause),
        }
    }
}
