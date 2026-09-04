//! Manifest and WorkflowDef resolution: two files, checked against each other.
//!
//! Owns scan, propose, select and verify, plus Check and Command definitions.
//! A Manifest is an `armada.yml` at a workspace root, version-controlled with
//! the project it configures; the nearest one up the tree owns a path, and the
//! root owns what no Workspace claims. **There is no Kit**, though this line
//! said so until `#60`: `settings.toml` resolves most values *Kit → Manifest*,
//! nothing here parses one, and the tiers are a constant, this Manifest, a step.
//!
//! **Everything either schema holds and this crate does not read is an unknown
//! key, and hard-fails.** [`manifest`](mod@manifest) and
//! [`workflow`](mod@workflow) each name the keys they take. That looks like an
//! unfinished parser and is not: a key nothing reads is a promise the file
//! makes and the system does not keep, so refusing it means each deferred
//! section lands with the code that honours it, and stays additive rather than
//! becoming a migration of every file already written.
//!
//! **Bytes enter here, and JSON does not.** Gate rule five scopes untyped JSON
//! reading to `store` and `ipc`; `armada.yml` and a WorkflowDef are YAML, walked
//! as an untyped value rather than deserialized into a struct —
//! [`yaml`](mod@yaml) holds why a derive could not carry the refusals.
//!
//! **Nothing here reads a clock.** Parsing and validation are pure: no mtime
//! check, no staleness window, no cache expiry. A Manifest is as fresh as the
//! last time somebody read it, and who reads it again is not asked here.

mod error;
mod judge;
mod loops;
mod manifest;
mod resolve;
mod roster;
mod scope;
mod workflow;
mod yaml;

#[cfg(test)]
mod tests;

pub use error::{Fault, LoadError, Refusal, ResolveError, UnknownCheck};
pub use manifest::{Check, Command, Manifest, Preparation};
pub use resolve::ResolvedWorkflow;
pub use roster::Roster;
pub use workflow::{MechanicalCheck, Step, Structure, WorkflowDef};

// Re-exported, not re-declared. A Job carries its resolved workflow, so these
// are spelled in `core-model` where the record is — and every caller that
// already said `config::ResolvedStep` still means the same type. `GateVerdict`
// joined them when a step's routing became something a Job freezes.
pub use core_model::{
    AdvanceGate, ContextSource, Covers, DeclarePlanAt, DeclaredPaths, EvidenceScope, EvidenceType,
    FrozenWorkflow, GateVerdict, JudgeCheck, JudgeCriterion, PathPattern, ResolvedCheck,
    ResolvedStep,
};
