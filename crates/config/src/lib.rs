//! Kit and Manifest resolution, and the merge strategies between them.
//!
//! Owns scan, propose, select and verify — the part of v1 that ported most
//! cleanly — plus Check and Command definitions, and the Kit and Manifest health
//! probes, which read and validate their own files.
//!
//! A Manifest is an `armada.yml` at a workspace root, version-controlled with
//! the project it configures. Path ownership is nearest-ancestor: the nearest
//! `armada.yml` up the tree owns a path, and the root owns whatever no Workspace
//! claims.
//!
//! # What is built, and what is refused
//!
//! M1 reads **seven keys** from an `armada.yml` — `version`, `id`,
//! `checks.<name>.run`, `checks.<name>.when`, `commands.<name>.run`,
//! `commands.<name>.destructive`, `setup.requires` —
//! and **five fields** of a WorkflowDef. Everything else in either schema is an
//! unknown key and hard-fails.
//!
//! That is the decision worth restating, because it looks like the parser is
//! unfinished and it is not. A key nothing reads is a promise the file makes
//! and the system does not keep: an `armada.yml` carrying a `budget:` section
//! that no code consumes reads to its author as a budget that is set. Refusing
//! it means each deferred section lands with the code that honours it, and
//! stays additive rather than becoming a migration of every file already
//! written.
//!
//! # Bytes enter here, and JSON does not
//!
//! Gate rule five scopes untyped JSON reading to `store` and `ipc`. Nothing
//! here parses JSON: `armada.yml` and a WorkflowDef are YAML, and the document
//! is walked as an untyped value rather than deserialized into a struct. See
//! [`yaml`](mod@yaml)'s own comment for why a derive could not carry the
//! refusals this crate has to produce.
//!
//! # Nothing here reads a clock
//!
//! Parsing and validation are pure. There is no mtime check, no staleness
//! window and no cache expiry — a Manifest is as fresh as the last time
//! somebody read it, and who decides to read it again is not this crate's
//! question.

mod error;
mod judge;
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
// four are spelled in `core-model` where the record is — and every caller that
// already said `config::ResolvedStep` still means the same type.
pub use core_model::{
    AdvanceGate, ContextSource, Covers, DeclarePlanAt, DeclaredPaths, EvidenceScope, EvidenceType,
    FrozenWorkflow, JudgeCheck, JudgeCriterion, PathPattern, ResolvedCheck, ResolvedStep,
};
