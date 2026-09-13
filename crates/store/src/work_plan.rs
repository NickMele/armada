//! A Job's plan, kept as the history of every change made to it.
//!
//! **Appended and never edited**, by trigger as `job_events` is, and the plan
//! itself is [`WorkPlan::fold`] over the rows: a stored list would be a second
//! record that could disagree with its history.
//!
//! **A change is checked against the plan inside the transaction that writes
//! it**, so two changes cannot both be judged against the plan before either.

use core_model::{
    Approach, Attempt, JobId, NewTask, PlanAuthor, PlanChange, PlanEntry, PlanRefused, StepId,
    TaskId, TaskUpdate, Timestamp, WorkPlan,
};
use rusqlite::{Connection, Row};

use crate::attempt::attempt_now;
use crate::error::{fault, DatabaseFault, LoadJobError, RowError};
use crate::open::Store;
use crate::row::{column, maybe, maybe_number, string};

const CHANGES: &str = "job_work_plan_changes";

/// Version 59 — a Job's plan: its changes, and the tasks each recording named.
///
/// **Nothing to backfill.** A plan written to a file before this is not one
/// Fleet held, and a Job with no rows reads as a Job with no plan.
pub(crate) const V59: &str = r#"
CREATE TABLE job_work_plan_changes (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    seq        INTEGER NOT NULL CHECK (seq > 0),
    change     TEXT NOT NULL CHECK (change IN ('recorded', 'added', 'updated')),
    by_step    TEXT,
    by_attempt INTEGER CHECK (by_attempt IS NULL OR by_attempt > 0),
    at         TEXT NOT NULL,
    approach   TEXT,
    task_id    INTEGER CHECK (task_id IS NULL OR task_id > 0),
    title      TEXT,
    detail     TEXT,
    after_task INTEGER CHECK (after_task IS NULL OR after_task > 0),
    state      TEXT CHECK (state IS NULL OR state IN ('open', 'working', 'done', 'dropped')),
    reason     TEXT,
    PRIMARY KEY (job_id, seq),
    CHECK ((by_step IS NULL) = (by_attempt IS NULL)),
    CHECK (state IS NULL OR ((state = 'dropped') = (reason IS NOT NULL AND trim(reason) <> '')))
) STRICT;

CREATE TABLE job_work_plan_tasks (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    seq     INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    title   TEXT NOT NULL CHECK (trim(title) <> ''),
    detail  TEXT NOT NULL,
    PRIMARY KEY (job_id, seq, ordinal)
) STRICT;

CREATE TRIGGER job_work_plan_changes_are_never_edited
BEFORE UPDATE ON job_work_plan_changes
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never edited');
END;

CREATE TRIGGER job_work_plan_changes_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_changes
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never removed from a Job that exists');
END;

CREATE TRIGGER job_work_plan_tasks_are_never_edited
BEFORE UPDATE ON job_work_plan_tasks
BEGIN
    SELECT RAISE(ABORT, 'a recorded task is never edited');
END;

CREATE TRIGGER job_work_plan_tasks_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_tasks
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a recorded task is never removed from a Job that exists');
END;
"#;

/// Whose change this is, before the run it belongs to is read off the log.
///
/// **A step names no run**, for `record_step_plan`'s reason: the store derives
/// it inside the transaction, so no caller can file a change under a run the
/// history does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanHand<'a> {
    Step(&'a StepId),
    Person,
}

/// Why a change to a plan was not kept.
#[derive(Debug)]
pub enum PlanNotKept {
    /// The plan as it stands cannot take the change. Nothing was written.
    Refused(PlanRefused),
    /// The history already on file would not read, so no change was judged.
    Unreadable(RowError),
    Database(DatabaseFault),
}

impl std::fmt::Display for PlanNotKept {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanNotKept::Refused(why) => write!(out, "{why}"),
            PlanNotKept::Unreadable(why) => write!(out, "the plan on file will not read: {why}"),
            PlanNotKept::Database(why) => write!(out, "{why}"),
        }
    }
}

impl std::error::Error for PlanNotKept {}

impl Store {
    /// Append one change to a Job's plan, and answer with the plan it leaves.
    pub fn change_plan(
        &mut self,
        job_id: &JobId,
        change: &PlanChange,
        by: PlanHand<'_>,
        at: &Timestamp,
    ) -> Result<WorkPlan, PlanNotKept> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a plan change"))
            .map_err(PlanNotKept::Database)?;
        let by = match by {
            PlanHand::Step(step_id) => PlanAuthor::Step {
                step_id: step_id.clone(),
                attempt: attempt_now(&tx, job_id, step_id).map_err(PlanNotKept::Database)?,
            },
            PlanHand::Person => PlanAuthor::Person,
        };
        let history = history_in(&tx, job_id).map_err(PlanNotKept::Unreadable)?;
        let current = WorkPlan::fold(&history)
            .map_err(|why| PlanNotKept::Unreadable(does_not_replay(why)))?;
        let entry = PlanEntry {
            change: change.clone(),
            by,
            at: at.clone(),
        };
        let plan = WorkPlan::after(current.as_ref(), &entry).map_err(PlanNotKept::Refused)?;
        appended(&tx, job_id, &entry).map_err(PlanNotKept::Database)?;
        tx.commit()
            .map_err(fault("committing a plan change"))
            .map_err(PlanNotKept::Database)?;
        Ok(plan)
    }

    /// Every change made to a Job's plan, oldest first.
    pub fn plan_history(&self, job_id: &JobId) -> Result<Vec<PlanEntry>, LoadJobError> {
        history_in(&self.conn, job_id).map_err(LoadJobError::Unreadable)
    }

    /// The plan a Job's history leaves, or `None` where none was recorded.
    pub fn work_plan(&self, job_id: &JobId) -> Result<Option<WorkPlan>, LoadJobError> {
        WorkPlan::fold(&self.plan_history(job_id)?)
            .map_err(|why| LoadJobError::Unreadable(does_not_replay(why)))
    }
}

fn does_not_replay(why: PlanRefused) -> RowError {
    RowError::MalformedColumn {
        table: CHANGES,
        column: "change",
        detail: format!("the history does not replay: {why}"),
    }
}

fn malformed(column: &'static str, detail: &str) -> RowError {
    RowError::MalformedColumn {
        table: CHANGES,
        column,
        detail: detail.to_string(),
    }
}

fn appended(conn: &Connection, job_id: &JobId, entry: &PlanEntry) -> Result<(), DatabaseFault> {
    let seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM job_work_plan_changes WHERE job_id = ?1",
            (job_id.as_str(),),
            |row| row.get(0),
        )
        .map_err(fault("numbering a plan change"))?;
    let (by_step, by_attempt) = match &entry.by {
        PlanAuthor::Step { step_id, attempt } => (Some(step_id.as_str()), Some(attempt.number())),
        PlanAuthor::Person => (None, None),
    };
    let (kind, approach, task, title, detail, after, state, reason) = match &entry.change {
        PlanChange::Recorded { approach, .. } => (
            "recorded",
            Some(approach.as_str()),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        PlanChange::Added { task, after } => (
            "added",
            None,
            None,
            Some(task.title()),
            Some(task.detail()),
            after.map(TaskId::number),
            None,
            None,
        ),
        PlanChange::Updated { task, to } => (
            "updated",
            None,
            Some(task.number()),
            None,
            None,
            None,
            Some(to.state().as_wire()),
            to.reason(),
        ),
    };
    conn.execute(
        "INSERT INTO job_work_plan_changes (job_id, seq, change, by_step, by_attempt, at, \
         approach, task_id, title, detail, after_task, state, reason) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            job_id.as_str(),
            seq,
            kind,
            by_step,
            by_attempt,
            entry.at.as_str(),
            approach,
            task,
            title,
            detail,
            after,
            state,
            reason,
        ],
    )
    .map_err(fault("appending a plan change"))?;
    if let PlanChange::Recorded { tasks, .. } = &entry.change {
        for (ordinal, task) in tasks.iter().enumerate() {
            conn.execute(
                "INSERT INTO job_work_plan_tasks (job_id, seq, ordinal, title, detail) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    job_id.as_str(),
                    seq,
                    ordinal as i64,
                    task.title(),
                    task.detail()
                ],
            )
            .map_err(fault("appending a recorded task"))?;
        }
    }
    Ok(())
}

fn history_in(conn: &Connection, job_id: &JobId) -> Result<Vec<PlanEntry>, RowError> {
    let reading = "reading a plan's recorded tasks";
    let mut asked = conn
        .prepare(
            "SELECT seq, title, detail FROM job_work_plan_tasks \
             WHERE job_id = ?1 ORDER BY seq, ordinal",
        )
        .map_err(fault(reading))
        .map_err(RowError::Database)?;
    let mut recorded: Vec<(i64, NewTask)> = Vec::new();
    for row in asked
        .query_map((job_id.as_str(),), |row| {
            Ok((
                row.get::<_, i64>("seq"),
                string(row, "title"),
                string(row, "detail"),
            ))
        })
        .map_err(fault(reading))
        .map_err(RowError::Database)?
    {
        let (seq, title, detail) = row.map_err(fault(reading)).map_err(RowError::Database)?;
        let seq = seq.map_err(column("job_work_plan_tasks", "seq"))?;
        let task = NewTask::new(&title?, &detail?)
            .ok_or_else(|| malformed("title", "a recorded task has no title"))?;
        recorded.push((seq, task));
    }

    let reading = "reading a plan's changes";
    let mut asked = conn
        .prepare("SELECT * FROM job_work_plan_changes WHERE job_id = ?1 ORDER BY seq")
        .map_err(fault(reading))
        .map_err(RowError::Database)?;
    let rows = asked
        .query_map((job_id.as_str(),), |row| Ok(entry_of(row, &recorded)))
        .map_err(fault(reading))
        .map_err(RowError::Database)?;
    let mut history = Vec::new();
    for row in rows {
        history.push(row.map_err(fault(reading)).map_err(RowError::Database)??);
    }
    Ok(history)
}

fn entry_of(row: &Row<'_>, recorded: &[(i64, NewTask)]) -> Result<PlanEntry, RowError> {
    let seq: i64 = row.get("seq").map_err(column(CHANGES, "seq"))?;
    let by = match (maybe(row, "by_step")?, maybe_number(row, "by_attempt")?) {
        (Some(step), Some(number)) => PlanAuthor::Step {
            step_id: StepId::new(step),
            attempt: Attempt::stored(number)
                .ok_or_else(|| malformed("by_attempt", "an attempt is one-based"))?,
        },
        (None, None) => PlanAuthor::Person,
        _ => {
            return Err(malformed(
                "by_step",
                "a step and its run are named together",
            ))
        }
    };
    let task_id = |name: &'static str| -> Result<Option<TaskId>, RowError> {
        Ok(maybe_number(row, name)?
            .and_then(std::num::NonZeroU32::new)
            .map(TaskId::numbered))
    };
    let change = match string(row, "change")?.as_str() {
        "recorded" => PlanChange::Recorded {
            approach: Approach::new(&maybe(row, "approach")?.unwrap_or_default())
                .ok_or_else(|| malformed("approach", "a recorded plan has no approach"))?,
            tasks: recorded
                .iter()
                .filter(|(at, _)| *at == seq)
                .map(|(_, task)| task.clone())
                .collect(),
        },
        "added" => PlanChange::Added {
            task: NewTask::new(
                &maybe(row, "title")?.unwrap_or_default(),
                &maybe(row, "detail")?.unwrap_or_default(),
            )
            .ok_or_else(|| malformed("title", "an added task has no title"))?,
            after: task_id("after_task")?,
        },
        "updated" => PlanChange::Updated {
            task: task_id("task_id")?
                .ok_or_else(|| malformed("task_id", "an update names no task"))?,
            to: TaskUpdate::read(
                &maybe(row, "state")?.unwrap_or_default(),
                &maybe(row, "reason")?.unwrap_or_default(),
            )
            .map_err(|why| malformed("state", &format!("{why:?}")))?,
        },
        other => return Err(malformed("change", &format!("`{other}` is not a change"))),
    };
    Ok(PlanEntry {
        change,
        by,
        at: Timestamp::from_rfc3339(string(row, "at")?),
    })
}
