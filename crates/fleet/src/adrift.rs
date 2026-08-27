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
    IllegalDroneMove, IllegalStepTransition, IllegalTransition, JobId, JobStatus, StepId,
};
use store::{LoadAllError, LoadJobError, WriteError};

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
    /// A proposal carried a title nothing could be picked out of a list by.
    Unnameable,
    /// A proposal named a workflow this Fleet does not hold.
    ///
    /// **Nothing checked this until now.** `ResolvedWorkflow` carried no id, so
    /// the proposal's was written onto the record unverified and the Job sat on
    /// the board claiming a workflow Fleet had never heard of — the same class
    /// as the blank model, a value that cannot work accepted where it enters.
    NoSuchWorkflow { named: String, held: String },
    /// A proposal named a Manifest this Fleet does not hold. The same fault as
    /// [`Adrift::NoSuchWorkflow`], for the other id a proposal carries.
    NoSuchManifest { named: String, held: String },
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
            Adrift::Unnameable => out.write_str("a Job needs a title somebody can read"),
            Adrift::NoSuchWorkflow { named, held } => write!(
                out,
                "no workflow is named `{named}`. This Fleet holds `{held}` — a proposal may name \
                 that one, or none at all, and `list_workflows` says what is there"
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
            Adrift::NotTold { cause, .. }
            | Adrift::NotReaped { cause, .. }
            | Adrift::NoTranscript { cause, .. }
            | Adrift::AttachmentUnreadable { cause, .. } => Some(cause),
            Adrift::NotDelivered { .. }
            | Adrift::Unworkable { .. }
            | Adrift::NotConfigurable { .. }
            | Adrift::NoSuchStep { .. }
            | Adrift::NotRedispatchable { .. }
            | Adrift::NeverRan { .. }
            | Adrift::NotReplaceable { .. }
            | Adrift::Unnameable
            | Adrift::NoSuchWorkflow { .. }
            | Adrift::NoSuchManifest { .. }
            | Adrift::Modelless => None,
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
