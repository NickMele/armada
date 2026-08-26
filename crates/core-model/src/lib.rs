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
//! The Job record and its status machine are here too, built from
//! `domain/`, which is the authority on both. What is *not* here is the inner
//! step machine: the registry gives step states no edge table, so `job_steps`
//! rows are written at creation and nothing yet advances one. [`Job`] and
//! [`JobStatus`] carry the reasoning.
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
pub use job::{
    AcceptanceCriterion, CriteriaOwed, CriterionId, CriterionSource, DependencyDirection,
    DependencyEdge, DispatchOrigin, DroneId, Edge, EscalationTrigger, Facts, GateManifest,
    GateOutcome, IllegalTransition, Job, JobEvent, JobId, JobStatus, JobStep, ManifestId,
    ModelName, NewJob, NotRunDisposition, NotRunReason, Origin, PilotReason, RepoPath,
    ScopeRevision, ScopeRevisionOutcome, StepId, StepSeed, StepState, StepVerdict, Subject, Target,
    TopLevelOrigin, TransitionReason, Transitioned, TriggerKind, TriggerLevel, Urgency, WorkflowId,
    WriteTargets, EDGES,
};
