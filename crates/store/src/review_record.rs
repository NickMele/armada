//! What Armada's review of a step said, kept once Fleet accepted it. #903.
//!
//! **Rows, not a document**, `crate::work_plan`'s shape: each part in a table of its own,
//! written whole for one run of one step and replaced by that run's next submission.

use core_model::{
    Area, Bucket, ChangedTest, Confidence, Finding, JobId, Proves, ReviewRecord, StepId,
    TestChange, TestsInChange, Timestamp, Untested,
};
use rusqlite::{Connection, Row};

use crate::attempt::attempt_now;
use crate::error::{fault, DatabaseFault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::enum_value;

/// Version 60 — Armada's review of a step, in the parts it is made of.
///
/// **Nothing to backfill.** No step asked for review evidence before this, and a Job
/// with no rows reads as a Job with no review.
pub(crate) const V60: &str = r#"
CREATE TABLE job_step_reviews (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL CHECK (attempt >= 1),
    confidence  TEXT NOT NULL CHECK (confidence IN ('confident', 'not_confident')),
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt)
) STRICT;

CREATE TABLE job_step_review_reasons (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    reason  TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_areas (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    name    TEXT NOT NULL,
    what    TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_area_files (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    area    INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    path    TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, area, ordinal)
) STRICT;

CREATE TABLE job_step_review_proves (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    area    TEXT NOT NULL,
    what    TEXT NOT NULL,
    tests   INTEGER NOT NULL CHECK (tests >= 0),
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_changed (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL,
    ordinal     INTEGER NOT NULL,
    name        TEXT NOT NULL,
    change      TEXT NOT NULL CHECK (change IN ('removed', 'loosened')),
    replaced_by TEXT,
    why         TEXT,
    unexplained INTEGER NOT NULL CHECK (unexplained IN (0, 1)),
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_untested (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    code    TEXT NOT NULL,
    why     TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;

CREATE TABLE job_step_review_findings (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    bucket  TEXT NOT NULL CHECK (bucket IN ('needs_you', 'small_fix', 'for_context')),
    finding TEXT NOT NULL,
    why     TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;
"#;

/// Every table one review is written to, emptied together before a run's review is rewritten.
const TABLES: &[&str] = &[
    "job_step_reviews",
    "job_step_review_reasons",
    "job_step_review_areas",
    "job_step_review_area_files",
    "job_step_review_proves",
    "job_step_review_changed",
    "job_step_review_untested",
    "job_step_review_findings",
];

impl Store {
    /// Keep an accepted review, replacing what an earlier submission in the same run kept.
    pub fn record_confidence(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        record: &ReviewRecord,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a review record"))
            .map_err(WriteError::Database)?;
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;
        written(
            &tx,
            (
                job_id.as_str(),
                step_id.as_str(),
                i64::from(attempt.number()),
            ),
            record,
            at,
        )
        .map_err(WriteError::Database)?;
        tx.commit()
            .map_err(fault("committing a review record"))
            .map_err(WriteError::Database)
    }

    /// The review last kept for a Job, or `None` where no step's review was.
    pub fn confidence_record(&self, job_id: &JobId) -> Result<Option<ReviewRecord>, LoadJobError> {
        latest(&self.conn, job_id)
    }
}

type Key<'a> = (&'a str, &'a str, i64);

fn written(
    conn: &Connection,
    key: Key<'_>,
    record: &ReviewRecord,
    at: &Timestamp,
) -> Result<(), DatabaseFault> {
    for table in TABLES {
        conn.execute(
            &format!("DELETE FROM {table} WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3"),
            key,
        )
        .map_err(fault("clearing a review record"))?;
    }
    let (job, step, attempt) = key;
    conn.execute(
        "INSERT INTO job_step_reviews (job_id, step_id, attempt, confidence, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![job, step, attempt, record.confidence.as_wire(), at.as_str()],
    )
    .map_err(fault("writing a review"))?;
    for (n, reason) in record.reasons.iter().enumerate() {
        conn.execute(
            "INSERT INTO job_step_review_reasons (job_id, step_id, attempt, ordinal, reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![job, step, attempt, n as i64, reason],
        )
        .map_err(fault("writing a review's reasons"))?;
    }
    for (n, area) in record.areas.iter().enumerate() {
        conn.execute(
            "INSERT INTO job_step_review_areas (job_id, step_id, attempt, ordinal, name, what)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![job, step, attempt, n as i64, area.name(), area.what()],
        )
        .map_err(fault("writing a review's areas"))?;
        for (f, path) in area.files().iter().enumerate() {
            conn.execute(
                "INSERT INTO job_step_review_area_files
                     (job_id, step_id, attempt, area, ordinal, path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![job, step, attempt, n as i64, f as i64, path],
            )
            .map_err(fault("writing a review area's files"))?;
        }
    }
    for (n, proves) in record.tests.proves.iter().enumerate() {
        conn.execute(
            "INSERT INTO job_step_review_proves (job_id, step_id, attempt, ordinal, area, what, tests)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                job,
                step,
                attempt,
                n as i64,
                proves.area(),
                proves.what(),
                i64::from(proves.tests())
            ],
        )
        .map_err(fault("writing what a review's tests prove"))?;
    }
    for (n, test) in record.tests.changed.iter().enumerate() {
        let replaced_by = match &test.change {
            TestChange::Removed { replaced_by } => replaced_by.as_deref(),
            TestChange::Loosened => None,
        };
        let unexplained = record
            .unexplained_tests
            .iter()
            .any(|given| given.name == test.name);
        conn.execute(
            "INSERT INTO job_step_review_changed
                 (job_id, step_id, attempt, ordinal, name, change, replaced_by, why, unexplained)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                job,
                step,
                attempt,
                n as i64,
                test.name,
                test.change.as_wire(),
                replaced_by,
                test.why,
                i64::from(unexplained)
            ],
        )
        .map_err(fault("writing a review's changed tests"))?;
    }
    for (n, untested) in record.tests.untested.iter().enumerate() {
        conn.execute(
            "INSERT INTO job_step_review_untested (job_id, step_id, attempt, ordinal, code, why)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                job,
                step,
                attempt,
                n as i64,
                untested.code(),
                untested.why()
            ],
        )
        .map_err(fault("writing a review's untested code"))?;
    }
    for (n, finding) in record.findings.iter().enumerate() {
        conn.execute(
            "INSERT INTO job_step_review_findings
                 (job_id, step_id, attempt, ordinal, bucket, finding, why)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                job,
                step,
                attempt,
                n as i64,
                finding.bucket().as_wire(),
                finding.finding(),
                finding.why()
            ],
        )
        .map_err(fault("writing a review's findings"))?;
    }
    Ok(())
}

fn latest(conn: &Connection, job_id: &JobId) -> Result<Option<ReviewRecord>, LoadJobError> {
    let found = conn.query_row(
        "SELECT step_id, attempt, confidence FROM job_step_reviews
         WHERE job_id = ?1 ORDER BY recorded_at DESC, attempt DESC LIMIT 1",
        (job_id.as_str(),),
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    );
    let (step, attempt, confidence) = match found {
        Ok(found) => found,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(other) => return Err(LoadJobError::Database(fault("reading a review")(other))),
    };
    let confidence = enum_value(
        Confidence::from_wire,
        "job_step_reviews",
        "confidence",
        &confidence,
    )
    .map_err(LoadJobError::Unreadable)?;
    let key: Key<'_> = (job_id.as_str(), step.as_str(), attempt);

    let reasons = rows(
        conn,
        "SELECT reason FROM job_step_review_reasons
         WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 ORDER BY ordinal",
        key,
        |row| row.get::<_, String>(0),
    )?;
    let files = rows(
        conn,
        "SELECT area, path FROM job_step_review_area_files
         WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 ORDER BY area, ordinal",
        key,
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    )?;
    let areas = rows(
        conn,
        "SELECT ordinal, name, what FROM job_step_review_areas
         WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 ORDER BY ordinal",
        key,
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?
    .into_iter()
    .map(|(ordinal, name, what)| {
        let held: Vec<&str> = files
            .iter()
            .filter(|(area, _)| *area == ordinal)
            .map(|(_, path)| path.as_str())
            .collect();
        Area::of(&name, &what, &held)
    })
    .collect();
    let proves = rows(
        conn,
        "SELECT area, what, tests FROM job_step_review_proves
         WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 ORDER BY ordinal",
        key,
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?
    .into_iter()
    .map(|(area, what, tests)| {
        let tests = u32::try_from(tests).map_err(|_| {
            LoadJobError::Unreadable(RowError::MalformedColumn {
                table: "job_step_review_proves",
                column: "tests",
                detail: format!("{tests} is not a count"),
            })
        })?;
        Ok(Proves::of(&area, &what, tests))
    })
    .collect::<Result<Vec<_>, LoadJobError>>()?;
    let mut unexplained_tests = Vec::new();
    let mut changed = Vec::new();
    for (name, change, replaced_by, why, unexplained) in rows(
        conn,
        "SELECT name, change, replaced_by, why, unexplained FROM job_step_review_changed
         WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 ORDER BY ordinal",
        key,
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        },
    )? {
        let change = enum_value(
            |word| TestChange::from_wire(word, replaced_by.clone()),
            "job_step_review_changed",
            "change",
            &change,
        )
        .map_err(LoadJobError::Unreadable)?;
        let test = ChangedTest { name, change, why };
        if unexplained == 1 {
            unexplained_tests.push(test.clone());
        }
        changed.push(test);
    }
    let untested = rows(
        conn,
        "SELECT code, why FROM job_step_review_untested
         WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 ORDER BY ordinal",
        key,
        |row| {
            Ok(Untested::of(
                &row.get::<_, String>(0)?,
                &row.get::<_, String>(1)?,
            ))
        },
    )?;
    let findings = rows(
        conn,
        "SELECT bucket, finding, why FROM job_step_review_findings
         WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3 ORDER BY ordinal",
        key,
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?
    .into_iter()
    .map(|(bucket, finding, why)| {
        let bucket = enum_value(
            Bucket::from_wire,
            "job_step_review_findings",
            "bucket",
            &bucket,
        )
        .map_err(LoadJobError::Unreadable)?;
        Ok(Finding::of(bucket, &finding, &why))
    })
    .collect::<Result<Vec<_>, LoadJobError>>()?;

    Ok(Some(ReviewRecord {
        confidence,
        reasons,
        areas,
        tests: TestsInChange {
            proves,
            changed,
            untested,
        },
        findings,
        unexplained_tests,
    }))
}

/// Every row one query answers for one review, read in full or not at all.
fn rows<T>(
    conn: &Connection,
    sql: &str,
    key: Key<'_>,
    read: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, LoadJobError> {
    let mut statement = conn
        .prepare(sql)
        .map_err(|why| LoadJobError::Database(fault("preparing a review read")(why)))?;
    let answered = statement
        .query_map(key, read)
        .map_err(|why| LoadJobError::Database(fault("reading a review")(why)))?;
    answered
        .collect::<rusqlite::Result<Vec<T>>>()
        .map_err(|why| LoadJobError::Database(fault("reading a review's rows")(why)))
}
