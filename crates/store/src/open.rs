//! Opening the store, and the checks that decide whether it may be opened.
//!
//! # An empty database and a damaged one are different events
//!
//! That distinction is the whole of this file. A path with nothing at it is
//! created, migrated and opened — that is a first run. A path with something at
//! it that is not what this build can read is **refused**, every time, with the
//! reason named. Nothing here has a fallback that produces an empty store,
//! because a fallback that produces an empty store is how somebody loses a
//! week's Jobs and is shown a clean Board.
//!
//! # What "corrupt" means here, in the order it is checked
//!
//! | Check | Refused because |
//! |---|---|
//! | `PRAGMA integrity_check` | The pages are damaged. What still parses cannot be trusted either |
//! | WAL was refused | The mode is part of the durability the rest of the design assumes |
//! | Tables, but no `armada_meta` | Not an Armada store. Migrating into it would write Jobs into somebody else's file |
//! | A schema version above this build's | A newer Armada wrote it, and older assumptions would misread it |
//! | `armada_meta` with no readable version | The marker is there and says nothing. Guessing is the failure the version exists to prevent |
//! | `PRAGMA foreign_key_check` | An event points at a Job that is gone. Part of the authority has already been lost |
//!
//! A malformed *row* is deliberately not on that list — one Job that will not
//! read back is not a reason to refuse every other Job on the Board. Those
//! surface at load, as a [`RowError`](crate::RowError) the caller cannot drop.

use std::fmt;
use std::path::Path;

use rusqlite::Connection;

use crate::error::{fault, OpenError};
use crate::schema::{KNOWN_SCHEMA_VERSION, MIGRATIONS, SCHEMA_VERSION_KEY};

/// The database, open and checked.
///
/// # There is no in-memory constructor
///
/// A `Store::in_memory()` would be convenient and would quietly exempt every
/// test that used it from WAL, from the file layout, and from the reopen that
/// is the only way to prove a Job survived. Tests here open a real file in a
/// temporary directory for exactly that reason.
pub struct Store {
    pub(crate) conn: Connection,
    pub(crate) path: String,
}

impl Store {
    /// Open the store at `path`, creating and migrating it if there is nothing
    /// there, and refusing it if there is something there this build cannot
    /// read.
    pub fn open(path: &Path) -> Result<Store, OpenError> {
        let shown = path.display().to_string();
        let conn = Connection::open(path)
            .map_err(fault("opening the database"))
            .map_err(OpenError::Database)?;

        integrity(&conn, &shown)?;
        wal(&conn, &shown)?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(fault("turning foreign keys on"))
            .map_err(OpenError::Database)?;

        let mut store = Store { conn, path: shown };
        store.migrate()?;
        store.no_dangling_references()?;
        Ok(store)
    }

    /// Where this store is. Carried so that a refusal names the file rather
    /// than "the database".
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Bring the file up to [`KNOWN_SCHEMA_VERSION`], or say why it cannot be.
    fn migrate(&mut self) -> Result<(), OpenError> {
        let from = self.schema_version()?;
        if from > KNOWN_SCHEMA_VERSION {
            return Err(OpenError::SchemaVersionFromTheFuture {
                path: self.path.clone(),
                found: from,
                known: KNOWN_SCHEMA_VERSION,
            });
        }
        for (index, script) in MIGRATIONS.iter().enumerate().skip(from as usize) {
            let applied = index as u32 + 1;
            let tx = self
                .conn
                .transaction()
                .map_err(fault("starting a migration"))
                .map_err(OpenError::Database)?;
            tx.execute_batch(script)
                .map_err(fault("applying a migration"))
                .map_err(OpenError::Database)?;
            tx.execute(
                "INSERT INTO armada_meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                (SCHEMA_VERSION_KEY, applied.to_string()),
            )
            .map_err(fault("recording the schema version"))
            .map_err(OpenError::Database)?;
            tx.commit()
                .map_err(fault("committing a migration"))
                .map_err(OpenError::Database)?;
        }
        Ok(())
    }

    /// How many migrations this file has had applied.
    ///
    /// Zero means nothing is there yet — and *only* an empty file may say zero.
    /// A file with tables and no `armada_meta` is refused here rather than
    /// treated as fresh, which is the check that keeps "empty" and "not ours"
    /// apart.
    fn schema_version(&self) -> Result<u32, OpenError> {
        let has_meta = self.table_exists("armada_meta")?;
        if !has_meta {
            let tables = self.user_tables()?;
            if tables > 0 {
                return Err(OpenError::NotAnArmadaStore {
                    path: self.path.clone(),
                    tables,
                });
            }
            return Ok(0);
        }
        let recorded: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM armada_meta WHERE key = ?1",
                (SCHEMA_VERSION_KEY,),
                |row| row.get(0),
            )
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(fault("reading the schema version"))
            .map_err(OpenError::Database)?;
        match recorded {
            None => Err(OpenError::SchemaVersionUnreadable {
                path: self.path.clone(),
                found: None,
            }),
            Some(value) => value
                .parse::<u32>()
                .map_err(|_| OpenError::SchemaVersionUnreadable {
                    path: self.path.clone(),
                    found: Some(value),
                }),
        }
    }

    fn table_exists(&self, name: &str) -> Result<bool, OpenError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                (name,),
                |row| row.get(0),
            )
            .map_err(fault("looking for a table"))
            .map_err(OpenError::Database)?;
        Ok(count > 0)
    }

    fn user_tables(&self) -> Result<usize, OpenError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(fault("counting tables"))
            .map_err(OpenError::Database)?;
        Ok(count as usize)
    }

    /// No row points at a record that is gone.
    ///
    /// Foreign keys are on, so this cannot happen through this crate. It is
    /// checked anyway because the file is a file: something else edited it,
    /// a restore put back half of it, or a page went. An event with no Job is
    /// the log having already lost part of itself.
    fn no_dangling_references(&self) -> Result<(), OpenError> {
        let mut statement = self
            .conn
            .prepare("PRAGMA foreign_key_check")
            .map_err(fault("preparing the reference check"))
            .map_err(OpenError::Database)?;
        let rows = statement
            .query_map([], |_| Ok(()))
            .map_err(fault("running the reference check"))
            .map_err(OpenError::Database)?
            .count();
        if rows > 0 {
            return Err(OpenError::DanglingReferences {
                path: self.path.clone(),
                rows,
            });
        }
        Ok(())
    }
}

/// The path, and nothing else. A `Connection` is not `Debug`, and a store's
/// contents are not something a format string should reach for.
impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store").field("path", &self.path).finish()
    }
}

/// `PRAGMA integrity_check` answers `ok` on one row, or names findings.
fn integrity(conn: &Connection, path: &str) -> Result<(), OpenError> {
    let finding: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(fault("checking integrity"))
        .map_err(OpenError::Database)?;
    if finding != "ok" {
        return Err(OpenError::IntegrityCheckFailed {
            path: path.to_string(),
            finding,
        });
    }
    Ok(())
}

/// WAL, and a refusal if the filesystem will not give it.
fn wal(conn: &Connection, path: &str) -> Result<(), OpenError> {
    let actual: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(fault("setting WAL"))
        .map_err(OpenError::Database)?;
    if !actual.eq_ignore_ascii_case("wal") {
        return Err(OpenError::JournalModeRefused {
            path: path.to_string(),
            actual,
        });
    }
    // The pairing WAL is chosen for: a commit is durable through a process
    // crash, and a full fsync is paid at checkpoint rather than per commit.
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(fault("setting synchronous"))
        .map_err(OpenError::Database)?;
    Ok(())
}
