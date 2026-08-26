//! What this crate refuses, by name.
//!
//! Typed leaf enums with structured fields — the values that failed, not a
//! sentence about them. Nothing here is a `String` standing in for a reason,
//! and nothing here is `Box<dyn Error>` at the top level: the box appears only
//! as a *source*, where the underlying fault belongs to SQLite and cannot be
//! named outside this crate, because SQLite is scoped to this crate alone.
//!
//! The wrap into `ArmadaError` happens at a crate boundary and that wrapper is
//! not built. Each enum below is what a caller matches on until it is.
//!
//! # The two kinds of failure, and why they are separate types
//!
//! **A file-level fault refuses the open.** A damaged database, a schema from a
//! newer Armada, a file that is not an Armada store — none of them can be
//! narrowed to one Job, and continuing means starting empty with work on disk
//! that nobody is being told about. [`OpenError`] exists so that path ends at a
//! caller who has to handle it.
//!
//! **A row-level fault refuses one Job.** [`RowError`] names a Job whose stored
//! shape cannot be read back. It does not take the process down, and it is
//! never dropped either: [`LoadAllError::SomeJobsUnreadable`] hands back the
//! Jobs that loaded *and* the ones that did not, so a caller cannot end up with
//! a short list and no error — the v1 bug that lost twenty-one Jobs.

use std::error::Error;
use std::fmt;

use core_model::{IllegalStepTransition, IllegalTransition, JobId, JobStatus, StepId, StepState};

use crate::read::Loaded;

/// A fault that belongs to SQLite. The cause is carried rather than formatted,
/// so the chain stays traversable up to the wire, where it is flattened once.
#[derive(Debug)]
pub struct DatabaseFault {
    /// What was being done. A fixed string chosen here, never user input.
    pub doing: &'static str,
    pub cause: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for DatabaseFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.doing, self.cause)
    }
}

pub(crate) fn fault(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> DatabaseFault {
    move |cause| DatabaseFault {
        doing,
        cause: Box::new(cause),
    }
}

/// Why the store would not open. **Every variant is fatal on purpose.**
///
/// An empty database and a damaged one are different events here: an empty file
/// is migrated and opened, and each of these is refused. Starting empty over a
/// database that exists is how a person loses work and is told nothing.
#[derive(Debug)]
pub enum OpenError {
    Database(DatabaseFault),
    /// `PRAGMA integrity_check` did not answer `ok`. The pages are damaged;
    /// nothing read out of them can be trusted, including the parts that parse.
    IntegrityCheckFailed {
        path: String,
        finding: String,
    },
    /// `PRAGMA foreign_key_check` found rows pointing at nothing — an event
    /// whose Job is gone, a step with no Job. The log is the authority and a
    /// dangling entry means part of it has already been lost.
    DanglingReferences {
        path: String,
        rows: usize,
    },
    /// The file has tables and no `armada_meta`. Either it is not an Armada
    /// store, or it predates versioning; both are refused rather than migrated
    /// into, because migrating into the wrong file writes Jobs into it.
    NotAnArmadaStore {
        path: String,
        tables: usize,
    },
    /// The file records more migrations than this build knows. A newer Armada
    /// wrote it. Reading it with the older crate's assumptions is the quiet
    /// corruption this refuses.
    SchemaVersionFromTheFuture {
        path: String,
        found: u32,
        known: u32,
    },
    /// `armada_meta` exists and its version row is absent or is not a number.
    SchemaVersionUnreadable {
        path: String,
        found: Option<String>,
    },
    /// WAL was requested and SQLite answered something else — some filesystems
    /// refuse it. Recorded rather than accepted: the mode is part of the
    /// durability the rest of the design assumes.
    JournalModeRefused {
        path: String,
        actual: String,
    },
}

/// A Job whose stored shape cannot be read back.
///
/// Each variant is a distinguishable kind of damage, because "the row is bad"
/// is not an answer anybody can act on.
#[derive(Debug)]
pub enum RowError {
    Database(DatabaseFault),
    /// A stored enum column holds a value this build does not have. Written by
    /// something that did not share the enum, or by a newer one.
    UnknownEnumValue {
        table: &'static str,
        column: &'static str,
        value: String,
    },
    /// One of the three `array<...>` columns is not the shape it was written
    /// as.
    MalformedColumn {
        table: &'static str,
        column: &'static str,
        detail: String,
    },
    /// `origin` is `sub_dispatched` and `dispatched_by` is null. The row claims
    /// a parent step and does not name one, so the Job cannot be recreated
    /// through the constructor that made it.
    SubDispatchedWithoutOrigin {
        job_id: JobId,
    },
    /// The first event's `status_from` is not the status the Job was created
    /// at, or an event's `status_from` is not where the previous one left off.
    /// An event is missing, or two are out of order.
    EventDiscontinuity {
        job_id: JobId,
        seq: i64,
        folded: JobStatus,
        recorded: JobStatus,
    },
    /// The log records a move the machine does not admit. The history is not a
    /// history this Armada could have produced.
    IllegalRecordedTransition {
        job_id: JobId,
        seq: i64,
        cause: IllegalTransition,
    },
    /// The event's reason is not one its destination stores. `escalated` with
    /// no trigger, `killed` with one.
    ReasonDoesNotFitStatus {
        job_id: JobId,
        seq: i64,
        status: JobStatus,
        reason_kind: String,
    },
    /// A step row does not leave the state the fold has that step in. The
    /// counterpart of [`EventDiscontinuity`](Self::EventDiscontinuity) for the
    /// inner machine — a step move is missing, or two are out of order.
    ///
    /// `folded` is `None` where the Job has no such step at all, which is a row
    /// naming a step of some other Job's WorkflowDef.
    StepEventDiscontinuity {
        job_id: JobId,
        seq: i64,
        step_id: StepId,
        folded: Option<StepState>,
        recorded: StepState,
    },
    /// The log records a step arriving at a state no `StepTarget` names.
    ///
    /// `awaiting_human` needs a human advance gate, and `retrying` and
    /// `stopped` need a retry budget; M1 has neither, so no value in this build
    /// moves a step to one. A row saying a step got there was written by a
    /// machine this build does not have, and folding it as anything else would
    /// be the store lying rather than refusing.
    StepStateNotReachable {
        job_id: JobId,
        seq: i64,
        step_id: StepId,
        state: StepState,
    },
    /// The log records a step move the inner machine does not admit.
    IllegalRecordedStepTransition {
        job_id: JobId,
        seq: i64,
        cause: IllegalStepTransition,
    },
    /// `assigned_drone` is set, and the record offers no way to put it back.
    ///
    /// The log carries what a machine moved, and nothing moves this one: there
    /// is no event for assigning a Drone, so the rebuild would drop the value
    /// silently. `current_step_id` was here too until a step move became a row
    /// in the log — which is the shape a writer for this field needs before the
    /// column can be read back.
    ColumnNotReconstructable {
        job_id: JobId,
        column: &'static str,
        value: String,
    },
}

/// Why a write did not land.
#[derive(Debug)]
pub enum WriteError {
    Database(DatabaseFault),
    /// A Job with this id is already stored. Creation is not an update, and
    /// there is no method here that would make it one.
    JobAlreadyExists {
        job_id: JobId,
    },
    /// A transition was recorded against a Job that was never inserted.
    NoSuchJob {
        job_id: JobId,
    },
    /// A step move was recorded against a `job_steps` row that is not there.
    /// The rows are written at creation and never added to, so this is a move
    /// against a step of some other Job.
    NoSuchStep {
        job_id: JobId,
        step_id: StepId,
    },
}

/// Why one Job would not load.
#[derive(Debug)]
pub enum LoadJobError {
    Database(DatabaseFault),
    NoSuchJob { job_id: JobId },
    Unreadable(RowError),
}

/// Why the boot read did not come back whole.
///
/// **There is no variant that means "here is a shorter list".** A caller either
/// gets every Job or gets an error carrying both halves.
#[derive(Debug)]
pub enum LoadAllError {
    Database(DatabaseFault),
    /// Some Jobs rebuilt and some did not. Both are here: the ones that loaded
    /// so a caller need not throw away good work, and the ones that failed so
    /// it cannot pretend they were never there.
    SomeJobsUnreadable {
        loaded: Loaded,
        failed: Vec<RowError>,
    },
}

macro_rules! display {
    ($ty:ty, |$self:ident, $f:ident| $body:expr) => {
        impl fmt::Display for $ty {
            fn fmt(&$self, $f: &mut fmt::Formatter<'_>) -> fmt::Result {
                $body
            }
        }
        impl Error for $ty {}
    };
}

display!(OpenError, |self, f| match self {
    OpenError::Database(fault) => write!(f, "{fault}"),
    OpenError::IntegrityCheckFailed { path, finding } =>
        write!(f, "{path} failed its integrity check: {finding}"),
    OpenError::DanglingReferences { path, rows } => write!(
        f,
        "{path} has {rows} rows referencing records that are gone"
    ),
    OpenError::NotAnArmadaStore { path, tables } => write!(
        f,
        "{path} has {tables} tables and no armada_meta — it is not an Armada store"
    ),
    OpenError::SchemaVersionFromTheFuture { path, found, known } => write!(
        f,
        "{path} is at schema version {found} and this build knows {known}"
    ),
    OpenError::SchemaVersionUnreadable { path, found } => match found {
        Some(value) => write!(
            f,
            "{path} records schema version `{value}`, which is not a number"
        ),
        None => write!(f, "{path} has armada_meta and no schema version"),
    },
    OpenError::JournalModeRefused { path, actual } =>
        write!(f, "{path} refused WAL and is in `{actual}` mode"),
});

display!(RowError, |self, f| match self {
    RowError::Database(fault) => write!(f, "{fault}"),
    RowError::UnknownEnumValue {
        table,
        column,
        value,
    } => write!(
        f,
        "{table}.{column} holds `{value}`, which is not one this build knows"
    ),
    RowError::MalformedColumn {
        table,
        column,
        detail,
    } => write!(
        f,
        "{table}.{column} is not the shape it was written as: {detail}"
    ),
    RowError::SubDispatchedWithoutOrigin { job_id } => write!(
        f,
        "job {} is sub_dispatched and names no dispatching step",
        job_id.as_str()
    ),
    RowError::EventDiscontinuity {
        job_id,
        seq,
        folded,
        recorded,
    } => write!(
        f,
        "job {}: event {seq} leaves {} and the log had reached {}",
        job_id.as_str(),
        recorded.as_wire(),
        folded.as_wire()
    ),
    RowError::IllegalRecordedTransition { job_id, seq, cause } =>
        write!(f, "job {}: event {seq} records {cause}", job_id.as_str()),
    RowError::ReasonDoesNotFitStatus {
        job_id,
        seq,
        status,
        reason_kind,
    } => write!(
        f,
        "job {}: event {seq} arrives at {} carrying a `{reason_kind}` reason",
        job_id.as_str(),
        status.as_wire()
    ),
    RowError::StepEventDiscontinuity {
        job_id,
        seq,
        step_id,
        folded,
        recorded,
    } => write!(
        f,
        "job {}: event {seq} leaves step {} at {}, and the log had reached {}",
        job_id.as_str(),
        step_id.as_str(),
        recorded.as_wire(),
        match folded {
            Some(state) => state.as_wire(),
            None => "no such step",
        }
    ),
    RowError::StepStateNotReachable {
        job_id,
        seq,
        step_id,
        state,
    } => write!(
        f,
        "job {}: event {seq} puts step {} in `{}`, which nothing in this build reaches",
        job_id.as_str(),
        step_id.as_str(),
        state.as_wire()
    ),
    RowError::IllegalRecordedStepTransition { job_id, seq, cause } =>
        write!(f, "job {}: event {seq} records {cause}", job_id.as_str()),
    RowError::ColumnNotReconstructable {
        job_id,
        column,
        value,
    } => write!(
        f,
        "job {} has {column} = `{value}`, and the record offers no way to set it",
        job_id.as_str()
    ),
});

display!(WriteError, |self, f| match self {
    WriteError::Database(fault) => write!(f, "{fault}"),
    WriteError::JobAlreadyExists { job_id } =>
        write!(f, "job {} is already stored", job_id.as_str()),
    WriteError::NoSuchJob { job_id } => write!(f, "job {} was never inserted", job_id.as_str()),
    WriteError::NoSuchStep { job_id, step_id } => write!(
        f,
        "job {} has no step {}",
        job_id.as_str(),
        step_id.as_str()
    ),
});

display!(LoadJobError, |self, f| match self {
    LoadJobError::Database(fault) => write!(f, "{fault}"),
    LoadJobError::NoSuchJob { job_id } => write!(f, "no job {}", job_id.as_str()),
    LoadJobError::Unreadable(cause) => write!(f, "{cause}"),
});

display!(LoadAllError, |self, f| match self {
    LoadAllError::Database(fault) => write!(f, "{fault}"),
    LoadAllError::SomeJobsUnreadable { loaded, failed } => write!(
        f,
        "{} jobs rebuilt and {} did not: {}",
        loaded.jobs.len(),
        failed.len(),
        failed
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    ),
});
