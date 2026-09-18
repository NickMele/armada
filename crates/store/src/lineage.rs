//! A redispatch read from both ends, off the one column that records it.
//!
//! `jobs.redispatched_from` is the whole record, written once on the
//! replacement. Read as a predicate it answers *which Job replaced this one*;
//! read as a column and followed it answers *which Job this one replaced* — so
//! there is no second column to disagree with the first, and forgetting a Job
//! at either end takes the link with it. `docs/concepts/job.md`, *A redispatch
//! is read from both ends*.
//!
//! **Both reads answer the number and the title, never a composed handle.**
//! `core_model` owns that one string and Fleet composes it at the seam; a
//! second composition here is a second answer to what a Job is called.

use core_model::{JobId, JobNumber, Title, Ulid};

use crate::error::{fault, LoadJobError, RowError};
use crate::open::Store;
use crate::row::{column, malformed, string};

/// Version 82 — find a Job by the Job it replaced, on every open of a Job.
///
/// Beside the read that needs it, like [`V36`](crate::numbering::V36): the
/// order of the migration list is `migrations.rs`'s alone.
///
/// Not unique. A second redispatch of one original is legal today, and an
/// index refusing it would fail the write rather than the act.
pub(crate) const V82: &str = r#"
CREATE INDEX jobs_by_redispatched_from ON jobs (redispatched_from);
"#;

/// The Job that replaced another: enough to name it and to open it.
///
/// **The number and the title, never a composed handle.** `core_model` owns
/// that one string, and a second composition here is a second answer to what a
/// Job is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplacedBy {
    pub job_id: JobId,
    pub number: JobNumber,
    pub title: Title,
}

/// The Job a redispatch replaced: the same three fields, read the other way.
///
/// **A shape of its own rather than [`ReplacedBy`] reused**, though the fields
/// match today. The two answer opposite questions and a single type would let a
/// caller hand a predecessor where a successor is meant with nothing to say so
/// — the direction is the whole content of the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replaces {
    pub job_id: JobId,
    pub number: JobNumber,
    pub title: Title,
}

impl Store {
    /// The Job that replaced this one, where a redispatch minted one.
    ///
    /// **The direct successor, and no walk.** A replacement that was itself
    /// redispatched answers this question for itself, so a chain is read a hop
    /// at a time and there is no traversal here to go round.
    ///
    /// `None` is every Job nothing replaced — and a Job whose replacement has
    /// been forgotten, which is the reading the record can still support.
    pub fn replaced_by(&self, job_id: &JobId) -> Result<Option<ReplacedBy>, LoadJobError> {
        // Newest wins where two Jobs name one predecessor. Fleet refuses no
        // such thing — Bridge's `already_redispatching` guards one press being
        // sent twice, not one Job being redispatched twice — and the later
        // press is the one that meant it.
        let found = self
            .conn
            .query_row(
                r#"SELECT job_id, number, title
                   FROM jobs
                   WHERE redispatched_from = ?1
                   ORDER BY created_at DESC, job_id DESC
                   LIMIT 1"#,
                (job_id.as_str(),),
                |row| Ok(replacement(row)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadJobError::Database(fault(
                    "reading the job that replaced one",
                )(other))),
            })?;
        found.transpose().map_err(LoadJobError::Unreadable)
    }

    /// The Job this one replaced, where a redispatch minted this one.
    ///
    /// **The predecessor's own row, joined through this Job's
    /// `redispatched_from`.** The column holds an id and nothing a person can
    /// read, so the name is fetched on open the way the other direction's is —
    /// nothing here writes a second column.
    ///
    /// **One hop, and no walk**, for [`replaced_by`](Store::replaced_by)'s
    /// reason: a predecessor that itself replaced something answers that for
    /// itself, so a chain cannot loop inside this read.
    ///
    /// `None` is every Job no redispatch minted — and a Job whose predecessor
    /// has been forgotten, which leaves the id on the row pointing at nothing
    /// and is the same reading `replaced_by` already supports.
    pub fn replaces(&self, job_id: &JobId) -> Result<Option<Replaces>, LoadJobError> {
        // Two primary-key reads as one join. `jobs_by_redispatched_from` (V82)
        // is the other direction's index and is not wanted here: both ends of
        // this join are `job_id`.
        let found = self
            .conn
            .query_row(
                r#"SELECT prior.job_id, prior.number, prior.title
                   FROM jobs AS job
                   JOIN jobs AS prior ON prior.job_id = job.redispatched_from
                   WHERE job.job_id = ?1"#,
                (job_id.as_str(),),
                |row| Ok(predecessor(row)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadJobError::Database(fault(
                    "reading the job that one replaced",
                )(other))),
            })?;
        found.transpose().map_err(LoadJobError::Unreadable)
    }
}

fn replacement(row: &rusqlite::Row<'_>) -> Result<ReplacedBy, RowError> {
    let (job_id, number, title) = named(row)?;
    Ok(ReplacedBy {
        job_id,
        number,
        title,
    })
}

fn predecessor(row: &rusqlite::Row<'_>) -> Result<Replaces, RowError> {
    let (job_id, number, title) = named(row)?;
    Ok(Replaces {
        job_id,
        number,
        title,
    })
}

/// What either direction reads off a `jobs` row: the id, and the two fields a
/// handle is made of. **One reader, so the two ends cannot start disagreeing
/// about what a malformed row means.**
fn named(row: &rusqlite::Row<'_>) -> Result<(JobId, JobNumber, Title), RowError> {
    // `number` is `NOT NULL` and allocated as `max + 1` from zero, so a value
    // outside `u32` is a row nothing in this crate could have written.
    let number: i64 = row.get("number").map_err(column("jobs", "number"))?;
    Ok((
        JobId::carried(Ulid::carried(string(row, "job_id")?)),
        JobNumber::carried(number.max(0) as u32),
        // Blank is refused rather than named "Untitled", `read.rs`'s rule: the
        // triggers in V2 do not admit one, so a blank arrived from outside.
        Title::new(&string(row, "title")?)
            .map_err(|blank| malformed("title")(blank.to_string()))?,
    ))
}
