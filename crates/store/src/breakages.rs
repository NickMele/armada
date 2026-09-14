//! A test broken on main, and the Job drafted to fix it. #999.
//!
//! One row per repository, Check and test, owned by the fix: forgetting the fix
//! removes its claims, and Fleet gives them back when the fix settles.

use core_model::{Breakage, BreakageClaim, FixWaiter, JobId, ManifestId, Timestamp, Ulid};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 68 — which Job is fixing which test broken on main.
///
/// **`reported_by` is not a reference.** The reporting Job can be forgotten
/// while the fix is still worked, and a second key to `jobs` would make that
/// forget fail at commit. `job_id` is the fix, which is what `forget_job` keys on.
pub(crate) const V68: &str = r#"
CREATE TABLE job_breakage_claims (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    failure     TEXT NOT NULL,
    reported_by TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (repository, check_name, test)
) STRICT;
"#;

/// Version 69 — which Jobs failed on a claimed test, pointed at its fix. #1001.
///
/// **`fix` is not a reference**, for `reported_by`'s reason one table up.
/// `job_id` is the waiting Job, so forgetting it removes its pointers.
pub(crate) const V69: &str = r#"
CREATE TABLE job_fix_waiters (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    fix         TEXT NOT NULL,
    repository  TEXT NOT NULL,
    check_name  TEXT NOT NULL,
    test        TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (job_id, repository, check_name, test)
) STRICT;
"#;

impl Store {
    /// Claim a breakage for its fix. `false` where the same test in the same
    /// repository is already claimed, so a second report drafts nothing.
    ///
    /// **One statement**, so two claims cannot both land.
    pub fn claim_breakage(
        &mut self,
        claim: &BreakageClaim,
        at: &Timestamp,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_breakage_claims
                   (job_id, repository, check_name, test, failure, reported_by, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (repository, check_name, test) DO NOTHING",
                rusqlite::params![
                    claim.fix.as_str(),
                    claim.repository.as_str(),
                    claim.breakage.check,
                    claim.breakage.test,
                    claim.breakage.failure,
                    claim.reported_by.as_str(),
                    at.as_str(),
                ],
            )
            .map(|claimed| claimed == 1)
            .map_err(fault("claiming a breakage"))
            .map_err(WriteError::Database)
    }

    /// The claim on one test in one repository, where there is one.
    pub fn breakage_claimed(
        &self,
        repository: &ManifestId,
        check: &str,
        test: &str,
    ) -> Result<Option<BreakageClaim>, LoadJobError> {
        Ok(self
            .breakage_claims(
                "WHERE repository = ?1 AND check_name = ?2 AND test = ?3",
                rusqlite::params![repository.as_str(), check, test],
            )?
            .into_iter()
            .next())
    }

    /// Every breakage one fix Job claims, oldest first.
    pub fn breakages_claimed_by(&self, fix: &JobId) -> Result<Vec<BreakageClaim>, LoadJobError> {
        self.breakage_claims(
            "WHERE job_id = ?1 ORDER BY at, check_name, test",
            rusqlite::params![fix.as_str()],
        )
    }

    /// Every claimed fix for a test one Job's Drone reported, oldest first.
    pub fn breakages_reported_by(
        &self,
        reporter: &JobId,
    ) -> Result<Vec<BreakageClaim>, LoadJobError> {
        self.breakage_claims(
            "WHERE reported_by = ?1 ORDER BY at, check_name, test",
            rusqlite::params![reporter.as_str()],
        )
    }

    /// Every standing claim in one repository, oldest first.
    pub fn breakages_claimed_in(
        &self,
        repository: &ManifestId,
    ) -> Result<Vec<BreakageClaim>, LoadJobError> {
        self.breakage_claims(
            "WHERE repository = ?1 ORDER BY at, check_name, test",
            rusqlite::params![repository.as_str()],
        )
    }

    /// Point a Job at the fix claiming a test it failed on. `false` where it
    /// already was, so it is told once.
    pub fn point_at_fix(&mut self, waiter: &FixWaiter, at: &Timestamp) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_fix_waiters (job_id, fix, repository, check_name, test, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (job_id, repository, check_name, test) DO NOTHING",
                rusqlite::params![
                    waiter.waiting.as_str(),
                    waiter.fix.as_str(),
                    waiter.repository.as_str(),
                    waiter.check,
                    waiter.test,
                    at.as_str(),
                ],
            )
            .map(|pointed| pointed == 1)
            .map_err(fault("pointing a Job at a fix"))
            .map_err(WriteError::Database)
    }

    /// Every Job pointed at one fix, oldest first.
    pub fn waiting_on_fix(&self, fix: &JobId) -> Result<Vec<FixWaiter>, LoadJobError> {
        self.fix_waiters(
            "WHERE fix = ?1 ORDER BY at, job_id",
            rusqlite::params![fix.as_str()],
        )
    }

    /// Every fix one Job is pointed at, oldest first.
    pub fn fixes_waited_on_by(&self, job: &JobId) -> Result<Vec<FixWaiter>, LoadJobError> {
        self.fix_waiters(
            "WHERE job_id = ?1 ORDER BY at, check_name, test",
            rusqlite::params![job.as_str()],
        )
    }

    /// Drop every pointer at a fix, once the Jobs waiting on it have been told.
    pub fn release_waiters(&mut self, fix: &JobId) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_fix_waiters WHERE fix = ?1",
                (fix.as_str(),),
            )
            .map(|_| ())
            .map_err(fault("releasing the Jobs waiting on a fix"))
            .map_err(WriteError::Database)
    }

    fn fix_waiters(
        &self,
        filter: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<FixWaiter>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading fix waiters")(why));
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT job_id, fix, repository, check_name, test FROM job_fix_waiters {filter}"
            ))
            .map_err(unreadable)?;
        let rows = statement
            .query_map(params, |row| {
                Ok(FixWaiter {
                    waiting: JobId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    fix: JobId::carried(Ulid::carried(row.get::<_, String>(1)?)),
                    repository: ManifestId::carried(Ulid::carried(row.get::<_, String>(2)?)),
                    check: row.get(3)?,
                    test: row.get(4)?,
                })
            })
            .map_err(unreadable)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(unreadable)
    }

    /// Give back every claim a fix Job holds, once it has ended.
    pub fn release_breakages(&mut self, fix: &JobId) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_breakage_claims WHERE job_id = ?1",
                (fix.as_str(),),
            )
            .map(|_| ())
            .map_err(fault("releasing a fix's breakages"))
            .map_err(WriteError::Database)
    }

    fn breakage_claims(
        &self,
        filter: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<BreakageClaim>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading breakage claims")(why));
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT job_id, repository, check_name, test, failure, reported_by
                 FROM job_breakage_claims {filter}"
            ))
            .map_err(unreadable)?;
        let rows = statement
            .query_map(params, |row| {
                Ok(BreakageClaim {
                    fix: JobId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    repository: ManifestId::carried(Ulid::carried(row.get::<_, String>(1)?)),
                    breakage: Breakage {
                        check: row.get(2)?,
                        test: row.get(3)?,
                        failure: row.get(4)?,
                    },
                    reported_by: JobId::carried(Ulid::carried(row.get::<_, String>(5)?)),
                })
            })
            .map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unreadable)
    }
}
