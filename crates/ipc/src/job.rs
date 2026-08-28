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
//! `branch` crosses too: it is a name in Armada's own namespace, not a path,
//! and it is what a person merges once the Job is done.
//!
//! `title` is the field that goes the other way, and the only free text on the
//! record that does: `facts` is redacted because it is the likeliest place a
//! secret lands, and a title is the one string on a Job written to be read off
//! a screen. A list of ids and statuses with no name on any row is a list
//! nobody can use.
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
use crate::ids::{DroneId, Instant, JobId, ManifestId, StepId, WorkflowId};

use crate::enums::{
    CriterionSource, DependencyDirection, JobStatus, Origin, QueuedReason, TopLevelOrigin, Urgency,
};

/// One Job, as a Board row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: JobId,
    /// The name a person reads in the row. **The reason the list is worth
    /// looking at** — everything else here is an id, a status or a flag, and
    /// none of them says what the Job is.
    ///
    /// A `String` and not a `Title`: the newtype's guarantee is that it cannot
    /// be constructed blank, and a DTO is deserialised rather than constructed.
    /// The refusal belongs where the text is typed and at the Fleet boundary
    /// where it becomes a Job, not in a wire struct that would carry a second
    /// copy of the rule.
    pub title: String,
    pub status: JobStatus,
    /// When the Job was created. **The instant elapsed is measured from**, and
    /// the reason it is on the row rather than only on the detail: a Board that
    /// cannot draw how long a Job has been going needs one request per row to
    /// answer "is this stuck", which is the question the column exists for.
    ///
    /// Read from the record, never derived from the id's ULID prefix — that
    /// would be a second source for an instant that is already stored.
    pub created_at: Instant,
    /// The branch the Job's worktree is on. **Absent until a worktree exists**
    /// — a Job at the approval gate has no branch and does not claim one, and
    /// absent is never `null`.
    ///
    /// It is what a person merges once the Job completes, so a row that names
    /// it is a row somebody can act on without opening the Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The qualifying reason the Job's last transition stored, where it stored
    /// one. **Absent is not "no reason"** — `queued` computes its readiness
    /// reason at read time from dependencies and live headroom, so it is not in
    /// the log for this to carry and rides on
    /// [`queued_reason`](JobSummary::queued_reason) instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// Why an approved Job has not started yet.
    ///
    /// **Its own field and not [`reason`](JobSummary::reason)**, because the
    /// two are different facts: that one is what a transition recorded, this
    /// one is computed from the board a moment ago. A caller handed one field
    /// could not tell a reason that was written down from one worked out on the
    /// way past.
    ///
    /// **Absent on every status but `queued`**, and absent on a `queued` Job
    /// that nothing is holding — which is the registry's `none`, carried as the
    /// absence of a value rather than as a variant nothing renders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_reason: Option<QueuedReason>,
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
    /// The Job this one replaces. **Lineage, and the only field on the row a
    /// repeat can be counted along** — a redispatch mints a new Job, so
    /// without this a Board reads every second failure as a first one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redispatched_from: Option<JobId>,
}

impl JobSummary {
    /// A Job, plus the reason its last recorded transition carried.
    ///
    /// The reason is a second argument because it is not on the record: the
    /// `jobs` row stores `status` and `job_events` stores why. Only a caller
    /// holding the log can supply it, which is Fleet.
    pub fn of(
        job: &core_model::Job,
        reason: Option<&core_model::TransitionReason>,
        queued_reason: Option<core_model::QueuedReason>,
    ) -> JobSummary {
        JobSummary {
            id: job.id().into(),
            title: job.title().as_str().to_string(),
            status: job.status().into(),
            created_at: job.created_at().into(),
            branch: job.branch().map(|branch| branch.as_str().to_string()),
            reason: reason.and_then(Reason::of),
            queued_reason: queued_reason.map(QueuedReason::from),
            workflow_id: job.workflow_id().into(),
            owner_manifest_id: job.owner_manifest_id().into(),
            origin: job.origin().into(),
            urgency: job.urgency().into(),
            atomic: job.atomic(),
            model: job.model().as_str().to_string(),
            current_step_id: job.current_step_id().map(StepId::from),
            assigned_drone: job.assigned_drone().map(DroneId::from),
            redispatched_from: job.redispatched_from().map(JobId::from),
        }
    }
}

/// What a redispatch did. **Two Jobs, because a redispatch is two acts.**
///
/// A replacement is minted carrying `redispatched_from`, and the original is
/// killed where it was still killable; both are answered so a caller learns the
/// new id without waiting for the stream and re-reading the Board to find it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Redispatched {
    /// The Job that stopped: `killed` if it was escalated, otherwise where it
    /// already ended. Its worktree and branch are as its Drone left them —
    /// nothing in Armada removes either.
    pub replaced: JobSummary,
    /// The replacement, at the approval gate, with `redispatched_from` set to
    /// [`replaced`](Redispatched::replaced)'s id.
    pub dispatched: JobSummary,
}

/// A person's instruction to a Drone that is there. The request half of
/// `redirect_drone`.
///
/// **The one place a person's own words reach a Drone.** Every other
/// Drone-facing string in the workspace is assembled by Fleet from the record;
/// this is Intervention Ladder rung one, and steering with better information
/// is the whole act. `docs/contracts/agent-prompt.md` gives the turn no wording
/// of its own for the same reason.
///
/// Blank is refused at the Fleet boundary rather than here: a decoded request
/// is well-formed, and an instruction with nothing in it is a value that cannot
/// work — which is a 422 and not a 400.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Redirection {
    pub instruction: String,
}

/// The redaction, at the Fleet boundary, with no reason to hand.
///
/// **No queued reason either**, and that is not a gap: this conversion is what
/// an event publish uses, and nothing publishes a summary of a `queued` Job —
/// creation, a step advancing and a Drone arriving or leaving all carry a Job
/// in some other status. A publish that did would need the board, which is
/// exactly what this conversion does not have.
impl From<&core_model::Job> for JobSummary {
    fn from(job: &core_model::Job) -> JobSummary {
        JobSummary::of(job, None, None)
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
    /// What the Job is called. **Required, and no `serde(default)`** — a
    /// proposal without one does not decode, which is what makes the field
    /// required rather than merely expected.
    pub title: String,
    pub workflow_id: WorkflowId,
    pub owner_manifest_id: ManifestId,
    /// Which of the four top-level origins the proposer claims. `sub_dispatched`
    /// does not deserialise.
    pub origin: TopLevelOrigin,
    pub urgency: Urgency,
    pub atomic: bool,
    /// Which model the Drone is spawned as. **Optional, and absent is the
    /// ordinary case** — Fleet fills it from configuration, so a caller that
    /// has no opinion sends nothing rather than sending `""`.
    ///
    /// A `String` and not a `ModelName`: the newtype's guarantee is that it
    /// cannot be constructed blank, and a DTO is deserialised rather than
    /// constructed. `""` therefore decodes here, and is refused at the Fleet
    /// boundary where text becomes a Job — the same division `title` draws.
    ///
    /// It was required and non-optional, and the empty string it invited was
    /// accepted, stored, shown on the board and refused at spawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
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
    /// The peers this Job is sequenced against. **Additive, like `model`** — a
    /// caller with no graph to draw sends nothing.
    ///
    /// A peer named here must already exist: an edge is a pointer, and Fleet
    /// mints the ids. That is what makes a plan's Jobs creatable in dependency
    /// order and no other.
    #[serde(default)]
    pub dependencies: Vec<DependencyEdge>,
    /// Files a person attached to the brief before proposing it. **Additive,
    /// like `model`** — a caller that predates this field sends nothing and
    /// decodes exactly as it did before.
    ///
    /// Each entry names a path Bridge already wrote bytes to, on the same
    /// machine Fleet runs on — the assumption `docs/practices/protocol.md`
    /// already makes for a staged path rather than a payload. `drafted()` is
    /// where Fleet promotes each into its own keeping and where a path that
    /// does not exist is refused rather than silently dropped.
    #[serde(default)]
    pub attachments: Vec<AttachmentRef>,
}

/// What a person described, before anything has decided what it is. The request
/// half of `propose_from_request`.
///
/// **One field, and no `workflow_id` among them.** Naming the workflow is the
/// act this operation exists to remove; a request that carried one would be
/// `propose_job` with an extra model call in front of it.
///
/// Blank is refused at the Fleet boundary rather than here, the division
/// [`ProposeJob::title`] already draws: a decoded request is well-formed, and a
/// description with nothing in it is a value that cannot work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRequest {
    /// What the person wrote — a prompt, or a link to a ticket. **Carried
    /// verbatim**: Fleet opens no link and fetches nothing, so what the
    /// proposer reads is what was typed.
    pub request: String,
}

/// One DAG link, sequencing this Job against a peer.
///
/// **`blocks` is expressible and the proposer never writes one.** A plan is
/// created in dependency order, so the Job being created can only point
/// backwards — `depends_on` at the pointing end says the same thing as
/// `blocks` at the other, and one direction written consistently is one
/// direction to read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub direction: DependencyDirection,
    pub peer: JobId,
}

/// What one reading of a request proposed. The answer half of
/// `propose_from_request`.
///
/// **A list even where it holds one**, because approving is a different act
/// depending on how many: one Job is dispatched by its approval, and several
/// are a plan whose members each take their own approval in turn. A shape that
/// answered with one Job would make the second case unrepresentable rather
/// than merely unbuilt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedPlan {
    /// Every Job the request became, in dependency order — an upstream always
    /// before what points at it. **All at `awaiting_approval`**: nothing here
    /// has been approved and nothing is running.
    pub jobs: Vec<JobSummary>,
}

/// One staged file, named by where Bridge already wrote it.
///
/// **A path, never bytes.** The wire carries a pointer to a file already on
/// disk rather than a base64 payload, for the reason `write_targets` and every
/// other same-machine path on this DTO already carries one: Bridge and Fleet
/// share a filesystem, and a payload round-tripped through this channel would
/// duplicate bytes a path can name for free.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRef {
    pub staged_path: String,
    pub filename: String,
    pub mime_type: String,
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
