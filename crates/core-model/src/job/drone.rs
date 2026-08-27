//! A Drone arriving on a Job, and leaving it — the third thing the log folds.
//!
//! # Presence, not a machine
//!
//! `job-fields.toml` gives `assigned_drone` no states: it is a pointer that is
//! set or null, and null also suspends the liveness clock. So there is no edge
//! table here and no [`StepTarget`](crate::StepTarget) equivalent — two moments
//! change the pointer and nothing else about a Drone's life is recorded.
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
use crate::job::ids::{DroneId, JobId};
use crate::job::record::Job;
use crate::job::status::JobStatus;

/// Whether a Drone arrived on a Job or left it.
///
/// **Presence, not state.** `job-fields.toml` gives `assigned_drone` no states
/// of its own — it is a pointer that is set or null — so the log carries the
/// two moments that change it and nothing else about a Drone's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DronePresence {
    /// A Drone started against the Job's worktree. `assigned_drone` is now it.
    Spawned,
    /// It is gone, however it went. `assigned_drone` is null again, which also
    /// suspends the liveness clock.
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

/// A Drone arriving on a Job, or leaving it. **The Job does not move.**
///
/// Minted only by [`Job::drone_spawned`](crate::Job::drone_spawned) and
/// [`Job::drone_exited`](crate::Job::drone_exited), for the reason [`JobEvent`]
/// is minted only by `Job::transition`. Like [`StepEvent`] it carries the
/// status it happened *under*, so the fold checks continuity over one log
/// rather than believing a second one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneMoved {
    job_id: JobId,
    drone_id: DroneId,
    presence: DronePresence,
    under: JobStatus,
    actor: Actor,
    at: Timestamp,
}

impl DroneMoved {
    pub(crate) fn recorded(
        job_id: JobId,
        drone_id: DroneId,
        presence: DronePresence,
        under: JobStatus,
        actor: Actor,
        at: Timestamp,
    ) -> Self {
        DroneMoved {
            job_id,
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

/// Why a Drone could not be put on a Job, or taken off it.
///
/// Two variants, because `assigned_drone` is one pointer: it is either held or
/// it is not, and each refusal names which.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IllegalDroneMove {
    /// A Drone is already on the Job. The one held is named, because it is what
    /// a caller has to end before another can start.
    AlreadyAssigned { held: DroneId, offered: DroneId },
    /// Nothing is on the Job, so nothing can leave it.
    NoneAssigned,
}

impl core::fmt::Display for IllegalDroneMove {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IllegalDroneMove::AlreadyAssigned { held, offered } => write!(
                f,
                "drone {} is already on this Job; {} cannot also be",
                held.as_str(),
                offered.as_str()
            ),
            IllegalDroneMove::NoneAssigned => {
                f.write_str("no drone is on this Job, so none can have exited")
            }
        }
    }
}

impl core::error::Error for IllegalDroneMove {}
