//! The wire vocabulary, as DTOs rather than domain types.
//!
//! `From<core_model::Job> for ipc::JobSummary` at the Fleet boundary is where
//! redaction becomes an explicit, visible step. **A domain type on the wire is a
//! redaction decision nobody made.**
//!
//! `PROTOCOL_VERSION` is emitted by `build.rs` from `protocol-version.toml` at
//! the repo root, and a codegen step emits matching TypeScript from this crate's
//! types. Both generated outputs are checked in, so a cross-language breaking
//! change is a build failure rather than a runtime surprise.
//!
//! One of two crates permitted to deserialize, because this is where bytes
//! arrive from outside.
//!
//! # Where the conversion lives, and why it is here rather than in Fleet
//!
//! `docs/practices/protocol.md` sketches the redaction as Fleet's — and the
//! *decision* is, since Fleet is the only caller. The `impl` cannot be: neither
//! `core_model::Job` nor `ipc::JobSummary` belongs to `fleet`, so the orphan
//! rule puts every `From` at this boundary in this crate. What Fleet keeps is
//! the choice of which conversion to call and what to pass it.
//!
//! # Nothing here restates a spelling
//!
//! `core-model` carries an `as_wire`/`from_wire` pair beside every enum and the
//! wire value **is** the registry key. Every closed set below holds the domain
//! value and spells it through that pair. There is no variant-to-string `match`
//! in this crate, which is the defect just removed from `store` and the one a
//! second vocabulary always becomes.
//!
//! # What this crate is not
//!
//! Not the full protocol surface. `operations.toml` inventories every
//! operation; the types here serve the ones M1 needs — `list_jobs`,
//! `get_job`, `propose_job`, `approve_dispatch`, `kill_drone`, `kill_job`,
//! `redispatch_job` and the event stream. Neither kill adds a type: both name
//! a Job and answer with one. `redispatch_job` adds [`Redispatched`], because
//! it is the one command that leaves two Jobs behind. The `/v0` lifeboat is
//! the Ship milestone's and is deliberately absent rather than stubbed: a
//! lifeboat that shares a type with the protocol it is the lifeboat for is not
//! one. [`Skew`] decides when Bridge needs it.

mod checks;
mod codec;
mod detail;
mod enums;
mod error;
mod event;
mod ids;
mod job;
/// The Evidence tool's transport. **A different seam** — Fleet to Drone, not
/// Fleet to Bridge — so it is a module rather than a flat re-export and none of
/// its types are in `operations.toml`.
pub mod mcp;
mod setup;
mod turn;
/// The two numbers both sides read, and what a mismatch between them means.
/// `build.rs` embeds them from `protocol-version.toml`.
mod version;

#[cfg(test)]
mod tests;

pub use checks::{CheckRun, DeclaredCheck};
pub use codec::{decode, encode, Undecodable, Unencodable};
pub use detail::{Criterion, Dependency, JobDetail, StepDetail, StepFacts, Verdict};
pub use enums::{
    Actor, CheckOutcome, CriterionSource, DependencyDirection, JobStatus, Origin, StepState,
    TopLevelOrigin, Urgency,
};
pub use error::{RunId, WireError, WireValue};
pub use event::{
    Cursor, Delivered, DroneExited, DroneSpawned, Event, JobCreated, JobStateChanged,
    JobStepAdvanced, Missed, Reason, Resync, StreamMessage,
};
pub use ids::{CriterionId, DroneId, Instant, JobId, ManifestId, StepId, WorkflowId};
pub use job::{
    JobList, JobSummary, ProposeJob, ProposedCriterion, Redispatched, Subject, UnreadableJob,
};
pub use setup::{ManifestSummary, ModelChoices, WorkflowStep, WorkflowSummary};
pub use turn::{Closed, Opened, Saw, Shown, Silence, TranscriptRow, TurnMessage, Withheld};
pub use version::{ProtocolVersion, Skew, PROTOCOL_VERSION};
