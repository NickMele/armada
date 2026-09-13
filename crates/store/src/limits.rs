//! The three Fleet limits a person changed while it ran: Drones at once, the
//! memory share, the disk floor.
//!
//! **One row or none, and a `NULL` is the shipped value.** A column nobody set
//! stays `NULL` rather than being written as today's constant, so a later build
//! that ships a different number reaches every field a person never touched.
//!
//! **No range is checked here.** Which values are allowed is the wire's —
//! `ipc::SaveLimits` cannot hold one out of range — and Fleet ignores a stored
//! value outside it rather than trusting a file somebody edited by hand.

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

/// Version 55 — the limits a person saved. `CHECK (id = 1)` is what makes it
/// one row: a second insert is a conflict, never a second opinion.
pub(crate) const V55: &str = r#"
CREATE TABLE fleet_limits (
    id                   INTEGER PRIMARY KEY CHECK (id = 1),
    concurrency          INTEGER,
    memory_spare_percent INTEGER,
    disk_floor_gib       INTEGER
) STRICT;
"#;

/// What a person saved, field by field. `None` is nobody having saved one.
///
/// **`Default` is honest here**, unlike Fleet's dials: it is the empty record,
/// which is exactly what a store nobody has saved into holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SavedLimits {
    pub concurrency: Option<u32>,
    pub memory_spare_percent: Option<u32>,
    pub disk_floor_gib: Option<u32>,
}

impl Store {
    /// The saved limits. **No row reads as nothing saved**, not as a fault.
    pub fn saved_limits(&self) -> Result<SavedLimits, DatabaseFault> {
        let read = self.conn.query_row(
            "SELECT concurrency, memory_spare_percent, disk_floor_gib
             FROM fleet_limits WHERE id = 1",
            [],
            |row| {
                Ok(SavedLimits {
                    concurrency: row.get(0)?,
                    memory_spare_percent: row.get(1)?,
                    disk_floor_gib: row.get(2)?,
                })
            },
        );
        match read {
            Ok(saved) => Ok(saved),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(SavedLimits::default()),
            Err(other) => Err(fault("reading the saved fleet limits")(other)),
        }
    }

    /// Replace the saved limits **whole**. Merging an omitted field with what
    /// was there is the caller's, which holds the request that omitted it.
    pub fn save_limits(&mut self, limits: &SavedLimits) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO fleet_limits (id, concurrency, memory_spare_percent, disk_floor_gib)
                 VALUES (1, ?1, ?2, ?3)
                 ON CONFLICT (id) DO UPDATE SET
                     concurrency = excluded.concurrency,
                     memory_spare_percent = excluded.memory_spare_percent,
                     disk_floor_gib = excluded.disk_floor_gib",
                (
                    limits.concurrency,
                    limits.memory_spare_percent,
                    limits.disk_floor_gib,
                ),
            )
            .map_err(fault("saving the fleet limits"))
            .map_err(WriteError::Database)?;
        Ok(())
    }
}
