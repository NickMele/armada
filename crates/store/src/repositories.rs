//! The repositories a Fleet serves, remembered across a restart, and the
//! main-checkout port claim keyed by which repository's checkout it is.

use core_model::Timestamp;

use crate::error::{fault, LoadAllError, WriteError};
use crate::open::Store;

/// Version 57 — the repositories a Fleet serves, and one main-checkout span
/// per repository rather than one for the whole Fleet.
///
/// **`port_claims` is rebuilt, for [`V50`](crate::ports::V50)'s reason**: a
/// `CHECK` cannot change in place. `main_checkout` becomes the repository's
/// root. A main-checkout row from before this names no root, so it is released
/// rather than guessed at; the next need claims a fresh span, probed as always.
pub(crate) const V57: &str = r#"
CREATE TABLE fleet_repositories (
    root     TEXT PRIMARY KEY NOT NULL,
    added_at TEXT NOT NULL
) STRICT;

ALTER TABLE port_claims RENAME TO port_claims_before_repositories;

CREATE TABLE port_claims (
    job_id         TEXT REFERENCES jobs(job_id),
    main_checkout  TEXT,
    fleet_listener INTEGER,
    base           INTEGER NOT NULL,
    width          INTEGER NOT NULL,
    claimed_at     TEXT NOT NULL,
    CHECK (base > 0 AND width > 0),
    CHECK ((job_id IS NOT NULL) + (main_checkout IS NOT NULL)
           + (fleet_listener IS NOT NULL) = 1),
    CHECK (main_checkout IS NULL OR main_checkout <> ''),
    CHECK (fleet_listener IS NULL OR fleet_listener = 1)
) STRICT;

INSERT INTO port_claims (job_id, fleet_listener, base, width, claimed_at)
SELECT job_id, fleet_listener, base, width, claimed_at
FROM port_claims_before_repositories
WHERE main_checkout IS NULL;

DROP TABLE port_claims_before_repositories;

CREATE UNIQUE INDEX port_claims_by_job ON port_claims (job_id) WHERE job_id IS NOT NULL;
CREATE UNIQUE INDEX port_claims_main_checkout ON port_claims (main_checkout) WHERE main_checkout IS NOT NULL;
CREATE UNIQUE INDEX port_claims_fleet_listener ON port_claims (fleet_listener) WHERE fleet_listener IS NOT NULL;
"#;

impl Store {
    /// Remember a repository Fleet serves. **Idempotent**: serving one again
    /// keeps when it was first added.
    pub fn remember_repository(&mut self, root: &str, at: &Timestamp) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO fleet_repositories (root, added_at) VALUES (?1, ?2) \
                 ON CONFLICT (root) DO NOTHING",
                (root, at.as_str()),
            )
            .map_err(fault("remembering a repository Fleet serves"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every repository remembered, in the order each was first added.
    pub fn remembered_repositories(&self) -> Result<Vec<String>, LoadAllError> {
        let reading = fault("reading the repositories Fleet serves");
        let mut asked = self
            .conn
            .prepare("SELECT root FROM fleet_repositories ORDER BY added_at, rowid")
            .map_err(reading)
            .map_err(LoadAllError::Database)?;
        let rows = asked
            .query_map([], |row| row.get::<_, String>("root"))
            .map_err(fault("reading the repositories Fleet serves"))
            .map_err(LoadAllError::Database)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(fault("reading a repository Fleet serves"))
            .map_err(LoadAllError::Database)
    }
}
