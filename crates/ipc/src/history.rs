//! One Job's transition history, as the log recorded it. The answer to
//! `get_job_events`.
//!
//! # It is read and rendered, never replayed
//!
//! `crates/store/src/fold.rs` owns the machine: every recorded move goes back
//! through `Job::transition` there, and a history the machine would not admit
//! fails to fold rather than producing a Job no legal sequence could reach.
//! **Nothing on this side of the wire does that.** A client that folded these
//! would be a second machine, agreeing with the first only until one changed —
//! and it would gain nothing, because Fleet loads the Job before it reads the
//! log, so a history that arrives is one the machine already admitted.
//!
//! # Its own operation, and not a field on [`JobDetail`](crate::JobDetail)
//!
//! `get_job` is fetched on opening a Job, to draw a summary. A history has no
//! bound: it grows for as long as the Job lives, a retried step is a row per
//! attempt plus the moves around it, and the surface that draws it is folded
//! away by default. A field on the detail would make the common read pay for
//! the rare one, on every open.
//!
//! `event.rs` already says this from the other side — a resync carries where
//! every Job is and not the path taken, and "a surface that draws a timeline
//! reads `get_job_events`". This is that operation.

use serde::{Deserialize, Serialize};

use crate::enums::{Actor, DronePresence, JobStatus, StepState};
use crate::event::Reason;
use crate::ids::{DroneId, Instant, JobId, StepId};

/// Every move one Job made, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobHistory {
    /// The Job this is the history of. Carried so an answer can be bound to
    /// the question, the same way [`JobFilesChanged`](crate::JobFilesChanged)
    /// names its Job.
    pub job_id: JobId,
    /// Every recorded move, in `seq` order.
    ///
    /// **Empty is a real answer.** A Job created and not yet moved has no
    /// events at all — creation is not a transition, it has no `from`, and no
    /// row describes it.
    pub moves: Vec<Recorded>,
}

/// One row of the log.
///
/// **Both machines, in one order.** A status transition, a step move and a
/// Drone arriving are rows in the same table, which is what lets a step move be
/// ordered against the status transitions around it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recorded {
    /// The key the log assigned. Monotonic, never reused, and what orders the
    /// list — **never `at`**, which is injected rather than read from a clock,
    /// so two moves inside one millisecond carry the same instant.
    pub seq: i64,
    /// The status the Job stood in when this happened. For a status move it is
    /// the status it **left**; for a step or Drone move it is the status it
    /// stayed in, because neither of those moves the Job.
    pub status: JobStatus,
    pub moved: Movement,
    /// Who caused it. Three ways, and a row that did not record which never
    /// will.
    pub actor: Actor,
    pub at: Instant,
}

/// What the row says moved. The three shapes the log admits, and no fourth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Movement {
    /// The Job left [`Recorded::status`].
    Status(StatusMoved),
    /// One step moved and the Job did not.
    Step(StepMoved),
    /// A Drone arrived on the Job or left it, and neither machine moved.
    Drone(DroneMoved),
}

/// The Job's own machine moved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusMoved {
    /// Where it arrived. [`Recorded::status`] is where it left.
    pub to: JobStatus,
    /// The qualifying reason the transition stored, where it stored one.
    ///
    /// The same [`Reason`] a `job.state_changed` carries, so a timeline and the
    /// stream say a move's reason in one shape. Absent on the eight
    /// destinations that store none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
}

/// The inner machine moved, beneath a status that did not.
///
/// It has no status target for that reason: `status_from` and `status_to` on
/// the row are the same value, and it is [`Recorded::status`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepMoved {
    pub step_id: StepId,
    pub from: StepState,
    pub to: StepState,
    /// Why the step stopped, on the one move that stops it.
    ///
    /// A string rather than a mirrored enum, for the reason
    /// [`Verdict::trigger`](crate::Verdict) and [`Reason::named`] are strings:
    /// it is a trigger spelling from the registry, and a closed set restated
    /// here would be a second authority for a list that already has one. Absent
    /// on every move that does not stop a step, which is most of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// A Drone arrived or left.
///
/// **Presence, not a state pair.** `assigned_drone` is a pointer that is set or
/// null and has no states of its own, so what a row carries is which of the two
/// moments it was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneMoved {
    /// Which Drone. **The question an exit has to answer** once a Job has had
    /// more than one, which a restart or a redispatch gives it.
    pub drone_id: DroneId,
    pub presence: DronePresence,
}
