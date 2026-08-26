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

use adapter_traits::{SpawnConfigRefused, WorktreeSpecRefused};
use core_model::{IllegalStepTransition, IllegalTransition, JobId, StepId};
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
    /// The gate ruled and the Drone could not be told. **The step still
    /// advanced** — the ruling is Fleet's and does not depend on the Drone
    /// hearing it — so this says a session went deaf, not that a verdict was
    /// lost.
    NotTold { job: JobId, cause: io::Error },
    /// The frozen workflow has no step by this name. A Job dispatched against a
    /// workflow that has since been edited, or a workflow with no steps at all.
    NoSuchStep { job: JobId, step: Option<StepId> },
    /// Whether a Drone's process had exited could not be established.
    /// **Not the same as "it is gone"** — folding a failed reading into absence
    /// is how a live Drone gets declared interrupted.
    NotReaped { job: JobId, cause: io::Error },
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
            Adrift::Unworkable { job, cause } => {
                write!(
                    out,
                    "no worktree can be named for {}: {}",
                    job.as_str(),
                    cause.said()
                )
            }
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
            Adrift::NoWorktree { cause, .. } | Adrift::NoDrone { cause, .. } => {
                Some(cause.as_ref())
            }
            Adrift::NotTold { cause, .. } | Adrift::NotReaped { cause, .. } => Some(cause),
            Adrift::Unworkable { .. }
            | Adrift::NotConfigurable { .. }
            | Adrift::NoSuchStep { .. }
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
    /// The call itself was not a submission — a `facts_note` with no note, an
    /// empty `shown_by`.
    Malformed(verification::NotASubmission),
}

impl fmt::Display for NotSubmitted {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotSubmitted::NothingIsWorking => {
                out.write_str("no Job is being worked, so there is nothing to submit against")
            }
            NotSubmitted::Malformed(cause) => write!(out, "{cause}"),
        }
    }
}

impl Error for NotSubmitted {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NotSubmitted::NothingIsWorking => None,
            NotSubmitted::Malformed(cause) => Some(cause),
        }
    }
}
