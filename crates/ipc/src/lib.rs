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
//! `get_job`, `get_job_events`, `propose_job`, `propose_from_request`,
//! `approve_dispatch`, `kill_drone`, `kill_job`, `redispatch_job`, the three
//! acts a person takes on finished work — `approve_review`, `request_changes`,
//! `reject_job` — with `get_evidence` and `get_diff`, which are the material
//! those acts are taken on, and `override_verdict`, which is the act on work a
//! gate refused rather than work a gate held, and the event
//! stream. Neither
//! kill adds a type: both name a Job and answer with one. `redispatch_job`
//! adds [`Redispatched`] because it is the one command that leaves two Jobs
//! behind, and `propose_from_request` adds [`JobRequest`] because it is the one
//! that carries no Job at all. The `/v0` lifeboat is
//! the Ship milestone's and is deliberately absent rather than stubbed: a
//! lifeboat that shares a type with the protocol it is the lifeboat for is not
//! one. [`Skew`] decides when Bridge needs it.

mod checks;
mod codec;
mod detail;
mod enums;
mod error;
mod event;
mod history;
mod ids;
mod job;
/// The Evidence tool's transport. **A different seam** — Fleet to Drone, not
/// Fleet to Bridge — so it is a module rather than a flat re-export and none of
/// its types are in `operations.toml`.
pub mod mcp;
/// What a person says went wrong, with the Job's own record attached.
mod report;
mod setup;
mod turn;
/// The two numbers both sides read, and what a mismatch between them means.
/// `build.rs` embeds them from `protocol-version.toml`.
mod version;
/// The material a reviewing person reads, and what their note carries.
mod work;

#[cfg(test)]
mod tests;

pub use checks::{CheckRun, DeclaredCheck, DeclaredJudge};
pub use codec::{decode, encode, Undecodable, Unencodable};
pub use detail::{
    Criterion, Dependency, Flagged, JobDetail, JudgeInFlight, Judged, RedirectInFlight, StepDetail,
    StepFacts, Stuck, Verdict,
};
pub use enums::{
    Actor, AdvanceGate, CheckOutcome, CriterionSource, DependencyDirection, DronePresence,
    EvidenceType, JobStatus, JudgeVerdict, Origin, QueuedReason, Recourse, StepState,
    TopLevelOrigin, Urgency,
};
pub use error::{RunId, WireError, WireValue};
pub use event::{
    ChangeKind, ChangedFile, Cursor, Delivered, DroneExited, DroneSpawned, Event, JobCreated,
    JobFilesChanged, JobJudging, JobStateChanged, JobStepAdvanced, Missed, Reason, Resync,
    StreamMessage,
};
pub use history::{DroneMoved, JobHistory, Movement, Recorded, StatusMoved, StepMoved};
pub use ids::{CriterionId, DroneId, Instant, JobId, ManifestId, StepId, WorkflowId};
pub use job::{
    AttachmentRef, DependencyEdge, JobList, JobRequest, JobSummary, ProposeJob, ProposedCriterion,
    ProposedPlan, Redirection, Redispatched, Subject, UnreadableJob,
};
pub use report::{Calibration, Claim, FileReport, Report, ReportId, ReportList, ReportOrigin};
pub use setup::{ManifestSummary, ModelChoices, WorkflowStep, WorkflowSummary};
pub use turn::{Closed, Opened, Saw, Shown, Silence, TranscriptRow, TurnMessage, Withheld};
pub use version::{ProtocolVersion, Skew, PROTOCOL_VERSION};
pub use work::{
    ChangesRequested, DeclaredPlan, JobDiff, JobEvidence, JobFootprint, Overruled, Submitted,
    TouchedFile, Work,
};
