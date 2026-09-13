//! A person's preferences, kept the way limits are: one row per preference,
//! and an absent row is the shipped default.
//!
//! **Keyed by name, not by column.** `crate::limits`'s table is one column per
//! limit, which is right there because the three are fixed and reading all of
//! them is the only read this crate ever does. A preference is added one at a
//! time — the first is `where_things_are_open` and it will not be the last —
//! and a column-per-preference table would make every new one a migration.
//! `CHECK (name IN (...))` is the closed set the wire's own refusal answers
//! to: [`Store::save_preference`] refuses an unrecognised name before it ever
//! reaches SQL, and the constraint is what keeps a second writer — a hand-edited
//! file, a Store built without this check — from putting one there anyway.

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

/// Version 62 — a person's preferences, one row per name. `CHECK (value IN (0,
/// 1))` is what makes the one preference this build has a boolean; a
/// preference that is not one will need its own column or its own table, not a
/// looser check here.
pub(crate) const V62: &str = r#"
CREATE TABLE preferences (
    name  TEXT PRIMARY KEY CHECK (name IN ('where_things_are_open')),
    value INTEGER NOT NULL CHECK (value IN (0, 1))
) STRICT;
"#;

/// The one preference name this build reads and writes. Kept in the store, not
/// in `ipc`, because it is the store's `CHECK` that is authoritative and a
/// second copy of the set at the wire would be the thing that drifts from it.
const WHERE_THINGS_ARE_OPEN: &str = "where_things_are_open";

/// Every preference Fleet knows, and what a person saved for each. **A row
/// nobody wrote reads as the shipped default**, not as unset — there is
/// nothing else it could mean, since a preference with no row has never been
/// touched.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preferences {
    pub where_things_are_open: bool,
}

impl Store {
    /// Every preference, folded over the rows a person actually saved.
    ///
    /// **A name this build does not read is skipped, not refused.** It can
    /// only arrive from a newer Armada's row in an older one's database, the
    /// same reading an unknown field on the wire gets — ignored rather than a
    /// reason to fail the whole read.
    pub fn preferences(&self) -> Result<Preferences, DatabaseFault> {
        let mut statement = self
            .conn
            .prepare("SELECT name, value FROM preferences")
            .map_err(fault("reading the saved preferences"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(fault("reading the saved preferences"))?;
        let mut preferences = Preferences::default();
        for row in rows {
            let (name, value) = row.map_err(fault("reading the saved preferences"))?;
            if name == WHERE_THINGS_ARE_OPEN {
                preferences.where_things_are_open = value != 0;
            }
        }
        Ok(preferences)
    }

    /// Save one preference by name, and answer with every preference now in
    /// force. **A name outside the closed set is [`WriteError::UnknownPreference`]**
    /// — checked here, before a statement is built, so the message names
    /// exactly what was sent rather than reporting whatever SQLite's own
    /// `CHECK` violation says.
    pub fn save_preference(&mut self, name: &str, value: bool) -> Result<Preferences, WriteError> {
        if name != WHERE_THINGS_ARE_OPEN {
            return Err(WriteError::UnknownPreference {
                name: name.to_string(),
            });
        }
        self.conn
            .execute(
                "INSERT INTO preferences (name, value) VALUES (?1, ?2)
                 ON CONFLICT (name) DO UPDATE SET value = excluded.value",
                (name, i64::from(value)),
            )
            .map_err(fault("saving a preference"))
            .map_err(WriteError::Database)?;
        self.preferences().map_err(WriteError::Database)
    }
}
