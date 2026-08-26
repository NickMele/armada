//! SQLite in WAL mode, migrations, retention — and **the only crate that
//! deserializes**.
//!
//! That is a Cargo-graph fact rather than a review item: the SQLite dependency
//! is scoped to this crate alone, so nothing else *can* read a row. `ipc` is the
//! other place bytes enter the process, from the wire.
//!
//! # What is authoritative
//!
//! **`job_events` is, for `status`. The `jobs` row is, for everything else.**
//!
//! `job-fields.toml` calls the status column "a cache of the fold over
//! `job_events`", and this crate follows that literally: nothing here returns a
//! Job by reading its status column. [`Store::load_job`] and
//! [`Store::load_all_jobs`] rebuild the Job at its creation state and replay
//! every event through `Job::transition` — the same function that produced
//! them. A history the machine would not admit fails to load instead of
//! producing a Job no legal sequence of moves could reach.
//!
//! What that costs: the log carries transitions only, so it can restore
//! `status` and nothing else. Every other field is authoritative on the row,
//! and a later writer for one of them needs an event of its own or it will not
//! survive a restart the way status does. Those fields are checked rather than
//! assumed — see [`RowError::ColumnNotReconstructable`].
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

mod columns;
mod error;
mod fold;
mod open;
mod read;
mod schema;
mod write;

#[cfg(test)]
mod tests;

pub use error::{DatabaseFault, LoadAllError, LoadJobError, OpenError, RowError, WriteError};
pub use fold::RecordedEvent;
pub use open::Store;
pub use read::{Loaded, StatusRepair};
pub use schema::KNOWN_SCHEMA_VERSION;
