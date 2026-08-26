//! Reading Jobs back — which means folding them, not selecting them.
//!
//! # `status` is a cache and this is where the log wins
//!
//! [`Store::load_all_jobs`] is the boot read. It rebuilds every Job from its
//! events, and where the cached column disagrees with the fold it **writes the
//! fold back** and reports the repair. `job-fields.toml` prescribes exactly
//! that: "Fleet re-folds non-terminal Jobs at boot and lets the log win."
//!
//! It takes `&mut self` for that reason, which is the signature saying out loud
//! that the boot read may write.
//!
//! # What the log is authoritative for, and what it is not
//!
//! Only `status`. `job_events` records transitions, and a transition is the
//! only thing that moves. Every other column on the Job row is the authority
//! for its own field, because nothing in the log describes it — which is the
//! cost of caching the fold rather than deriving everything from it. If a
//! later step gives Fleet a writer for `assigned_drone` or `current_step_id`,
//! those moves need events of their own or they will not survive a restart the
//! way status does. **They are checked, not assumed:** a row holding either one
//! is refused rather than quietly dropped, because the rebuild has no way to
//! put it back.

use core_model::{
    Actor, DispatchOrigin, Facts, GateManifest, GateOutcome, Job, JobId, JobStatus, ManifestId,
    ModelName, NewJob, Origin, RepoPath, StepId, StepSeed, Subject, Timestamp, Title, Ulid,
    Urgency, WorkflowId, WriteTargets,
};
use rusqlite::Row;

use crate::columns;
use crate::error::{fault, LoadAllError, LoadJobError, RowError};
use crate::fold::{replay, RecordedEvent};
use crate::open::Store;

/// What the boot read produced.
#[derive(Debug, Default)]
pub struct Loaded {
    /// Every Job, as its log says it is.
    pub jobs: Vec<Job>,
    /// Rows whose cached status did not match the fold, now corrected. Empty is
    /// the ordinary case; a non-empty list is a torn write worth a log line.
    pub repaired: Vec<StatusRepair>,
}

/// A Job as the log says it is, beside the status its row claimed. The pair
/// travels together so the boot read can tell a stale cache from a good one.
type Rebuilt = Result<(Job, JobStatus), RowError>;

/// A cached status column that disagreed with the log, and was corrected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusRepair {
    pub job_id: JobId,
    pub cached: JobStatus,
    pub folded: JobStatus,
}

impl Store {
    /// One Job, folded from its events. `status` on the row is not read.
    pub fn load_job(&self, job_id: &JobId) -> Result<Job, LoadJobError> {
        let found = self
            .conn
            .query_row(SELECT_JOB, (job_id.as_str(),), |row| Ok(self.rebuild(row)))
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => LoadJobError::NoSuchJob {
                    job_id: job_id.clone(),
                },
                other => LoadJobError::Database(fault("reading a job row")(other)),
            })?;
        found.map(|(job, _)| job).map_err(LoadJobError::Unreadable)
    }

    /// Every Job, each folded from its own events. **The boot read.**
    ///
    /// A Job that will not rebuild does not take the others down with it and is
    /// not dropped either: the error carries both halves, so a caller cannot
    /// end up holding a short list with nothing saying so.
    pub fn load_all_jobs(&mut self) -> Result<Loaded, LoadAllError> {
        let attempts = self.rebuild_every_job()?;
        let mut loaded = Loaded::default();
        let mut failed = Vec::new();
        for attempt in attempts {
            match attempt {
                Ok((job, cached)) => {
                    if cached != job.status() {
                        loaded.repaired.push(StatusRepair {
                            job_id: job.id().clone(),
                            cached,
                            folded: job.status(),
                        });
                    }
                    loaded.jobs.push(job);
                }
                Err(error) => failed.push(error),
            }
        }
        for repair in &loaded.repaired {
            self.conn
                .execute(
                    "UPDATE jobs SET status = ?2 WHERE job_id = ?1",
                    (repair.job_id.as_str(), repair.folded.as_wire()),
                )
                .map_err(fault("repairing a cached status"))
                .map_err(LoadAllError::Database)?;
        }
        if failed.is_empty() {
            Ok(loaded)
        } else {
            Err(LoadAllError::SomeJobsUnreadable { loaded, failed })
        }
    }

    /// One Job's history, oldest first. Never edited, never removed, and here
    /// so that something other than the fold can read it.
    pub fn events_for(&self, job_id: &JobId) -> Result<Vec<RecordedEvent>, RowError> {
        let mut statement = self
            .conn
            .prepare(SELECT_EVENTS)
            .map_err(fault("preparing the event read"))
            .map_err(RowError::Database)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| Ok(event(row)))
            .map_err(fault("reading events"))
            .map_err(RowError::Database)?;
        let mut events = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault("reading an event"))
                .map_err(RowError::Database)?;
            events.push(row?);
        }
        Ok(events)
    }

    fn rebuild_every_job(&self) -> Result<Vec<Rebuilt>, LoadAllError> {
        let mut statement = self
            .conn
            .prepare(SELECT_ALL_JOBS)
            .map_err(fault("preparing the job read"))
            .map_err(LoadAllError::Database)?;
        let rows = statement
            .query_map([], |row| Ok(self.rebuild(row)))
            .map_err(fault("reading job rows"))
            .map_err(LoadAllError::Database)?;
        let mut attempts = Vec::new();
        for row in rows {
            attempts.push(
                row.map_err(fault("reading a job row"))
                    .map_err(LoadAllError::Database)?,
            );
        }
        Ok(attempts)
    }

    /// The Job as its log says it is, and the status its row claimed.
    fn rebuild(&self, row: &Row<'_>) -> Rebuilt {
        let job_id = JobId::carried(Ulid::carried(string(row, "job_id")?));
        let cached = enum_value(
            JobStatus::from_wire,
            "jobs",
            "status",
            &string(row, "status")?,
        )?;
        let created_at = Timestamp::from_rfc3339(string(row, "created_at")?);

        refuse_if_set(&job_id, "current_step_id", maybe(row, "current_step_id")?)?;
        refuse_if_set(&job_id, "assigned_drone", maybe(row, "assigned_drone")?)?;

        let new = NewJob {
            id: job_id.clone(),
            // Refused rather than substituted. A blank here is a row the
            // triggers in V2 do not admit, so it arrived from outside this
            // crate — and naming the Job "Untitled" on the way past would hide
            // exactly that.
            title: Title::new(&string(row, "title")?).map_err(|blank| {
                RowError::MalformedColumn {
                    table: "jobs",
                    column: "title",
                    detail: blank.to_string(),
                }
            })?,
            workflow_id: WorkflowId::carried(Ulid::carried(string(row, "workflow_id")?)),
            owner_manifest_id: ManifestId::carried(Ulid::carried(string(
                row,
                "owner_manifest_id",
            )?)),
            urgency: enum_value(
                Urgency::from_wire,
                "jobs",
                "urgency",
                &string(row, "urgency")?,
            )?,
            atomic: row.get("atomic").map_err(column("jobs", "atomic"))?,
            model: ModelName::new(string(row, "model")?),
            acceptance_criteria: columns::read_acceptance_criteria(&string(
                row,
                "acceptance_criteria",
            )?)
            .map_err(malformed("acceptance_criteria"))?,
            steps: self.step_seeds(&job_id)?,
            dependencies: columns::read_dependencies(&string(row, "dependencies")?)
                .map_err(malformed("dependencies"))?,
            gate_manifests: self.gate_manifests(&job_id)?,
            write_targets: self.write_targets(&job_id, row)?,
            subject: subject(row)?,
            redispatched_from: maybe(row, "redispatched_from")?
                .map(|id| JobId::carried(Ulid::carried(id))),
            facts: Facts::new(string(row, "facts")?),
            scope_revisions: columns::read_scope_revisions(&string(row, "scope_revisions")?)
                .map_err(malformed("scope_revisions"))?,
        };

        let origin = enum_value(Origin::from_wire, "jobs", "origin", &string(row, "origin")?)?;
        let dispatched_by = dispatched_by(row)?;
        let created = match origin.top_level() {
            Some(top_level) => {
                // `dispatched_by` wins where the two disagree, and the
                // top-level constructor has nowhere to put one. Refused rather
                // than dropped.
                if let Some(by) = dispatched_by {
                    return Err(RowError::ColumnNotReconstructable {
                        job_id,
                        column: "dispatched_by",
                        value: by.job_id.as_str().to_string(),
                    });
                }
                Job::create_top_level(new, top_level, created_at)
            }
            None => {
                let by = dispatched_by.ok_or_else(|| RowError::SubDispatchedWithoutOrigin {
                    job_id: job_id.clone(),
                })?;
                Job::create_sub_dispatched(new, by, created_at)
            }
        };

        let events = self.events_for(&job_id)?;
        Ok((replay(created, &events)?, cached))
    }

    /// The `job_steps` rows, as the seeds creation took.
    ///
    /// Nothing advances a step yet, so the rebuild reproduces `not_started`
    /// with no verdict — and a row saying anything else is refused, because
    /// silently returning it as `not_started` is the loss this step exists to
    /// prevent.
    fn step_seeds(&self, job_id: &JobId) -> Result<Vec<StepSeed>, RowError> {
        self.collect(
            "SELECT step_id, ordinal, state, last_verdict FROM job_steps
             WHERE job_id = ?1 ORDER BY ordinal",
            job_id,
            "reading step rows",
            |row| {
                let step_id = StepId::new(string(row, "step_id")?);
                let state = string(row, "state")?;
                let verdict: Option<String> = maybe(row, "last_verdict")?;
                if state != "not_started" || verdict.is_some() {
                    return Err(RowError::StepStateNotReconstructable {
                        job_id: job_id.clone(),
                        step_id,
                        state: verdict.map_or(state.clone(), |v| format!("{state}/{v}")),
                    });
                }
                Ok(StepSeed {
                    step_id,
                    ordinal: row.get("ordinal").map_err(column("job_steps", "ordinal"))?,
                })
            },
        )
    }

    fn gate_manifests(&self, job_id: &JobId) -> Result<Vec<GateManifest>, RowError> {
        self.collect(
            "SELECT manifest_id, outcome, not_run_reason FROM job_manifests
             WHERE job_id = ?1 ORDER BY ordinal",
            job_id,
            "reading gate manifests",
            |row| {
                let outcome = string(row, "outcome")?;
                let reason: Option<String> = maybe(row, "not_run_reason")?;
                Ok(GateManifest {
                    manifest_id: ManifestId::carried(Ulid::carried(string(row, "manifest_id")?)),
                    outcome: GateOutcome::from_wire(&outcome, reason.as_deref()).ok_or(
                        RowError::UnknownEnumValue {
                            table: "job_manifests",
                            column: "outcome",
                            value: outcome,
                        },
                    )?,
                })
            },
        )
    }

    /// `None` is scope not yet determined and `Some` with no paths is
    /// determined to write nothing, which zero rows cannot tell apart —
    /// `write_targets_known` on the Job row is what does.
    fn write_targets(
        &self,
        job_id: &JobId,
        row: &Row<'_>,
    ) -> Result<Option<WriteTargets>, RowError> {
        let known: bool = row
            .get("write_targets_known")
            .map_err(column("jobs", "write_targets_known"))?;
        if !known {
            return Ok(None);
        }
        let paths = self.collect(
            "SELECT path FROM job_write_targets WHERE job_id = ?1 ORDER BY ordinal",
            job_id,
            "reading write targets",
            |row| Ok(RepoPath::new(string(row, "path")?)),
        )?;
        Ok(Some(WriteTargets::of(paths)))
    }

    fn collect<T>(
        &self,
        sql: &str,
        job_id: &JobId,
        doing: &'static str,
        read: impl Fn(&Row<'_>) -> Result<T, RowError>,
    ) -> Result<Vec<T>, RowError> {
        let mut statement = self
            .conn
            .prepare(sql)
            .map_err(fault(doing))
            .map_err(RowError::Database)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| Ok(read(row)))
            .map_err(fault(doing))
            .map_err(RowError::Database)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(fault(doing)).map_err(RowError::Database)??);
        }
        Ok(out)
    }
}

const SELECT_JOB: &str = "SELECT * FROM jobs WHERE job_id = ?1";
const SELECT_ALL_JOBS: &str = "SELECT * FROM jobs ORDER BY job_id";
const SELECT_EVENTS: &str = "SELECT seq, job_id, status_from, status_to, reason_kind,
                             reason_value, actor, at FROM job_events
                             WHERE job_id = ?1 ORDER BY seq";

fn event(row: &Row<'_>) -> Result<RecordedEvent, RowError> {
    let reason_kind = string(row, "reason_kind")?;
    let reason_value: Option<String> = maybe(row, "reason_value")?;
    let actor = string(row, "actor")?;
    Ok(RecordedEvent {
        seq: row.get("seq").map_err(column("job_events", "seq"))?,
        job_id: JobId::carried(Ulid::carried(string(row, "job_id")?)),
        from: enum_value(
            JobStatus::from_wire,
            "job_events",
            "status_from",
            &string(row, "status_from")?,
        )?,
        to: enum_value(
            JobStatus::from_wire,
            "job_events",
            "status_to",
            &string(row, "status_to")?,
        )?,
        reason: columns::read_reason(&reason_kind, reason_value.as_deref()).map_err(|detail| {
            RowError::MalformedColumn {
                table: "job_events",
                column: "reason_value",
                detail,
            }
        })?,
        actor: Actor::from_wire(&actor).ok_or(RowError::UnknownEnumValue {
            table: "job_events",
            column: "actor",
            value: actor,
        })?,
        at: Timestamp::from_rfc3339(string(row, "at")?),
    })
}

fn subject(row: &Row<'_>) -> Result<Option<Subject>, RowError> {
    match (maybe(row, "subject_kind")?, maybe(row, "subject_ref")?) {
        (Some(kind), Some(reference)) => Ok(Some(Subject { kind, reference })),
        (None, None) => Ok(None),
        _ => Err(RowError::MalformedColumn {
            table: "jobs",
            column: "subject_kind",
            detail: "half of a subject is present".to_string(),
        }),
    }
}

fn dispatched_by(row: &Row<'_>) -> Result<Option<DispatchOrigin>, RowError> {
    match (
        maybe(row, "dispatched_by_job_id")?,
        maybe(row, "dispatched_by_step_id")?,
    ) {
        (Some(job_id), Some(step_id)) => Ok(Some(DispatchOrigin {
            job_id: JobId::carried(Ulid::carried(job_id)),
            step_id: StepId::new(step_id),
        })),
        (None, None) => Ok(None),
        _ => Err(RowError::MalformedColumn {
            table: "jobs",
            column: "dispatched_by_job_id",
            detail: "half of a dispatch origin is present".to_string(),
        }),
    }
}

/// A column the record offers no way to set. Refused, never dropped.
fn refuse_if_set(
    job_id: &JobId,
    name: &'static str,
    value: Option<String>,
) -> Result<(), RowError> {
    match value {
        None => Ok(()),
        Some(value) => Err(RowError::ColumnNotReconstructable {
            job_id: job_id.clone(),
            column: name,
            value,
        }),
    }
}

fn string(row: &Row<'_>, name: &'static str) -> Result<String, RowError> {
    row.get(name).map_err(column("jobs", name))
}

fn maybe(row: &Row<'_>, name: &'static str) -> Result<Option<String>, RowError> {
    row.get(name).map_err(column("jobs", name))
}

fn column(table: &'static str, name: &'static str) -> impl Fn(rusqlite::Error) -> RowError {
    move |error| match error {
        rusqlite::Error::InvalidColumnType(..) => RowError::MalformedColumn {
            table,
            column: name,
            detail: "the column is not the type it was declared".to_string(),
        },
        other => RowError::Database(fault("reading a column")(other)),
    }
}

fn malformed(name: &'static str) -> impl Fn(String) -> RowError {
    move |detail| RowError::MalformedColumn {
        table: "jobs",
        column: name,
        detail,
    }
}

fn enum_value<T>(
    from_wire: impl Fn(&str) -> Option<T>,
    table: &'static str,
    column: &'static str,
    value: &str,
) -> Result<T, RowError> {
    from_wire(value).ok_or_else(|| RowError::UnknownEnumValue {
        table,
        column,
        value: value.to_string(),
    })
}
