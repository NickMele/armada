//! Reading Jobs back — which means folding them, not selecting them.
//! **`status` is a cache, and this is where the log wins.**
//! [`Store::load_all_jobs`] is the boot read: it rebuilds every Job from its
//! events, and where the cached column disagrees with the fold it **writes the
//! fold back** and reports the repair. `job-fields.toml` prescribes exactly
//! that — "Fleet re-folds non-terminal Jobs at boot and lets the log win" — and
//! the `&mut self` is the signature saying out loud that the boot read may
//! write.
//!
//! **The log is authoritative for `status`, `current_step_id` and every
//! `job_steps` row.** `job_events` records what a machine moved and there are
//! two machines: a status transition and a step move are both rows in it, in
//! one order, so both fold back. Those columns are caches of that fold in
//! exactly the sense `status` is, and this file does not read them back.
//! `assigned_drone` folds too, now that a Drone arriving on a step is a row in
//! the same log — the refusal `current_step_id` had before a step move became a
//! row, because a rebuild that cannot put a value back must say so rather than
//! drop it.
//!
//! **Every other column on the Job row is the authority for its own field**,
//! because nothing in the log describes it. `workflow` is the newest of them
//! and the only one whose absence is fatal: a Job that cannot say what it froze
//! cannot be dispatched against anything.

use core_model::{
    Attachment, Branch, CheckOutcome, CriterionId, DispatchOrigin, EvidenceType, Facts,
    GateManifest, GateOutcome, Job, JobId, JobStatus, JudgeVerdict, Judgment, ManifestId,
    ModelName, NewJob, Origin, RedirectWaiting, RepoPath, StepCheck, StepEvidence, StepId,
    StepSeed, StepState, Subject, Timestamp, Title, Ulid, Urgency, WriteTargets,
};
use rusqlite::Row;

use crate::columns;
use crate::error::{fault, LoadAllError, LoadJobError, RowError};
use crate::fold::replay;
use crate::open::Store;
use crate::row::{column, enum_value, malformed, maybe, string};

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
///
/// Boxed on the `Err` side: the identity and the refusal together are wide
/// enough that every `Ok` would otherwise carry the width.
type Rebuilt = Result<(Job, JobStatus), Box<UnreadableRow>>;

/// A row the fold refused, and what the row still says about itself.
///
/// **A row that will not rebuild still has an id**, and removing one needs no
/// more than that — so the identity travels with the refusal rather than the
/// refusal alone. `armada clean` is what reads it.
#[derive(Debug)]
pub struct UnreadableRow {
    /// `None` only when `job_id` itself did not read, which leaves nothing to
    /// derive from and nothing safe to remove.
    pub row: Option<RowIdentity>,
    pub why: RowError,
}

/// The two columns a rebuild never has to get past: which Job, and whose.
///
/// Both have been `TEXT NOT NULL` since the first migration, so a row orphaned
/// by a later one still carries them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowIdentity {
    pub job_id: JobId,
    pub owner_manifest_id: ManifestId,
}

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
        let attempts: Vec<Rebuilt> = self.rebuild_every_job()?;
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
                Err(error) => failed.push(*error),
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

    fn rebuild_every_job(&self) -> Result<Vec<Rebuilt>, LoadAllError> {
        let mut statement = self
            .conn
            .prepare(SELECT_ALL_JOBS)
            .map_err(fault("preparing the job read"))
            .map_err(LoadAllError::Database)?;
        let rows = statement
            .query_map([], |row| Ok(self.attempt(row)))
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

    /// One rebuild, with the row's own identity kept where it failed.
    fn attempt(&self, row: &Row<'_>) -> Rebuilt {
        self.rebuild(row).map_err(|why| {
            Box::new(UnreadableRow {
                row: identifies(row),
                why,
            })
        })
    }

    /// The Job as its log says it is, and the status its row claimed.
    fn rebuild(&self, row: &Row<'_>) -> Result<(Job, JobStatus), RowError> {
        let job_id = JobId::carried(Ulid::carried(string(row, "job_id")?));
        let cached = enum_value(
            JobStatus::from_wire,
            "jobs",
            "status",
            &string(row, "status")?,
        )?;
        let created_at = Timestamp::from_rfc3339(string(row, "created_at")?);

        // Both are caches of the fold now, like `status`, and neither is read
        // back: a step move and a Drone arriving are each a row in the log.

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
            // The declaration the Job froze, not the id beside it. The
            // `workflow_id` column is written from this and never read into it.
            workflow: columns::read_workflow(&maybe(row, "workflow")?.ok_or_else(|| {
                RowError::WorkflowNotFrozen {
                    job_id: job_id.clone(),
                }
            })?)
            .map_err(malformed("workflow"))?,
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
            model: ModelName::new(&string(row, "model")?).map_err(|blank| {
                RowError::MalformedColumn {
                    table: "jobs",
                    column: "model",
                    detail: blank.to_string(),
                }
            })?,
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
            attachments: self.attachments(&job_id)?,
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

        // Not folded, and not on `NewJob`: the worktree is made after creation
        // and no event describes it, so the column is this field's authority
        // the way every non-status column is.
        let created = match maybe(row, "branch")? {
            Some(name) => created.on_branch(Branch::new(&name).map_err(|blank| {
                RowError::MalformedColumn {
                    table: "jobs",
                    column: "branch",
                    detail: blank.to_string(),
                }
            })?),
            None => created,
        };

        // Read back for the same reason `branch` is: no event describes a
        // person's note, so this column is its own authority and the fold has
        // nothing to say about it. A blank is refused rather than carried —
        // `RedirectWaiting::saying` is what makes the empty note
        // unrepresentable, and a row holding one came from outside this crate.
        let created = match maybe(row, "redirect_waiting")? {
            Some(note) => {
                let waiting = RedirectWaiting::saying(&note).ok_or(RowError::MalformedColumn {
                    table: "jobs",
                    column: "redirect_waiting",
                    detail: String::from("a note waiting for the next Drone says nothing"),
                })?;
                created
                    .redirect_waits(waiting)
                    .map_err(|held| RowError::MalformedColumn {
                        table: "jobs",
                        column: "redirect_waiting",
                        detail: held.to_string(),
                    })?
            }
            None => created,
        };

        let events = self.events_for(&job_id)?;
        Ok((replay(created, &events)?, cached))
    }

    /// The `job_steps` rows, as the seeds creation took: which step, and where
    /// in the order. Nothing about where the step got to, which the log says.
    ///
    /// `state` is read and then thrown away, which is deliberate. The column is
    /// a cache of the fold and the fold is what the rebuild uses — but a value
    /// in it that is not one of the six was written by something that did not
    /// share the enum, and that is worth refusing even though nothing here
    /// needs the answer.
    fn step_seeds(&self, job_id: &JobId) -> Result<Vec<StepSeed>, RowError> {
        self.collect(
            "SELECT step_id, ordinal, state FROM job_steps
             WHERE job_id = ?1 ORDER BY ordinal",
            job_id,
            "reading step rows",
            |row| {
                let _: StepState = enum_value(
                    StepState::from_wire,
                    "job_steps",
                    "state",
                    &string(row, "state")?,
                )?;
                Ok(StepSeed {
                    step_id: StepId::new(string(row, "step_id")?),
                    ordinal: row.get("ordinal").map_err(column("job_steps", "ordinal"))?,
                })
            },
        )
    }

    /// What each of a Job's declared Checks did, grouped by step and in the
    /// order the step declares them.
    ///
    /// **Not folded, and not on the Job.** A Check result is a fact Fleet
    /// observed rather than a move a machine made, so it is read beside the
    /// record rather than through it — the same shape `last_reason` has, and
    /// for the same reason: it is not a field of `core_model::Job`.
    ///
    /// A step with no rows comes back absent from the list, which is what "the
    /// gate has not run this step's checks" looks like.
    ///
    /// **The latest run of each step**, which is what this answered before a
    /// step could keep more than one. Every run is
    /// [`step_checks_every_attempt`](Store::step_checks_every_attempt).
    pub fn step_checks(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(StepId, Vec<StepCheck>)>, LoadJobError> {
        let rows = self
            .collect(
                "SELECT step_id, name, outcome, expected, produced, output_path
                 FROM job_step_checks AS c WHERE job_id = ?1
                   AND attempt = (SELECT max(attempt) FROM job_step_checks
                                  WHERE job_id = c.job_id AND step_id = c.step_id)
                 ORDER BY step_id, ordinal",
                job_id,
                "reading check results",
                |row| {
                    let outcome = string(row, "outcome")?;
                    Ok((
                        StepId::new(string(row, "step_id")?),
                        StepCheck {
                            name: string(row, "name")?,
                            outcome: enum_value(
                                CheckOutcome::from_wire,
                                "job_step_checks",
                                "outcome",
                                &outcome,
                            )?,
                            expected: maybe(row, "expected")?,
                            produced: maybe(row, "produced")?,
                            output_path: maybe(row, "output_path")?,
                        },
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;

        let mut grouped: Vec<(StepId, Vec<StepCheck>)> = Vec::new();
        for (step_id, check) in rows {
            match grouped.last_mut() {
                Some((last, checks)) if *last == step_id => checks.push(check),
                _ => grouped.push((step_id, vec![check])),
            }
        }
        Ok(grouped)
    }

    /// What the Judge said about each of a Job's steps **on its latest run**,
    /// in the order asked.
    ///
    /// Read beside the record for [`step_checks`](Store::step_checks)'s reason.
    /// A step with no rows is absent from the list, which is what "the Judge
    /// never ran on this step" looks like — and it is the ordinary case,
    /// because most steps ask nothing.
    ///
    /// **This is the wrong read for "did the same note go unaddressed".** It
    /// answers where the step stands, which is what a resume and a Board row
    /// want. The question the design asks — four runs, four sets of verdicts —
    /// is
    /// [`step_judgments_every_attempt`](Store::step_judgments_every_attempt).
    pub fn step_judgments(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(StepId, Vec<Judgment>)>, LoadJobError> {
        let rows = self
            .collect(
                "SELECT step_id, criterion, verdict, expected, produced, consequence,
                        brief_path
                 FROM job_step_judgments AS j WHERE job_id = ?1
                   AND attempt = (SELECT max(attempt) FROM job_step_judgments
                                  WHERE job_id = j.job_id AND step_id = j.step_id)
                 ORDER BY step_id, ordinal",
                job_id,
                "reading judgments",
                |row| {
                    let verdict = string(row, "verdict")?;
                    Ok((
                        StepId::new(string(row, "step_id")?),
                        Judgment {
                            criterion_id: CriterionId::new(string(row, "criterion")?),
                            verdict: enum_value(
                                JudgeVerdict::from_wire,
                                "job_step_judgments",
                                "verdict",
                                &verdict,
                            )?,
                            expected: maybe(row, "expected")?,
                            produced: maybe(row, "produced")?,
                            consequence: maybe(row, "consequence")?,
                            brief_path: maybe(row, "brief_path")?,
                        },
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;

        let mut grouped: Vec<(StepId, Vec<Judgment>)> = Vec::new();
        for (step_id, judgment) in rows {
            match grouped.last_mut() {
                Some((last, judgments)) if *last == step_id => judgments.push(judgment),
                _ => grouped.push((step_id, vec![judgment])),
            }
        }
        Ok(grouped)
    }

    /// The evidence each of a Job's steps had accepted **on its latest run**,
    /// keyed by step.
    ///
    /// Shaped like [`step_judgments`](Store::step_judgments): a step that has
    /// submitted none is absent rather than present and blank. **A gaming
    /// check whose `baseline_ref` names an absent step runs with no baseline**
    /// and says so, where a blank one would read as a comparison against
    /// nothing.
    ///
    /// The latest run is the right baseline and stays the one this answers:
    /// `docs/concepts/workflow.md` carries only the most recent iteration's
    /// work product forward. Every run is
    /// [`step_evidence_every_attempt`](Store::step_evidence_every_attempt).
    pub fn step_evidence(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(StepId, StepEvidence)>, LoadJobError> {
        self.collect(
            "SELECT step_id, evidence_type, claimed, shown_by, not_claimed
             FROM job_step_evidence AS e WHERE job_id = ?1
               AND attempt = (SELECT max(attempt) FROM job_step_evidence
                              WHERE job_id = e.job_id AND step_id = e.step_id)
             ORDER BY step_id",
            job_id,
            "reading step evidence",
            |row| {
                let kind = string(row, "evidence_type")?;
                Ok((
                    StepId::new(string(row, "step_id")?),
                    StepEvidence {
                        evidence_type: enum_value(
                            EvidenceType::from_wire,
                            "job_step_evidence",
                            "evidence_type",
                            &kind,
                        )?,
                        claimed: string(row, "claimed")?,
                        shown_by: string(row, "shown_by")?,
                        not_claimed: string(row, "not_claimed")?,
                    },
                ))
            },
        )
        .map_err(LoadJobError::Unreadable)
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

    /// Every file handed to the Job at proposal time. Unlike `write_targets`,
    /// zero rows is unambiguous here: an attachment list has no "determined to
    /// carry nothing" state to distinguish from "none was ever attached".
    fn attachments(&self, job_id: &JobId) -> Result<Vec<Attachment>, RowError> {
        self.collect(
            "SELECT filename, mime_type, byte_size, storage_ref FROM job_attachments
             WHERE job_id = ?1 ORDER BY filename",
            job_id,
            "reading attachments",
            |row| {
                let byte_size: i64 = row
                    .get("byte_size")
                    .map_err(column("job_attachments", "byte_size"))?;
                Ok(Attachment {
                    filename: string(row, "filename")?,
                    mime_type: string(row, "mime_type")?,
                    byte_size: byte_size as u64,
                    storage_ref: string(row, "storage_ref")?,
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

    pub(crate) fn collect<T>(
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

/// Who the row is, read without folding anything.
///
/// Deliberately tolerant of everything but its own two columns: this is the one
/// read that has to work on a row the rebuild has already refused.
fn identifies(row: &Row<'_>) -> Option<RowIdentity> {
    Some(RowIdentity {
        job_id: JobId::carried(Ulid::carried(string(row, "job_id").ok()?)),
        owner_manifest_id: ManifestId::carried(Ulid::carried(
            string(row, "owner_manifest_id").ok()?,
        )),
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
