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
//! # Five event kinds are produced at M1
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

use serde::{Deserialize, Serialize};

use crate::enums::{Actor, JobStatus, StepState};
use crate::ids::{CriterionId, DroneId, Instant, JobId, StepId};
use crate::job::{JobList, JobSummary};

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
    pub protocol_version: u32,
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
            drone_id: event.drone_id().into(),
            actor: event.actor().into(),
            at: event.at().into(),
        }
    }
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
