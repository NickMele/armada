//! How a Job meets a Judge criterion that refuses, the question a person is
//! being asked while it holds, and which criteria this repository has stood
//! down.
//!
//! **Two facts and a setting, none of them an event.** [`crate::allowing`]'s
//! own header gives the reason: none of the three moves a status or a step,
//! so each column and table is the authority for its own field.
//!
//! **One open question per Job.** A second refusal reaching the gate while the
//! first is still unanswered does not arrive here — `fleet::judging::looks`
//! asks about one criterion at a time — so `job_id` alone is the key rather
//! than `(job_id, criterion_id)`.
//!
//! **A tolerance is repository-wide and outlives every Job.** This file is
//! opened once per repository, so `criterion_id` alone is the key: there is no
//! second repository's row to collide with.

use core_model::{Actor, CriterionId, JobId, Timestamp, WhenRefused};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value};

/// Version 52 — how a Job meets a Judge criterion that refuses, the question
/// held open while a person answers, and the criteria this repository has
/// stood down. Beside the change it makes, like every migration since
/// [`V17`](crate::report::V17) — `schema.rs` is at the 900 lines the gate
/// refuses at.
///
/// **`when_refused` defaults to `per_criterion`**, which is what a Job written
/// before this setting existed reads as: each criterion decides for itself,
/// off its own declaration.
///
/// **`job_judge_questions` has `job_id` alone as its key.** Only one criterion
/// is ever asked about at a time — `fleet::judging::looks::JudgeFold` asks
/// about the first ask-eligible refusal a pass produces — so a second row for
/// the same Job would mean this crate's caller lost track of the first
/// question rather than a real second one existing.
///
/// **`repository_judge_tolerances` has no `job_id` at all.** A store is opened
/// once per repository, so `criterion_id` alone is the key, and the fact
/// outlives whichever Job recorded it — read by every Job's gate from here on,
/// per `docs/concepts/judge.md`'s asking design.
pub(crate) const V52: &str = r#"
ALTER TABLE jobs ADD COLUMN when_refused TEXT NOT NULL DEFAULT 'per_criterion';

CREATE TABLE job_judge_questions (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    criterion_id TEXT NOT NULL,
    question    TEXT NOT NULL,
    expected    TEXT NOT NULL,
    produced    TEXT NOT NULL,
    consequence TEXT NOT NULL,
    asked_at    TEXT NOT NULL,
    PRIMARY KEY (job_id)
) STRICT;

CREATE TABLE repository_judge_tolerances (
    criterion_id TEXT NOT NULL PRIMARY KEY,
    recorded_at  TEXT NOT NULL,
    "by"         TEXT NOT NULL
) STRICT;
"#;

/// A question held open on a Job, exactly as [`Store::open_judge_question`]
/// read it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenJudgeQuestion {
    pub step_id: String,
    pub criterion_id: CriterionId,
    pub question: String,
    pub expected: String,
    pub produced: String,
    pub consequence: String,
    pub asked_at: Timestamp,
}

impl Store {
    /// How this Job meets a Judge criterion that refuses.
    pub fn when_refused(&self, job_id: &JobId) -> Result<WhenRefused, LoadJobError> {
        let held: String = self
            .conn
            .query_row(
                "SELECT when_refused FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => LoadJobError::NoSuchJob {
                    job_id: job_id.clone(),
                },
                other => LoadJobError::Unreadable(column("jobs", "when_refused")(other)),
            })?;
        enum_value(WhenRefused::from_wire, "jobs", "when_refused", &held)
            .map_err(LoadJobError::Unreadable)
    }

    /// Change how this Job meets a Judge criterion that refuses. The next
    /// gate reads the column, so nothing has to respawn for it to hold.
    pub fn set_when_refused(
        &mut self,
        job_id: &JobId,
        setting: WhenRefused,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET when_refused = ?2 WHERE job_id = ?1",
                (job_id.as_str(), setting.as_wire()),
            )
            .map_err(fault("recording how a job meets a judge refusal"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// The question held open on this Job, where a person is being asked one.
    /// `None` where nothing is open — which is every Job most of the time.
    pub fn open_judge_question(
        &self,
        job_id: &JobId,
    ) -> Result<Option<OpenJudgeQuestion>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT step_id, criterion_id, question, expected, produced, consequence, \
                 asked_at FROM job_judge_questions WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    Ok(OpenJudgeQuestion {
                        step_id: row.get(0)?,
                        criterion_id: CriterionId::new(row.get::<_, String>(1)?),
                        question: row.get(2)?,
                        expected: row.get(3)?,
                        produced: row.get(4)?,
                        consequence: row.get(5)?,
                        asked_at: Timestamp::from_rfc3339(&row.get::<_, String>(6)?),
                    })
                },
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadJobError::Unreadable(RowError::Database(fault(
                    "reading the question held open on a job",
                )(
                    other
                )))),
            })
    }

    /// Open a question on this Job. **Replaces whatever was open before**,
    /// which is never reached in practice — `fleet::judging::looks::JudgeFold`
    /// asks about one criterion per pass, and a step holds at the gate until
    /// its question is answered.
    ///
    /// **`question`, `expected`, `produced` and `consequence` are handed in
    /// rather than read off `judgment` here.** The last three are on
    /// [`Judgment`] and could be read off it; `question` is not — neither it
    /// nor its citations carry the plain text a criterion asked, which
    /// `fleet::judging::looks::question_text_of` resolves before this is
    /// called. Four strings in one signature rather than three off the type
    /// and one beside it would be the harder shape to read at the call site.
    #[allow(clippy::too_many_arguments)]
    pub fn record_judge_question(
        &mut self,
        job_id: &JobId,
        step_id: &str,
        criterion_id: &CriterionId,
        question: &str,
        expected: &str,
        produced: &str,
        consequence: &str,
        asked_at: &Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_judge_questions \
                 (job_id, step_id, criterion_id, question, expected, produced, consequence, asked_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT (job_id) DO UPDATE SET \
                 step_id = excluded.step_id, criterion_id = excluded.criterion_id, \
                 question = excluded.question, expected = excluded.expected, \
                 produced = excluded.produced, consequence = excluded.consequence, \
                 asked_at = excluded.asked_at",
                (
                    job_id.as_str(),
                    step_id,
                    criterion_id.as_str(),
                    question,
                    expected,
                    produced,
                    consequence,
                    asked_at.as_str(),
                ),
            )
            .map_err(fault("opening a judge question"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Clear the question this Job held open, once a person has answered it.
    /// `false` where none was open.
    pub fn clear_judge_question(&mut self, job_id: &JobId) -> Result<bool, WriteError> {
        let removed = self
            .conn
            .execute(
                "DELETE FROM job_judge_questions WHERE job_id = ?1",
                (job_id.as_str(),),
            )
            .map_err(fault("clearing a job's judge question"))
            .map_err(WriteError::Database)?;
        Ok(removed > 0)
    }

    /// Every criterion this repository has stood down with "always disagree".
    /// Read before every Judge call this file's own header describes.
    pub fn tolerated_criteria(&self) -> Result<Vec<CriterionId>, RowError> {
        let mut asking = self
            .conn
            .prepare("SELECT criterion_id FROM repository_judge_tolerances")
            .map_err(fault("reading this repository's judge tolerances"))
            .map_err(RowError::Database)?;
        let rows = asking
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(fault("reading this repository's judge tolerances"))
            .map_err(RowError::Database)?;
        let mut tolerated = Vec::new();
        for row in rows {
            let named = row
                .map_err(fault("reading this repository's judge tolerances"))
                .map_err(RowError::Database)?;
            tolerated.push(CriterionId::new(named));
        }
        Ok(tolerated)
    }

    /// Stand this criterion down for the repository: every later Job's gate
    /// treats a refusal on it as already disagreed with, and never asks about
    /// it again. **Idempotent** — a criterion stood down twice records the
    /// first time it happened.
    pub fn record_tolerance(
        &mut self,
        criterion_id: &CriterionId,
        recorded_at: &Timestamp,
        by: Actor,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO repository_judge_tolerances (criterion_id, recorded_at, \"by\") \
                 VALUES (?1, ?2, ?3) ON CONFLICT (criterion_id) DO NOTHING",
                (criterion_id.as_str(), recorded_at.as_str(), by.as_wire()),
            )
            .map_err(fault("standing down a judge criterion for this repository"))
            .map_err(WriteError::Database)?;
        Ok(())
    }
}
