//! Which run of a step a record belongs to, and the whole record across runs.
//!
//! # The defect this closes
//!
//! Every per-step table was keyed by step alone and replaced whole on a second
//! visit, so a step that ran four times kept one set of verdicts and read
//! identically to one that passed first time. `docs/concepts/workflow.md`:
//! *"keeping all the verdicts is what shows the same note went unaddressed
//! three times."*
//!
//! # Three readings of one log, and no two are the same number
//!
//! [`Store::step_attempt`] climbs forever and keys the per-run records;
//! [`Store::step_spent`] resets on a return, because `retry_limit` does;
//! [`Store::step_iteration`] counts passes and charges them to the step that
//! *caused* them. `core_model` gives each its own type so a call site cannot
//! quietly take the wrong one — which is how `may_hand_back` spent a looping
//! step's retry budget on its own iterations until `#263`.
//!
//! All three are read off `job_events` inside the transaction that writes, and
//! **no caller supplies a number**, so none can disagree with the history.
//! **Nothing here folds and the fold reads none of it**: an attempt is a
//! coordinate on evidence about a `Job` state, never a way to reach one.
use core_model::{
    Attempt, CheckOutcome, CriterionId, EvidenceType, GamingFlag, GamingPattern, Iteration, JobId,
    JudgeVerdict, Judgment, Spent, StepCheck, StepEvidence, StepId, Timestamp,
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
    /// step that has routed nothing back is on its first pass — which is every
    /// step of every linear workflow, and why the answer is never `None`.
    ///
    /// **It counts the returns this step *caused*, not the returns that landed
    /// on it.** `iteration_count` is the emitting step's, settled in
    /// `docs/journeys/triage-queue.md` — the cap and the count it bounds live
    /// on one step or `loop_cap` never fires, and the routed-to reading would
    /// sum two loops sharing a target step into one number. The emitting step
    /// makes no move of its own on a return, so the row the routed-to step
    /// writes names it in `returned_by`, and this counts by that column.
    pub fn step_iteration(&self, job_id: &JobId, step_id: &StepId) -> Result<Iteration, RowError> {
        iteration_now(&self.conn, job_id, step_id).map_err(RowError::Database)
    }

    /// How many runs the pass a step is on has spent — the count
    /// `ResolvedStep::may_hand_back` is asked against, and the one
    /// [`step_attempt`](Store::step_attempt) is not.
    ///
    /// **Derived, never stored**, for that call's reason and out of the same
    /// table. It is the entries into `running` *after* the last row that
    /// returned to this step, so it resets on every return —
    /// `workflowdef-fields.toml` on `retry_count`: *"Resets on a loop return —
    /// re-entry as designed is a fresh attempt budget."*
    ///
    /// Identical to the attempt on every step of every linear workflow, since
    /// there is no return for it to reset at. That is why the defect it closes
    /// was invisible: the two readings agreed until something looped.
    pub fn step_spent(&self, job_id: &JobId, step_id: &StepId) -> Result<Spent, RowError> {
        spent_now(&self.conn, job_id, step_id).map_err(RowError::Database)
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
///
/// **`returned_by` and not `step_id`.** The column names the step that emitted
/// the verdict, which is whose `iteration_count` this is; `step_id` on the same
/// row is the step being redone. [`V28`] is what added it, and the shape
/// trigger there is what makes it present on exactly these rows.
pub(crate) fn iteration_now(
    conn: &Connection,
    job_id: &JobId,
    step_id: &StepId,
) -> Result<Iteration, DatabaseFault> {
    let returns: i64 = conn
        .query_row(
            "SELECT count(*) FROM job_events
             WHERE job_id = ?1 AND returned_by = ?2
               AND kind = 'step_transition'
               AND state_from = 'advanced' AND state_to = 'running'",
            (job_id.as_str(), step_id.as_str()),
            |row| row.get(0),
        )
        .map_err(fault("counting the loop returns a step caused"))?;
    Ok(Iteration::returns_made(returns.max(0) as u32))
}

/// The runs the current pass over a step has spent, over the connection or the
/// transaction asking.
///
/// The third reading, and the one with a `seq` bound: everything after the last
/// row that opened a pass on this step. `seq` is the order the fold uses and
/// never `at`, which is injected and may repeat — a pass boundary read off
/// timestamps could put a run on the wrong side of it.
///
/// **Both of the loop's edges open a pass**, which is the whole of why the
/// inner query names two `state_from` values. `advanced -> running` is a
/// verdict routed back to this step and `running -> running` is the loop
/// coming round to the step that emitted it; each is a re-entry as designed,
/// and `retry_count`'s registry row gives a re-entry as designed a fresh
/// budget. `retrying -> running` and `stopped -> running` are inside a pass —
/// a failure answered and a person restarting — and must not reset it.
///
/// `coalesce(max(seq), 0)` is the linear case written once: a step no loop has
/// touched has no boundary, so every entry into `running` is inside its one
/// pass and this answers exactly what [`attempt_now`] does.
pub(crate) fn spent_now(
    conn: &Connection,
    job_id: &JobId,
    step_id: &StepId,
) -> Result<Spent, DatabaseFault> {
    let runs: i64 = conn
        .query_row(
            "SELECT count(*) FROM job_events
             WHERE job_id = ?1 AND step_id = ?2
               AND kind = 'step_transition' AND state_to = 'running'
               AND seq >= coalesce(
                   (SELECT max(seq) FROM job_events
                    WHERE job_id = ?1 AND step_id = ?2
                      AND kind = 'step_transition'
                      AND state_from IN ('advanced', 'running')
                      AND state_to = 'running'), 0)",
            (job_id.as_str(), step_id.as_str()),
            |row| row.get(0),
        )
        .map_err(fault("counting the runs a step's current pass has spent"))?;
    Ok(Spent::runs_this_pass(runs.max(0) as u32))
}

/// Version 28 — a loop return says which step caused it.
///
/// Beside the reading it serves rather than in `schema.rs`, which is at the 900
/// the gate refuses at — [`crate::report::V17`]'s precedent. The order still
/// lives in `MIGRATIONS`, which is the part that may not be anywhere else.
///
/// **The column is on the routed-*to* step's row, and it names the *emitting*
/// step.** `iteration_count` is the emitter's — `docs/journeys/triage-queue.md`
/// settles it, because a cap split from the count it bounds never fires and
/// because two loops sharing a target step would otherwise sum into one number
/// — and the emitter makes no move of its own on a return, so there was nothing
/// in the log to count against it. The alternatives were a seventh `StepState`,
/// which breaks the wire, and reusing `stopped`, whose registry row refuses
/// exactly this. Both were rejected; this is what is left.
///
/// **Nothing to backfill.** The `advanced -> running` edge landed one commit
/// before this and nothing walked it, so no stored row is a return. One written
/// by a build between the two would fold as `StepStateNotReachable` rather than
/// as an unattributed pass, which is the honest direction to be wrong in.
///
/// The shape trigger is rewritten whole because `SQLite` cannot alter one. The
/// step arm gains a second `CASE` beside the reason's, in the same shape and
/// for the same reason: the column is required on exactly the edge that stores
/// it and refused everywhere else, so a row cannot say a step looped without
/// saying whose loop it was.
pub(crate) const V28: &str = r#"
ALTER TABLE job_events ADD COLUMN returned_by TEXT;

DROP TRIGGER job_events_hold_one_whole_shape;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NULL
        AND NEW.returned_by IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL AND NEW.drone_id IS NULL
        AND NEW.status_from = NEW.status_to
        AND (CASE WHEN NEW.state_to IN ('stopped', 'retrying')
                    OR (NEW.state_to = 'advanced' AND NEW.state_from = 'stopped')
                  THEN NEW.reason_kind = 'escalation' AND NEW.reason_value IS NOT NULL
                  ELSE NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL
             END)
        AND (CASE WHEN NEW.state_from = 'advanced' AND NEW.state_to = 'running'
                  THEN NEW.returned_by IS NOT NULL
                  ELSE NEW.returned_by IS NULL
             END))
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NOT NULL AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL AND NEW.drone_id IS NOT NULL
        AND NEW.returned_by IS NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified' AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, hands it back, or advances one that stopped, and the step that routed it only where it is a loop return, or a drone arriving on a step or leaving it beneath one');
END;
"#;

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
