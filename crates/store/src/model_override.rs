//! A person's choice of model for a Job's later steps.
//!
//! **Off the [`Job`](core_model::Job) row's own fields, like
//! [`crate::manifest_snapshot`]**: no rule in `core-model` reads it, and Fleet
//! asks for it at the one moment it matters, a spawn. **No event**, for
//! [`crate::allowing`]'s reason: a choice moves no status and no step, so the
//! column is the authority for its own field.
//!
//! **Any text is kept.** Which names a person may choose is Fleet's list, and
//! this crate does not hold it.

use core_model::JobId;

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 49 — the model a person chose for a Job's later steps.
///
/// **Nullable, and no `DEFAULT`**: `NULL` is nobody having chosen, which is
/// every Job written before this and most written after it.
pub(crate) const V49: &str = r#"
ALTER TABLE jobs ADD COLUMN model_override TEXT;
"#;

impl Store {
    /// The model a person chose for this Job's later steps. `None` is nobody
    /// having chosen, so each step runs as its own or the Job's.
    pub fn model_override(&self, job_id: &JobId) -> Result<Option<String>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT model_override FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => LoadJobError::NoSuchJob {
                    job_id: job_id.clone(),
                },
                other => LoadJobError::Database(fault("reading a job's chosen model")(other)),
            })
    }

    /// Set what [`model_override`](Store::model_override) answers, or clear it
    /// with `None`. The next spawn reads the column, so the Drone already
    /// running keeps the model it was started as.
    pub fn set_model_override(
        &mut self,
        job_id: &JobId,
        model: Option<&str>,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET model_override = ?2 WHERE job_id = ?1",
                (job_id.as_str(), model),
            )
            .map_err(fault("recording a job's chosen model"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }
}
