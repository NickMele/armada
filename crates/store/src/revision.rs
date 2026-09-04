//! Appending to a Job's scope history after the Job was inserted.
//!
//! # The one column with two writers
//!
//! [`insert_job`](Store::insert_job) says creation is not an update and that
//! there is no method there that would make it one — true of every column it
//! writes, and it stays true: this does not update the Job row's other columns,
//! and there is no path through it by which a title, a status or a workflow
//! could move. What it writes is `scope_revisions` and the paths that entry
//! took, which is one act.
//!
//! # The whole list, because the column is the whole list
//!
//! `scope_revisions[]` is `storage = "Column"` in
//! `crates/core-model/domain/job-fields.toml`, so the history is one TEXT
//! value and there is no row to append. What is written is the list the caller
//! already holds, folded by `Job::scope_revised` — this does not re-read the
//! column and does not decide what the list should contain.
//!
//! **So the write is last-writer-wins, and what stops that mattering is
//! upstream.** A revision is asked for by the Drone in a Job's working slot,
//! under that slot's lock, and at most once per step; two overlapping folds of
//! one Job's history are a thing no caller can produce. A second asker would
//! need this to read the column back inside the transaction, and that is the
//! change to make rather than a lock held out here.
//!
//! # The scope moves with the entry, or neither moves
//!
//! `job_write_targets` is replaced from the Job the caller folded the entry
//! into, in the same transaction as the history. A history saying a widening
//! took, beside a scope that did not take it, is the disagreement the whole
//! feature exists to make impossible — the drift check would then measure
//! against a scope the record says was corrected.

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
