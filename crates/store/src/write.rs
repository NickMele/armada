//! The two writes, and the fact that there are only two.
//!
//! # There is no method here that sets a status
//!
//! [`Store::record_transition`] takes a [`Transitioned`], and a `Transitioned`
//! holds a [`JobEvent`](core_model::JobEvent) whose only constructor is
//! `pub(crate)` inside `core-model`. **Nothing outside `Job::transition` can
//! produce one.** So "the status column and the event row always agree" is not
//! a rule this crate follows carefully; it is the only call that exists.
//!
//! The alternatives were an `update_status` plus an `append_event`, which makes
//! forgetting the second the easy path, or one function taking both halves
//! separately, which makes passing mismatched halves possible. Both are runtime
//! checks waiting to be written. This is neither.
//!
//! # One transaction, both rows
//!
//! `job-fields.toml` requires it: the column is a cache of the fold, and a
//! cache written outside the transaction that wrote the log is a cache that can
//! outlive a crash on its own.
//!
//! # Time is injected
//!
//! Nothing here reads a clock. The transition's instant arrives on the event,
//! stamped by whoever called `Job::transition`; creation's arrives as an
//! argument. A store that timestamped its own writes could not be replayed and
//! could not be tested.

use core_model::{GateManifest, Job, JobStep, Timestamp, Transitioned, WriteTargets};
use rusqlite::Transaction;

use crate::columns;
use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Store a Job as created. **Not an upsert** — a second call with the same
    /// id is [`WriteError::JobAlreadyExists`], because creation is not an
    /// update and there is no method here that would make it one.
    ///
    /// `created_at` is the instant passed to the constructor that made the Job.
    /// The rebuild calls that same constructor and it stamps the `job_steps`
    /// rows, so the instant has to be recorded — no event describes creation.
    pub fn insert_job(&mut self, job: &Job, created_at: &Timestamp) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the insert"))
            .map_err(WriteError::Database)?;

        let inserted = tx
            .execute(
                "INSERT OR IGNORE INTO jobs (
                     job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
                     atomic, model, acceptance_criteria, current_step_id, assigned_drone,
                     dependencies, dispatched_by_job_id, dispatched_by_step_id,
                     redispatched_from, subject_kind, subject_ref, facts, scope_revisions,
                     write_targets_known, created_at
                 ) VALUES (
                     ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                     ?16, ?17, ?18, ?19, ?20, ?21, ?22
                 )",
                rusqlite::params![
                    job.id().as_str(),
                    // Bound on every write, so the `NOT NULL DEFAULT ''` the
                    // `ALTER` in V2 had to carry is never the value used.
                    job.title().as_str(),
                    job.status().as_wire(),
                    job.workflow_id().as_str(),
                    job.owner_manifest_id().as_str(),
                    job.origin().as_wire(),
                    job.urgency().as_wire(),
                    job.atomic(),
                    job.model().as_str(),
                    columns::write_acceptance_criteria(job.acceptance_criteria()),
                    job.current_step_id().map(|id| id.as_str()),
                    job.assigned_drone().map(|id| id.as_str()),
                    columns::write_dependencies(job.dependencies()),
                    job.dispatched_by().map(|by| by.job_id.as_str()),
                    job.dispatched_by().map(|by| by.step_id.as_str()),
                    job.redispatched_from().map(|id| id.as_str()),
                    job.subject().map(|subject| subject.kind.as_str()),
                    job.subject().map(|subject| subject.reference.as_str()),
                    job.facts().as_str(),
                    columns::write_scope_revisions(job.scope_revisions()),
                    job.write_targets().is_some(),
                    created_at.as_str(),
                ],
            )
            .map_err(fault("writing the job row"))
            .map_err(WriteError::Database)?;
        if inserted == 0 {
            return Err(WriteError::JobAlreadyExists {
                job_id: job.id().clone(),
            });
        }

        write_steps(&tx, job)?;
        write_targets(&tx, job)?;
        write_manifests(&tx, job)?;

        tx.commit()
            .map_err(fault("committing the insert"))
            .map_err(WriteError::Database)
    }

    /// Record a transition: the event row, and the status column it caches.
    ///
    /// Returns the key the store assigned. `JobEvent` carries no id of its own
    /// — `core-model` mints nothing — and the key is a monotonic sequence
    /// rather than a stamp, because the fold has to order events and the
    /// timestamps on them are injected.
    ///
    /// Only `status` is written back to the Job row. `Job::transition` changes
    /// that field and no other, so writing the rest would be this crate
    /// deciding that a value it was handed supersedes what is stored.
    pub fn record_transition(&mut self, transitioned: &Transitioned) -> Result<i64, WriteError> {
        let event = &transitioned.event;
        let job_id = event.job_id().as_str();
        let (reason_kind, reason_value) = columns::write_reason(event.reason());

        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the transition"))
            .map_err(WriteError::Database)?;

        let updated = tx
            .execute(
                "UPDATE jobs SET status = ?2 WHERE job_id = ?1",
                (job_id, event.to().as_wire()),
            )
            .map_err(fault("updating the cached status"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: event.job_id().clone(),
            });
        }

        tx.execute(
            "INSERT INTO job_events (
                 job_id, status_from, status_to, reason_kind, reason_value, actor, at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                job_id,
                event.from().as_wire(),
                event.to().as_wire(),
                reason_kind,
                reason_value,
                event.actor().as_wire(),
                event.at().as_str(),
            ],
        )
        .map_err(fault("appending the event"))
        .map_err(WriteError::Database)?;

        let seq = tx.last_insert_rowid();
        tx.commit()
            .map_err(fault("committing the transition"))
            .map_err(WriteError::Database)?;
        Ok(seq)
    }
}

/// One row per step of the frozen WorkflowDef, as the Job holds them.
fn write_steps(tx: &Transaction<'_>, job: &Job) -> Result<(), WriteError> {
    for step in job.steps() {
        tx.execute(
            "INSERT INTO job_steps (
                 job_id, step_id, ordinal, state, last_verdict, entered_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                job.id().as_str(),
                step.step_id().as_str(),
                step.ordinal(),
                step.state().as_wire(),
                verdict(step),
                step.entered_at().as_str(),
                step.updated_at().as_str(),
            ],
        )
        .map_err(fault("writing a step row"))
        .map_err(WriteError::Database)?;
    }
    Ok(())
}

/// `None` until a gate has ruled on the step, which nothing does yet.
fn verdict(step: &JobStep) -> Option<&'static str> {
    step.last_verdict().map(|verdict| verdict.as_wire())
}

/// One row per declared path. Zero rows is ambiguous on its own — null is not
/// empty — so `jobs.write_targets_known` carries which of the two this is.
fn write_targets(tx: &Transaction<'_>, job: &Job) -> Result<(), WriteError> {
    let Some(targets) = job.write_targets() else {
        return Ok(());
    };
    for (ordinal, path) in WriteTargets::paths(targets).iter().enumerate() {
        tx.execute(
            "INSERT INTO job_write_targets (job_id, ordinal, path) VALUES (?1, ?2, ?3)",
            rusqlite::params![job.id().as_str(), ordinal as i64, path.as_str()],
        )
        .map_err(fault("writing a write target"))
        .map_err(WriteError::Database)?;
    }
    Ok(())
}

/// The gating Manifests, in the order the Job holds them. Keyed by ordinal
/// rather than by Manifest so that the order survives, which a set would lose.
fn write_manifests(tx: &Transaction<'_>, job: &Job) -> Result<(), WriteError> {
    for (ordinal, gate) in job.gate_manifests().iter().enumerate() {
        let GateManifest {
            manifest_id,
            outcome,
        } = gate;
        let (outcome, not_run_reason) = outcome.as_wire();
        tx.execute(
            "INSERT INTO job_manifests (job_id, ordinal, manifest_id, outcome, not_run_reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                job.id().as_str(),
                ordinal as i64,
                manifest_id.as_str(),
                outcome,
                not_run_reason,
            ],
        )
        .map_err(fault("writing a gate manifest"))
        .map_err(WriteError::Database)?;
    }
    Ok(())
}
