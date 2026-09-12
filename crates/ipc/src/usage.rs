//! What the fleet has spent, against the ceilings that refuse the next Drone.
//!
//! **Spend, not quota.** `settings.budget-quota-floor-for-interactive-use`
//! records that no quantity reaches Armada from a Drone's stream, so there is
//! no percentage to hold a floor against and nothing here claims one.

use serde::{Deserialize, Serialize};

/// The fleet's spend, summed across every Job it holds.
///
/// **Notional dollars.** `docs/spikes/005-what-does-a-job-cost.md` priced the
/// figure at list price on an account nothing bills per token — a runaway
/// detector rather than an invoice.
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
    /// The machine-tier ceiling one Job may spend before admission refuses its
    /// next Drone. **Per Job, never fleet-wide**: nothing holds a ceiling on
    /// the total above, and a reader adding one up would invent a gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_cap_micros_per_job: Option<u64>,
    /// The other machine-tier ceiling, in turns, refused the same way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_cap_per_job: Option<u64>,
    /// The handles of Jobs held out of dispatch by one of the two ceilings.
    /// Empty is the ordinary answer.
    pub over_budget: Vec<String>,
}
