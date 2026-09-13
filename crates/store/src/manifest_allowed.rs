//! Commands a person always-allowed for a whole Manifest, kept by Fleet
//! itself rather than committed into `armada.yml` — `#836`, reversing `#643`'s
//! *"an always allow lands on the Job's branch"*: that commit was in
//! `changed_files` on every later step of the Job it was pressed on, and the
//! absolute boundary in `crates/verification/src/forbidden.rs` refused it.
//!
//! **Keyed by Manifest, never by Job.** [`crate::allowing`]'s table answers
//! for one Job; this one answers for every Job this Fleet ever spawns against
//! the same `armada.yml`, which is the whole point of "repository-wide."

use core_model::{Actor, AllowedCommand, ManifestId, Reach, Timestamp};

use crate::error::{fault, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value};

/// Version 56 — the commands a person always-allowed for a Manifest's
/// repository, kept here instead of a commit on some Job's branch.
///
/// **No foreign key.** A `jobs` row and a Manifest are not the same lifetime —
/// this outlives every Job that ever reads it — so there is nothing here for
/// [`crate::migrations::tables_pointing_at_a_job`] to find, and `forget_job`
/// has nothing to delete from it.
pub(crate) const V56: &str = r#"
CREATE TABLE manifest_allowed_commands (
    manifest_id TEXT NOT NULL,
    run         TEXT NOT NULL,
    allowed_at  TEXT NOT NULL,
    "by"        TEXT NOT NULL,
    PRIMARY KEY (manifest_id, run)
) STRICT;
"#;

impl Store {
    /// Every command a person always-allowed for this Manifest, oldest first.
    ///
    /// **Every row this Fleet ever wrote for the Manifest**, whether or not
    /// today's `armada.yml` still leaves it grantable — a person removing one
    /// needs to see it whether or not it is presently shadowed by a command
    /// the file now declares destructive. What a live permission question
    /// grants is narrower, and is `fleet`'s to filter.
    pub fn repository_allowed_commands(
        &self,
        manifest_id: &ManifestId,
    ) -> Result<Vec<AllowedCommand>, RowError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT run, allowed_at, \"by\" FROM manifest_allowed_commands \
                 WHERE manifest_id = ?1 ORDER BY allowed_at, rowid",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map([manifest_id.as_str()], |row| Ok(read_allowed(row)))
            .map_err(unreadable)?;
        let mut allowed = Vec::new();
        for row in rows {
            allowed.push(row.map_err(unreadable)??);
        }
        Ok(allowed)
    }

    /// Write down that a person always-allowed this rule for the Manifest's
    /// repository. **Idempotent**: a second allow of the same rule leaves the
    /// first's `allowed_at` and `by` as they were, the way
    /// [`crate::allowing::Store::allow_command`] does for a Job's own row.
    pub fn allow_repository_command(
        &mut self,
        manifest_id: &ManifestId,
        allowed: &AllowedCommand,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO manifest_allowed_commands (manifest_id, run, allowed_at, \"by\") \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (manifest_id, run) DO NOTHING",
                (
                    manifest_id.as_str(),
                    allowed.run.as_str(),
                    allowed.allowed_at.as_str(),
                    allowed.by.as_wire(),
                ),
            )
            .map_err(fault(
                "recording a command a person always-allowed for a repository",
            ))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Take back a person's always-allow of this rule for this Manifest.
    /// `true` where a row went, `false` where there was none. The text must
    /// match the allow exactly, since it is the key.
    pub fn remove_repository_allowed_command(
        &mut self,
        manifest_id: &ManifestId,
        run: &str,
    ) -> Result<bool, WriteError> {
        let removed = self
            .conn
            .execute(
                "DELETE FROM manifest_allowed_commands WHERE manifest_id = ?1 AND run = ?2",
                (manifest_id.as_str(), run),
            )
            .map_err(fault(
                "taking back a command a person always-allowed for a repository",
            ))
            .map_err(WriteError::Database)?;
        Ok(removed > 0)
    }
}

/// One row. `reach` is not a column here — every row is
/// [`Reach::Repository`], which is the whole reason this table exists apart
/// from [`crate::allowing`]'s.
fn read_allowed(row: &rusqlite::Row<'_>) -> Result<AllowedCommand, RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(TABLE, name))
    };
    Ok(AllowedCommand {
        run: text("run")?,
        reach: Reach::Repository,
        allowed_at: Timestamp::from_rfc3339(text("allowed_at")?),
        by: enum_value(Actor::from_wire, TABLE, "by", &text("by")?)?,
    })
}

fn unreadable(cause: rusqlite::Error) -> RowError {
    RowError::Database(fault(
        "reading the commands a person always-allowed for a repository",
    )(cause))
}

/// Named once, because every error above points at the same table.
const TABLE: &str = "manifest_allowed_commands";
