//! What a Job looks like to Bridge, and what Bridge may propose.
//! **The conversion is the redaction.** [`JobSummary`] is built by hand from
//! `core_model::Job`, field by field, which is the whole reason it exists. `Job`
//! will accrete fields as Fleet's needs grow, and were the record itself
//! serialised, every new field would be redacted or not by whatever serde does
//! by default: **a domain type on the wire is a redaction nobody decided.**
//!
//! | Left behind | Why |
//! |---|---|
//! | `write_targets` | Repo-relative paths. A Board on a shared screen shows no filesystem |
//! | `facts` | Free text handed to a model whole; the likeliest place a secret lands |
//! | `scope_revisions` | Carries `rationale`, which is free text for the same reason |
//! | `acceptance_criteria` | The requester's words. `get_job` returns the Job in full; this is the list |
//! | `dependencies`, `gate_manifests` | The graph, which the Board does not draw at M1 |
//!
//! `branch` crosses because it is a name in Armada's own namespace rather than a
//! path, and is what a person merges. `title` crosses because it is the one
//! string on a Job written to be read off a screen: a list of ids and statuses
//! with no name on any row is a list nobody can use.
//!
//! **`dispatched_by`'s own id crosses too, `#1165`, and that is narrower than
//! it looks.** The parent's id alone joins the row — not `step_id`, not
//! `dependencies`, not `gate_manifests` — so a caller with every row still
//! cannot draw the fan-out a step made, only that a Job has a parent.
//!
//! **The list carries its failures.** [`JobList`] is not a `Vec<JobSummary>`:
//! `store` hands back the Jobs that loaded *and* the ones that did not, and a
//! wire shape dropping the second half would reintroduce the v1 bug — twenty-one
//! Jobs missing from a well-typed list with nothing in the signature saying so —
//! one layer further out, where nobody would look for it.

use serde::{Deserialize, Serialize};

use crate::detail::Settled;
use crate::event::Reason;
use crate::ids::{DroneId, Instant, JobId, ManifestId, StepId, WorkflowId};

use crate::enums::{
    BudgetHold, CriterionSource, DependencyDirection, JobStatus, Origin, QueuedReason, Resumption,
    TopLevelOrigin, Urgency,
};

/// One Job, as a Board row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: JobId,
    /// What a person calls this Job — `12-the-drone-count-is-wrong`.
    ///
    /// **Beside `id`, never instead of it.** The id is what everything joins
    /// on and what every other message names; this is what a person reads,
    /// types, and finds in a branch name, a worktree directory and a pull
    /// request title. A ULID is unique and unsayable, and two Jobs from one
    /// proposal are minted in the same millisecond — so the ones hardest to
    /// tell apart are the ones a reader most needs to.
    ///
    /// Derived by Fleet from the Job's number and title, both frozen at
    /// creation. A surface renders it and never composes one.
    #[serde(default)]
    pub handle: String,
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
    /// When the Job was created. **Not the instant elapsed is measured from**
    /// — [`started_at`](JobSummary::started_at) is, since waiting for approval
    /// and waiting in the queue must not count — but on the row for
    /// [`started_at`]'s own reason: a Board that cannot draw how long a Job has
    /// been going needs one request per row to answer "is this stuck", which is
    /// the question the column exists for.
    ///
    /// Read from the record, never derived from the id's ULID prefix — that
    /// would be a second source for an instant that is already stored.
    pub created_at: Instant,
    /// When the Job's first Drone started: the first dated arrival at
    /// `running` in the log, off [`first_started_at`](crate::attempt::first_started_at).
    /// **Absent until then, and never `null`.**
    ///
    /// **The instant a whole-Job elapsed is measured from**, and the whole
    /// reason it exists apart from [`created_at`](JobSummary::created_at):
    /// time spent at `awaiting_approval`, and time spent `queued` before that
    /// first run, must read as nothing rather than as a run that is already
    /// under way. Once set it never moves, even where the Job returns to
    /// `awaiting_approval` for a sub-dispatch approval or to `queued` on a
    /// restart — only the very first arrival counts.
    ///
    /// Filled by Fleet from the log, like [`landed`](JobSummary::landed) is
    /// filled from the store: `core_model::Job` carries no instant for this,
    /// only the log does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Instant>,
    /// When the Job arrived at a terminal status. **Absent is a Job still
    /// going**, never `null`. Since 14.1.
    ///
    /// Overview 28 (#1092): a Job killed, failed or rejected has nowhere to
    /// stand once it is over — the Board's own Done section is collapsed and
    /// Overview drops it outright — so this is the fact that lets a
    /// **Recently ended** list say when. Filled from the log the way
    /// [`started_at`](JobSummary::started_at) is, and for the same reason:
    /// `core_model::Job` carries no instant for either, only the log does.
    ///
    /// **Read off the log rather than worked out from the row.** `status`
    /// alone says a Job is over, not when — and the log already answers that
    /// for `started_at`'s own first arrival at `running`. A terminal status
    /// has no outbound edge, so at most one arrival exists to read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
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
    /// Which of the two ceilings is holding it, where `queued_reason` is
    /// `over_budget`.
    ///
    /// **Absent whenever that one is not `over_budget`**, and never present on
    /// its own: it qualifies that label and says nothing without it.
    ///
    /// **It costs nothing to carry.** `Fleet::overspent` already answers which
    /// ceiling it was, in the same read `queued_reason` is computed from — this
    /// is that answer reaching the wire instead of being folded away. What it
    /// buys is the difference between a Board that says a Job is over budget
    /// and a Board a person can act on: the two ceilings take different acts,
    /// and until Sept 2026 only one of them had an act at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_hold: Option<BudgetHold>,
    /// The gating Manifests that are frozen: on a `queued` Job where
    /// `queued_reason` is `frozen`, and on an `awaiting_review` Job, whose merge
    /// and next step both wait for the freeze. Left out when empty. Filled by
    /// Fleet after [`JobSummary::of`], as `tasks` is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frozen_by: Vec<ManifestId>,
    /// Which act a person took to put this Job back in the queue.
    ///
    /// **The other axis over `queued`, and the one that says somebody is
    /// waiting.** `queued_reason` says what the Job is waiting for; this says
    /// it is here because a person pressed something. Absent on a Job approved
    /// and never run — which is how a Job arrives at `queued` rather than
    /// returns to it — and absent on every other status.
    ///
    /// Without it, pressing restart while the bound is spent moves nothing on
    /// screen. That reading was correct and looked like a dropped press, and it
    /// is new: those acts started a Drone on the spot until re-admission put
    /// them behind the bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumption: Option<Resumption>,
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
    /// The Job whose Drone dispatched this one. **The parent's id alone, not
    /// `core_model::DispatchOrigin`'s `step_id`** — that would start to name
    /// the fan-out a step made rather than one Job's lineage.
    ///
    /// Since 14.2, `#1165`: `sub_dispatched`'s registry sentence had nothing
    /// to fill its slot with, so the fact was withheld rather than shown
    /// half-true. Crosses on [`redispatched_from`]'s own reasoning — every
    /// row still cannot draw the DAG `dependencies` or `gate_manifests` would.
    ///
    /// [`redispatched_from`]: JobSummary::redispatched_from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatched_by: Option<JobId>,
    /// Whether this Job's Drone is waiting on an answer from a person.
    ///
    /// # A flag, and deliberately not the question
    ///
    /// The question, its options and what each commits to are on
    /// [`JobDetail::asking`](crate::JobDetail), which is one Job somebody
    /// opened. This is a Board drawn for every Job at once, and a list carrying
    /// a paragraph of prose per row to say a single true-or-false is the cost
    /// [`facts`](crate::JobDetail::facts) is redacted from the summary for.
    ///
    /// # One of the two things on the row that are not from the record
    ///
    /// [`landed`](JobSummary::landed) is the other. Every other field is read
    /// off `core_model::Job`; this is read off the working slot, which is why
    /// [`JobSummary::of`] takes it rather than finding it — and why it is
    /// `false` on every summary built where no slot was in hand, which is
    /// every event publish. That is correct rather than a gap: a Job created,
    /// advancing a step or losing its Drone has not just asked something, and
    /// the one message that says a question exists is `job.asking`.
    ///
    /// # Why the Board needs it at all
    ///
    /// `who_is_acting` on `running` is `Drone`, so without this a Job waiting on
    /// a person sits under **Running** and a question on a Job nobody has open
    /// is invisible. `docs/concepts/job-board.md` carries the rule.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub asking: bool,
    /// What became of the pull request this Job opened. **Absent is every Job
    /// that has not opened one, and every one whose pull request nobody has
    /// merged yet** — the two are one absence because neither is news, and the
    /// board draws nothing for either.
    ///
    /// # The second field here that is not from the record
    ///
    /// [`asking`](JobSummary::asking) is the other. This one is read from the
    /// delivery columns beside the row rather than from `core_model::Job`,
    /// which has no field for it: Armada opens a pull request and a person
    /// merges it, so what became of it is not something the Job's own machine
    /// could ever know. `JobSummary::of` leaves it `None` and the Board fills
    /// it in from one read for the whole list — a read per row would be a query
    /// per row on a list that redraws on every event.
    ///
    /// # Why the board needs it
    ///
    /// It is the only question anybody has about finished work. Without it,
    /// every terminal row on the board says the same thing whether the work is
    /// in `main` or has been sitting unread for a week.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landed: Option<Settled>,
    /// When this Job's worktree and branch were given back while its record
    /// stayed. **Absent is a Job whose disk still stands** — every Job before
    /// a reclaim, and every Job a reclaim has not yet reached.
    ///
    /// This is what a `Cleared` tab is keyed off: the reclaim that ran did
    /// not delete a row for it to notice, and there is no other way to tell a
    /// cleared Job from a finished one apart from asking whether its worktree
    /// happens to exist on disk right now — which is not a record of an act,
    /// only a directory that might be gone for some other reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reclaimed_at: Option<Instant>,
    /// How many of the Job's plan tasks stand where. **Absent is a Job with no
    /// plan**, not a plan of none — a row draws no task field for it.
    ///
    /// Filled by Fleet off the store, like [`landed`](JobSummary::landed):
    /// `core_model::Job` holds no plan. Since 13.21.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks: Option<crate::TaskCounts>,
}

impl JobSummary {
    /// A Job, plus the reason its last recorded transition carried.
    ///
    /// The reason is a second argument because it is not on the record: the
    /// `jobs` row stores `status` and `job_events` stores why. Only a caller
    /// holding the log can supply it, which is Fleet.
    /// `asking` is the fourth for [`queued_reason`](JobSummary::queued_reason)'s
    /// reason, one layer further out: it is not on the record either, and only a
    /// caller holding the working slot can supply it. A chained setter was the
    /// alternative and was rejected — a summary built without the call would say
    /// `false` silently, which is the redaction decision nobody made.
    ///
    /// `started_at` and `ended_at` are last, for the same reason as each
    /// other: the first dated arrival at `running`, and the one arrival at a
    /// terminal status, are both in `job_events`, not on `core_model::Job`, so
    /// only a caller holding the log can supply either.
    #[allow(clippy::too_many_arguments)]
    pub fn of(
        job: &core_model::Job,
        reason: Option<&core_model::TransitionReason>,
        queued_reason: Option<core_model::QueuedReason>,
        budget_hold: Option<core_model::BudgetHold>,
        asking: bool,
        resumption: Option<core_model::Resumption>,
        started_at: Option<core_model::Timestamp>,
        ended_at: Option<core_model::Timestamp>,
    ) -> JobSummary {
        JobSummary {
            id: job.id().into(),
            handle: job.handle(),
            title: job.title().as_str().to_string(),
            status: job.status().into(),
            created_at: job.created_at().into(),
            started_at: started_at.as_ref().map(Instant::from),
            ended_at: ended_at.as_ref().map(Instant::from),
            branch: job.branch().map(|branch| branch.as_str().to_string()),
            reason: reason.and_then(Reason::of),
            queued_reason: queued_reason.map(QueuedReason::from),
            budget_hold: budget_hold.map(BudgetHold::from),
            frozen_by: Vec::new(),
            resumption: resumption.map(Resumption::from),
            workflow_id: job.workflow_id().into(),
            owner_manifest_id: job.owner_manifest_id().into(),
            origin: job.origin().into(),
            urgency: job.urgency().into(),
            atomic: job.atomic(),
            model: job.model().as_str().to_string(),
            current_step_id: job.current_step_id().map(StepId::from),
            assigned_drone: job.assigned_drone().map(DroneId::from),
            redispatched_from: job.redispatched_from().map(JobId::from),
            dispatched_by: job.dispatched_by().map(|by| JobId::from(&by.job_id)),
            reclaimed_at: job.reclaimed_at().map(Instant::from),
            asking,
            // Filled by the caller that has it, and `None` here on purpose:
            // it is not a field of `core_model::Job` at all — it is what a
            // person did to the pull request afterwards, which lives in three
            // columns beside the row. A Board reads it for every row at once
            // and a single-Job answer leaves it out, because the alternative
            // is a store read per row on a list that redraws on every event.
            landed: None,
            // Filled by the caller that holds a store, for `landed`'s reason.
            tasks: None,
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

/// What forgetting a Job leaves to say. **The id, and nothing else** — there
/// is no row left for a summary to describe.
///
/// **Command response and event payload, deliberately the same type.**
/// `forget_job` answers with this and `job.forgotten` carries it: the two
/// have exactly one fact between them, so a second name for it would be a
/// synonym rather than a distinction. `Event::JobForgotten(JobForgotten)`
/// wraps it the same way `Event::JobCreated(JobCreated)` wraps its own type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobForgotten {
    pub job_id: JobId,
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

/// What a person says as they restart a step. The request half of
/// `restart_step`, and **the whole body is optional**.
///
/// # Absent is a plain restart, and there is no third spelling
///
/// `restart_step` took no body until this and a restart with nothing to say
/// still sends none, byte for byte. So there is no `Option` inside this type:
/// `{"note": null}` and `{}` would be two more spellings of the empty request,
/// and a reader would test three things to learn one.
///
/// # Its own type though it is structurally [`Redirection`]'s string
///
/// For that type's own reason: a redirect steers a Drone that is *there*, and
/// this opens the brief of one that does not exist yet. The two are told apart
/// on this seam by which route the bytes arrived on.
///
/// **It is not a reason for the restart**, and the record keeps none. What
/// happens to these words is that `redirect_waiting` holds them and the next
/// Drone's brief is built from them — `request_changes`'s road, and the block
/// `crossing::Redirected` renders.
///
/// Blank is refused at the Fleet boundary rather than here, for
/// [`Redirection`]'s reason: a decoded request is well-formed, and a note with
/// nothing in it is a value that cannot work — a 422 and not a 400.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartRequested {
    /// The person's own words, held on the Job and delivered into the opening
    /// brief of the Drone this restart asks for.
    pub note: String,
}

/// The redaction, at the Fleet boundary, with no reason to hand.
///
/// **No queued reason and no resumption either**, and that is not a gap: this
/// conversion is what an event publish uses, and nothing publishes a summary of
/// a `queued` Job — creation, a step advancing and a Drone arriving or leaving
/// all carry a Job in some other status. A publish that did would need the
/// board, which is exactly what this conversion does not have.
///
/// **`started_at` and `ended_at` are `None` here too, and that is only ever
/// right for a Job just created** — the one caller left on this conversion,
/// every one of which mints a fresh Job at its approval gate or its
/// sub-dispatch entry, neither of which has run, let alone ended. A caller
/// publishing about a Job that has already run — or ended — must call
/// [`JobSummary::of`] instead, with the log's own answer: this conversion
/// holds no log to ask.
impl From<&core_model::Job> for JobSummary {
    fn from(job: &core_model::Job) -> JobSummary {
        // **`asking` is `false` here and that is the answer, not a default.**
        // This conversion is what an event publish uses and it holds no slot;
        // creation, a step advancing and a Drone arriving or leaving are none of
        // them a Job that has just asked something. The message that says a
        // question exists is `job.asking`, which carries the question itself.
        //
        // `budget_hold` is `None` for the same reason `queued_reason` is: both
        // are read off the board rather than off the record, and this
        // conversion holds only the record.
        JobSummary::of(job, None, None, None, false, None, None, None)
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
    /// Which top-level origin the proposer claims. `sub_dispatched` does not
    /// deserialise.
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
    /// A token of the caller's own, echoed back on every event about this
    /// proposal so the caller can recognise its own call.
    ///
    /// **Opaque to Fleet, which neither reads it nor keeps it.** See
    /// [`ProposalInFlight::client_ref`](crate::ProposalInFlight::client_ref)
    /// for what it is for and why matching on the request's text is not it.
    ///
    /// Optional, and absent is the ordinary case: a caller with no surface has
    /// nothing to correlate. **It is not an id and Fleet mints nothing from
    /// it** — a caller sending the same token twice gets two proposals, each
    /// with its own [`ProposalId`](crate::ProposalId).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_ref: Option<String>,
    /// Files a person attached to the request before dispatching it.
    /// **Additive, like [`ProposeJob::attachments`]** — a caller that predates
    /// this field sends nothing and decodes exactly as it did before.
    ///
    /// A request can become several Jobs. These land on the head of the plan
    /// alone, not on every member: `docs/concepts/job-proposer.md`'s "a member
    /// of a split gets its scope line and nothing else" argues against
    /// duplicating an attachment onto Jobs that never asked for it.
    #[serde(default)]
    pub attachments: Vec<AttachmentRef>,
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
