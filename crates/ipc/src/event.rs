//! What travels the socket: the resync, the events, and the admission that
//! events were dropped.
//!
//! # A reconnection resyncs; it does not replay
//!
//! Every connection opens with a [`StreamMessage::Resync`] carrying the current
//! state of every Job and the [`Cursor`] that state is current as of. Replay
//! from the beginning was rejected on what it would cost mid-Job: a Bridge
//! reopened after lunch would receive hours of transitions to fold before it
//! could draw anything, and it would be folding them into a Board that Fleet
//! could already state in one message.
//!
//! **What a client can rebuild from a resync:** where every Job is now, why —
//! where the reason was stored — which step it is on, and whether a Drone is on
//! it.
//!
//! **What it cannot:** the path taken. No transition history, no instants for
//! anything that already happened, and nothing at all about a Job that reached
//! a terminal status and was retained out. A surface that draws a timeline
//! reads `get_job_events`, and its timeline begins at the connection.
//!
//! # The stream is global, and a client subscribes to nothing
//!
//! One socket carries every Job, because Bridge holds exactly one connection
//! and the Board renders every Job on it. A per-Job subscription would put
//! state on a connection whose entire value is being cheap to drop and remake,
//! and would need a subscribe message, an unsubscribe message and a rule for
//! what a resync means when the set changes mid-stream.
//!
//! # Six event kinds are produced at M1
//!
//! The rest of `operations.toml`'s — `alert.raised`, `review.ready`,
//! `evidence.submitted` and `usage.threshold` — describe records this workspace
//! has no type for yet, and are **not stubbed**: a kind that exists and never
//! fires reads as a stream that is working.
//!
//! The Drone lifecycle pair left that list when `assigned_drone` got an event
//! that sets it. A Board could show a Drone only by re-reading the Job, and
//! nothing told it to.
//!
//! `job.step_advanced` was in that list until the inner machine arrived, and
//! then a Job running through four steps still emitted one event and nothing
//! after it. What changes most often during a run was what the stream did not
//! carry.
//!
//! `job.files_changed` is the sixth, and it is the only kind that describes a
//! worktree rather than a record. Fleet already read the footprint every turn a
//! step watched its scope and kept only the paths that had drifted; what a
//! person needs while a Drone works is the whole list, and nothing carried it.
//! **Bridge does not read a worktree** — no surface on the other side of this
//! seam opens a repository, so the only way a file list reaches one is as an
//! event like every other.
//!
//! # `job.created` is a kind and not a state change, and that is why it exists
//!
//! A Job proposed while a client was connected never reached it: creation
//! publishes nothing, so nothing woke Bridge, and the row appeared only when
//! something else forced a re-read.
//!
//! The alternative was publishing a [`JobStateChanged`] on creation. It was
//! rejected on what that message would have to say: the type has a `from` and
//! a `to`, and a created Job has no `from` — the honest fields would be
//! `from: awaiting_approval, to: awaiting_approval`, a transition the edge
//! table does not contain, from a status the Job was never in. Every client
//! folding the stream would apply a move that did not happen. A creation is a
//! row appearing, not a row moving, and the two are different messages.
//!
//! It carries the whole [`JobSummary`] for the same reason: a kind that only
//! named an id would make every client fetch the row it was just told about.
//!
//! # `job.forgotten` is the opposite message, and carries the opposite payload
//!
//! Where `job.created` carries the row whole because there is nothing yet for
//! a client to have, `job.forgotten` carries only the id — a `forget_job` is a
//! real deletion, through `Store::forget_job`, and by the time the event is
//! published there is no row left to carry. A client drops it from whatever
//! it is holding rather than replacing it.

use serde::{Deserialize, Serialize};

use crate::detail::JudgeInFlight;
use crate::enums::{Actor, JobStatus, StepState};
use crate::ids::{CriterionId, DroneId, Instant, JobId, StepId};
use crate::job::{JobForgotten, JobList, JobSummary};
use crate::version::ProtocolVersion;

/// A position in the stream. Monotonic, assigned by Fleet, never reused.
///
/// It orders messages and lets a client see a gap for itself. It is **not** a
/// resume token: a reconnection resyncs to current state, and nothing accepts a
/// cursor back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cursor(u64);

impl Cursor {
    pub fn at(position: u64) -> Self {
        Cursor(position)
    }
    pub fn position(&self) -> u64 {
        self.0
    }
}

/// One message from Fleet to a connected client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum StreamMessage {
    /// The first message on every connection, and the message after a drop.
    Resync(Resync),
    Event(Delivered),
    /// The bound was reached and the oldest were dropped. **Always followed by
    /// a resync**, because a client that knows only the count cannot repair
    /// what it holds — and a client that believes its history is complete when
    /// it is not renders a Board that is quietly wrong.
    Missed(Missed),
}

/// Current state, whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resync {
    /// Restated on the stream so a client that reached the socket without
    /// reading the runtime file still learns what it is talking to.
    pub protocol_version: ProtocolVersion,
    /// The position this state is current as of. The next `Event` carries a
    /// later one.
    pub cursor: Cursor,
    pub jobs: JobList,
}

/// One event, at its position.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delivered {
    pub cursor: Cursor,
    pub event: Event,
}

/// How many events the client will never see.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Missed {
    pub dropped: u64,
}

/// An unsolicited push. The `kind` is the dotted name `operations.toml` keys it
/// under, so a rule can compare the two without a mapping in between.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Event {
    #[serde(rename = "job.created")]
    JobCreated(JobCreated),
    #[serde(rename = "job.state_changed")]
    JobStateChanged(JobStateChanged),
    #[serde(rename = "job.step_advanced")]
    JobStepAdvanced(JobStepAdvanced),
    #[serde(rename = "drone.spawned")]
    DroneSpawned(DroneSpawned),
    #[serde(rename = "drone.exited")]
    DroneExited(DroneExited),
    #[serde(rename = "job.files_changed")]
    JobFilesChanged(JobFilesChanged),
    #[serde(rename = "job.judging")]
    JobJudging(JobJudging),
    #[serde(rename = "job.forgotten")]
    JobForgotten(JobForgotten),
}

/// A Job exists that did not before, whole enough to draw.
///
/// **The row, not a pointer to it.** A client is told what appeared rather than
/// that something appeared, so a Board can insert the row without a round trip
/// — which is what a Job proposed over the API and never seen in Bridge was
/// missing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobCreated {
    pub job: JobSummary,
    /// Who created it. A proposal is a human or Helm act; nothing here is Fleet
    /// deciding on its own.
    pub actor: Actor,
    pub at: Instant,
}

/// A Job moved. The one event the Job record can produce.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobStateChanged {
    pub job_id: JobId,
    pub from: JobStatus,
    pub to: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// Who caused it. Three ways, and it cannot be reconstructed afterwards.
    pub actor: Actor,
    pub at: Instant,
}

impl From<&core_model::JobEvent> for JobStateChanged {
    fn from(event: &core_model::JobEvent) -> JobStateChanged {
        JobStateChanged {
            job_id: event.job_id().into(),
            from: event.from().into(),
            to: event.to().into(),
            reason: Reason::of(event.reason()),
            actor: event.actor().into(),
            at: event.at().into(),
        }
    }
}

/// A step of the frozen WorkflowDef moved. **The Job did not.**
///
/// It carries the whole [`JobSummary`] for the reason [`JobCreated`] does: the
/// Job's `current_step_id` moves when a step enters `running`, and a kind that
/// named only ids would make every client re-read the row it was just told
/// about — which is the reload this event exists to stop.
///
/// `status` is the status the move happened *beneath*, not a move. The inner
/// machine advances only while the Job is `running` or `awaiting_review`, and
/// a client folding this must not read it as a status change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobStepAdvanced {
    /// The Job as it now stands. Replaces the row whole.
    pub job: JobSummary,
    pub step_id: StepId,
    pub from: StepState,
    pub to: StepState,
    /// The status the step moved beneath. Unchanged by this event.
    pub status: JobStatus,
    /// The trigger the move carried, on the two moves that carry one: a step
    /// stopping, and a stopped step advanced by a person who overruled the
    /// verdict.
    ///
    /// The same field [`StepMoved::why`](crate::StepMoved) carries in the
    /// history, and a string for that field's reason — a trigger spelling
    /// belongs to the registry, and a closed set restated here would be a
    /// second authority for a list that already has one. **Absent on every
    /// other move**, and `from: stopped, to: advanced` is what makes it an
    /// override rather than a stop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    pub actor: Actor,
    pub at: Instant,
}

impl JobStepAdvanced {
    /// The move, plus the Job it happened on. The Job is a second argument
    /// because the event does not carry one — `core-model` records the move,
    /// and only a caller holding the record can redact it.
    pub fn of(event: &core_model::StepEvent, job: JobSummary) -> JobStepAdvanced {
        JobStepAdvanced {
            job,
            step_id: event.step_id().into(),
            from: event.from().into(),
            to: event.to().into(),
            status: event.under().into(),
            why: event.why().map(|why| why.as_wire().to_string()),
            actor: event.actor().into(),
            at: event.at().into(),
        }
    }
}

/// A Drone started against a Job's worktree.
///
/// It carries the whole [`JobSummary`] for the reason [`JobStepAdvanced`] does:
/// `assigned_drone` is a field of that row, so a client replaces the row rather
/// than re-reading it.
///
/// **`branch` is here and not on the summary.** The registry describes this
/// event as a Drone starting *against a worktree*, and the branch is what a
/// person checks out to see what it is doing. It is absent only where the
/// worktree could not name one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneSpawned {
    pub job: JobSummary,
    /// The step it was put on. A Drone belongs to a workflow step, and this is
    /// which `job_steps` row a client sets its `assigned_drone` on.
    pub step_id: StepId,
    pub drone_id: DroneId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Always Fleet. Carried anyway, because an actor cannot be reconstructed
    /// afterwards and a field absent on one event kind and present on the rest
    /// is a shape a client has to special-case.
    pub actor: Actor,
    pub at: Instant,
}

/// A Drone is gone, however it went.
///
/// **It does not say why.** What a Drone's ending means is the Job's own
/// transition, which is a `job.state_changed` of its own — an outcome carried
/// here too would be a second statement of it, and the two would disagree the
/// first time one path forgot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneExited {
    /// The Job as it now stands, with `assigned_drone` gone.
    pub job: JobSummary,
    /// The step it left. The row whose `assigned_drone` a client clears — and
    /// the reason the pair of them can be told apart on a Job that has had
    /// several.
    pub step_id: StepId,
    /// The Drone that left. Named, because "which one" is the question an exit
    /// has to answer once a Job has had more than one.
    pub drone_id: DroneId,
    pub actor: Actor,
    pub at: Instant,
}

impl DroneSpawned {
    /// The arrival, plus the Job it happened on. The Job is a second argument
    /// for the reason [`JobStepAdvanced::of`] takes one: `core-model` records
    /// the move, and only a caller holding the record can redact it.
    pub fn of(event: &core_model::DroneMoved, job: JobSummary, branch: Option<String>) -> Self {
        DroneSpawned {
            job,
            step_id: event.step_id().into(),
            drone_id: event.drone_id().into(),
            branch,
            actor: event.actor().into(),
            at: event.at().into(),
        }
    }
}

impl DroneExited {
    pub fn of(event: &core_model::DroneMoved, job: JobSummary) -> Self {
        DroneExited {
            job,
            step_id: event.step_id().into(),
            drone_id: event.drone_id().into(),
            actor: event.actor().into(),
            at: event.at().into(),
        }
    }
}

/// What happened to one file, in the vocabulary a person reads.
///
/// **A closed set that is not expected to grow.** Every delta Fleet's reading
/// can produce has a variant, so a client's `match` on this is exhaustive today
/// and stays exhaustive — which is what keeps a later addition from being the
/// major bump the protocol table says a new matched-on variant is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// The file did not exist when the branch was cut. Staged or not — an
    /// untracked file a Drone wrote is added.
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    /// A file became a directory or a symlink, or the reverse. Not a content
    /// change.
    TypeChanged,
    Conflicted,
    /// In the diff, and could not be read. **Not an absence of change** — one
    /// path's reading failed and the rest did not.
    Unreadable,
}

/// One file in the Drone's footprint.
///
/// **A name and a kind, never bytes.** What changed inside a file is the patch,
/// which is read only when a Judge fires and is deliberately not on this seam:
/// a stream carrying diffs at Drone speed is the thing the event channel's
/// bound exists to keep off it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedFile {
    /// Repository-relative, exactly as git spells it.
    pub path: String,
    pub change: ChangeKind,
    /// This path is not covered by the plan the step declared.
    ///
    /// **A mark, not a judgement.** It restates the comparison the live scope
    /// check already made and decides nothing: drift does not fail a step, and
    /// a Drone that finds the real work elsewhere answers by declaring again.
    /// Always `false` where the step declared no plan, which is what
    /// [`JobFilesChanged::plan_declared`] is for.
    #[serde(default)]
    pub outside_plan: bool,
}

/// What the working Drone has changed in its worktree, as of one reading.
///
/// **The whole footprint, not a delta.** A client replaces the list it holds
/// rather than folding this into one, so a file that stopped being changed —
/// a revert, a `git checkout` — leaves the view by not being in the next
/// reading. A stream of additions could never say that.
///
/// It names no [`JobSummary`], unlike the kinds that move a row: nothing on the
/// Board changes when a file does, and this is read by a detail view somebody
/// opened on one Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobFilesChanged {
    pub job_id: JobId,
    /// Which step's Drone did it. The footprint is measured from where the
    /// branch was cut, so it is the Job's whole work and not this step's — the
    /// step is here to say who is holding the pen.
    pub step_id: StepId,
    pub drone_id: DroneId,
    /// Whether the step has a declared plan for `outside_plan` to mean
    /// anything. **False is "there is no plan", not "nothing drifted"**, and a
    /// surface that drew the two the same way would report every unscoped step
    /// as perfectly on plan.
    pub plan_declared: bool,
    /// Every file, in the order the reading found them. Empty is a real
    /// answer: a Drone that has changed nothing yet.
    pub files: Vec<ChangedFile>,
    /// Always Fleet. Carried for the reason [`DroneSpawned`] carries one — a
    /// field absent on one kind and present on the rest is a shape a client has
    /// to special-case.
    pub actor: Actor,
    pub at: Instant,
}

/// A Judge call went out on a step, or the one that was out came back.
///
/// **Two messages per call and never a third.** The one that goes out carries
/// [`judging`](JobJudging::judging) and the instant it went out; the one that
/// comes back carries nothing. A surface ages the wait from `since` rather than
/// being told the time, so a call that legitimately takes the whole two-minute
/// budget costs this channel two messages and not a hundred and twenty.
///
/// **It names no [`JobSummary`].** Nothing on the Board's row changes when a
/// call goes out — the Job is `running` before and after — and this is read by
/// a detail view somebody has open on one Job, exactly as
/// [`JobFilesChanged`] is.
///
/// # What it costs the channel, stated
///
/// The bound is `api::stream::BACKLOG` and it is shared by every kind. This one
/// produces at the rate the *step's own declaration* implies: two messages per
/// call, criteria × panel size calls, plus the looks Fleet adds. A step
/// declaring nothing produces none at all. Unlike the footprint it does not ask
/// whether anybody is watching, because there is nothing to decline — the value
/// is already in hand when the call goes out, and a publish nobody is
/// subscribed to is a drop that costs nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobJudging {
    pub job_id: JobId,
    /// Which step is being asked about. The Job may be `running` under any of
    /// its steps, and a Job's gate asks about one at a time.
    pub step_id: StepId,
    /// The call that went out. **Absent because it came back** — however it
    /// came back, with a verdict or with a failure, and the answer itself
    /// arrives as `judged`/`flagged` on the step rather than here.
    ///
    /// This is the field that makes the absence legible: a step nobody is
    /// asking about and a step whose call has just returned are the same, and
    /// they are both this message with nothing in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judging: Option<JudgeInFlight>,
    /// Always Fleet. A Judge call authenticates as Fleet and no Drone can cause
    /// one. Carried for the reason [`DroneSpawned`] carries one.
    pub actor: Actor,
    pub at: Instant,
}

/// The qualifying reason a transition carried.
///
/// Two fields rather than one string, because two of the five stored reasons
/// are not a closed-set name: an attestation debt is a list of criterion
/// references, and `queued`'s readiness is derived at read time and stored
/// nowhere.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    /// The closed-set spelling, where the reason has one — an escalation
    /// trigger or a pilot reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
    /// The criteria owed, where the Job is waiting on an attestation. Never
    /// empty when present: a Job cannot wait on an attestation it does not owe.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub criteria_owed: Vec<CriterionId>,
}

impl Reason {
    /// The reason as the wire carries it, or `None` where the transition
    /// carried nothing to say.
    ///
    /// `None` covers the eight destinations that store no reason and `queued`,
    /// whose reason a reader recomputes. Both are absent rather than present
    /// and empty — the absent-versus-null rule the log envelope holds.
    pub fn of(reason: &core_model::TransitionReason) -> Option<Reason> {
        let named = reason.as_wire().map(str::to_string);
        let criteria_owed = match reason {
            core_model::TransitionReason::Attestation(owed) => {
                owed.ids().map(CriterionId::from).collect()
            }
            _ => Vec::new(),
        };
        if named.is_none() && criteria_owed.is_empty() {
            return None;
        }
        Some(Reason {
            named,
            criteria_owed,
        })
    }
}
