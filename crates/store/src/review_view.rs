//! The View chains a kept review's areas and findings carry. #904.
//!
//! Its own table beside `crate::review_record`'s, one row per step, keyed by what owns it.

use core_model::ViewStep;
use rusqlite::Connection;

use crate::error::{fault, DatabaseFault, LoadJobError};

/// Version 61 — the steps of each View a review carries.
///
/// **Nothing to backfill.** A review kept before this carried no View, and reads as one with none.
pub(crate) const V61: &str = r#"
CREATE TABLE job_step_review_view (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL,
    owner       TEXT NOT NULL CHECK (owner IN ('area', 'finding')),
    owned_by    INTEGER NOT NULL,
    ordinal     INTEGER NOT NULL,
    file        TEXT NOT NULL,
    hunk        TEXT NOT NULL,
    summary     TEXT NOT NULL,
    tie_to_next TEXT,
    PRIMARY KEY (job_id, step_id, attempt, owner, owned_by, ordinal)
) STRICT;
"#;

pub(crate) const TABLE: &str = "job_step_review_view";

/// Which part of a review a View belongs to.
#[derive(Clone, Copy)]
pub(crate) enum Owner {
    Area,
    Finding,
}

impl Owner {
    fn as_sql(self) -> &'static str {
        match self {
            Owner::Area => "area",
            Owner::Finding => "finding",
        }
    }
}

/// Write one area's or finding's View, the `owned_by`th of its kind.
pub(crate) fn write_view(
    conn: &Connection,
    (job, step, attempt): (&str, &str, i64),
    owner: Owner,
    owned_by: usize,
    view: &[ViewStep],
) -> Result<(), DatabaseFault> {
    for (n, one) in view.iter().enumerate() {
        conn.execute(
            "INSERT INTO job_step_review_view
                 (job_id, step_id, attempt, owner, owned_by, ordinal, file, hunk, summary, tie_to_next)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                job,
                step,
                attempt,
                owner.as_sql(),
                owned_by as i64,
                n as i64,
                one.file,
                one.hunk,
                one.summary,
                one.tie_to_next
            ],
        )
        .map_err(fault("writing a review's View"))?;
    }
    Ok(())
}

/// Every View of one kind in a kept review, as `(owned_by, steps)` in order.
pub(crate) fn views(
    conn: &Connection,
    key: (&str, &str, i64),
    owner: Owner,
) -> Result<Vec<(i64, Vec<ViewStep>)>, LoadJobError> {
    let unreadable = |why| LoadJobError::Database(fault("reading a review's View")(why));
    let mut statement = conn
        .prepare(
            "SELECT owned_by, file, hunk, summary, tie_to_next FROM job_step_review_view
             WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 AND owner = ?4
             ORDER BY owned_by, ordinal",
        )
        .map_err(unreadable)?;
    let rows = statement
        .query_map(
            rusqlite::params![key.0, key.1, key.2, owner.as_sql()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    ViewStep {
                        file: row.get(1)?,
                        hunk: row.get(2)?,
                        summary: row.get(3)?,
                        tie_to_next: row.get(4)?,
                    },
                ))
            },
        )
        .map_err(unreadable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unreadable)?;
    let mut grouped: Vec<(i64, Vec<ViewStep>)> = Vec::new();
    for (owned_by, step) in rows {
        match grouped.last_mut() {
            Some((last, steps)) if *last == owned_by => steps.push(step),
            _ => grouped.push((owned_by, vec![step])),
        }
    }
    Ok(grouped)
}

/// The steps `views` read for the `owned_by`th, or none.
pub(crate) fn of(grouped: &[(i64, Vec<ViewStep>)], owned_by: i64) -> Vec<ViewStep> {
    grouped
        .iter()
        .find(|(owner, _)| *owner == owned_by)
        .map(|(_, steps)| steps.clone())
        .unwrap_or_default()
}
