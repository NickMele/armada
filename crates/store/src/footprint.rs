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
//! # No drift mark
//!
//! `outside_plan` is not here, and [`crate::schema`]'s `V15` carries why: a
//! plan belongs to the step that declared it and this is the Job's whole work
//! since the branch was cut. The live reading keeps the mark, where the step
//! that declared is the step being watched.

use adapter_traits::{Change, Changed, ChangedFile};
use core_model::{JobId, Timestamp};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::string;

/// One Job's footprint as it was written down, and when.
///
/// **Absent and empty are different sentences.** No [`Footprinted`] at all is a
/// Job nothing recorded — still running, or finished before the record existed.
/// One with no files is a worktree that was opened and held no change, which is
/// what a `diff_nonempty` check refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footprinted {
    /// The reading, in the order it was found. Never re-sorted.
    pub files: Vec<ChangedFile>,
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
        changed: &Changed,
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
                "INSERT INTO job_footprint_files (job_id, ordinal, path, change)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    job_id.as_str(),
                    ordinal as i64,
                    file.path(),
                    file.change().as_wire(),
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
                "SELECT path, change FROM job_footprint_files
                 WHERE job_id = ?1 ORDER BY ordinal",
                job_id,
                "reading a job's changed files",
                |row| {
                    let change = string(row, "change")?;
                    Ok(ChangedFile::new(
                        string(row, "path")?,
                        Change::from_wire(&change).ok_or(RowError::UnknownEnumValue {
                            table: "job_footprint_files",
                            column: "change",
                            value: change,
                        })?,
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(Some(Footprinted {
            files,
            recorded_at: Timestamp::from_rfc3339(recorded_at),
        }))
    }
}
