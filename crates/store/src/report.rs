//! What a person says went wrong, kept after the Job it is about is gone.
//!
//! **Every other record in this crate answers *did this fail*.** None answers
//! *did this fail correctly*, and neither set contains the other: a Job the
//! Judge refused on a sentence that existed nowhere is `escalated` with a real
//! `gate_failure` on it, and nothing anywhere says the refusal was wrong. This
//! table is where that goes.
//!
//! **It outlives the Job, which is why there is no foreign key.** Every other
//! table under a Job points at `jobs(job_id)`, which is what
//! [`tables_pointing_at_a_job`](crate::schema::tables_pointing_at_a_job) reads
//! and [`forget_job`](Store::forget_job) empties. This one points at nothing,
//! so `armada clean` cannot reach it — and a report about a Job you have since
//! cleaned up is exactly the report still worth having. `job_id` is therefore a
//! dangling id by design, and `record` is the copy that does not depend on it.
//!
//! **One store, origin as a column**, which v1 settled: a person triaging on a
//! Monday morning does not care which half of the machine noticed. Only `human`
//! is written today; the column is here so the day something else files one,
//! that is a value rather than a migration.
//!
//! **Neither `origin` nor `claim` is interpreted here.** Both are wire
//! spellings kept as the text they arrived as: nothing in this crate or in the
//! Job machine branches on either, and the count groups on the column. `ipc` is
//! their sole authority, and a `from_wire` here would be a second vocabulary.

// The migration is in this file rather than `schema.rs`, which is at the 900
// lines the gate refuses at and says so in V16's own comment. The order still
// lives there, which is the part that may not be in two places.

use core_model::{CriterionId, JobId, StepId, Timestamp};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{maybe, string};

/// Version 17 — what a person says went wrong, and the record they say it
/// about.
///
/// **No `REFERENCES jobs(job_id)`, and that is the whole design of the table.**
/// See this module's own note: the delete that forgets a Job is derived from
/// the file's catalog of tables pointing at `jobs`, so a report is out of its
/// reach by construction rather than by a rule somebody has to remember.
///
/// `record` is the Job's own evidence as it stood when the report was filed,
/// rendered once and stored. Not a join, because the rows it joins to are
/// exactly the rows `armada clean` takes away.
pub(crate) const V17: &str = r#"
CREATE TABLE reports (
    report_id  TEXT PRIMARY KEY NOT NULL,
    filed_at   TEXT NOT NULL,
    -- `human` today, and the only value written. A wire spelling, never read
    -- back into an enum here.
    origin     TEXT NOT NULL,
    -- What the person says the machine got wrong. A wire spelling, and what a
    -- count groups on.
    claim      TEXT NOT NULL,
    -- The id the Job had. Deliberately not a foreign key: the Job may be gone.
    job_id     TEXT NOT NULL,
    job_title  TEXT NOT NULL,
    -- How narrowly the claim is aimed: both null for the whole Job, `step_id`
    -- alone for a step no criterion was judged on, both set for one verdict.
    -- A criterion with no step is the one combination that is malformed.
    step_id    TEXT,
    criterion  TEXT,
    -- The person's own sentence. The finding.
    said       TEXT NOT NULL,
    -- The Job's record, rendered at filing time. The evidence.
    record     TEXT NOT NULL
) STRICT;
"#;

/// One report, as it was filed.
///
/// **`said` is the finding and `record` is the evidence**, and the two are
/// separate fields because they are separate things: a bundle that says nothing
/// is the terminal-paste this record exists to remove, automated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Minted by Fleet, like every other id that names a record.
    pub report_id: String,
    pub filed_at: Timestamp,
    /// `human` or whatever a later writer spells itself. Never interpreted here.
    pub origin: String,
    /// What the person says was wrong. Never interpreted here.
    pub claim: String,
    /// The Job it is about. **May name a Job that has since been forgotten.**
    pub job_id: JobId,
    /// What the Job was called, copied so the report reads without it.
    pub job_title: String,
    /// The step the report is about, where it is about one.
    pub step_id: Option<StepId>,
    /// The criterion whose verdict is disputed, where one is. Set with
    /// `step_id` and never without it — a criterion id is unique inside a step.
    /// **`step_id` without this is a report about a step no criterion was
    /// judged on**, which is what an undecided gate leaves behind.
    pub criterion_id: Option<CriterionId>,
    /// The person's own words.
    pub said: String,
    /// The Job's record as it stood, rendered.
    pub record: String,
}

impl Store {
    /// File one report. **There is no update and no delete.**
    ///
    /// A report is a thing a person said at a moment, so editing one would make
    /// the record disagree with what was said — the same property `job_events`
    /// has by trigger, held here by there being no method.
    pub fn record_report(&mut self, report: &Report) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO reports
                   (report_id, filed_at, origin, claim, job_id, job_title,
                    step_id, criterion, said, record)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    report.report_id,
                    report.filed_at.as_str(),
                    report.origin,
                    report.claim,
                    report.job_id.as_str(),
                    report.job_title,
                    report.step_id.as_ref().map(|step| step.as_str()),
                    report.criterion_id.as_ref().map(|id| id.as_str()),
                    report.said,
                    report.record,
                ],
            )
            .map_err(fault("filing a report"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every report, newest first.
    ///
    /// Newest first because a report is read to see what has been said lately,
    /// where a Job's log is read in the order it happened. The id is a ULID, so
    /// the sort is the filing order without reading the timestamp.
    pub fn reports(&self) -> Result<Vec<Report>, LoadJobError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT report_id, filed_at, origin, claim, job_id, job_title,
                        step_id, criterion, said, record
                 FROM reports ORDER BY report_id DESC",
            )
            .map_err(fault("preparing the report read"))
            .map_err(RowError::Database)
            .map_err(LoadJobError::Unreadable)?;
        let rows = statement
            .query_map([], |row| Ok(read(row)))
            .map_err(fault("reading reports"))
            .map_err(RowError::Database)
            .map_err(LoadJobError::Unreadable)?;
        let mut reports = Vec::new();
        for row in rows {
            let report = row
                .map_err(fault("reading a report"))
                .map_err(RowError::Database)
                .map_err(LoadJobError::Unreadable)?;
            reports.push(report.map_err(LoadJobError::Unreadable)?);
        }
        Ok(reports)
    }

    /// How many criteria the Judge has refused, over every Job in the store.
    ///
    /// **The denominator of nothing, deliberately.** It is served beside the
    /// count of refusals a person has disputed so the two can be read together,
    /// and it is not divided by anything here: a rate whose denominator counts
    /// Jobs nobody read would say the Judge is right about work no one has
    /// looked at. #117's own warning, kept as an absence rather than a note.
    ///
    /// Every attempt counts, not the latest: a refusal that was retried away
    /// still happened, and the question is how often the Judge refuses.
    pub fn refusals_recorded(&self) -> Result<u32, LoadJobError> {
        self.count("SELECT count(*) FROM job_step_judgments WHERE verdict = 'not_met'")
    }

    /// How many reports carry each claim, as `(claim, count)`, ordered by the
    /// spelling so a caller reading two of them reads them the same way twice.
    pub fn reports_by_claim(&self) -> Result<Vec<(String, u32)>, LoadJobError> {
        let mut statement = self
            .conn
            .prepare("SELECT claim, count(*) AS filed FROM reports GROUP BY claim ORDER BY claim")
            .map_err(fault("preparing the claim count"))
            .map_err(RowError::Database)
            .map_err(LoadJobError::Unreadable)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(fault("counting claims"))
            .map_err(RowError::Database)
            .map_err(LoadJobError::Unreadable)?;
        let mut counts = Vec::new();
        for row in rows {
            let (claim, filed) = row
                .map_err(fault("counting a claim"))
                .map_err(RowError::Database)
                .map_err(LoadJobError::Unreadable)?;
            counts.push((claim, filed.max(0) as u32));
        }
        Ok(counts)
    }

    fn count(&self, sql: &str) -> Result<u32, LoadJobError> {
        let counted: i64 = self
            .conn
            .query_row(sql, [], |row| row.get(0))
            .map_err(fault("counting"))
            .map_err(RowError::Database)
            .map_err(LoadJobError::Unreadable)?;
        Ok(counted.max(0) as u32)
    }
}

/// One row, or the column that would not read.
///
/// A criterion with no step is [`RowError::MalformedColumn`] rather than a scope
/// silently dropped: a report that claimed a criterion was wrong and came back
/// naming none would be the disputed verdict quietly widening to the Job.
///
/// **A step with no criterion is a row and not a fault.** It is what a report
/// about a step the gate judged nothing on looks like — `gate_undecided` records
/// no verdict, so there is no criterion to name and the step is the whole scope.
fn read(row: &rusqlite::Row<'_>) -> Result<Report, RowError> {
    let step_id = maybe(row, "step_id")?;
    let criterion = maybe(row, "criterion")?;
    if step_id.is_none() && criterion.is_some() {
        return Err(RowError::MalformedColumn {
            table: "reports",
            column: "criterion",
            detail: "a criterion with no step".to_string(),
        });
    }
    Ok(Report {
        report_id: string(row, "report_id")?,
        filed_at: Timestamp::from_rfc3339(string(row, "filed_at")?),
        origin: string(row, "origin")?,
        claim: string(row, "claim")?,
        job_id: JobId::carried(core_model::Ulid::carried(string(row, "job_id")?)),
        job_title: string(row, "job_title")?,
        step_id: step_id.map(StepId::new),
        criterion_id: criterion.map(CriterionId::new),
        said: string(row, "said")?,
        record: string(row, "record")?,
    })
}
