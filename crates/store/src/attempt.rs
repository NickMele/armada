//! Which run of a step a record belongs to, and the whole record across runs.
//!
//! # The defect this closes
//!
//! Every per-step table was keyed by step alone and replaced whole on a second
//! visit, so a step that ran four times kept one set of verdicts and read
//! identically to one that passed first time. `docs/concepts/workflow.md`:
//! *"keeping all the verdicts is what shows the same note went unaddressed
//! three times."*
//! # The ordinal is read off the log, not carried to the writer
//!
//! [`Store::step_attempt`] counts the step's `job_events` rows arriving at
//! `running`, and every writer calls it inside its own transaction. **No caller
//! supplies an attempt number**, so none can disagree with the history — which
//! is why this is a module and not four more arguments. **Nothing here folds
//! and the fold reads none of it**: an attempt is a coordinate on evidence
//! about a `Job` state, never a way to reach one.
//! Counting entries into `running` rather than back edges is what let this
//! survive the open question of a loop's return shape: a return is another run
//! and increments the attempt like any other. [`Store::step_iteration`] reads
//! the same table for the one edge that says a step is on a new pass.
//!
//! **Not fixed here**: `ResolvedStep::may_hand_back` is asked with
//! [`Store::step_attempt`], which now climbs across a return, so a looping
//! step's iterations would spend its retry budget. `#263`.

use core_model::{
    Attempt, CheckOutcome, CriterionId, EvidenceType, GamingFlag, GamingPattern, Iteration, JobId,
    JudgeVerdict, Judgment, StepCheck, StepEvidence, StepId, Timestamp,
};
use rusqlite::{Connection, Row};

use crate::error::{fault, DatabaseFault, LoadJobError, RowError};
use crate::open::Store;
use crate::row::{enum_value, maybe, string};

/// One step's record from one of the times it ran.
///
/// `record` is the whole of what that run produced of the kind asked for — the
/// list of Checks, of judgments or of flags, or the one piece of evidence. A
/// writer replaces a run's rows whole, so every row in a group came from one
/// write and they share `at`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempted<T> {
    pub step_id: StepId,
    pub attempt: Attempt,
    /// When that run wrote this down. Injected, like every instant here.
    pub at: Timestamp,
    pub record: T,
}

impl Store {
    /// Which run of a step anything written now belongs to.
    ///
    /// **Derived, never stored.** The count is over `job_events`, which is
    /// append-only in the database itself, so there is no second record of how
    /// many times a step has run and therefore no pair that can disagree.
    ///
    /// A step whose log holds no `running` row is on its first attempt — see
    /// [`Attempt::runs_begun`]. That is the ordinary case for a fixture and for
    /// anything recorded against a step before it was entered.
    pub fn step_attempt(&self, job_id: &JobId, step_id: &StepId) -> Result<Attempt, RowError> {
        attempt_now(&self.conn, job_id, step_id).map_err(RowError::Database)
    }

    /// Which pass over a step the work now belongs to.
    ///
    /// **Derived, never stored**, for [`step_attempt`](Store::step_attempt)'s
    /// reason and out of the same table. `job-fields.toml` types
    /// `iteration_count` a `job_steps` column; it is not one and neither is
    /// `retry_count`, because a column beside an append-only log is a second
    /// record of the same fact and a pair that can disagree.
    ///
    /// The count is over the one edge `STEP_EDGES` gives the loop return, so a
    /// step nothing has routed back to is on its first pass — which is every
    /// step of every linear workflow, and why the answer is never `None`.
    ///
    /// **It answers about the step named and makes no claim beyond it.**
    /// `iteration_count` is the *emitting* step's, settled in
    /// `docs/journeys/triage-queue.md`, and the emitting step has no move of
    /// its own on a return — so it has nothing here to count. Asking this
    /// about the routed-to step is asking how many times that step has been
    /// redone, which is true, renders as `workflows.toml` describes the canvas
    /// drawing it, and is not the number `iteration_cap` is asked against.
    pub fn step_iteration(&self, job_id: &JobId, step_id: &StepId) -> Result<Iteration, RowError> {
        iteration_now(&self.conn, job_id, step_id).map_err(RowError::Database)
    }

    /// What each of a Job's declared Checks did, **every run of every step**,
    /// oldest run first.
    ///
    /// The companion of [`step_checks`](Store::step_checks), which answers the
    /// latest run alone.
    pub fn step_checks_every_attempt(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<Attempted<Vec<StepCheck>>>, LoadJobError> {
        let rows = self
            .collect(
                "SELECT step_id, attempt, ran_at, name, outcome, expected, produced, output_path
                 FROM job_step_checks WHERE job_id = ?1 ORDER BY step_id, attempt, ordinal",
                job_id,
                "reading check results",
                |row| Ok((coordinate(row, "job_step_checks", "ran_at")?, check(row)?)),
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(grouped(rows))
    }

    /// What the Judge said, **every run of every step**, oldest run first.
    ///
    /// This is the read the design asks for by name: four runs of a step come
    /// back as four sets of verdicts, so the note that went unaddressed each
    /// time is on the record rather than inferred from the last one.
    pub fn step_judgments_every_attempt(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<Attempted<Vec<Judgment>>>, LoadJobError> {
        let rows = self
            .collect(
                "SELECT step_id, attempt, judged_at, criterion, verdict, expected, produced,
                        consequence, brief_path
                 FROM job_step_judgments WHERE job_id = ?1 ORDER BY step_id, attempt, ordinal",
                job_id,
                "reading judgments",
                |row| {
                    Ok((
                        coordinate(row, "job_step_judgments", "judged_at")?,
                        judgment(row)?,
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(grouped(rows))
    }

    /// What the gaming check flagged, **every run of every step**, oldest run
    /// first. The same pattern flagged on three runs is three groups here and
    /// was one row before.
    pub fn step_gaming_flags_every_attempt(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<Attempted<Vec<GamingFlag>>>, LoadJobError> {
        let rows = self
            .collect(
                "SELECT step_id, attempt, flagged_at, pattern, cited, cited_file, cited_line
                 FROM job_step_gaming_flags WHERE job_id = ?1
                 ORDER BY step_id, attempt, ordinal",
                job_id,
                "reading gaming flags",
                |row| {
                    Ok((
                        coordinate(row, "job_step_gaming_flags", "flagged_at")?,
                        flag(row)?,
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(grouped(rows))
    }

    /// The evidence each run of each step submitted, oldest run first.
    ///
    /// One row per run rather than a list, which is why this returns the
    /// evidence itself where the other three return a `Vec`: a run submits one
    /// piece of evidence, and a resubmission **inside the same run** still
    /// supersedes — a superseded submission is not a baseline.
    pub fn step_evidence_every_attempt(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<Attempted<StepEvidence>>, LoadJobError> {
        self.collect(
            "SELECT step_id, attempt, recorded_at, evidence_type, claimed, shown_by, not_claimed
             FROM job_step_evidence WHERE job_id = ?1 ORDER BY step_id, attempt",
            job_id,
            "reading step evidence",
            |row| {
                let (step_id, attempt, at) = coordinate(row, "job_step_evidence", "recorded_at")?;
                Ok(Attempted {
                    step_id,
                    attempt,
                    at,
                    record: evidence(row)?,
                })
            },
        )
        .map_err(LoadJobError::Unreadable)
    }
}

/// The attempt anything written now belongs to, over the connection or the
/// transaction doing the writing.
///
/// Taken as `&Connection` so a `&Transaction` derefs into it: the count has to
/// be read inside the transaction that writes the rows, or a step that stopped
/// and restarted between the two would file the rows under the wrong run.
pub(crate) fn attempt_now(
    conn: &Connection,
    job_id: &JobId,
    step_id: &StepId,
) -> Result<Attempt, DatabaseFault> {
    let runs: i64 = conn
        .query_row(
            "SELECT count(*) FROM job_events
             WHERE job_id = ?1 AND step_id = ?2
               AND kind = 'step_transition' AND state_to = 'running'",
            (job_id.as_str(), step_id.as_str()),
            |row| row.get(0),
        )
        .map_err(fault("counting a step's runs"))?;
    Ok(Attempt::runs_begun(runs.max(0) as u32))
}

/// The pass a step is on, over the connection or the transaction asking.
///
/// The mirror of [`attempt_now`] and the same shape, over the one edge that is
/// a loop return: `advanced -> running` is walked by `StepTarget::Returned`
/// alone, which `core_model::step_machine` narrows so that no dispatch or
/// resume can write this row.
pub(crate) fn iteration_now(
    conn: &Connection,
    job_id: &JobId,
    step_id: &StepId,
) -> Result<Iteration, DatabaseFault> {
    let returns: i64 = conn
        .query_row(
            "SELECT count(*) FROM job_events
             WHERE job_id = ?1 AND step_id = ?2
               AND kind = 'step_transition'
               AND state_from = 'advanced' AND state_to = 'running'",
            (job_id.as_str(), step_id.as_str()),
            |row| row.get(0),
        )
        .map_err(fault("counting a step's loop returns"))?;
    Ok(Iteration::returns_made(returns.max(0) as u32))
}

/// The three columns that say which run a row is from.
fn coordinate(
    row: &Row<'_>,
    table: &'static str,
    stamped: &'static str,
) -> Result<(StepId, Attempt, Timestamp), RowError> {
    let number: i64 = row
        .get("attempt")
        .map_err(crate::row::column(table, "attempt"))?;
    let attempt = u32::try_from(number)
        .ok()
        .and_then(Attempt::stored)
        .ok_or_else(|| RowError::MalformedColumn {
            table,
            column: "attempt",
            detail: "an attempt is one-based and this is not".to_string(),
        })?;
    Ok((
        StepId::new(string(row, "step_id")?),
        attempt,
        Timestamp::from_rfc3339(string(row, stamped)?),
    ))
}

fn check(row: &Row<'_>) -> Result<StepCheck, RowError> {
    let outcome = string(row, "outcome")?;
    Ok(StepCheck {
        name: string(row, "name")?,
        outcome: enum_value(
            CheckOutcome::from_wire,
            "job_step_checks",
            "outcome",
            &outcome,
        )?,
        expected: maybe(row, "expected")?,
        produced: maybe(row, "produced")?,
        output_path: maybe(row, "output_path")?,
    })
}

fn judgment(row: &Row<'_>) -> Result<Judgment, RowError> {
    let verdict = string(row, "verdict")?;
    Ok(Judgment {
        criterion_id: CriterionId::new(string(row, "criterion")?),
        verdict: enum_value(
            JudgeVerdict::from_wire,
            "job_step_judgments",
            "verdict",
            &verdict,
        )?,
        expected: maybe(row, "expected")?,
        produced: maybe(row, "produced")?,
        consequence: maybe(row, "consequence")?,
        brief_path: maybe(row, "brief_path")?,
    })
}

fn flag(row: &Row<'_>) -> Result<GamingFlag, RowError> {
    let pattern = string(row, "pattern")?;
    Ok(GamingFlag {
        pattern: enum_value(
            GamingPattern::from_wire,
            "job_step_gaming_flags",
            "pattern",
            &pattern,
        )?,
        cited: string(row, "cited")?,
        at: crate::gaming::cited_at(row)?,
    })
}

fn evidence(row: &Row<'_>) -> Result<StepEvidence, RowError> {
    let kind = string(row, "evidence_type")?;
    Ok(StepEvidence {
        evidence_type: enum_value(
            EvidenceType::from_wire,
            "job_step_evidence",
            "evidence_type",
            &kind,
        )?,
        claimed: string(row, "claimed")?,
        shown_by: string(row, "shown_by")?,
        not_claimed: string(row, "not_claimed")?,
    })
}

/// One group per step per run, in the order the rows came back.
///
/// A linear pass rather than a map, for the reason `gaming::grouped` gives: the
/// rows are already ordered by the key being grouped on, so a map would be a
/// second index over a sorted list. The key is the pair now, so a step's second
/// run opens a new group rather than joining its first.
fn grouped<T>(rows: Vec<((StepId, Attempt, Timestamp), T)>) -> Vec<Attempted<Vec<T>>> {
    let mut grouped: Vec<Attempted<Vec<T>>> = Vec::new();
    for ((step_id, attempt, at), one) in rows {
        match grouped.last_mut() {
            Some(last) if last.step_id == step_id && last.attempt == attempt => {
                last.record.push(one)
            }
            _ => grouped.push(Attempted {
                step_id,
                attempt,
                at,
                record: vec![one],
            }),
        }
    }
    grouped
}
