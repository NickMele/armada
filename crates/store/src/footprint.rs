//! What a Job's worktree held when the Job stopped.
//!
//! # A record, not a reading
//!
//! Everywhere else a footprint appears it is read live: `fleet`'s watcher opens
//! the worktree while a Drone works, and `get_diff` opens it again for a person
//! who asked. Both need the directory to still be there, and `armada clean`
//! gives worktrees back. **This is the one footprint that survives the
//! worktree**, because it was taken at the moment the Job stopped and written
//! down rather than re-derived afterwards.
//!
//! That is the whole reason it is not recomputed on read. A reading taken now
//! is a guess about a directory that may not exist; a row written at the
//! terminal transition is a record of one that did.
//!
//! # No drift mark on the row, and none derived here
//!
//! `outside_plan` is not a column, and [`crate::schema`]'s `V15` carries why.
//! What the record is measured against is [`crate::plan`]; the comparison is
//! made where the two are served and is stored in neither, for
//! [`crate::attempt`]'s reason.
//!
//! # The counts are here and on no live seam
//!
//! `added` and `deleted` arrived in [`V25`], because counting a file costs the
//! xdiff that renders its patch — 25ms over a hundred files, 90ms over four
//! hundred, against under a microsecond for the path list. That is affordable
//! once, on the transition that ends a Job, and it is not affordable on a
//! reading taken every two seconds inside a 250ms turn. So the record carries
//! them and `job.files_changed` does not.

use adapter_traits::{Change, ChangedFile, Counted, CountedFile, LineCount};
use core_model::{JobId, Timestamp};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, string};

/// Version 25 — what each file in a footprint gained and lost.
///
/// Beside the table it changes, like [`V17`](crate::report::V17) and the
/// migrations after it: `schema.rs` is at the 900 lines the gate refuses at.
///
/// **Nullable, and null is "not counted" rather than zero.** A binary file has
/// no patch to count and a rename that edited nothing is a real `0` — a column
/// that could not tell those apart would turn an unmeasured file into a file
/// that changed nothing.
///
/// **The pair may not half-arrive**, which is why the table is rebuilt rather
/// than altered: `SQLite` cannot add a `CHECK` to a table that exists, and the
/// counts come from one walk of one patch, so a row holding one number and not
/// the other is a state no reading can produce and none should be able to
/// write. **Nothing to backfill**: a footprint written before this has no
/// counts, which is what null says.
pub(crate) const V25: &str = r#"
CREATE TABLE job_footprint_files_counted (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal INTEGER NOT NULL,
    path    TEXT NOT NULL,
    change  TEXT NOT NULL,
    added   INTEGER,
    deleted INTEGER,
    PRIMARY KEY (job_id, ordinal),
    CHECK ((added IS NULL) = (deleted IS NULL))
) STRICT;

INSERT INTO job_footprint_files_counted (job_id, ordinal, path, change)
SELECT job_id, ordinal, path, change FROM job_footprint_files;

DROP TABLE job_footprint_files;

ALTER TABLE job_footprint_files_counted RENAME TO job_footprint_files;
"#;

/// One Job's footprint as it was written down, and when.
///
/// **Absent and empty are different sentences.** No [`Footprinted`] at all is a
/// Job nothing recorded — still running, or finished before the record existed.
/// One with no files is a worktree that was opened and held no change, which is
/// what a `diff_nonempty` check refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footprinted {
    /// The reading, in the order it was found. Never re-sorted.
    ///
    /// A file with no [`LineCount`] is one nothing could count, which is not a
    /// file that changed no lines.
    pub files: Vec<CountedFile>,
    /// When the reading was taken, which is when the Job reached its terminal
    /// status and not when anybody asked for it.
    pub recorded_at: Timestamp,
}

impl Store {
    /// Write down what the worktree held, replacing whatever a previous write
    /// left.
    ///
    /// **Replacing, because a Job reaches a terminal status once and a second
    /// write means the first was wrong** — a Fleet that recorded, crashed
    /// before the transition landed and recorded again on the way back through
    /// must end with one reading rather than two interleaved.
    ///
    /// A transaction, because the header and its rows are one record: a header
    /// with nobody's rows under it says the Job changed nothing, which is a
    /// claim no torn write may make.
    pub fn record_footprint(
        &mut self,
        job_id: &JobId,
        changed: &Counted,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the footprint record"))
            .map_err(WriteError::Database)?;

        tx.execute(
            "DELETE FROM job_footprint_files WHERE job_id = ?1",
            rusqlite::params![job_id.as_str()],
        )
        .map_err(fault("clearing an earlier footprint"))
        .map_err(WriteError::Database)?;

        tx.execute(
            "INSERT INTO job_footprint (job_id, recorded_at) VALUES (?1, ?2)
             ON CONFLICT (job_id) DO UPDATE SET recorded_at = excluded.recorded_at",
            rusqlite::params![job_id.as_str(), at.as_str()],
        )
        .map_err(fault("writing a job's footprint"))
        .map_err(WriteError::Database)?;

        for (ordinal, file) in changed.files().iter().enumerate() {
            tx.execute(
                "INSERT INTO job_footprint_files (job_id, ordinal, path, change, added, deleted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    job_id.as_str(),
                    ordinal as i64,
                    file.path(),
                    file.change().as_wire(),
                    file.lines().map(|lines| i64::from(lines.added())),
                    file.lines().map(|lines| i64::from(lines.deleted())),
                ],
            )
            .map_err(fault("writing a changed file"))
            .map_err(WriteError::Database)?;
        }

        tx.commit()
            .map_err(fault("committing the footprint record"))
            .map_err(WriteError::Database)
    }

    /// What was written down for this Job, or `None` where nothing was.
    ///
    /// **The header decides**, and the files are read only where there is one.
    /// Asking the file rows alone could not tell a Job that was never recorded
    /// from one recorded as having changed nothing.
    ///
    /// A `change` value this build does not know is
    /// [`RowError::UnknownEnumValue`] and refuses the whole read, rather than
    /// dropping the file: a footprint short one row reads as work that was
    /// never done, which is the failure mode this record exists to end.
    pub fn footprint(&self, job_id: &JobId) -> Result<Option<Footprinted>, LoadJobError> {
        let recorded: Option<String> = self
            .conn
            .query_row(
                "SELECT recorded_at FROM job_footprint WHERE job_id = ?1",
                rusqlite::params![job_id.as_str()],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(RowError::Database(fault("reading a job's footprint")(
                    other,
                ))),
            })
            .map_err(LoadJobError::Unreadable)?;
        let Some(recorded_at) = recorded else {
            return Ok(None);
        };
        let files = self
            .collect(
                "SELECT path, change, added, deleted FROM job_footprint_files
                 WHERE job_id = ?1 ORDER BY ordinal",
                job_id,
                "reading a job's changed files",
                |row| {
                    let change = string(row, "change")?;
                    let file = ChangedFile::new(
                        string(row, "path")?,
                        Change::from_wire(&change).ok_or(RowError::UnknownEnumValue {
                            table: "job_footprint_files",
                            column: "change",
                            value: change,
                        })?,
                    );
                    Ok(CountedFile::new(file, counted(row)?))
                },
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(Some(Footprinted {
            files,
            recorded_at: Timestamp::from_rfc3339(recorded_at),
        }))
    }
}

/// What one stored row says the file gained and lost, or nothing where it was
/// never counted.
///
/// **A half-filled pair is a refusal, not an absence.** [`V25`]'s `CHECK` means
/// no write can make one, so a row holding one number and not the other was
/// written by something that is not this crate — and reading it as "not
/// counted" would let a file that lost four hundred lines report nothing at
/// all.
fn counted(row: &rusqlite::Row<'_>) -> Result<Option<LineCount>, RowError> {
    let added: Option<i64> = row
        .get("added")
        .map_err(column("job_footprint_files", "added"))?;
    let deleted: Option<i64> = row
        .get("deleted")
        .map_err(column("job_footprint_files", "deleted"))?;
    match (added, deleted) {
        (Some(added), Some(deleted)) => Ok(Some(LineCount::of(added as u32, deleted as u32))),
        (None, None) => Ok(None),
        _ => Err(RowError::MalformedColumn {
            table: "job_footprint_files",
            column: "added",
            detail: "one of the line counts is null and the other is not".to_string(),
        }),
    }
}
