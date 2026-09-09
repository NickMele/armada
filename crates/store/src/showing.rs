//! The frames a step's harness produced, and where each one was kept.
//!
//! A step whose evidence is what it looks like is reviewed by looking at it,
//! and until this table nothing recorded that there was anything to look at.
//! The rows here are the record half of `fleet::showing`'s run: one per frame,
//! keyed the way a Check's result is keyed — Job, step, attempt, ordinal — so
//! two runs of one step do not read as one run that captured twice.
//!
//! **The row is a path, not the image.** `job_step_checks.output_path` holds a
//! Check's output the same way and for the same reason: a frame is hundreds of
//! kilobytes, `get_job` is re-read on every event naming the open Job, and
//! `/events` is one drop-oldest channel carrying every Job. The cheap fact —
//! that a frame exists, what it is called and what it weighs — rides the
//! record, and the bytes are fetched once by whoever opens it.
//!
//! **Beside the change it makes rather than in `schema.rs`**, like
//! [`V29`](crate::proving::V29) and for its reason: that file is at the 900
//! lines the gate refuses at.

use core_model::{JobId, Side, StepFrame, StepId, Timestamp};
use rusqlite::Row;

use crate::attempt::attempt_now;
use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, string};

/// Version 37 — the frames a step's harness produced.
///
/// **`REFERENCES jobs(job_id)`, unlike [`commit_checks`](crate::proving::V29).**
/// A frame belongs to one step of one Job and to nothing else, so it belongs in
/// `forget_job`'s sweep — which reads the file's foreign keys rather than a
/// list, so this row needs no second registration anywhere.
///
/// `attempt` carries the same `CHECK (attempt >= 1)` `job_step_checks` does,
/// for that column's reason: zero is not an attempt and `Attempt` cannot hold
/// one, so the database refuses such a row rather than reading it as a first
/// run.
///
/// **No column for which side the frame came from.** That waited for the run
/// against `base` to exist and it now does — [`V41`] adds it, and adds it
/// backfilled rather than nullable, which is what V5's rule permits here: every
/// row this table held before that migration was taken on the branch, because
/// the branch was the only place a harness ran. That is an observation, not a
/// default.
///
/// **`bytes` and no media type.** The file's own extension is in `name` and in
/// `path`, and a second field naming the same fact is a second place to keep it
/// true. What serves the bytes reads the extension off the path it opened.
pub(crate) const V37: &str = r#"
CREATE TABLE job_step_frames (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL CHECK (attempt >= 1),
    ordinal INTEGER NOT NULL,
    name    TEXT NOT NULL,
    path    TEXT NOT NULL,
    bytes   INTEGER NOT NULL,
    kept_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;
"#;

/// Version 41 — which checkout each frame is a photograph of.
///
/// **Backfilled to `branch` rather than left null**, which is the one shape V5's
/// rule allows and only because the fact is observed: until the migration above
/// this one, `fleet::showing` served and shot the Job's own worktree and
/// nothing else, so every existing row *is* a branch frame. A nullable column
/// would say Fleet cannot tell, which would be untrue of every row it applies
/// to.
///
/// **A trigger and not a `CHECK`.** SQLite's `ALTER TABLE ADD COLUMN` takes no
/// constraint, and the alternative — rebuilding the table to carry one — would
/// move every row to add a word. The trigger is what `jobs_are_never_given_a_
/// blank_branch` already does for the same reason, and it refuses the same
/// thing: a value nothing in [`Side`](core_model::Side) spells, which would
/// reach a reader as a frame belonging to neither side of a pair.
pub(crate) const V41: &str = r#"
ALTER TABLE job_step_frames ADD COLUMN side TEXT NOT NULL DEFAULT 'branch';

CREATE TRIGGER frames_are_taken_on_one_of_two_sides
BEFORE INSERT ON job_step_frames
WHEN NEW.side NOT IN ('base', 'branch')
BEGIN
    SELECT RAISE(ABORT, 'a frame is taken on the base or on the branch');
END;
"#;

fn unreadable(cause: rusqlite::Error) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(fault("reading a step's frames")(cause)))
}

fn frame(row: &Row<'_>) -> Result<StepFrame, RowError> {
    Ok(StepFrame {
        name: string(row, "name")?,
        path: string(row, "path")?,
        // A negative size is not a row this ever writes, and clamping is what
        // keeps a corrupt one from becoming a panic on a read taken to draw a
        // panel. `u64` is the shape `CheckOutput::bytes` already crosses as.
        bytes: row
            .get::<_, i64>("bytes")
            .map_err(column("job_step_frames", "bytes"))?
            .max(0) as u64,
        // A word `Side` does not spell is a row nothing wrote — the trigger on
        // V41 refuses one — so it is a corrupt row rather than an old one, and
        // it is refused here rather than read as the branch. Calling an unknown
        // photograph the *after* is exactly what would put it beside a real one
        // and label the two a pair.
        side: Side::of(&string(row, "side")?)
            .ok_or_else(|| column("job_step_frames", "side")(rusqlite::Error::InvalidQuery))?,
    })
}

impl Store {
    /// Write down the frames one run of one step produced, replacing whatever a
    /// previous pass **over that same run** wrote.
    ///
    /// The same shape as [`record_step_checks`](Store::record_step_checks) and
    /// for its reason: a step is shown afresh each time it is submitted, and
    /// two passes' frames interleaved would read as one run that captured
    /// twice as much. The run is derived from the log inside this transaction
    /// rather than passed in, so a caller cannot file rows under a run the
    /// history does not have.
    ///
    /// **A run that produced no frame writes nothing**, so the absence of a row
    /// stays the absence of a capture rather than becoming a capture of
    /// nothing. What a run that was attempted and produced nothing says is
    /// `fleet::showing`'s to say, in the step's own log.
    pub fn record_step_frames(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        frames: &[StepFrame],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        if frames.is_empty() {
            return Ok(());
        }
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the frame record"))
            .map_err(WriteError::Database)?;
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;
        tx.execute(
            "DELETE FROM job_step_frames WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3",
            (job_id.as_str(), step_id.as_str(), attempt.number()),
        )
        .map_err(fault("clearing this run's previous frames"))
        .map_err(WriteError::Database)?;
        for (ordinal, frame) in frames.iter().enumerate() {
            tx.execute(
                "INSERT INTO job_step_frames (
                     job_id, step_id, attempt, ordinal, name, path, bytes, kept_at, side
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    job_id.as_str(),
                    step_id.as_str(),
                    attempt.number(),
                    ordinal as i64,
                    frame.name.as_str(),
                    frame.path.as_str(),
                    frame.bytes as i64,
                    at.as_str(),
                    frame.side.as_str(),
                ],
            )
            .map_err(fault("writing a frame"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing the frame record"))
            .map_err(WriteError::Database)
    }

    /// Every frame this Job kept, by step and by run, oldest run first.
    ///
    /// **Every attempt's and not the latest's**, which is
    /// `step_checks_every_attempt`'s shape and its reason: a person comparing
    /// a run that was handed back against the one that passed needs both, and
    /// a read that answered with the latest would make the earlier capture
    /// unreachable while its rows were still there.
    ///
    /// **One flat list rather than a map.** Its two callers want opposite
    /// groupings — the detail draws frames under a step, and the bytes route
    /// resolves one file name against every row the Job has — and a shape
    /// chosen for one would be undone by the other.
    pub fn step_frames_every_attempt(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<KeptFrame>, LoadJobError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT step_id, attempt, name, path, bytes, side FROM job_step_frames
                 WHERE job_id = ?1 ORDER BY step_id, attempt, ordinal",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map((job_id.as_str(),), |row| {
                let step: String = row.get("step_id")?;
                let attempt: i64 = row.get("attempt")?;
                Ok((step, attempt, frame(row)))
            })
            .map_err(unreadable)?;
        let mut kept = Vec::new();
        for row in rows {
            let (step, attempt, frame) = row.map_err(unreadable)?;
            kept.push(KeptFrame {
                step: StepId::new(step),
                attempt: attempt.max(1) as u32,
                frame: frame.map_err(LoadJobError::Unreadable)?,
            });
        }
        Ok(kept)
    }
}

/// One frame, stamped with the step and the run it came from.
///
/// **The stamp is on the row rather than implied by position**, which is
/// `ipc::CheckRun::attempt`'s rule: the list holds every attempt's frames, so a
/// reader has to be told which run a frame is from rather than counting through
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFrame {
    pub step: StepId,
    /// Counted from one, the same ordinal a Check's row carries.
    pub attempt: u32,
    pub frame: StepFrame,
}
