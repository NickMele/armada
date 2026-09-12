//! The processes Fleet is holding, and what one of them has been doing.
//!
//! **Read off the roster, never off the Jobs.** A Job that escalated keeps its
//! Drone alive and idle so a redirect costs no respawn, so a list derived from
//! statuses would omit exactly the Drone somebody is asking about.
//!
//! What drifted outside a declaration is not here: it rides on `get_diff`'s
//! file list, where the whole Job's work is measured rather than one slot's.

use serde::{Deserialize, Serialize};

use crate::ids::{DroneId, Instant, JobId, StepId};
use crate::turn::TranscriptRow;

/// One Drone in a working slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneSummary {
    pub drone_id: DroneId,
    pub job_id: JobId,
    /// What a person calls the Job, and what the Drone's transcript file is
    /// named under.
    pub handle: String,
    /// The step it was put on. **It never moves**: a slot does not outlive a
    /// step boundary, so this is both where it started and where it is.
    pub step_id: StepId,
    /// The checkout it is writing in. **Absent where the worktree is gone** —
    /// a Drone outliving its checkout is a real state and not a blank string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    /// The process Fleet is holding. **The fact a Doctor probe asked for** —
    /// which process is working which Job.
    pub pid: u32,
    /// When the Drone arrived on the step, off the Job's own log. **Absent
    /// where the log has no arrival for it**, which a reclaimed record gives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Instant>,
}

/// Every Drone Fleet is holding.
///
/// **Read off the process register, never off the slots.** A slot is held for
/// the length of a Check, so a read that took one would block behind a gate;
/// what this walks is the pid map, which nothing holds across an await.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneList {
    pub drones: Vec<DroneSummary>,
}

/// One Drone, what it promised, and what it has said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneDetail {
    pub drone: DroneSummary,
    /// The paths the Drone said this step's work would be in, as the record
    /// kept them. **`None` until it declares**, which is a different answer
    /// from an empty declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared: Option<Vec<String>>,
    /// When that declaration was taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_at: Option<Instant>,
    /// This Drone's own rows, oldest last, narrowed for a viewer. **A window,
    /// not the transcript**: see [`DroneDetail::older`].
    pub turns: Vec<TranscriptRow>,
    /// How many older rows this window left out. **Non-zero means the answer
    /// is a tail**, and a caller that drew a conclusion about when something
    /// started from a tail would be reading the window rather than the Drone.
    pub older: u64,
}
