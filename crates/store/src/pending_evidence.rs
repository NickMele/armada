//! Evidence a Drone submitted, kept durable between the tool call returning
//! and the gate ruling on it. #796.
//!
//! `EvidenceInbox` (`fleet::evidence`) holds a submission only in memory
//! between the two, and a Fleet that stopped in that window used to lose it
//! silently. This is the durable half: written the instant a submission is
//! accepted, cleared once the in-memory queue has given the entry up for
//! good.
//!
//! **One row per Job, upserted, [`crate::process::V27`]'s shape** — a Job's
//! Drone has one submission outstanding at a time.
//!
//! **The review is not carried** — `fleet::regating::submitted_already`
//! already establishes that nothing reads it back out once accepted; it is
//! kept in `job_step_reviews` instead.

use core_model::{EvidenceType, JobId, StepId, Timestamp, Ulid};

use crate::error::{fault, LoadAllError, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value};

/// Version 76 — evidence waiting for the gate, one row per Job.
///
/// Beside the table it creates, like [`V27`](crate::process::V27): `schema.rs`
/// is at the 900 lines the gate refuses at. Nothing is backfilled — a
/// submission that landed before this migration reads the same as a Job
/// whose Drone never submitted.
pub(crate) const V76: &str = r#"
CREATE TABLE job_pending_evidence (
    job_id        TEXT NOT NULL PRIMARY KEY REFERENCES jobs(job_id),
    step_id       TEXT NOT NULL,
    evidence_type TEXT NOT NULL,
    claimed       TEXT NOT NULL,
    shown_by      TEXT NOT NULL,
    not_claimed   TEXT NOT NULL,
    landed_at     TEXT NOT NULL
) STRICT;
"#;

/// One submission, as it is durable between arriving and being ruled on.
/// The same four fields [`StepEvidence`](core_model::StepEvidence) carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingEvidence {
    pub job_id: JobId,
    /// Not on `fleet::evidence::Landed` — the slot it came from knows it —
    /// but there is no slot to ask once a restart has taken it.
    pub step_id: StepId,
    pub evidence_type: EvidenceType,
    pub claimed: String,
    pub shown_by: String,
    pub not_claimed: String,
    /// `Landed::at`'s own reading, carried through.
    pub landed_at: Timestamp,
}

impl Store {
    /// Write a submission down the instant it is accepted, before the
    /// receipt returns to the Drone.
    ///
    /// **An upsert on the Job**, [`record_drone_process`](Store::record_drone_process)'s
    /// reason: one submission is outstanding for a Job at a time.
    pub fn record_pending_evidence(&mut self, pending: &PendingEvidence) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_pending_evidence \
                 (job_id, step_id, evidence_type, claimed, shown_by, not_claimed, landed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT (job_id) DO UPDATE SET \
                 step_id = excluded.step_id, \
                 evidence_type = excluded.evidence_type, \
                 claimed = excluded.claimed, \
                 shown_by = excluded.shown_by, \
                 not_claimed = excluded.not_claimed, \
                 landed_at = excluded.landed_at",
                (
                    pending.job_id.as_str(),
                    pending.step_id.as_str(),
                    pending.evidence_type.as_wire(),
                    pending.claimed.as_str(),
                    pending.shown_by.as_str(),
                    pending.not_claimed.as_str(),
                    pending.landed_at.as_str(),
                ),
            )
            // Off the error code and never the message — `record_drone_process`'s rule.
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.extended_code == FOREIGN_KEY_VIOLATION =>
                {
                    WriteError::NoSuchJob {
                        job_id: pending.job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("recording pending evidence")(other)),
            })?;
        Ok(())
    }

    /// The gate has given this Job's submission up for good.
    ///
    /// **A Job with no row is `Ok`**, [`forget_drone_process`](Store::forget_drone_process)'s
    /// reason: every caller reaches this once the entry is already given up.
    pub fn forget_pending_evidence(&mut self, job_id: &JobId) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_pending_evidence WHERE job_id = ?1",
                (job_id.as_str(),),
            )
            .map_err(fault("clearing pending evidence"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every submission still waiting for the gate, over every Job.
    ///
    /// **Boot's own read**, one pass rather than a query per Job — the shape
    /// [`drone_processes`](Store::drone_processes) reads its own row with.
    pub fn pending_evidence(&self) -> Result<Vec<PendingEvidence>, LoadAllError> {
        let mut asked = self
            .conn
            .prepare(
                "SELECT job_id, step_id, evidence_type, claimed, shown_by, not_claimed, landed_at \
                 FROM job_pending_evidence ORDER BY job_id",
            )
            .map_err(fault("preparing the pending evidence read"))
            .map_err(LoadAllError::Database)?;
        let rows = asked
            .query_map([], |row| Ok(read_pending(row)))
            .map_err(fault("reading every pending submission"))
            .map_err(LoadAllError::Database)?;
        let mut held = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault("reading a pending evidence row"))
                .map_err(LoadAllError::Database)?;
            held.push(row.map_err(|why| {
                LoadAllError::Database(fault("reading a pending evidence row")(
                    rusqlite::Error::InvalidColumnName(why.to_string()),
                ))
            })?);
        }
        Ok(held)
    }
}

/// One row, narrowed.
fn read_pending(row: &rusqlite::Row<'_>) -> Result<PendingEvidence, RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(TABLE, name))
    };
    let kind = text("evidence_type")?;
    Ok(PendingEvidence {
        job_id: JobId::carried(Ulid::carried(text("job_id")?)),
        step_id: StepId::new(text("step_id")?),
        evidence_type: enum_value(EvidenceType::from_wire, TABLE, "evidence_type", &kind)?,
        claimed: text("claimed")?,
        shown_by: text("shown_by")?,
        not_claimed: text("not_claimed")?,
        landed_at: Timestamp::from_rfc3339(text("landed_at")?),
    })
}

/// Named once, because every error above points at the same table.
const TABLE: &str = "job_pending_evidence";

/// `SQLITE_CONSTRAINT_FOREIGNKEY` — `crate::process`'s own constant, spelled
/// out again: `rusqlite` exposes the extended code as a bare integer.
const FOREIGN_KEY_VIOLATION: i32 = 787;
