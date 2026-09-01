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
/// The note a boundary is holding, and the column it waits in.
mod note;
mod open;
/// What a step said its work would be, kept after the slot that held it is
/// gone.
mod plan;
mod read;
/// What a person says went wrong, kept after the Job it is about is gone.
mod report;
mod row;
mod schema;
/// What a Job's Drones have cost it: one row per Drone, summed per Job.
mod spend;
#[cfg(feature = "wreckage")]
mod wreckage;
mod write;

#[cfg(test)]
mod tests;

pub use attempt::Attempted;
pub use delivery::Delivery;
pub use error::{DatabaseFault, LoadAllError, LoadJobError, OpenError, RowError, WriteError};
pub use fold::{Moved, RecordedEvent};
pub use footprint::Footprinted;
pub use forget::Forgotten;
pub use open::Store;
pub use plan::DeclaredPlan;
pub use read::{Loaded, RowIdentity, StatusRepair, UnreadableRow};
pub use report::Report;
pub use schema::KNOWN_SCHEMA_VERSION;
pub use spend::{DroneSpend, Spend};
