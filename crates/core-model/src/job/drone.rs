//! A Drone arriving on a step, and leaving it — the third thing the log folds.
//!
//! # Presence, not a machine
//!
//! `job-fields.toml` gives `assigned_drone` no states: it is a pointer that is
//! set or null, and null also suspends the liveness clock. So there is no edge
//! table here and no [`StepTarget`](crate::StepTarget) equivalent — two moments
//! change the pointer and nothing else about a Drone's life is recorded.
//!
//! # The pointer is per step, and so is every move of it
//!
//! A Drone belongs to a workflow step, so the pointer is a `job_steps` column
//! and every row here names the step it is about — the way [`StepEvent`]
//! already does. Without the step the fold has nothing to key by, and a Job
//! that ran four Drones would collapse them into one pointer that names the
//! last.
//!
//! [`StepEvent`]: crate::StepEvent
//!
//! # Why it is an event rather than a column written directly
//!
//! `branch` is a column written directly, because nothing can be derived from
//! when a worktree appeared. A Drone's arrival and departure are what a Job's
//! own history has to be able to answer — which Drone worked it, and when it
//! stopped — and a column overwritten in place answers neither.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

use crate::envelope::{Actor, FieldValue, Timestamp};
use crate::job::ids::{DroneId, JobId, StepId};
use crate::job::record::Job;
use crate::job::status::JobStatus;

/// Whether a Drone arrived on a Job or left it.
///
/// **Presence, not state.** `job-fields.toml` gives `assigned_drone` no states
/// of its own — it is a pointer that is set or null — so the log carries the
/// two moments that change it and nothing else about a Drone's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DronePresence {
    /// A Drone started against the Job's worktree, working one step. That
    /// step's `assigned_drone` is now it.
    Spawned,
    /// It is gone, however it went. The step's `assigned_drone` is null again,
    /// which also suspends the liveness clock.
    Exited,
}

impl DronePresence {
    pub fn as_wire(&self) -> &'static str {
        match self {
            DronePresence::Spawned => "drone_spawned",
            DronePresence::Exited => "drone_exited",
        }
    }

    pub fn from_wire(value: &str) -> Option<DronePresence> {
        match value {
            "drone_spawned" => Some(DronePresence::Spawned),
            "drone_exited" => Some(DronePresence::Exited),
            _ => None,
        }
    }
}

/// A Drone arriving on a step of a Job, or leaving it. **The Job does not
/// move, and neither does the step.**
///
/// Minted only by [`Job::drone_spawned`](crate::Job::drone_spawned) and
/// [`Job::drone_exited`](crate::Job::drone_exited), for the reason [`JobEvent`]
/// is minted only by `Job::transition`. Like [`StepEvent`] it carries the
/// status it happened *under*, so the fold checks continuity over one log
/// rather than believing a second one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneMoved {
    job_id: JobId,
    step_id: StepId,
    drone_id: DroneId,
    presence: DronePresence,
    under: JobStatus,
    actor: Actor,
    at: Timestamp,
}

impl DroneMoved {
    pub(crate) fn recorded(
        job_id: JobId,
        step_id: StepId,
        drone_id: DroneId,
        presence: DronePresence,
        under: JobStatus,
        actor: Actor,
        at: Timestamp,
    ) -> Self {
        DroneMoved {
            job_id,
            step_id,
            drone_id,
            presence,
            under,
            actor,
            at,
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }
    /// The step the Drone was put on, or taken off. **Present on both**, for
    /// the reason `drone_id` is: it is the key the fold applies the move by,
    /// and an exit that did not name its step could be applied to any of them.
    pub fn step_id(&self) -> &StepId {
        &self.step_id
    }
    /// The Drone that arrived, or the one that left. Present on both, because
    /// "which Drone exited" is the question an exit line has to answer.
    pub fn drone_id(&self) -> &DroneId {
        &self.drone_id
    }
    pub fn presence(&self) -> DronePresence {
        self.presence
    }
    /// The Job status this happened beneath. The Job does not leave it — a
    /// Drone arriving is not the Job moving.
    pub fn under(&self) -> JobStatus {
        self.under
    }
    pub fn actor(&self) -> Actor {
        self.actor
    }
    pub fn at(&self) -> &Timestamp {
        &self.at
    }

    /// The move as structured log fields, for the reason [`JobEvent::fields`]
    /// gives: nothing greps a sentence.
    pub fn fields(&self) -> BTreeMap<String, FieldValue> {
        let mut fields = BTreeMap::new();
        fields.insert(
            "drone_id".to_string(),
            FieldValue::Str(self.drone_id.as_str().to_string()),
        );
        fields.insert(
            "step_id".to_string(),
            FieldValue::Str(self.step_id.as_str().to_string()),
        );
        fields.insert(
            "drone_presence".to_string(),
            FieldValue::Str(self.presence.as_wire().to_string()),
        );
        fields.insert(
            "job_status".to_string(),
            FieldValue::Str(self.under.as_wire().to_string()),
        );
        fields
    }
}

/// A Job, and the Drone arrival or departure that produced it.
///
/// The pair travels together for the reason [`Transitioned`] does: the column
/// and its log entry are written in one transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneAssigned {
    pub job: Job,
    pub event: DroneMoved,
}

/// Why a Drone could not be put on a step, or taken off it.
///
/// **The refusal narrowed with the pointer; it did not go away.** A Drone per
/// step still means one Drone per step, and `restart_step` is the act that puts
/// a second one on the *same* step — so a spawn over a live one would still
/// lose the first Drone's id, which is the only thing naming its transcript.
/// Every variant names the step, because that is what the caller has to act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IllegalDroneMove {
    /// A Drone is already on that step. The one held is named, because it is
    /// what a caller has to end before another can start.
    AlreadyAssigned {
        step: StepId,
        held: DroneId,
        offered: DroneId,
    },
    /// Nothing is on that step, so nothing can leave it.
    NoneAssigned { step: StepId },
    /// The Job has no such step.
    ///
    /// **Unrepresentable while the pointer was per Job**, and it is the reason
    /// this variant exists: a drone row naming a step `job_steps` does not have
    /// would otherwise fold as a success that changed nothing.
    NoSuchStep { step: StepId },
}

impl core::fmt::Display for IllegalDroneMove {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IllegalDroneMove::AlreadyAssigned {
                step,
                held,
                offered,
            } => write!(
                f,
                "drone {} is already on step {}; {} cannot also be",
                held.as_str(),
                step.as_str(),
                offered.as_str()
            ),
            IllegalDroneMove::NoneAssigned { step } => write!(
                f,
                "no drone is on step {}, so none can have exited",
                step.as_str()
            ),
            IllegalDroneMove::NoSuchStep { step } => write!(
                f,
                "this Job has no step {}, so no drone can be on it",
                step.as_str()
            ),
        }
    }
}

impl core::error::Error for IllegalDroneMove {}
