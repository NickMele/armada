//! How a Job meets a command its Drone was not granted, and the commands a
//! person allowed it.
//!
//! **No event for either**, for [`record_redirect_waiting`]'s reason: neither
//! moves a status or a step, so the column and the table are each the
//! authority for their own field and [`crate::read`] folds nothing from them.
//!
//! [`record_redirect_waiting`]: Store::record_redirect_waiting

use core_model::{Actor, AllowedCommand, JobId, Reach, Timestamp, WhenBlocked};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value};

/// Version 44 — how a Job meets a blocked command, and what a person allowed.
///
/// Beside the change it makes, like [`V43`](crate::shown_again::V43):
/// `schema.rs` is at the 900 lines the gate refuses at.
///
/// **The `DEFAULT` is the backfill only**: a Job written before this reads as
/// `refuse_and_hold`, which is what it already was. [`Store::insert_job`]
/// binds `ask_me` for a Job created after, the way it binds `title`, so this
/// `DEFAULT` is never the value a new Job gets. The table starts empty, which
/// is what a Job nobody allowed anything reads as.
///
/// `by` is quoted because it is an SQL keyword, and the key is the pair so a
/// second allow of one command is a conflict rather than a second row.
pub(crate) const V44: &str = r#"
ALTER TABLE jobs ADD COLUMN when_blocked TEXT NOT NULL DEFAULT 'refuse_and_hold';

CREATE TABLE job_allowed_commands (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    run        TEXT NOT NULL,
    reach      TEXT NOT NULL,
    allowed_at TEXT NOT NULL,
    "by"       TEXT NOT NULL,
    PRIMARY KEY (job_id, run)
) STRICT;
"#;

impl Store {
    /// How this Job meets a command its Drone was not granted.
    pub fn when_blocked(&self, job_id: &JobId) -> Result<WhenBlocked, LoadJobError> {
        let held: String = self
            .conn
            .query_row(
                "SELECT when_blocked FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => LoadJobError::NoSuchJob {
                    job_id: job_id.clone(),
                },
                other => LoadJobError::Unreadable(column("jobs", "when_blocked")(other)),
            })?;
        enum_value(WhenBlocked::from_wire, "jobs", "when_blocked", &held)
            .map_err(LoadJobError::Unreadable)
    }

    /// Change how this Job meets a blocked command. The next permission
    /// question reads the column, so nothing has to respawn for it to hold.
    pub fn set_when_blocked(
        &mut self,
        job_id: &JobId,
        setting: WhenBlocked,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET when_blocked = ?2 WHERE job_id = ?1",
                (job_id.as_str(), setting.as_wire()),
            )
            .map_err(fault("recording how a Job meets a blocked command"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// Every command a person allowed for this Job, oldest first.
    ///
    /// A Job with none answers an empty list, and so does an id naming no Job:
    /// [`remarks_taken_up`](Store::remarks_taken_up)'s answer, for its reason.
    /// Instants are injected and may tie, so insertion order breaks a tie.
    pub fn allowed_commands(&self, job_id: &JobId) -> Result<Vec<AllowedCommand>, LoadJobError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT run, reach, allowed_at, \"by\" FROM job_allowed_commands \
                 WHERE job_id = ?1 ORDER BY allowed_at, rowid",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map([job_id.as_str()], |row| Ok(read_allowed(row)))
            .map_err(unreadable)?;
        let mut allowed = Vec::new();
        for row in rows {
            allowed.push(row.map_err(unreadable)?.map_err(LoadJobError::Unreadable)?);
        }
        Ok(allowed)
    }

    /// Write down that a person allowed this command for this Job.
    ///
    /// **Idempotent on the command.** A second allow changes one thing, reach,
    /// and only upward: a command made permanent in the repository is not
    /// narrowed by a later allow for the Job alone. `allowed_at` and `by` stay
    /// the first allow's, so the list keeps the order things were allowed in.
    pub fn allow_command(
        &mut self,
        job_id: &JobId,
        allowed: &AllowedCommand,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_allowed_commands (job_id, run, reach, allowed_at, \"by\") \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT (job_id, run) DO UPDATE SET reach = excluded.reach \
                 WHERE excluded.reach = ?6",
                (
                    job_id.as_str(),
                    allowed.run.as_str(),
                    allowed.reach.as_wire(),
                    allowed.allowed_at.as_str(),
                    allowed.by.as_wire(),
                    Reach::Repository.as_wire(),
                ),
            )
            // The key conflict is the upsert's, and every column is bound, so
            // the one constraint left to fail is the foreign key: a Job that
            // does not exist, refused rather than left as an orphan row.
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("recording a command a person allowed")(other)),
            })?;
        Ok(())
    }

    /// Take back a person's allow of this command for this Job. `true` where
    /// a row went, `false` where there was none — an id naming no Job too, for
    /// [`allowed_commands`](Store::allowed_commands)' reason. The text must
    /// match the allow exactly, since it is the key.
    ///
    /// **The row only.** A command a person also made permanent is declared in
    /// `armada.yml` on the Job's branch, and nothing here reaches that file.
    pub fn remove_allowed_command(
        &mut self,
        job_id: &JobId,
        run: &str,
    ) -> Result<bool, WriteError> {
        let removed = self
            .conn
            .execute(
                "DELETE FROM job_allowed_commands WHERE job_id = ?1 AND run = ?2",
                (job_id.as_str(), run),
            )
            .map_err(fault("taking back a command a person allowed"))
            .map_err(WriteError::Database)?;
        Ok(removed > 0)
    }
}

/// One row, with its two enums refused by name rather than defaulted.
fn read_allowed(row: &rusqlite::Row<'_>) -> Result<AllowedCommand, RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(TABLE, name))
    };
    Ok(AllowedCommand {
        run: text("run")?,
        reach: enum_value(Reach::from_wire, TABLE, "reach", &text("reach")?)?,
        allowed_at: Timestamp::from_rfc3339(text("allowed_at")?),
        by: enum_value(Actor::from_wire, TABLE, "by", &text("by")?)?,
    })
}

fn unreadable(cause: rusqlite::Error) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(fault(
        "reading the commands a person allowed a Job",
    )(cause)))
}

/// Named once, because every error above points at the same table.
const TABLE: &str = "job_allowed_commands";
