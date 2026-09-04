//! Appending to a Job's scope history after the Job was inserted.
//!
//! **The scope moves with the entry or neither moves**, in one transaction. A
//! history saying a widening took, beside a `job_write_targets` that did not,
//! is a scope the drift check would measure against and the record would
//! contradict.
//!
//! **It writes the whole list**, because `scope_revisions[]` is one TEXT column
//! and there is no row to append — the list is the one the caller already holds,
//! folded by `Job::scope_revised`. So the write is last-writer-wins, and what
//! stops that mattering is upstream: one asker, in a Job's working slot, under
//! that slot's lock, at most once per step. A second asker needs the column
//! read back inside this transaction, and nothing here pretends it already is.
//!
//! It updates two things and no others: a title, a status or a workflow has no
//! path through here, so [`insert_job`](Store::insert_job)'s rule that creation
//! is not an update is unbroken.

use core_model::{Job, WriteTargets};

use crate::columns;
use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Write a Job's scope history and the scope it now carries.
    ///
    /// **Takes the Job rather than the revision**, so what is stored is what
    /// `Job::scope_revised` produced and not this module's own reading of what
    /// a revision should do to a scope. Two derivations of "which paths does
    /// this Job write" is two answers, and the one that is wrong is whichever
    /// a reader did not open.
    ///
    /// The Job must already be inserted: this updates, and a Job that is not
    /// there answers [`WriteError::NoSuchJob`] rather than creating one.
    pub fn record_scope_revision(&mut self, job: &Job) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the scope revision"))
            .map_err(WriteError::Database)?;

        let updated = tx
            .execute(
                "UPDATE jobs SET scope_revisions = ?2, write_targets_known = ?3 WHERE job_id = ?1",
                rusqlite::params![
                    job.id().as_str(),
                    columns::write_scope_revisions(job.scope_revisions()),
                    job.write_targets().is_some(),
                ],
            )
            .map_err(fault("writing the scope history"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job.id().clone(),
            });
        }

        // **Replaced rather than added to.** The rows carry an ordinal and the
        // Job carries the order, so appending here would file the new paths
        // under ordinals a reordering had already used. What is authoritative
        // is the list on the Job.
        tx.execute(
            "DELETE FROM job_write_targets WHERE job_id = ?1",
            (job.id().as_str(),),
        )
        .map_err(fault("clearing the previous write targets"))
        .map_err(WriteError::Database)?;
        if let Some(targets) = job.write_targets() {
            for (ordinal, path) in WriteTargets::paths(targets).iter().enumerate() {
                tx.execute(
                    "INSERT INTO job_write_targets (job_id, ordinal, path) VALUES (?1, ?2, ?3)",
                    rusqlite::params![job.id().as_str(), ordinal as i64, path.as_str()],
                )
                .map_err(fault("writing a widened write target"))
                .map_err(WriteError::Database)?;
            }
        }

        tx.commit()
            .map_err(fault("committing the scope revision"))
            .map_err(WriteError::Database)
    }
}
