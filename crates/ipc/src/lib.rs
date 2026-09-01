//! The wire vocabulary, as DTOs rather than domain types.
//! `From<core_model::Job> for ipc::JobSummary` at the Fleet boundary is where
//! redaction becomes an explicit, visible step. **A domain type on the wire is
//! a redaction decision nobody made.**
//!
//! `PROTOCOL_VERSION` is emitted by `build.rs` from `protocol-version.toml`, and
//! codegen emits matching TypeScript from these types. Both outputs are checked
//! in, so a cross-language breaking change is a build failure rather than a
//! runtime surprise. This is one of two crates permitted to deserialize.
//!
//! **The conversion lives here rather than in Fleet.** The *decision* is
//! Fleet's, since Fleet is the only caller, but the `impl` cannot be: neither
//! `core_model::Job` nor `ipc::JobSummary` belongs to `fleet`, so the orphan
//! rule puts every `From` at this boundary here. What Fleet keeps is which
//! conversion to call and what to pass it.
//!
//! **Nothing here restates a spelling.** `core-model` carries an
//! `as_wire`/`from_wire` pair beside every enum and the wire value **is** the
//! registry key, so every closed set below spells the domain value through it.
//! There is no variant-to-string `match` here — the defect just removed from
//! `store`, and the one a second vocabulary always becomes.
//!
//! **Not the full protocol surface.** `crates/ipc/operations.toml` inventories
//! every operation; the types here serve what M1 needs, and a command adds a
//! type only where a Job is not what it answers with.

/// How many times a step was worked, and what each run came to. **The record
/// held it and nothing served it** — see the module.
mod attempt;
/// How many Drones Fleet may run, how many it is running, and what holds the
/// next one back. **Fleet-wide, and not a Job's field.**
mod capacity;
mod checks;
mod codec;
mod detail;
mod enums;
mod error;
mod event;
mod history;
mod ids;
mod job;
mod judged;
/// The Evidence tool's transport. **A different seam** — Fleet to Drone, not
/// Fleet to Bridge — so it is a module rather than a flat re-export and none of
/// its types are in `operations.toml`.
pub mod mcp;
/// Where two Jobs claim the same paths. **A fact on the card, never a
/// verdict** — nothing in it is readable as a refusal.
mod overlap;
/// What a person says went wrong, with the Job's own record attached.
mod report;
mod setup;
mod turn;
/// The two numbers both sides read, and what a mismatch between them means.
/// `build.rs` embeds them from `protocol-version.toml`.
mod version;
/// What is outstanding on a live Drone, and what a person sends it back.
mod waiting;
/// The material a reviewing person reads, and what their note carries.
mod work;

#[cfg(test)]
mod tests;

pub use attempt::{Move, StepAttempt};
pub use capacity::{AdmissionHold, FleetCapacity};
pub use checks::{CheckRun, DeclaredCheck, DeclaredJudge};
pub use codec::{decode, encode, Undecodable, Unencodable};
pub use detail::{
    Criterion, Dependency, JobDelivery, JobDetail, JobSpend, JudgeInFlight, StepDetail, StepFacts,
    Stuck, Verdict,
};
pub use enums::{
    Actor, AdvanceGate, CheckOutcome, CriterionSource, DependencyDirection, DronePresence,
    EvidenceType, JobStatus, JudgeVerdict, Origin, QueuedReason, Recourse, Resumption, StepState,
    TopLevelOrigin, Urgency,
};
pub use error::{RunId, WireError, WireValue};
pub use event::{
    ChangeKind, ChangedFile, Cursor, Delivered, DroneExited, DroneSpawned, Event, JobAsking,
    JobCreated, JobFilesChanged, JobJudging, JobStateChanged, JobStepAdvanced, Missed, Reason,
    Resync, StreamMessage,
};
pub use history::{DroneMoved, JobHistory, Movement, Recorded, StatusMoved, StepMoved};
pub use ids::{CriterionId, DroneId, Instant, JobId, ManifestId, QuestionId, StepId, WorkflowId};
pub use job::{
    AttachmentRef, DependencyEdge, JobForgotten, JobList, JobRequest, JobSummary, ProposeJob,
    ProposedCriterion, ProposedPlan, Redirection, Redispatched, Subject, UnreadableJob,
};
pub use judged::{CitedAt, Flagged, Judged, KeptDeliverable};
pub use overlap::{ScopeOverlap, SharedPath};
pub use report::{Calibration, Claim, FileReport, Report, ReportId, ReportList, ReportOrigin};
pub use setup::{ManifestSummary, ModelChoices, WorkflowStep, WorkflowSummary};
pub use turn::{
    CallArguments, Closed, Opened, Saw, Shown, Silence, TranscriptRow, TurnMessage, Voice, Withheld,
};
pub use version::{ProtocolVersion, Skew, PROTOCOL_VERSION};
pub use waiting::{AskedOption, ChosenAnswer, QuestionInFlight, RedirectInFlight, RedirectWaiting};
pub use work::{
    ChangesRequested, DeclaredPlan, JobDiff, JobEvidence, JobFootprint, LineCount, Overruled,
    Submitted, TouchedFile, Work,
};
