//! The writes, and the fact that each writes one thing.
//!
//! # There is no method here that sets a status, or a step state
//!
//! [`Store::record_transition`] takes a [`Transitioned`], whose
//! [`JobEvent`](core_model::JobEvent) has no constructor outside
//! `Job::transition`; [`Store::record_step_transition`] takes a
//! [`StepTransitioned`], which only `Job::transition_step` mints, and writes
//! the `job_steps` row, the Job's cursor and the log entry together. So "the
//! column and the event row always agree" is not a rule this crate follows
//! carefully; it is the only call that exists.
//!
//! The alternatives were an `update_status` plus an `append_event`, which makes
//! forgetting the second the easy path, or one function taking both halves
//! separately, which makes passing mismatched halves possible. Both are runtime
//! checks waiting to be written.
//!
//! # One transaction, and no clock
//!
//! `job-fields.toml` requires the row and its log entry to land together: the
//! column is a cache of the fold, and a cache written outside the transaction
//! that wrote the log can outlive a crash on its own. Nothing here reads a
//! clock either — a store that timestamped its own writes could not be replayed
//! and could not be tested.
use core_model::{
    Attachment, DroneAssigned, DronePresence, GateManifest, Job, JobId, JobStep, Judgment,
    StepCheck, StepEvidence, StepId, StepLevelTrigger, StepTransitioned, Timestamp,
    TransitionReason, Transitioned, WriteTargets,
};
use rusqlite::Transaction;

use crate::attempt::attempt_now;
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
                     write_targets_known, created_at, branch, workflow
                 ) VALUES (
                     ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                     ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
                 )",
                rusqlite::params![
                    job.id().as_str(),
                    // Bound on every write, so the `NOT NULL DEFAULT ''` the
                    // `ALTER` in V2 had to carry is never the value used.
                    job.title().as_str(),
                    job.status().as_wire(),
                    // Derived from the frozen workflow rather than stored
                    // beside it, so the join key and the declaration cannot
                    // come to disagree.
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
                    // Ordinarily null: a Job is created before it has a
                    // worktree. Bound anyway, so a Job rebuilt and reinserted
                    // does not lose it.
                    job.branch().map(|branch| branch.as_str()),
                    columns::write_workflow(job.workflow()),
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
        write_attachments(&tx, job, created_at)?;

        tx.commit()
            .map_err(fault("committing the insert"))
            .map_err(WriteError::Database)
    }

    /// Record the branch a Job's worktree was made on, and nothing else.
    ///
    /// **The one column with no event behind it.** A worktree is not a
    /// transition — dispatch makes one between `queued -> running` and the step
    /// move that follows — so there is no `Transitioned` to carry it and this
    /// writes the column directly. Everything the log is authoritative for
    /// still goes through the two methods that mint an event.
    pub fn record_branch(&mut self, job: &Job) -> Result<(), WriteError> {
        let Some(branch) = job.branch() else {
            return Ok(());
        };
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET branch = ?2 WHERE job_id = ?1",
                (job.id().as_str(), branch.as_str()),
            )
            .map_err(fault("recording the branch"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job.id().clone(),
            });
        }
        Ok(())
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
                 kind, job_id, status_from, status_to, reason_kind, reason_value, actor, at
             ) VALUES ('job_transition', ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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

    /// Record a step move: the `job_steps` row, the cursor it moved, and the
    /// log entry all three cache.
    ///
    /// Returns the key the store assigned, from the same sequence a status
    /// transition draws from — which is the point of one log. A step move
    /// written to a table of its own could not be ordered against the status
    /// transitions around it, and the fold needs that order to know which
    /// status the Job stood in when the step moved.
    ///
    /// The row's `status_from` and `status_to` are both the status the move
    /// happened beneath. The Job did not move; saying so is what makes the fold
    /// able to check the claim rather than believe it.
    pub fn record_step_transition(&mut self, moved: &StepTransitioned) -> Result<i64, WriteError> {
        let event = &moved.event;
        let job_id = event.job_id().as_str();
        let step_id = event.step_id().as_str();
        let step = moved.job.step(event.step_id()).ok_or_else(|| {
            // Unreachable through `transition_step`, which refuses a step the
            // Job does not have. Named rather than unwrapped, because the value
            // arrives from outside this crate.
            WriteError::NoSuchStep {
                job_id: event.job_id().clone(),
                step_id: event.step_id().clone(),
            }
        })?;

        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the step transition"))
            .map_err(WriteError::Database)?;

        let updated = tx
            .execute(
                "UPDATE job_steps SET state = ?3, last_verdict = ?4, entered_at = ?5,
                 updated_at = ?6 WHERE job_id = ?1 AND step_id = ?2",
                rusqlite::params![
                    job_id,
                    step_id,
                    step.state().as_wire(),
                    verdict(step),
                    step.entered_at().as_str(),
                    step.updated_at().as_str(),
                ],
            )
            .map_err(fault("updating the cached step state"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchStep {
                job_id: event.job_id().clone(),
                step_id: event.step_id().clone(),
            });
        }

        // The cursor the move left, whether or not this move changed it. It is
        // read off the Job rather than computed here: which moves it is the
        // inner machine's rule, and a second copy of that rule in SQL is the
        // second vocabulary this codebase keeps refusing.
        tx.execute(
            "UPDATE jobs SET current_step_id = ?2 WHERE job_id = ?1",
            rusqlite::params![job_id, moved.job.current_step_id().map(|id| id.as_str())],
        )
        .map_err(fault("updating the cached cursor"))
        .map_err(WriteError::Database)?;

        let (reason_kind, reason_value) = columns::write_reason(&stop_reason(event.why()));
        tx.execute(
            "INSERT INTO job_events (
                 kind, job_id, status_from, status_to, reason_kind, reason_value,
                 step_id, state_from, state_to, actor, at
             ) VALUES ('step_transition', ?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                job_id,
                event.under().as_wire(),
                reason_kind,
                reason_value,
                step_id,
                event.from().as_wire(),
                event.to().as_wire(),
                event.actor().as_wire(),
                event.at().as_str(),
            ],
        )
        .map_err(fault("appending the step event"))
        .map_err(WriteError::Database)?;

        let seq = tx.last_insert_rowid();
        tx.commit()
            .map_err(fault("committing the step transition"))
            .map_err(WriteError::Database)?;
        Ok(seq)
    }

    /// Record a Drone arriving on a Job, or leaving it: the column, and the
    /// log entry it caches.
    ///
    /// Takes a [`DroneAssigned`], which only `Job::drone_spawned` and
    /// `Job::drone_exited` mint — the property [`record_transition`] has, for
    /// the same reason. `assigned_drone` and its event cannot disagree because
    /// there is no call that writes one without the other.
    ///
    /// [`record_transition`]: Store::record_transition
    pub fn record_drone_move(&mut self, moved: &DroneAssigned) -> Result<i64, WriteError> {
        let event = &moved.event;
        let job_id = event.job_id().as_str();

        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the drone move"))
            .map_err(WriteError::Database)?;

        let updated = tx
            .execute(
                "UPDATE jobs SET assigned_drone = ?2 WHERE job_id = ?1",
                rusqlite::params![job_id, moved.job.assigned_drone().map(|id| id.as_str())],
            )
            .map_err(fault("updating the assigned drone"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: event.job_id().clone(),
            });
        }

        tx.execute(
            "INSERT INTO job_events (
                 kind, job_id, status_from, status_to, reason_kind, reason_value,
                 drone_id, actor, at
             ) VALUES (?1, ?2, ?3, ?3, 'unqualified', NULL, ?4, ?5, ?6)",
            rusqlite::params![
                kind_of(event.presence()),
                job_id,
                event.under().as_wire(),
                event.drone_id().as_str(),
                event.actor().as_wire(),
                event.at().as_str(),
            ],
        )
        .map_err(fault("appending the drone event"))
        .map_err(WriteError::Database)?;

        let seq = tx.last_insert_rowid();
        tx.commit()
            .map_err(fault("committing the drone move"))
            .map_err(WriteError::Database)?;
        Ok(seq)
    }

    /// Record what each of a step's declared Checks did.
    ///
    /// **No event, like [`record_branch`](Store::record_branch).** A Check
    /// running is not a transition; it is the evidence one was derived from,
    /// and the transition it led to is already a row in the log.
    ///
    /// **This run's** rows are replaced whole rather than appended to, and an
    /// earlier run's are left standing. A second ruling inside one run
    /// supersedes the first, because a mixture of the two would be a set of
    /// results no single ruling ever produced; a second *run* of the step is a
    /// different question and is kept — see [`attempt`](crate::attempt).
    ///
    /// The run is [derived from the log](Store::step_attempt) inside this
    /// transaction rather than passed in, so a caller cannot file rows under a
    /// run the history does not have.
    pub fn record_step_checks(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        checks: &[StepCheck],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the check record"))
            .map_err(WriteError::Database)?;
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;

        tx.execute(
            "DELETE FROM job_step_checks WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3",
            (job_id.as_str(), step_id.as_str(), attempt.number()),
        )
        .map_err(fault("clearing this run's previous rows"))
        .map_err(WriteError::Database)?;

        for (ordinal, check) in checks.iter().enumerate() {
            tx.execute(
                "INSERT INTO job_step_checks (
                     job_id, step_id, attempt, ordinal, name, outcome, expected, produced,
                     ran_at, output_path
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    job_id.as_str(),
                    step_id.as_str(),
                    attempt.number(),
                    ordinal as i64,
                    check.name.as_str(),
                    check.outcome.as_wire(),
                    check.expected.as_deref(),
                    check.produced.as_deref(),
                    at.as_str(),
                    check.output_path.as_deref(),
                ],
            )
            .map_err(fault("writing a check result"))
            .map_err(WriteError::Database)?;
        }

        tx.commit()
            .map_err(fault("committing the check record"))
            .map_err(WriteError::Database)
    }

    /// Record what the Judge said about one run of one step, replacing whatever
    /// a previous pass **over that same run** wrote.
    ///
    /// Same shape as [`record_step_checks`](Store::record_step_checks) and for
    /// the same reason: a step is judged afresh each time it is submitted, and
    /// two passes' judgments interleaved would read as a panel. What changed is
    /// the scope of "afresh" — it is the run and no longer the step, so the
    /// verdicts a second run produces sit beside the first run's rather than on
    /// top of them. That is the whole of what `docs/concepts/workflow.md` means
    /// by carrying every prior verdict.
    pub fn record_step_judgments(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        judgments: &[Judgment],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the judgment record"))
            .map_err(WriteError::Database)?;
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;

        tx.execute(
            "DELETE FROM job_step_judgments WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3",
            (job_id.as_str(), step_id.as_str(), attempt.number()),
        )
        .map_err(fault("clearing this run's previous pass"))
        .map_err(WriteError::Database)?;

        for (ordinal, judgment) in judgments.iter().enumerate() {
            tx.execute(
                "INSERT INTO job_step_judgments (
                     job_id, step_id, attempt, ordinal, criterion, verdict, expected,
                     produced, consequence, judged_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    job_id.as_str(),
                    step_id.as_str(),
                    attempt.number(),
                    ordinal as i64,
                    judgment.criterion_id.as_str(),
                    judgment.verdict.as_wire(),
                    judgment.expected.as_deref(),
                    judgment.produced.as_deref(),
                    judgment.consequence.as_deref(),
                    at.as_str(),
                ],
            )
            .map_err(fault("writing a judgment"))
            .map_err(WriteError::Database)?;
        }

        tx.commit()
            .map_err(fault("committing the judgment record"))
            .map_err(WriteError::Database)
    }

    /// Record the evidence a step's gate accepted, replacing whatever an
    /// earlier submission **within the same run** wrote.
    ///
    /// One row per run, because a superseded submission is not a baseline: a
    /// later step's gaming check is judged against what the earlier step
    /// finally established, not against a draft of it. An earlier run's row
    /// stands, which is the half that was missing — the work product carried
    /// forward is still the latest, and the record of what each round actually
    /// submitted is what makes "the same note again" visible at all.
    ///
    /// A transaction where there was none, because the run has to be counted
    /// and the row written under one view of the log.
    pub fn record_step_evidence(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        evidence: &StepEvidence,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the evidence record"))
            .map_err(WriteError::Database)?;
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;

        tx.execute(
            "INSERT INTO job_step_evidence (
                 job_id, step_id, attempt, evidence_type, claimed, shown_by, not_claimed,
                 recorded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (job_id, step_id, attempt) DO UPDATE SET
                 evidence_type = excluded.evidence_type,
                 claimed       = excluded.claimed,
                 shown_by      = excluded.shown_by,
                 not_claimed   = excluded.not_claimed,
                 recorded_at   = excluded.recorded_at",
            rusqlite::params![
                job_id.as_str(),
                step_id.as_str(),
                attempt.number(),
                evidence.evidence_type.as_wire(),
                evidence.claimed,
                evidence.shown_by,
                evidence.not_claimed,
                at.as_str(),
            ],
        )
        .map_err(fault("writing a step's evidence"))
        .map_err(WriteError::Database)?;

        tx.commit()
            .map_err(fault("committing the evidence record"))
            .map_err(WriteError::Database)
    }
}

/// The `kind` column's value for a drone row. Read off the domain enum rather
/// than spelled here, so the schema's trigger and the fold agree by
/// construction.
fn kind_of(presence: DronePresence) -> &'static str {
    presence.as_wire()
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

/// `None` until a gate has ruled on the step. Advancing or stopping writes one.
fn verdict(step: &JobStep) -> Option<&'static str> {
    step.last_verdict().map(|verdict| verdict.as_wire())
}

/// A step stop's trigger, in the same two columns a status transition uses.
/// One `reason_kind` vocabulary across both machines rather than a second
/// spelling for the inner one.
fn stop_reason(why: Option<StepLevelTrigger>) -> TransitionReason {
    match why {
        Some(why) => TransitionReason::Escalation(why.trigger()),
        None => TransitionReason::Unqualified,
    }
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

/// One row per file handed to the Job at proposal time. Unordered, unlike
/// `job_write_targets`: a Drone opens an attachment by name, not by position,
/// so there is no `ordinal` to keep faith with.
fn write_attachments(
    tx: &Transaction<'_>,
    job: &Job,
    created_at: &Timestamp,
) -> Result<(), WriteError> {
    for attachment in job.attachments() {
        let Attachment {
            filename,
            mime_type,
            byte_size,
            storage_ref,
        } = attachment;
        tx.execute(
            "INSERT INTO job_attachments (
                 job_id, filename, mime_type, byte_size, storage_ref, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                job.id().as_str(),
                filename.as_str(),
                mime_type.as_str(),
                *byte_size as i64,
                storage_ref.as_str(),
                created_at.as_str(),
            ],
        )
        .map_err(fault("writing an attachment"))
        .map_err(WriteError::Database)?;
    }
    Ok(())
}
