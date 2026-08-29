//! SQLite in WAL mode, migrations, retention — and **the only crate that
//! deserializes**.
//!
//! That is a Cargo-graph fact rather than a review item: the SQLite dependency
//! is scoped to this crate alone, so nothing else *can* read a row. `ipc` is the
//! other place bytes enter the process, from the wire.
//!
//! # What is authoritative
//!
//! **`job_events` is, for `status`, for `current_step_id` and for every
//! `job_steps` row. The `jobs` row is, for everything else.**
//!
//! `job-fields.toml` calls the status column "a cache of the fold over
//! `job_events`", and this crate follows that literally: nothing here returns a
//! Job by reading its status column. [`Store::load_job`] and
//! [`Store::load_all_jobs`] rebuild the Job at its creation state and replay
//! every event through `Job::transition` — the same function that produced
//! them. A history the machine would not admit fails to load instead of
//! producing a Job no legal sequence of moves could reach.
//!
//! The log carries what a machine moved, and a status transition, a step move
//! and a Drone arriving are all rows in it, in one order. A step move that
//! could not be ordered against the status transitions around it could not be
//! replayed at all, because the inner machine only advances beneath two of the
//! twelve statuses.
//!
//! What that costs: a field with no event can only be authoritative on the row,
//! and it is checked rather than assumed — see
//! [`RowError::ColumnNotReconstructable`]. `branch` is the one left.
//!
//! # The rule that cost v1 twenty-one Jobs
//!
//! **Query functions never return pre-filtered results.** They return the parse
//! failures with them, in a shape the caller cannot silently ignore. v1 wrote
//! `.filter_map(Result::ok)` after a store call and dropped twenty-one real
//! Jobs without an error anywhere — the bug was not in the store, it was at the
//! call site, and a signature that hands back a bare `Vec` invites it. There is
//! no such signature here: [`LoadAllError::SomeJobsUnreadable`] carries the
//! Jobs that loaded *and* the ones that did not.
//!
//! # A corrupt store is refused, an empty one is created
//!
//! Six checks decide which is which, and every one of them is fatal — see
//! `open.rs`. Silently starting empty over a database that exists means somebody
//! loses work and is shown a clean Board.
//!
//! # Time is injected
//!
//! Nothing in this crate reads a clock. A transition's instant arrives on the
//! event; creation's arrives as an argument to [`Store::insert_job`].
//!
//! Schema versioning is a migration list plus a version row, applied on open.
//! `job_events` is append-only in the database itself, by trigger.

mod attempt;
mod columns;
mod error;
mod fold;
mod footprint;
mod forget;
mod gaming;
mod open;
/// What a step said its work would be, kept after the slot that held it is
/// gone.
mod plan;
mod read;
/// What a person says went wrong, kept after the Job it is about is gone.
mod report;
mod row;
mod schema;
#[cfg(feature = "wreckage")]
mod wreckage;
mod write;

#[cfg(test)]
mod tests;

pub use attempt::Attempted;
pub use error::{DatabaseFault, LoadAllError, LoadJobError, OpenError, RowError, WriteError};
pub use fold::{Moved, RecordedEvent};
pub use footprint::Footprinted;
pub use forget::Forgotten;
pub use open::Store;
pub use plan::DeclaredPlan;
pub use read::{Loaded, RowIdentity, StatusRepair, UnreadableRow};
pub use report::Report;
pub use schema::KNOWN_SCHEMA_VERSION;
