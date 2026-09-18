//! A redispatch read backwards: which Job replaced this one.
//!
//! `jobs.redispatched_from` is the whole record, written once on the
//! replacement. Read as a predicate rather than a column, it answers the other
//! direction — so there is no second column to disagree with the first, and
//! forgetting a replacement takes the link with it. `docs/concepts/job.md`,
//! *A redispatch is read from both ends*.

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
}

fn replacement(row: &rusqlite::Row<'_>) -> Result<ReplacedBy, RowError> {
    // `number` is `NOT NULL` and allocated as `max + 1` from zero, so a value
    // outside `u32` is a row nothing in this crate could have written.
    let number: i64 = row.get("number").map_err(column("jobs", "number"))?;
    Ok(ReplacedBy {
        job_id: JobId::carried(Ulid::carried(string(row, "job_id")?)),
        number: JobNumber::carried(number.max(0) as u32),
        // Blank is refused rather than named "Untitled", `read.rs`'s rule: the
        // triggers in V2 do not admit one, so a blank arrived from outside.
        title: Title::new(&string(row, "title")?)
            .map_err(|blank| malformed("title")(blank.to_string()))?,
    })
}
