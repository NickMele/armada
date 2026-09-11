//! What a person's press captured, as runs of their own.
//!
//! **A press adds a set beside the step's frames and never replaces them.**
//! That is the owner's decision on `#603`, and the whole reason this is not
//! more rows in `job_step_frames`: that table is keyed by the step's attempt and
//! [`Store::record_step_frames`] replaces a run's rows when the step is shown
//! afresh, which is right for a re-gate and would erase the one set the step
//! itself produced if a press wrote there. So nothing written here can reach a
//! row the step wrote, and two presses are two sets told apart by when each ran.
//!
//! **Beside [`crate::showing`] rather than inside it**, which is at the size the
//! gate asks about, and beside the change it makes rather than in `schema.rs`
//! for [`V29`](crate::proving::V29)'s reason.

use core_model::{Attempt, EvidenceType, JobId, Side, StepFrame, StepId, Timestamp};
use rusqlite::{OptionalExtension, Row};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, string};

/// Version 43 — the frames a person's press captured, as runs of their own.
///
/// **`press` is the key and `attempt` is not.** Pressing twice reruns the same
/// attempt's spec twice, and those are two sets. The number is the Job's own
/// count of presses, one-based; `pressed_at` is what a person tells them apart
/// by. `step_id` and `attempt` name the submission whose `shown_by` was rerun,
/// so a reader can put the set under the step it belongs to.
///
/// **No `side`.** A press runs in the worktree and nowhere else — there is no
/// base run to switch back on here — so every row is a branch frame by
/// construction, and a column would hold one value forever.
///
/// `REFERENCES jobs(job_id)` puts it in `forget_job`'s sweep, which reads the
/// file's foreign keys rather than a list.
pub(crate) const V43: &str = r#"
CREATE TABLE job_shown_again (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    press      INTEGER NOT NULL CHECK (press >= 1),
    step_id    TEXT NOT NULL,
    attempt    INTEGER NOT NULL CHECK (attempt >= 1),
    ordinal    INTEGER NOT NULL,
    name       TEXT NOT NULL,
    path       TEXT NOT NULL,
    bytes      INTEGER NOT NULL,
    digest     TEXT NOT NULL,
    pressed_at TEXT NOT NULL,
    PRIMARY KEY (job_id, press, ordinal)
) STRICT;
"#;

fn unreadable(cause: rusqlite::Error) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(fault("reading what a press kept")(cause)))
}

impl Store {
    /// The spec a Drone last named on a step whose evidence is what it looks
    /// like, and the run that named it — or `None` where no Drone ever did.
    ///
    /// **`shown` rows only.** `shown_by` is required on every submission, but
    /// only a `shown` step's names a spec: on any other step it points at
    /// whatever shows the claim, a test or a file, and a press that handed that
    /// to `evidence.run` would put a Drone's sentence into a command line the
    /// repository wrote for a spec. A `shown` step's value is already
    /// substituted there by the step's own run, so rerunning it opens nothing
    /// the step had not.
    ///
    /// **The latest by when it was recorded**, across every step: the owner's
    /// words are *the last spec the Drone named*. A resubmission inside one run
    /// replaces its row, so the latest is what the step last stood on.
    pub fn spec_last_named(&self, job_id: &JobId) -> Result<Option<SpecNamed>, LoadJobError> {
        let found = self
            .conn
            .query_row(
                "SELECT step_id, attempt, shown_by FROM job_step_evidence
                 WHERE job_id = ?1 AND evidence_type = ?2
                 ORDER BY recorded_at DESC, attempt DESC, step_id DESC
                 LIMIT 1",
                (job_id.as_str(), EvidenceType::Shown.as_wire()),
                |row| {
                    let step: String = row.get("step_id")?;
                    let attempt: i64 = row.get("attempt")?;
                    let spec: String = row.get("shown_by")?;
                    Ok((step, attempt, spec))
                },
            )
            .optional()
            .map_err(unreadable)?;
        let Some((step, attempt, spec)) = found else {
            return Ok(None);
        };
        // Zero is refused by the table's own `CHECK`, so a row holding one was
        // written by something that did not share it — a corrupt row, and
        // refused rather than read as a first run.
        let Some(attempt) = u32::try_from(attempt).ok().and_then(Attempt::stored) else {
            return Err(LoadJobError::Unreadable(RowError::MalformedColumn {
                table: "job_step_evidence",
                column: "attempt",
                detail: format!("{attempt} is not a run"),
            }));
        };
        Ok(Some(SpecNamed {
            step: StepId::new(step),
            attempt,
            spec,
        }))
    }

    /// The number the next press on this Job will be kept under.
    ///
    /// **Read before the run rather than inside the write**, because the number
    /// names the directory the frames are copied into and that happens first.
    /// Two presses on one Job are refused by the Fleet that runs them, and the
    /// primary key refuses a second writer that got past it rather than letting
    /// one set land inside another.
    pub fn next_press(&self, job_id: &JobId) -> Result<u32, LoadJobError> {
        let highest: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(press), 0) FROM job_shown_again WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(unreadable)?;
        Ok(u32::try_from(highest.max(0)).unwrap_or(u32::MAX - 1) + 1)
    }

    /// Write down one press's frames as a set of their own.
    ///
    /// **It deletes nothing, anywhere.** [`record_step_frames`] replaces a
    /// run's rows because a step shown again is the same run shown afresh; a
    /// press is a new run, so the step's rows and every earlier press's are
    /// left exactly where they were.
    ///
    /// **A press that captured nothing writes nothing**, for that function's
    /// reason: the absence of a set stays the absence of a capture. What the
    /// press came to is said in the Job's own log and in the answer to the
    /// person who pressed.
    ///
    /// [`record_step_frames`]: Store::record_step_frames
    pub fn record_shown_again(
        &mut self,
        job_id: &JobId,
        press: u32,
        named: &SpecNamed,
        frames: &[StepFrame],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        if frames.is_empty() {
            return Ok(());
        }
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the record of a press"))
            .map_err(WriteError::Database)?;
        for (ordinal, frame) in frames.iter().enumerate() {
            tx.execute(
                "INSERT INTO job_shown_again (
                     job_id, press, step_id, attempt, ordinal, name, path, bytes, digest,
                     pressed_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    job_id.as_str(),
                    press,
                    named.step.as_str(),
                    named.attempt.number(),
                    ordinal as i64,
                    frame.name.as_str(),
                    frame.path.as_str(),
                    frame.bytes as i64,
                    frame.digest.as_str(),
                    at.as_str(),
                ],
            )
            .map_err(fault("writing a frame a press captured"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing the record of a press"))
            .map_err(WriteError::Database)
    }

    /// Every press this Job kept, one set each, oldest press first.
    ///
    /// **Sets and not a flat list**, unlike
    /// [`step_frames_every_attempt`](Store::step_frames_every_attempt): the
    /// set is what a person reads — a press is one moment somebody asked — and
    /// the bytes route that wants every row flattens this in one line.
    pub fn shown_again_every_press(&self, job_id: &JobId) -> Result<Vec<ShownAgain>, LoadJobError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT press, step_id, attempt, name, path, bytes, digest, pressed_at
                 FROM job_shown_again WHERE job_id = ?1 ORDER BY press, ordinal",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map((job_id.as_str(),), |row| {
                let press: i64 = row.get("press")?;
                let step: String = row.get("step_id")?;
                let attempt: i64 = row.get("attempt")?;
                let pressed_at: String = row.get("pressed_at")?;
                Ok((press, step, attempt, pressed_at, pressed(row)))
            })
            .map_err(unreadable)?;
        let mut sets: Vec<ShownAgain> = Vec::new();
        for row in rows {
            let (press, step, attempt, pressed_at, frame) = row.map_err(unreadable)?;
            let frame = frame.map_err(LoadJobError::Unreadable)?;
            let press = press.max(1) as u32;
            match sets.last_mut() {
                Some(set) if set.press == press => set.frames.push(frame),
                _ => sets.push(ShownAgain {
                    press,
                    pressed_at: Timestamp::from_rfc3339(pressed_at),
                    step: StepId::new(step),
                    attempt: attempt.max(1) as u32,
                    frames: vec![frame],
                }),
            }
        }
        Ok(sets)
    }
}

/// One frame a press captured, read back. **Always the branch**, because a
/// press runs in the worktree and nowhere else — see [`V43`].
fn pressed(row: &Row<'_>) -> Result<StepFrame, RowError> {
    Ok(StepFrame {
        name: string(row, "name")?,
        path: string(row, "path")?,
        digest: string(row, "digest")?,
        bytes: row
            .get::<_, i64>("bytes")
            .map_err(column("job_shown_again", "bytes"))?
            .max(0) as u64,
        side: Side::Branch,
    })
}

/// The spec a press reruns, and the run of the step that named it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecNamed {
    pub step: StepId,
    pub attempt: Attempt,
    /// The Drone's own `shown_by`, as it submitted it.
    pub spec: String,
}

/// One press, and the frames it captured.
///
/// **Told apart by when it ran**, which is the owner's decision on `#603`: a
/// press never replaces the step's frames or an earlier press's, so the set is
/// the unit and `pressed_at` is what a person reads it by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownAgain {
    /// The Job's own count of presses, one-based.
    pub press: u32,
    pub pressed_at: Timestamp,
    /// The step whose spec was rerun, and the run of it that named the spec.
    pub step: StepId,
    pub attempt: u32,
    /// Never empty — a press that captured nothing wrote no set.
    pub frames: Vec<StepFrame>,
}
