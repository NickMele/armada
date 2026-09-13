//! A person's choice of model for the step that writes a Job's review. #903.
//!
//! Beside [`crate::model_override`] and on its terms: a column on the Job's row, read at
//! the one moment it matters, a spawn. **Any text is kept**, for that module's reason.

use core_model::JobId;

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 65 — the model a person chose for a Job's review step.
///
/// **Nullable, and no `DEFAULT`**: `NULL` is nobody having chosen, and the review step
/// runs on the Job's chosen model, or its own.
pub(crate) const V65: &str = r#"
ALTER TABLE jobs ADD COLUMN review_model_override TEXT;
"#;

impl Store {
    /// The model a person chose for this Job's review step. `None` is nobody having chosen.
    pub fn review_model_override(&self, job_id: &JobId) -> Result<Option<String>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT review_model_override FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => LoadJobError::NoSuchJob {
                    job_id: job_id.clone(),
                },
                other => {
                    LoadJobError::Database(fault("reading a job's chosen review model")(other))
                }
            })
    }

    /// Set what [`review_model_override`](Store::review_model_override) answers, or clear
    /// it with `None`.
    pub fn set_review_model_override(
        &mut self,
        job_id: &JobId,
        model: Option<&str>,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET review_model_override = ?2 WHERE job_id = ?1",
                (job_id.as_str(), model),
            )
            .map_err(fault("recording a job's chosen review model"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }
}
