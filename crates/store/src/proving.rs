//! What a repository's Checks said about a commit — **the one per-Check record
//! here that a Job does not own.**
//!
//! Two Jobs merging within a minute are two merges into one commit, so a record
//! keyed by whoever noticed first leaves the second re-running the suite or
//! reading somebody else's answer as its own. The key is the commit, and
//! [`Store::already_proved`] is the whole of the dedupe. `#474`, and
//! `docs/concepts/fleet.md` — *What Fleet knows after the merge*.
//!
//! **No foreign key to `jobs`, deliberately.** `forget_job` deletes from every
//! table that points at `jobs`, and forgetting one Job must not delete what a
//! commit another Job also merged into was proved to say.
//!
//! [`StepCheck`] and not a second shape: a Check's result is one fact, and two
//! structs for it would render two ways the first time a surface drew both.

use core_model::{CheckOutcome, StepCheck, Timestamp};
use rusqlite::Row;

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{enum_value, maybe, string};

/// Version 29 — what the repository's Checks said about one commit.
///
/// Beside the change it makes, like [`V21`](crate::delivery::V21): `schema.rs`
/// is at the 900 lines the gate refuses at.
///
/// **No `REFERENCES jobs(job_id)` and no `job_id` column.** Both are the point:
/// this is the one per-Check table in the file that a Job does not own, and a
/// foreign key would put it in `tables_pointing_at_a_job` and so into
/// `forget_job`'s sweep.
///
/// `at_commit` and not `commit`, which is a reserved word in SQLite and would
/// have to be quoted at every use — a quoting rule is a thing somebody
/// eventually forgets.
pub(crate) const V29: &str = r#"
CREATE TABLE commit_checks (
    at_commit   TEXT NOT NULL,
    ordinal     INTEGER NOT NULL,
    base        TEXT NOT NULL,
    name        TEXT NOT NULL,
    outcome     TEXT NOT NULL,
    expected    TEXT,
    produced    TEXT,
    ran_at      TEXT NOT NULL,
    output_path TEXT,
    PRIMARY KEY (at_commit, ordinal)
) STRICT;
"#;

/// What one repository's Checks said about one commit.
///
/// **The commit is the identity and the base is context.** A person reading
/// this wants to know which branch it was the tip of; nothing looks a proof up
/// by branch, because a branch names a different commit every day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proved {
    /// The commit the repository was standing on when the Checks ran.
    pub at_commit: String,
    /// The branch that commit was the tip of, as the forge named it.
    pub base: String,
    /// When the run was written down. Injected, like every instant here.
    pub at: Timestamp,
    /// One entry per Check the Manifest's `after_merge` named, in that order.
    ///
    /// **Never empty**, because a repository naming no Checks after a merge
    /// runs nothing and writes nothing — the absence of a row is the absence of
    /// a run, and there is no third state for a run that measured nothing.
    pub checks: Vec<StepCheck>,
}

impl Proved {
    /// Every Check that did not pass, by name, in the order they were declared.
    ///
    /// **What a person is told**, and the reason it lives beside the record
    /// rather than at the call site: the line Fleet writes into a log and any
    /// later surface reading the table are the same question, and two spellings
    /// of *what went wrong* would disagree the first time a `Skipped` appeared.
    pub fn unhappy(&self) -> Vec<&str> {
        self.checks
            .iter()
            .filter(|check| !check.outcome.advances())
            .map(|check| check.name.as_str())
            .collect()
    }
}

fn unreadable(cause: rusqlite::Error) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(fault(
        "reading what the Checks said about a commit",
    )(cause)))
}

fn check(row: &Row<'_>) -> Result<StepCheck, RowError> {
    let outcome = string(row, "outcome")?;
    Ok(StepCheck {
        name: string(row, "name")?,
        outcome: enum_value(
            CheckOutcome::from_wire,
            "commit_checks",
            "outcome",
            &outcome,
        )?,
        expected: maybe(row, "expected")?,
        produced: maybe(row, "produced")?,
        output_path: maybe(row, "output_path")?,
    })
}

impl Store {
    /// Whether this commit has already been proved.
    ///
    /// **The whole of the guard against running the suite twice**, and it is a
    /// question about the commit rather than about a Job — which is what makes
    /// two Jobs merging into one commit cost one run. Asked before anything is
    /// spawned, so the second Job pays a single indexed read.
    pub fn already_proved(&self, at_commit: &str) -> Result<bool, LoadJobError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM commit_checks WHERE at_commit = ?1)",
                (at_commit,),
                |row| row.get::<_, i64>(0),
            )
            .map(|found| found == 1)
            .map_err(unreadable)
    }

    /// Write down what the Checks said about a commit.
    ///
    /// **Once per commit, and a second write replaces the first.** The replace
    /// is not a feature anything uses — [`already_proved`](Store::already_proved)
    /// stops the second run happening at all — it is what stops a race between
    /// two notices in one process leaving half of each run's rows interleaved
    /// under one key.
    ///
    /// **A run with no Checks writes nothing**, so the absence of a row stays
    /// the absence of a run.
    pub fn record_commit_checks(&mut self, proved: &Proved) -> Result<(), WriteError> {
        if proved.checks.is_empty() {
            return Ok(());
        }
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the commit's check record"))
            .map_err(WriteError::Database)?;
        tx.execute(
            "DELETE FROM commit_checks WHERE at_commit = ?1",
            (proved.at_commit.as_str(),),
        )
        .map_err(fault("clearing this commit's previous rows"))
        .map_err(WriteError::Database)?;
        for (ordinal, check) in proved.checks.iter().enumerate() {
            tx.execute(
                "INSERT INTO commit_checks (
                     at_commit, ordinal, base, name, outcome, expected, produced, ran_at,
                     output_path
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    proved.at_commit.as_str(),
                    ordinal as i64,
                    proved.base.as_str(),
                    check.name.as_str(),
                    check.outcome.as_wire(),
                    check.expected.as_deref(),
                    check.produced.as_deref(),
                    proved.at.as_str(),
                    check.output_path.as_deref(),
                ],
            )
            .map_err(fault("writing what a Check said about a commit"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing the commit's check record"))
            .map_err(WriteError::Database)
    }

    /// What the Checks said about one commit, or `None` where none has run.
    ///
    /// **Absent is *nobody has proved this*, and it is not a failure.** A
    /// repository that names no `after_merge` Checks never proves anything, and
    /// so does a merge Fleet declined to fast-forward onto.
    pub fn proved(&self, at_commit: &str) -> Result<Option<Proved>, LoadJobError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT base, ran_at, name, outcome, expected, produced, output_path
                 FROM commit_checks WHERE at_commit = ?1 ORDER BY ordinal",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map((at_commit,), |row| {
                let base: String = row.get("base")?;
                let ran_at: String = row.get("ran_at")?;
                Ok((base, ran_at, check(row)))
            })
            .map_err(unreadable)?;
        let mut proved: Option<Proved> = None;
        for row in rows {
            let (base, ran_at, check) = row.map_err(unreadable)?;
            let check = check.map_err(LoadJobError::Unreadable)?;
            match proved.as_mut() {
                Some(held) => held.checks.push(check),
                None => {
                    proved = Some(Proved {
                        at_commit: at_commit.to_string(),
                        base,
                        at: Timestamp::from_rfc3339(ran_at),
                        checks: vec![check],
                    })
                }
            }
        }
        Ok(proved)
    }
}
