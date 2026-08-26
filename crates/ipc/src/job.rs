//! What a Job looks like to Bridge, and what Bridge may propose.
//!
//! # The conversion is the redaction
//!
//! [`JobSummary`] is built by hand from `core_model::Job`, field by field, and
//! that is the whole reason it exists. `Job` will accrete fields as Fleet's
//! needs grow — worktree paths, adapter state, whatever a scheduler wants — and
//! if the record itself were serialised, every new field would be redacted or
//! not by whatever serde does by default. **A domain type on the wire is a
//! redaction decision nobody made.**
//!
//! What is deliberately left behind, and why:
//!
//! | Left behind | Why |
//! |---|---|
//! | `write_targets` | Repo-relative paths. A Board on a shared screen shows no filesystem |
//! | `facts` | Free text handed to a model whole; the likeliest place a secret lands |
//! | `scope_revisions` | Carries `rationale`, which is free text for the same reason |
//! | `acceptance_criteria` | The requester's words. `get_job` returns the Job in full; this is the list |
//! | `dependencies`, `gate_manifests`, `dispatched_by` | The graph, which the Board does not draw at M1 |
//!
//! # The list carries its failures
//!
//! [`JobList`] is not a `Vec<JobSummary>`. `store` hands back the Jobs that
//! loaded *and* the ones that did not, and a wire shape that dropped the second
//! half would reintroduce the v1 bug — twenty-one Jobs missing from a
//! well-typed list with nothing in the signature saying so — one layer further
//! out, where nobody would look for it.

use serde::{Deserialize, Serialize};

use crate::event::Reason;
use crate::ids::{DroneId, JobId, ManifestId, StepId, WorkflowId};

use crate::enums::{CriterionSource, JobStatus, Origin, TopLevelOrigin, Urgency};

/// One Job, as a Board row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: JobId,
    pub status: JobStatus,
    /// The qualifying reason the Job's last transition stored, where it stored
    /// one. **Absent is not "no reason"** — `queued` computes its readiness
    /// reason at read time from dependencies and live headroom, and it is
    /// therefore not in the log for this to carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    pub workflow_id: WorkflowId,
    pub owner_manifest_id: ManifestId,
    pub origin: Origin,
    pub urgency: Urgency,
    /// Whether the write targets must land as one unit.
    pub atomic: bool,
    /// Which model the assigned Drone is using. A string on the record and a
    /// string here: naming a closed set would put a vendor's vocabulary on the
    /// wire.
    pub model: String,
    /// The `workflow_status` projection — which step the Job is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_step_id: Option<StepId>,
    /// **Presence, not state.** Absent is a Job no process is on, which is also
    /// what suspends the liveness clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_drone: Option<DroneId>,
}

impl JobSummary {
    /// A Job, plus the reason its last recorded transition carried.
    ///
    /// The reason is a second argument because it is not on the record: the
    /// `jobs` row stores `status` and `job_events` stores why. Only a caller
    /// holding the log can supply it, which is Fleet.
    pub fn of(job: &core_model::Job, reason: Option<&core_model::TransitionReason>) -> JobSummary {
        JobSummary {
            id: job.id().into(),
            status: job.status().into(),
            reason: reason.and_then(Reason::of),
            workflow_id: job.workflow_id().into(),
            owner_manifest_id: job.owner_manifest_id().into(),
            origin: job.origin().into(),
            urgency: job.urgency().into(),
            atomic: job.atomic(),
            model: job.model().as_str().to_string(),
            current_step_id: job.current_step_id().map(StepId::from),
            assigned_drone: job.assigned_drone().map(DroneId::from),
        }
    }
}

/// The redaction, at the Fleet boundary, with no reason to hand.
impl From<&core_model::Job> for JobSummary {
    fn from(job: &core_model::Job) -> JobSummary {
        JobSummary::of(job, None)
    }
}

/// Every Job, **and every one that would not load**.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobList {
    pub jobs: Vec<JobSummary>,
    /// Rows the store refused. Never filtered away, and never merged into
    /// `jobs` as a placeholder — a Board that shows nine of ten Jobs and says
    /// so is honest; one that shows nine is not.
    #[serde(default)]
    pub unreadable: Vec<UnreadableJob>,
}

/// A Job on disk that could not be read back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreadableJob {
    /// Absent where the damage is in the column that names the Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
    /// What the store refused, flattened to one line.
    ///
    /// A `String` and not a [`WireError`](crate::WireError) because the code
    /// manifest does not exist yet and a code invented here would not be in it.
    /// This becomes a `WireError` when `ArmadaError` lands.
    pub fault: String,
}

/// A Job drafted onto the approval gate. The request half of `propose_job`.
///
/// **It carries no id, no status and no steps.** The id is Fleet's to mint, the
/// status is the entry status of the constructor Fleet calls, and the steps are
/// the frozen WorkflowDef's — read from `workflow_id` at creation, so that what
/// was approved is what runs even if the workflow file is edited while the Job
/// waits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeJob {
    pub workflow_id: WorkflowId,
    pub owner_manifest_id: ManifestId,
    /// Which of the four top-level origins the proposer claims. `sub_dispatched`
    /// does not deserialise.
    pub origin: TopLevelOrigin,
    pub urgency: Urgency,
    pub atomic: bool,
    pub model: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<ProposedCriterion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Subject>,
    /// Context the Job needs to run. Append-only once the Job exists.
    #[serde(default)]
    pub facts: String,
    /// **Null is not empty.** Absent is scope not yet determined; present and
    /// empty is determined to write nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_targets: Option<Vec<String>>,
}

/// One criterion, in the requester's words.
///
/// No `criterion_id`: the frozen identifier is minted with the Job, because a
/// Judge citation references a criterion by its frozen position and an id
/// chosen by a peer is an id nothing else can join to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedCriterion {
    pub text: String,
    pub source: CriterionSource,
}

/// What a Job is about. Neither sequencing nor provenance.
///
/// `kind` is a string because the registry types the field `{kind, ref}` and
/// names no value set — carried as written rather than closed into an enum this
/// crate would have invented.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub kind: String,
    /// `ref` on the record, spelled out here because `ref` is a Rust keyword.
    pub reference: String,
}

impl From<&core_model::Subject> for Subject {
    fn from(subject: &core_model::Subject) -> Subject {
        Subject {
            kind: subject.kind.clone(),
            reference: subject.reference.clone(),
        }
    }
}
