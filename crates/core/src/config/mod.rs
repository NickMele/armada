//! The `armada.yml` contract: the schema, the structs that mirror it, and
//! resolution.
//!
//! **The JSON Schema is authoritative** (PLAN.md §4.1.1, decision 2). The
//! structs in [`model`] are hand-written to match it and neither generates the
//! other; the fixtures under `tests/fixtures/` are what keep them from
//! drifting, because every one of them is parsed by these structs *and*
//! validated against the schema in the same test run.
//!
//! Three stages, and the boundary between them is deliberate:
//!
//! | Stage | What it does | Owner |
//! |---|---|---|
//! | parse | YAML text → [`model::Config`] | here |
//! | resolve | defaults applied, ids derived, shapes normalised | here |
//! | verify | cross-references, the filesystem, argv[0] on PATH | `char config verify`, phase 5 |
//!
//! **Loading is not verifying.** Resolution answers questions it can settle
//! from one value or one entry; anything needing a second part of the document
//! or the filesystem is verify's, and the schema draws the same line.

mod model;
mod resolve;
mod resolved;
mod schema;

pub use model::{
    Check, CommandEntry, Component, Config, Document, OneOrMany, OwnsCommand, OwnsComponent,
    OwnsRun, Ready, Run, SecretProvider, SetupObject, SetupStep,
};
pub use resolve::{declared_workspaces, parse, resolve, Defaults, SECTION};
pub use resolved::{
    Need, ReadyKind, ResolvedCheck, ResolvedCommand, ResolvedComponent, ResolvedConfig,
    ResolvedReady, ResolvedRun, ResolvedService, ResolvedSetupStep, Scope, Stdio,
};
pub use schema::SCHEMA;
