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
/// A command a Drone was not given, and what a person answers about it. **Two
/// paths to one answer** — while the Drone waits, or after the Job stopped.
mod commanding;
mod detail;
mod enums;
mod error;
mod event;
/// What `search_files` found under the checkout, for the `@` mention popup.
mod files;
mod history;
/// What Fleet is holding disk for, and the test each one did not pass.
/// **A piloted worktree is not on this wire at all** — `#367`.
mod holding;
mod ids;
mod job;
/// What Fleet did to a Job, out of the Job's own log. **The third voice the
/// activity log was designed around and nothing produced.**
mod journal;
mod judged;
/// The Evidence tool's transport. **A different seam** — Fleet to Drone, not
/// Fleet to Bridge — so it is a module rather than a flat re-export and none of
/// its types are in `operations.toml`.
pub mod mcp;
/// Where two Jobs claim the same paths. **A fact on the card, never a
/// verdict** — nothing in it is readable as a refusal.
mod overlap;
mod proposing;
/// A new cost ceiling for one Job, and which surface asked for it.
mod raising;
/// Fleet's last read of `armada.yml`, held rather than announced. **The one
/// shape here about the fleet and not about a Job**, beside `capacity`.
mod reading;
/// What giving one Job's worktree and branch back did, half by half.
/// **Two halves, because half of it happening is a real outcome.**
mod reclaimed;
/// A person's run of one Manifest entry in a Job's worktree. **A rehearsal,
/// never a verdict** — nothing in it is a Check row or Evidence.
mod rehearsal;
/// What people wrote on a Job's pull request, and which of it a person picks.
/// **The one place this seam carries text from outside this machine.**
mod remarks;
/// What a person says went wrong, with the Job's own record attached.
mod report;
/// What one Job holds on this machine, and what came of asking whether it is
/// working. **The other axis from `spend`**, which answers the model's cost.
mod resources;
/// A Command that stays running, held by Fleet. **Lifecycle on `/events`,
/// output on a socket of its own.**
mod servers;
mod setup;
/// What a step's harness produced, as a client is told about it.
mod showing;
mod turn;
/// A step's Checks while the gate is running them, and the socket a running
/// Check's log is read over.
mod underway;
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
pub use checks::{CheckOutput, CheckRun, DeclaredCheck, DeclaredJudge};
pub use codec::{decode, encode, Undecodable, Unencodable};
pub use commanding::{
    AllowedCommandRow, AnswerCommand, CommandAnswer, CommandInFlight, Reach, RemoveAllowedCommand,
    SetModel, SetWhenBlocked, WhenBlocked,
};
pub use detail::{
    Criterion, Currency, Dependency, JobDelivery, JobDetail, JobReview, JobSpend, JudgeInFlight,
    PullRequestDetail, Refusal, ReviewedBy, Settled, StepDetail, StepFacts, Stuck, Verdict,
};
pub use enums::{
    Actor, AdvanceGate, BudgetHold, CheckOutcome, CriterionSource, DependencyDirection,
    DronePresence, EvidenceType, JobStatus, JudgeVerdict, Origin, QueuedReason, Recourse,
    Resumption, Side, StepState, TopLevelOrigin, Urgency,
};
pub use error::{RunId, WireError, WireValue};
pub use event::{
    ChangeKind, ChangedFile, Cursor, Delivered, DroneExited, DroneSpawned, Event, JobAsking,
    JobChecking, JobCommandWaiting, JobCreated, JobFilesChanged, JobJudging, JobLanded,
    JobRemarksChanged, JobStateChanged, JobStepAdvanced, Missed, ProposalMoved, Reason, Resync,
    StreamMessage,
};
pub use files::FilesFound;
pub use history::{DroneMoved, JobHistory, Movement, Recorded, StatusMoved, StepMoved};
pub use holding::{HeldReason, WorktreeHeld, WorktreesHeld};
pub use ids::{
    CriterionId, DroneId, Instant, JobId, ManifestId, ProposalId, QuestionId, StepId, WorkflowId,
};
pub use job::{
    AttachmentRef, DependencyEdge, JobForgotten, JobList, JobRequest, JobSummary, ProposeJob,
    ProposedCriterion, ProposedPlan, Redirection, Redispatched, RestartRequested, Subject,
    UnreadableJob,
};
pub use journal::{
    JournalClosed, JournalMessage, JournalOpened, LogNote, NoteLevel, NotedField, Quiet,
};
pub use judged::{Citation, CitedAt, Flagged, Given, Judged, KeptDeliverable};
pub use overlap::{ScopeOverlap, SharedPath};
pub use proposing::{ProposalInFlight, ProposalReach, ProposalStopped, StopProposal};
pub use raising::{CapRaise, RaisedBy, TurnRaise};
pub use reading::{ManifestFault, ManifestMoved, ManifestReading, ManifestRefused};
pub use reclaimed::{ReclaimedBranch, ReclaimedWorktree, WorktreeReclaimed};
pub use rehearsal::{
    NamedRun, RunEntry, RunList, RunMessage, RunOpened, RunOutput, RunRecord, RunSheet,
    RunUnderway, StartRun, UnreadableRun,
};
pub use remarks::{InlineContext, JobRemarks, Remark, RemarksTakenUp};
pub use report::{Calibration, Claim, FileReport, Report, ReportId, ReportList, ReportOrigin};
pub use resources::{
    Asked, Finding, Held, JobExamined, JobProcess, JobResources, Look, WorktreeOnDisk,
};
pub use servers::{
    NamedServer, ServerEntry, ServerLink, ServerList, ServerMessage, ServerOpened, ServerPhase,
    ServerPort, ServerState, StartServer, StartedBy,
};
pub use setup::{ManifestSummary, ModelChoices, WorkflowStep, WorkflowSummary};
pub use showing::{KeptFrame, NamedSpec, ShowAgain, ShownAgain, ShownSet};
pub use turn::{
    BlockKind, CallArguments, Closed, Opened, Saw, Shown, Silence, TranscriptRow, TurnMessage,
    Voice, Withheld,
};
pub use underway::{
    CheckUnderway, ChecksUnderway, OutputClosed, OutputEnded, OutputLines, OutputMessage,
    OutputOpened,
};
pub use version::{ProtocolVersion, Skew, PROTOCOL_VERSION};
pub use waiting::{AskedOption, ChosenAnswer, QuestionInFlight, RedirectInFlight, RedirectWaiting};
pub use work::{
    ChangesRequested, DeclaredPlan, JobDiff, JobEvidence, JobFootprint, LineCount, Overruled,
    Submitted, TouchedFile, Work,
};
