//! The four limits a person changes while Fleet runs — Drones at once, the
//! memory share, the disk floor, Checks at once — and how a saved one reaches
//! admission and the gate.
//!
//! **Shipped, overlaid by saved.** The composition root hands in the shipped
//! numbers as [`Fittings`](crate::daemon::Fittings)' `concurrency`, `headroom`
//! and `checks_at_once`; a field somebody saved replaces its shipped value at
//! assembly and again at every save. A stored value outside the range the wire
//! allows is ignored rather than trusted, since only a hand-edited file could
//! hold one.
//!
//! **A save changes the next admission and the next Check, and nothing else.**
//! The roster's bound and the headroom are replaced under the roster lock, so
//! no admission sees one limit changed and the other not; a Drone already
//! working keeps working, a gate already running keeps the headroom it began
//! with, and a saved Checks at once counts from the next place given out. No
//! Commands method admits — `crate::admitting` says why — so this does not either.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{DiskFloorGib, DronesAtOnce, FleetLimits, LimitValues, MemorySparePercent, SaveLimits};
use store::{LoadJobError, SavedLimits};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::headroom::{Bytes, Headroom, Spare};
use crate::places::ChecksAtOnce;
use crate::slots::Concurrency;

/// The limits admission holds a new Drone to, and a gate holds its Checks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub concurrency: Concurrency,
    pub headroom: Headroom,
    pub checks_at_once: ChecksAtOnce,
}

impl Limits {
    /// These limits with every value a person saved put over them.
    pub fn overlaid_by(self, saved: &SavedLimits) -> Limits {
        let concurrency = saved
            .concurrency
            .and_then(DronesAtOnce::new)
            .map(|jobs| Concurrency::of(jobs.get() as usize))
            .unwrap_or(self.concurrency);
        let spare = saved
            .memory_spare_percent
            .and_then(MemorySparePercent::new)
            .map(|share| Spare::percent(share.get()))
            .unwrap_or(self.headroom.memory_spare());
        let disk = saved
            .disk_floor_gib
            .and_then(DiskFloorGib::new)
            .map(|floor| Bytes::gibibytes(u64::from(floor.get())))
            .unwrap_or(self.headroom.disk_floor());
        let checks_at_once = saved
            .checks_at_once
            .and_then(ipc::ChecksAtOnce::new)
            .map(|checks| ChecksAtOnce::of(checks.get() as usize))
            .unwrap_or(self.checks_at_once);
        Limits {
            concurrency,
            headroom: Headroom::of(spare, disk),
            checks_at_once,
        }
    }

    /// As the wire spells them. Saturating rather than `as`, for
    /// `ipc::FleetCapacity::of`'s reason.
    pub fn values(&self) -> LimitValues {
        LimitValues {
            concurrency: u32::try_from(self.concurrency.jobs()).unwrap_or(u32::MAX),
            memory_spare_percent: self.headroom.memory_spare().percentage(),
            disk_floor_gib: u32::try_from(self.headroom.disk_floor().whole_gibibytes())
                .unwrap_or(u32::MAX),
            checks_at_once: u32::try_from(self.checks_at_once.get()).unwrap_or(u32::MAX),
        }
    }
}

/// A save put over what was saved before it: a field the save names replaces
/// its value, and one it omits keeps whatever was there — including nothing.
pub(crate) fn merged(before: SavedLimits, save: &SaveLimits) -> SavedLimits {
    SavedLimits {
        concurrency: save.concurrency.map(|v| v.get()).or(before.concurrency),
        memory_spare_percent: save
            .memory_spare_percent
            .map(|v| v.get())
            .or(before.memory_spare_percent),
        disk_floor_gib: save
            .disk_floor_gib
            .map(|v| v.get())
            .or(before.disk_floor_gib),
        checks_at_once: save
            .checks_at_once
            .map(|v| v.get())
            .or(before.checks_at_once),
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The limits in force and what shipped, read under the roster so the bound
    /// and the headroom are one instant.
    pub(crate) async fn limits_in_force(&self) -> FleetLimits {
        let slots = self.slots().lock().await;
        FleetLimits {
            values: Limits {
                concurrency: Concurrency::of(slots.cap()),
                headroom: self.headroom(),
                checks_at_once: self.checks_at_once(),
            }
            .values(),
            shipped: self.shipped().values(),
        }
    }

    /// Save, then put what was saved in force.
    ///
    /// **Roster, then store** — the order `crate::slots` states. The store is
    /// written before anything in memory moves, so a save that did not land
    /// changes nothing a person could then see disagree with a restart.
    pub(crate) async fn save_limits_now(&self, save: SaveLimits) -> Result<FleetLimits, Adrift> {
        let mut slots = self.slots().lock().await;
        let saved = {
            let mut store = self.store().lock().await;
            let before = store
                .saved_limits()
                .map_err(|fault| Adrift::Reading(LoadJobError::Database(fault)))?;
            let saved = merged(before, &save);
            store.save_limits(&saved).map_err(Adrift::Writing)?;
            saved
        };
        let limits = self.shipped().overlaid_by(&saved);
        slots.rebound(limits.concurrency);
        self.rehoused(limits.headroom);
        self.rechecked(limits.checks_at_once);
        Ok(FleetLimits {
            values: limits.values(),
            shipped: self.shipped().values(),
        })
    }
}
