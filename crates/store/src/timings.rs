//! How long each of a repository's Checks has taken, so Fleet can start the
//! fastest first. #1062.
//!
//! **The repository's, not a Job's.** No row points at `jobs`, so forgetting a
//! Job keeps what its gate measured: the next Job on the same repository runs
//! the same Checks.

use std::collections::BTreeMap;
use std::time::Duration;

use core_model::{ManifestId, Timestamp};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// How many runs of one Check are kept and averaged. Few, so a Check that got
/// slower reorders within a handful of runs.
const KEPT: i64 = 5;

/// Version 71 — one row per whole run of a Check that ran to an exit code.
pub(crate) const V71: &str = r#"
CREATE TABLE check_timings (
    repository TEXT NOT NULL,
    check_name TEXT NOT NULL,
    took_ms    INTEGER NOT NULL CHECK (took_ms >= 0),
    at         TEXT NOT NULL
) STRICT;
CREATE INDEX check_timings_by_check ON check_timings (repository, check_name, at);
"#;

/// Version 72 — a fix draft's one-test runs, apart from `check_timings`. A
/// test alone is seconds where its Check whole is minutes; folded into the
/// same average it would move the Check ahead of ones that really are
/// faster. #1072.
pub(crate) const V72: &str = r#"
CREATE TABLE one_test_timings (
    repository TEXT NOT NULL,
    check_name TEXT NOT NULL,
    took_ms    INTEGER NOT NULL CHECK (took_ms >= 0),
    at         TEXT NOT NULL
) STRICT;
CREATE INDEX one_test_timings_by_check ON one_test_timings (repository, check_name, at);
"#;

impl Store {
    /// Keep one run's duration, dropping all but the latest few of that Check.
    pub fn record_check_took(
        &mut self,
        repository: &ManifestId,
        check: &str,
        took: Duration,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let failed = |why| WriteError::Database(fault("keeping how long a Check took")(why));
        let took_ms = i64::try_from(took.as_millis()).unwrap_or(i64::MAX);
        let kept = self.conn.transaction().map_err(failed)?;
        kept.execute(
            "INSERT INTO check_timings (repository, check_name, took_ms, at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![repository.as_str(), check, took_ms, at.as_str()],
        )
        .map_err(failed)?;
        kept.execute(
            "DELETE FROM check_timings
             WHERE repository = ?1 AND check_name = ?2 AND rowid NOT IN (
                 SELECT rowid FROM check_timings
                 WHERE repository = ?1 AND check_name = ?2
                 ORDER BY at DESC, rowid DESC LIMIT ?3)",
            rusqlite::params![repository.as_str(), check, KEPT],
        )
        .map_err(failed)?;
        kept.commit().map_err(failed)
    }

    /// Each Check's recent average in one repository. A Check never timed is
    /// absent, which is not a Check that takes no time.
    pub fn check_timings(
        &self,
        repository: &ManifestId,
    ) -> Result<BTreeMap<String, Duration>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading Check timings")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT check_name, AVG(took_ms) FROM (
                     SELECT check_name, took_ms, ROW_NUMBER() OVER (
                         PARTITION BY check_name ORDER BY at DESC, rowid DESC) AS latest
                     FROM check_timings WHERE repository = ?1)
                 WHERE latest <= ?2 GROUP BY check_name",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map(rusqlite::params![repository.as_str(), KEPT], |row| {
                let average: f64 = row.get(1)?;
                Ok((
                    row.get::<_, String>(0)?,
                    Duration::from_millis(average.max(0.0).round() as u64),
                ))
            })
            .map_err(unreadable)?;
        rows.collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(unreadable)
    }

    /// Keep one fix draft's one-test run, dropping all but the latest few of
    /// that Check's. Never `check_timings` — see [`V72`].
    pub fn record_one_test_took(
        &mut self,
        repository: &ManifestId,
        check: &str,
        took: Duration,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let failed =
            |why| WriteError::Database(fault("keeping how long a fix draft's test took")(why));
        let took_ms = i64::try_from(took.as_millis()).unwrap_or(i64::MAX);
        let kept = self.conn.transaction().map_err(failed)?;
        kept.execute(
            "INSERT INTO one_test_timings (repository, check_name, took_ms, at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![repository.as_str(), check, took_ms, at.as_str()],
        )
        .map_err(failed)?;
        kept.execute(
            "DELETE FROM one_test_timings
             WHERE repository = ?1 AND check_name = ?2 AND rowid NOT IN (
                 SELECT rowid FROM one_test_timings
                 WHERE repository = ?1 AND check_name = ?2
                 ORDER BY at DESC, rowid DESC LIMIT ?3)",
            rusqlite::params![repository.as_str(), check, KEPT],
        )
        .map_err(failed)?;
        kept.commit().map_err(failed)
    }

    /// Each Check's recent one-test average in one repository, for a test
    /// only — never folded into [`Store::check_timings`]. #1072.
    pub fn one_test_timings(
        &self,
        repository: &ManifestId,
    ) -> Result<BTreeMap<String, Duration>, LoadJobError> {
        let unreadable = |why: rusqlite::Error| {
            LoadJobError::Database(fault("reading a fix draft's test timings")(why))
        };
        let mut statement = self
            .conn
            .prepare(
                "SELECT check_name, AVG(took_ms) FROM (
                     SELECT check_name, took_ms, ROW_NUMBER() OVER (
                         PARTITION BY check_name ORDER BY at DESC, rowid DESC) AS latest
                     FROM one_test_timings WHERE repository = ?1)
                 WHERE latest <= ?2 GROUP BY check_name",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map(rusqlite::params![repository.as_str(), KEPT], |row| {
                let average: f64 = row.get(1)?;
                Ok((
                    row.get::<_, String>(0)?,
                    Duration::from_millis(average.max(0.0).round() as u64),
                ))
            })
            .map_err(unreadable)?;
        rows.collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(unreadable)
    }
}
