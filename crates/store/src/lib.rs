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

// `columns::workflow`'s `json!` of one frozen step nests deep enough on its
// own — every Check, every judge criterion, every narrowing — that one more
// key (`places`, #1102) crossed the default limit. Raised rather than
// restructured: the nesting mirrors the record's own shape.
#![recursion_limit = "256"]

/// How a Job meets a command its Drone was not granted, and what a person
/// allowed it.
mod allowing;
mod asking;
mod attempt;
/// A test broken on main, and the Job drafted to fix it. #999.
mod breakages;
mod columns;
/// The Drone pointer, where it now lives: one column per step.
mod delivery;
mod drift;
mod drone;
mod error;
mod fold;
mod footprint;
mod forget;
mod gaming;
/// The session each Helm conversation resumes, one row per conversation.
mod helm_sessions;
/// Where a verdict's own question was kept, and the column that points at it.
mod judged;
/// Kit's MCP servers, and each Manifest's word over one. `#1275`.
mod kit;
/// The Fleet limits a person saved, one row or none.
mod limits;
/// A redispatch read backwards: which Job replaced this one.
mod lineage;
/// Commands a person always-allowed for a whole Manifest, kept here instead
/// of a commit on some Job's branch.
mod manifest_allowed;
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
/// Evidence a Drone submitted, kept durable until the gate rules on it. #796.
mod pending_evidence;
/// What a step said its work would be, kept after the slot that held it is
/// gone.
mod plan;
/// The span of ports a Job's worktree holds, or a no-Job server run holds.
mod ports;
/// A person's preferences, one row per name, kept beside `limits`.
mod preferences;
/// Which operating-system process is working a Job, so a restart can ask.
mod process;
mod proposing;
/// What a repository's Checks said about a commit — **the one per-Check record
/// here that is not keyed by a Job.**
mod proving;
mod read;
/// Giving a Job's resources back without giving up its record.
mod rechecking;
/// Which comments on a Job's pull request have already reached a Drone.
mod remarks;
/// What a person says went wrong, kept after the Job it is about is gone.
mod report;
mod repositories;
/// A ULID, a whole handle or a bare number in, one Job out — **a second way
/// in and never a second key.**
mod resolving;
mod retain;
mod retrace;
mod reuse;
/// The review Fleet composed at a Job's gate — the one builder's text, kept
/// beside the Job rather than only in the pull request it may also carry.
mod review;
mod review_dismissals;
mod review_followups;
mod review_model;
mod review_record;
mod review_view;
mod revision;
mod row;
mod schema;
/// The frames a step's harness produced, and where each one was kept.
mod showing;
mod shown_again;
/// What a Job's Drones have cost it: one row per Drone, summed per Job.
mod spend;
/// Every Studio a repository keeps, with its nodes and edges. `#1285`.
mod studio;
/// How long each of a repository's Checks has taken.
mod timings;
/// A Job's plan and its tasks, kept as every change made to them. Not
/// [`plan`](mod@plan), which is a step's declared scope.
mod work_plan;
#[cfg(feature = "wreckage")]
mod wreckage;
mod write;

#[cfg(test)]
mod tests;

pub use asking::OpenJudgeQuestion;
pub use attempt::Attempted;
pub use delivery::{Currency, Delivery, Unsettled};
pub use drift::ScopeDrift;
pub use error::{DatabaseFault, LoadAllError, LoadJobError, OpenError, RowError, WriteError};
pub use fold::{Moved, RecordedEvent};
pub use footprint::Footprinted;
pub use forget::Forgotten;
pub use limits::SavedLimits;
pub use lineage::{ReplacedBy, Replaces};
pub use migrations::KNOWN_SCHEMA_VERSION;
pub use open::Store;
pub use pending_evidence::PendingEvidence;
pub use plan::DeclaredPlan;
pub use ports::{PortClaim, PortClaimant};
pub use preferences::Preferences;
pub use process::DroneProcess;
pub use proving::Proved;
pub use read::{Loaded, RowIdentity, StatusRepair, UnreadableRow};
pub use report::Report;
pub use resolving::{NamedJob, ResolveJobError};
pub use retain::Retained;
pub use review::Review;
pub use showing::KeptFrame;
pub use shown_again::{ShownAgain, SpecNamed};
pub use spend::{DroneSpend, PastSpend, Spend};
pub use studio::{DispatchedFrom, JobOnStudio, StudioError, Unreadable, UnreadableContent};
pub use work_plan::{PlanHand, PlanNotKept};
