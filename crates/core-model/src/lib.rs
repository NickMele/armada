//! The vocabulary every other crate agrees on: the Job record, its states and
//! transitions, the escalation triggers, the workflow definition, evidence.
//!
//! **What may not enter this crate: no runtime, no I/O, no vendor.**
//! `core-model` is the one crate every other crate depends on, so a dependency
//! added here is a dependency added everywhere — which is why `cargo tree` on
//! this crate is a gate rule rather than a preference. No async runtime, no VCS
//! library, no HTTP client, reachable at any depth. Serialisation lives here
//! only as derives on types defined here: reading untyped JSON belongs to
//! `store` and `ipc`, the two places bytes enter the process.
//!
//! **The log envelope is here** because a line shape retrofitted after five
//! crates are already logging is a rewrite of all five, and `actor` cannot be
//! reconstructed afterwards at all.
//!
//! **The Job record and both halves of its machine** are here too, built from
//! `domain/`, which is the authority on the outer one. The registry gives step
//! states no edge table, so the inner machine declares its own edges and says
//! on each why it is there — [`StepTarget`] carries the reasoning, and what M1
//! cannot reach is unreachable because no value names it.
//!
//! **`no_std`, except under test.** The attribute is conditional only because
//! the unit test harness needs `std` to link; every shipped build of this crate
//! is `no_std` and depends on nothing.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod envelope;
mod job;

pub use envelope::{
    env_keys, Actor, AuditLine, Component, Envelope, FieldValue, Level, Timestamp, Ulid,
};
pub use job::{collisions, under};
pub use job::{
    handle_of, names_a_credential, AcceptanceCriterion, AdmissionHold, AdvanceGate, Attachment,
    Attempt, AutoMerge, BadPattern, BlankBranch, BlankModel, BlankTitle, Branch, CheckOutcome,
    Citation, CitedAt, Collision, ContextSource, Covers, CriteriaOwed, CriterionId,
    CriterionSource, DecidedBy, DeclarePlanAt, DeclaredPaths, DependencyDirection, DependencyEdge,
    DispatchOrigin, DroneAssigned, DroneId, DroneMoved, DronePresence, DroneStanding, Edge,
    EscalationTrigger, EvidenceRef, EvidenceScope, EvidenceType, Facts, FrozenWorkflow,
    GamingCheck, GamingFlag, GamingPattern, GateManifest, GateOutcome, GateVerdict, Given, Guard,
    IllegalDroneMove, IllegalStepTransition, IllegalTransition, Iteration, Job, JobEvent, JobId,
    JobNumber, JobStatus, JobStep, JudgeCheck, JudgeCriterion, JudgeVerdict, Judgment, ManifestId,
    ModelName, Narrowing, NewJob, NotRunDisposition, NotRunReason, Origin, PathPattern,
    PilotReason, Prerequisite, ProposalId, QueuedReason, Recourse, RedirectAlreadyWaiting,
    RedirectWaiting, Refusal, Refusals, RepoPath, ResolvedCheck, ResolvedStep, Resumption,
    ReviewGate, ScopeClaim, ScopeRevision, ScopeRevisionOutcome, Spent, Standing, StepCheck,
    StepEdge, StepEvent, StepEvidence, StepFrame, StepId, StepLevelTrigger, StepSeed, StepState,
    StepTarget, StepTransitioned, StepVerdict, Stuck, Subject, Target, Title, TopLevelOrigin,
    TransitionReason, Transitioned, TriggerKind, TriggerLevel, Urgency, WorkflowId, WriteTargets,
    ADVANCING_STATUSES, ARTIFACT_EXISTS, CREDENTIAL_NAMES, DIFF_NONEMPTY, EDGES,
    EVERY_MANIFEST_CHECK, MANIFEST_CHECK, STEP_EDGES,
};
