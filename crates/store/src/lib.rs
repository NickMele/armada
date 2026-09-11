//! SQLite in WAL mode, migrations, retention — and **the only crate that
//! deserializes**. A Cargo-graph fact rather than a review item: the SQLite
//! dependency is scoped here alone, so nothing else *can* read a row. `ipc` is
//! the other place bytes enter the process, from the wire.
//!
//! **`job_events` is authoritative** for `status`, `current_step_id` and every
//! `job_steps` row; the `jobs` row is for everything else, so nothing here
//! returns a Job by reading its status column — [`fold`](mod@fold) is the
//! replay. The cost is a field with no event being authoritative only on the
//! row, checked rather than assumed: [`RowError::ColumnNotReconstructable`].
//!
//! **Query functions never return pre-filtered results**, but the parse
//! failures alongside them. v1 wrote `.filter_map(Result::ok)` after a store
//! call and dropped twenty-one real Jobs with no error anywhere — the bug was
//! at the call site, and a signature handing back a bare `Vec` invites it.
//! [`LoadAllError::SomeJobsUnreadable`] carries both sets.
//!
//! **A corrupt store is refused and an empty one created**, on six checks in
//! `open.rs` that are each fatal: silently starting empty over a database that
//! exists means somebody loses work and is shown a clean Board.
//!
//! **Nothing here reads a clock**: a transition's instant is on the event and
//! creation's is an argument to [`Store::insert_job`]. Schema versioning is a
//! migration list and a version row, applied on open, and `job_events` is
//! append-only in the database itself, by trigger.

/// How a Job meets a command its Drone was not granted, and what a person
/// allowed it.
mod allowing;
mod attempt;
mod columns;
/// The Drone pointer, where it now lives: one column per step.
mod delivery;
mod drone;
mod error;
mod fold;
mod footprint;
mod forget;
mod gaming;
/// Where a verdict's own question was kept, and the column that points at it.
mod judged;
/// What the Manifest read at Job creation, kept whole and off the Job row's
/// own fields.
mod manifest_snapshot;
/// The migration list, and where a file stands against it. `V1`..`V16` stay in
/// `schema`; this is only what had to move to keep that file under the gate.
mod migrations;
/// The model a person chose for a Job's later steps.
mod model_override;
/// The note a boundary is holding, and the column it waits in.
mod note;
mod numbering;
mod open;
/// What a step said its work would be, kept after the slot that held it is
/// gone.
mod plan;
/// The span of ports a Job's worktree holds, or a no-Job server run holds.
mod ports;
/// Which operating-system process is working a Job, so a restart can ask.
mod process;
mod proposing;
/// What a repository's Checks said about a commit — **the one per-Check record
/// here that is not keyed by a Job.**
mod proving;
mod read;
/// Which comments on a Job's pull request have already reached a Drone.
mod remarks;
/// What a person says went wrong, kept after the Job it is about is gone.
mod report;
/// A ULID, a whole handle or a bare number in, one Job out — **a second way
/// in and never a second key.**
mod resolving;
/// Giving a Job's resources back without giving up its record.
mod retain;
mod revision;
mod row;
mod schema;
/// The frames a step's harness produced, and where each one was kept.
mod showing;
mod shown_again;
/// What a Job's Drones have cost it: one row per Drone, summed per Job.
mod spend;
#[cfg(feature = "wreckage")]
mod wreckage;
mod write;

#[cfg(test)]
mod tests;

pub use attempt::Attempted;
pub use delivery::{Delivery, Unsettled};
pub use error::{DatabaseFault, LoadAllError, LoadJobError, OpenError, RowError, WriteError};
pub use fold::{Moved, RecordedEvent};
pub use footprint::Footprinted;
pub use forget::Forgotten;
pub use migrations::KNOWN_SCHEMA_VERSION;
pub use open::Store;
pub use plan::DeclaredPlan;
pub use ports::{PortClaim, PortClaimant};
pub use process::DroneProcess;
pub use proving::Proved;
pub use read::{Loaded, RowIdentity, StatusRepair, UnreadableRow};
pub use report::Report;
pub use resolving::{NamedJob, ResolveJobError};
pub use retain::Retained;
pub use showing::KeptFrame;
pub use shown_again::{ShownAgain, SpecNamed};
pub use spend::{DroneSpend, Spend};
