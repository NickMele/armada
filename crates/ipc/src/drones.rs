//! The processes Fleet is holding, and what one of them has been doing.
//!
//! **Read off the roster, never off the Jobs.** A Job that escalated keeps its
//! Drone alive and idle so a redirect costs no respawn, so a list derived from
//! statuses would omit exactly the Drone somebody is asking about.

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
    /// The checkout it is writing in.
    pub worktree: String,
    /// When the step in this slot began, by Fleet's clock.
    pub since: Instant,
    /// Whether Fleet adopted this Drone rather than spawning it. **An adopted
    /// Drone has no pipe to speak into**, so every act that would say something
    /// to it is refused — which is a fact about the slot, not about the Job.
    pub adopted: bool,
}

/// Every Drone Fleet is holding a slot for.
///
/// **`occupied` on `get_capacity` is this list's length**, taken from the same
/// roster under the same lock. Two counts that could disagree is what a second
/// derivation here would be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneList {
    pub drones: Vec<DroneSummary>,
}

/// One Drone, what it promised, and what it has said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneDetail {
    pub drone: DroneSummary,
    /// The paths the Drone said this step's work would be in. **`None` until it
    /// declares**, which is a different answer from an empty declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared: Option<Vec<String>>,
    /// Every file seen changed outside that declaration while the step ran, in
    /// the order first seen. It fails nothing — the Drone may declare again.
    pub drifted: Vec<String>,
    /// This Drone's own rows, oldest last, narrowed for a viewer. **A window,
    /// not the transcript**: see [`DroneDetail::older`].
    pub turns: Vec<TranscriptRow>,
    /// How many older rows this window left out. **Non-zero means the answer
    /// is a tail**, and a caller that drew a conclusion about when something
    /// started from a tail would be reading the window rather than the Drone.
    pub older: u64,
}
