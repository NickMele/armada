//! Which operating-system process is working a Job, so a restart can ask.
//!
//! # A pid held only in memory is a pid a restart has lost
//!
//! `fleet::Drones` indexes a Job against its Drone's pid, and `Working` holds
//! the session that pid belongs to. Both are memory, so a Fleet that comes back
//! knows a step still names a Drone and has no way to ask what became of the
//! process — which is why reconciliation asserted the Drone was gone rather
//! than checking. This table is the pid crossing that gap.
//!
//! # The pid is not the identity, and a row that carried only a pid would lie
//!
//! Pids are reused. A row saying "Job X is pid 4096" is answered "yes,
//! something is there" by every liveness check on the machine, including for a
//! process that took the number after the Drone died. So the row carries the
//! second half `fleet::process::holder_of` reads: **when the process started**,
//! absolute, as the operating system reports it. The pair is the identity, and
//! the same pair is what `fleet::runtime` already uses to tell one Fleet from
//! the pid it left behind.
//!
//! The column is `TEXT` and nothing here parses it. It is an identity token
//! belonging to whoever took the reading, compared for equality and never for
//! order — see `fleet::process::StartedAt`, which is the type on the other
//! side of it.
//!
//! # Not a fold cache
//!
//! There is no event for it and nothing in [`crate::read`] rebuilds it, for
//! [`crate::spend`]'s reason: a process existing is not a move. What writes the
//! row is the spawn, what deletes it is the departure, and the two are the same
//! pair of moments that write and clear `job_steps.assigned_drone` — so a row
//! here with no pointer beside it is a departure that half-happened, which is
//! the one disagreement a reader should treat as a fault.

use core_model::{DroneId, JobId, StepId, Timestamp, Ulid};

use crate::error::{fault, LoadAllError, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::column;

/// Version 27 — the process working a Job, by pid and by when it started.
///
/// Beside the table it creates, like [`V23`](crate::spend::V23) and the
/// migrations either side of it: `schema.rs` is at the 900 lines the gate
/// refuses at.
///
/// **One row per Job and not per step**, because a Job holds one Drone at a
/// time — `fleet::steps_holding_a_drone` iterating is what makes the record's
/// answer the answer, and a second row here would be a second Drone this
/// workspace has no way to produce. The step is carried on the row rather than
/// in the key, so a reader knows which step's pointer the process belongs to
/// without joining.
///
/// **The foreign key is the point of `job_id` being a real reference.**
/// `forget_job` asks the file which tables point at `jobs` rather than reading
/// a list somebody maintains, so this table joins that walk by existing.
///
/// Nothing is backfilled. A Drone spawned before this has no row, which is the
/// same answer as a Job that never spawned one: nothing recorded, so nothing
/// can be adopted and the reconciliation falls back to what it always did.
pub(crate) const V27: &str = r#"
CREATE TABLE job_drone_process (
    job_id     TEXT    PRIMARY KEY NOT NULL REFERENCES jobs(job_id),
    step_id    TEXT    NOT NULL,
    drone_id   TEXT    NOT NULL,
    pid        INTEGER NOT NULL,
    started_at TEXT    NOT NULL,
    spawned_at TEXT    NOT NULL,
    CHECK (pid > 0)
) STRICT;
"#;

/// A Drone's process, as it was recorded at the moment it was spawned.
///
/// **Every field is what somebody read, never what anybody inferred.** A caller
/// that wants to know whether the process is still there asks the operating
/// system with these two numbers; this type answers no question about liveness
/// and has no method that could.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneProcess {
    pub job_id: JobId,
    /// The step whose `assigned_drone` this process is behind.
    pub step_id: StepId,
    /// The Drone the transcript is named by, so an adopted process can be
    /// joined back to what it had already said.
    pub drone_id: DroneId,
    pub pid: u32,
    /// When the process started, as the operating system reported it, opaque.
    /// **Half of the identity** — see this module's header.
    pub started_at: String,
    /// Fleet's own clock at the spawn. What an unobserved gap is measured from
    /// when nothing else can say how long a Drone has been running.
    pub spawned_at: Timestamp,
}

impl Store {
    /// Write down the process a Drone is running as.
    ///
    /// **An upsert on the Job, so a respawn replaces rather than collides.** A
    /// Job whose Drone left and was replaced has one process at a time, and the
    /// row is about the one that is there now — a second row would be a Drone
    /// this workspace cannot produce, and an insert that failed on the key
    /// would make the second spawn of a Job an error.
    pub fn record_drone_process(&mut self, process: &DroneProcess) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_drone_process \
                 (job_id, step_id, drone_id, pid, started_at, spawned_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT (job_id) DO UPDATE SET \
                 step_id = excluded.step_id, \
                 drone_id = excluded.drone_id, \
                 pid = excluded.pid, \
                 started_at = excluded.started_at, \
                 spawned_at = excluded.spawned_at",
                (
                    process.job_id.as_str(),
                    process.step_id.as_str(),
                    process.drone_id.as_str(),
                    i64::from(process.pid),
                    process.started_at.as_str(),
                    process.spawned_at.as_str(),
                ),
            )
            // Off the error code and never the message, which is
            // `crate::spend`'s rule: a Job that has been forgotten refuses the
            // write here rather than leaving a row naming nothing.
            //
            // **The extended code and not the primary one**, which is where
            // this table differs from that one: it carries a `CHECK` as well as
            // a foreign key, and both raise `ConstraintViolation`. Reading the
            // primary code alone would report a pid of zero as a Job that does
            // not exist — a refusal naming the wrong thing, which is worse than
            // one naming nothing.
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.extended_code == FOREIGN_KEY_VIOLATION =>
                {
                    WriteError::NoSuchJob {
                        job_id: process.job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("recording a Drone's process")(other)),
            })?;
        Ok(())
    }

    /// The Drone on this Job has gone, so nothing should be adoptable for it.
    ///
    /// **A Job with no row is `Ok`**, exactly as `drone_left` answers `Ok` for
    /// a step holding no Drone: every road that reaches this is a road where
    /// the process has gone, and a delete that found nothing agrees with that.
    pub fn forget_drone_process(&mut self, job_id: &JobId) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_drone_process WHERE job_id = ?1",
                (job_id.as_str(),),
            )
            .map_err(fault("clearing a Drone's process"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// The process recorded for one Job, if any was.
    pub fn drone_process(&self, job_id: &JobId) -> Result<Option<DroneProcess>, LoadJobError> {
        let found = self
            .conn
            .query_row(
                "SELECT job_id, step_id, drone_id, pid, started_at, spawned_at \
                 FROM job_drone_process WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| Ok(read_process(row)),
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadJobError::Unreadable(RowError::Database(fault(
                    "reading a Drone's process",
                )(
                    other
                )))),
            })?;
        found.transpose().map_err(LoadJobError::Unreadable)
    }

    /// Every process any Job has recorded, in Job order.
    ///
    /// **The reconciliation's read, and it takes one pass rather than a query
    /// per Job.** A boot walks every Job the store holds; asking per Job would
    /// be one statement each against a table that is at most one row per Job in
    /// the whole file.
    ///
    /// A row that will not read is a fault and not an absence: a pid nobody can
    /// parse is a process nobody can probe, and folding it into "no process
    /// recorded" would decide on no evidence that a live Drone is gone.
    pub fn drone_processes(&self) -> Result<Vec<DroneProcess>, LoadAllError> {
        let mut asked = self
            .conn
            .prepare(
                "SELECT job_id, step_id, drone_id, pid, started_at, spawned_at \
                 FROM job_drone_process ORDER BY job_id",
            )
            .map_err(fault("preparing the Drone process read"))
            .map_err(LoadAllError::Database)?;
        let rows = asked
            .query_map([], |row| Ok(read_process(row)))
            .map_err(fault("reading every Drone process"))
            .map_err(LoadAllError::Database)?;
        let mut held = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault("reading a Drone process row"))
                .map_err(LoadAllError::Database)?;
            held.push(row.map_err(|why| {
                LoadAllError::Database(fault("reading a Drone process row")(
                    rusqlite::Error::InvalidColumnName(why.to_string()),
                ))
            })?);
        }
        Ok(held)
    }
}

/// One row, narrowed. **The pid is refused rather than cast** — a value SQLite
/// holds that a `u32` cannot is a row nothing wrote through this crate, and a
/// silent truncation would name a different process.
fn read_process(row: &rusqlite::Row<'_>) -> Result<DroneProcess, RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(TABLE, name))
    };
    let pid: i64 = row.get("pid").map_err(column(TABLE, "pid"))?;
    Ok(DroneProcess {
        job_id: JobId::carried(Ulid::carried(text("job_id")?)),
        step_id: StepId::new(text("step_id")?),
        drone_id: DroneId::carried(Ulid::carried(text("drone_id")?)),
        pid: u32::try_from(pid).map_err(|_| RowError::MalformedColumn {
            table: TABLE,
            column: "pid",
            detail: "a pid is a positive number the platform can express, and this is not"
                .to_string(),
        })?,
        started_at: text("started_at")?,
        spawned_at: Timestamp::from_rfc3339(text("spawned_at")?),
    })
}

/// Named once, because every error above points at the same table.
const TABLE: &str = "job_drone_process";

/// `SQLITE_CONSTRAINT_FOREIGNKEY`. Spelled out rather than reached through
/// `rusqlite`, which exposes the primary code as an enum and the extended one
/// as the bare integer SQLite defines.
const FOREIGN_KEY_VIOLATION: i32 = 787;
