//! The three limits a person changes while Fleet runs: how many Drones at
//! once, how much memory must be free, how much disk must be free.
//!
//! **A value out of range cannot be decoded**, so it never becomes a request.
//! [`SaveLimits`] holds each field as a [`Within`], whose deserializer refuses
//! a number outside its bounds — the save route answers that as the 400 every
//! undecodable body gets, and Fleet is never asked. There is no range check
//! downstream to forget.
//!
//! **What comes back is plain numbers.** [`FleetLimits`] reports what is in
//! force and what shipped, and a shipped constant is not the person's to be
//! refused for.

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize, Serializer};

/// A whole number from `LO` to `HI`, both included, and no other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Within<const LO: u32, const HI: u32>(u32);

impl<const LO: u32, const HI: u32> Within<LO, HI> {
    /// The value, where it is in range.
    pub fn new(value: u32) -> Option<Within<LO, HI>> {
        (LO..=HI).contains(&value).then_some(Within(value))
    }

    pub const fn get(&self) -> u32 {
        self.0
    }
}

impl<const LO: u32, const HI: u32> Serialize for Within<LO, HI> {
    fn serialize<S: Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_u32(self.0)
    }
}

impl<'de, const LO: u32, const HI: u32> Deserialize<'de> for Within<LO, HI> {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(input)?;
        Within::new(value)
            .ok_or_else(|| D::Error::custom(format!("{value} is outside {LO} to {HI}")))
    }
}

/// Drones at once. `settings.concurrency-cap`.
pub type DronesAtOnce = Within<1, 8>;
/// The share of memory that must be free before a Drone starts, in whole
/// percent. `settings.cpu-mem-headroom-threshold-for-spawning`.
pub type MemorySparePercent = Within<0, 50>;
/// The gibibytes that must be free on the worktree volume.
/// `settings.disk-headroom-floor-for-spawning`.
pub type DiskFloorGib = Within<0, 100>;

/// One value for each of the three limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitValues {
    pub concurrency: u32,
    pub memory_spare_percent: u32,
    pub disk_floor_gib: u32,
}

/// The limits in force, and the ones Fleet shipped with.
///
/// **Flat, with `shipped` beside the three**, so a reader of the values in
/// force reads them where they would be with no `shipped` at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetLimits {
    #[serde(flatten)]
    pub values: LimitValues,
    pub shipped: LimitValues,
}

/// A save. **An omitted field keeps its current value**, saved or shipped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<DronesAtOnce>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_spare_percent: Option<MemorySparePercent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_floor_gib: Option<DiskFloorGib>,
}
