//! What the fleet has spent, against the ceilings that refuse the next Drone.
//!
//! **Spend, not quota.** `settings.budget-quota-floor-for-interactive-use`
//! records that no quantity reaches Armada from a Drone's stream, so there is
//! no percentage to hold a floor against and nothing here claims one.

use serde::{Deserialize, Serialize};

/// One Job admission will not start another Drone on, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overspending {
    /// What a person calls the Job.
    pub handle: String,
    /// Which ceiling it met, as `enum-verbs.toml` spells it — `cost` or
    /// `turns`. A string, for [`EventTally::kind`](crate::EventTally)'s reason.
    pub ceiling: String,
    /// What the Job has spent against that ceiling.
    pub spent: u64,
    /// The ceiling in force for this Job, resolved across the three tiers.
    pub allowed: u64,
}

/// The fleet's spend, summed across every Job it holds.
///
/// **No ceiling crosses, only what one did.** A cap resolves across three tiers
/// per Job — `fleet::Allowance::at` — so a single number here would be a tier
/// nothing asks about. What is actionable is which Jobs are held, below.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetUsage {
    /// Every priced Drone of every Job, added up.
    pub cost_micros: u64,
    /// Every turn of every Drone, added up. Turns sum across terminating lines
    /// where cost does not, which is why the two are counted separately.
    pub turns: u64,
    /// How many Drones this is the sum of.
    pub drones: u64,
    /// How many of those named no price. **`cost_micros` is a floor while this
    /// is non-zero** — cost reaches Armada on a session's final line, so a
    /// Drone signalled mid-run leaves none.
    pub unpriced: u64,
    /// How many Jobs were summed.
    pub jobs: u64,
    /// Jobs the ceilings are holding out of dispatch right now, with which
    /// ceiling each hit. Empty is the ordinary answer.
    pub over_budget: Vec<Overspending>,
}
