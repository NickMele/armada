//! The vocabulary every other crate agrees on: the Job record, its states and
//! transitions, the escalation triggers, the workflow definition, evidence.
//!
//! # What may not enter this crate
//!
//! No runtime, no I/O, no vendor. `core-model` is the one crate every other
//! crate depends on, so a dependency added here is a dependency added
//! everywhere — that is why `cargo tree` on this crate is a gate rule and not
//! a preference. No async runtime, no VCS library, no HTTP client, reachable
//! at any depth.
//!
//! Serialisation lives here only as derives on types defined here. Reading
//! untyped JSON belongs to `store` and `ipc`, which are the two places bytes
//! enter the process.
//!
//! # What is here, and what is not
//!
//! The log envelope is here, because a line shape retrofitted after five crates
//! are already logging is a rewrite of all five — and `actor` cannot be
//! reconstructed afterwards at all.
//!
//! The Job record and both halves of its machine are here too, built from
//! `domain/`, which is the authority on the outer one. The registry gives step
//! states no edge table, so the inner machine declares the three states M1
//! reaches rather than transcribing a table — [`StepTarget`] carries the
//! reasoning, and the three it cannot reach are unreachable because no value
//! names them.
//!
//! # `no_std`, except under test
//!
//! `#![no_std]` is conditional only because the unit test harness needs `std`
//! to link. Every shipped build of this crate is `no_std` and depends on
//! nothing.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod envelope;
mod job;

pub use envelope::{
    env_keys, Actor, AuditLine, Component, Envelope, FieldValue, Level, Timestamp, Ulid,
};
pub use job::under;
pub use job::{
    AcceptanceCriterion, AdvanceGate, Attachment, Attempt, BadPattern, BlankBranch, BlankModel,
    BlankTitle, Branch, CheckOutcome, ContextSource, Covers, CriteriaOwed, CriterionId,
    CriterionSource, DecidedBy, DeclarePlanAt, DeclaredPaths, DependencyDirection, DependencyEdge,
    DispatchOrigin, DroneAssigned, DroneId, DroneMoved, DronePresence, Edge, EscalationTrigger,
    EvidenceRef, EvidenceScope, EvidenceType, Facts, FrozenWorkflow, GamingCheck, GamingFlag,
    GamingPattern, GateManifest, GateOutcome, Guard, IllegalDroneMove, IllegalStepTransition,
    IllegalTransition, Job, JobEvent, JobId, JobStatus, JobStep, JudgeCheck, JudgeCriterion,
    JudgeVerdict, Judgment, ManifestId, ModelName, NewJob, NotRunDisposition, NotRunReason, Origin,
    PathPattern, PilotReason, QueuedReason, Recourse, RepoPath, ResolvedCheck, ResolvedStep,
    ScopeRevision, ScopeRevisionOutcome, Standing, StepCheck, StepEdge, StepEvent, StepEvidence,
    StepId, StepLevelTrigger, StepSeed, StepState, StepTarget, StepTransitioned, StepVerdict,
    Stuck, Subject, Target, Title, TopLevelOrigin, TransitionReason, Transitioned, TriggerKind,
    TriggerLevel, Urgency, WorkflowId, WriteTargets, ADVANCING_STATUSES, DIFF_NONEMPTY, EDGES,
    MANIFEST_CHECK, STEP_EDGES,
};
