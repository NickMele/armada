//! The seams: the traits an adapter implements, and nothing that implements
//! them.
//!
//! Five boundaries, one trait each, and `Secret<T>`. The implementations live
//! in `adapters`, which is the only crate permitted to know whose API it is
//! talking to — a vendor's name outside `adapters` is the boundary having
//! leaked, and it leaks in comments first.
//!
//! # What may not enter this crate
//!
//! No runtime, no I/O, no vendor, checked by `cargo tree`. A trait that needs
//! `tokio` to *declare* has already chosen the implementation's runtime for it.
//!
//! # Why the traits are here at all
//!
//! The acceptance test drives a Job with every adapter faked — no process
//! spawned, no repository touched, no network opened. That is only cheap
//! because the seam is a trait: `fleet` must never reach for the real CLI
//! directly, anywhere.

#![no_std]

extern crate alloc;

mod secret;

pub use secret::Secret;

/// The agent harness: what actually runs a Drone.
///
/// One headless agent CLI is the only implementation, and the trait exists so
/// it is not the only *possible* one — and, more immediately, so `testkit` can
/// drive a whole Job from a scripted NDJSON stream. Which CLI is a question
/// only `adapters` is allowed to answer.
pub trait AgentHarness {
    /// Errors this harness can raise. Named by the implementation so a caller
    /// matching on them is matching on something real.
    type Error;

    /// Start a Drone against a prepared worktree.
    ///
    /// The spawn configuration is typed and carries no raw argv builder and no
    /// escape-hatch constructor, so a caller cannot quietly drop the isolation
    /// flags the configuration exists to guarantee.
    fn spawn(&self, config: DroneSpawnConfig) -> Result<DroneHandle, Self::Error>;
}

/// Version control. **This is the Drone-facing shape and it has no push
/// method**, so a Drone cannot push because the call does not exist rather than
/// because a check refuses it.
pub trait Vcs {
    type Error;

    fn add_worktree(&self, spec: WorktreeSpec) -> Result<WorktreePath, Self::Error>;
    fn remove_worktree(&self, path: &WorktreePath) -> Result<(), Self::Error>;
    fn has_uncommitted_work(&self, path: &WorktreePath) -> Result<bool, Self::Error>;
}

/// Credential access, brokered. A Drone never holds a secret directly, and what
/// comes back is a [`Secret`], which cannot be printed.
pub trait Secrets {
    type Error;

    fn resolve(&self, key: &str) -> Result<Secret<alloc::string::String>, Self::Error>;
}

/// A model call that carries no toolset — the Judge, the Job-shape classifier,
/// and generated copy. Distinct from [`AgentHarness`], which carries one.
pub trait ModelClient {
    type Error;

    fn complete(&self, request: ModelRequest) -> Result<ModelResponse, Self::Error>;
}

/// One thing that can be up or down: version control, the container runtime,
/// the agent CLI, the credential store, the database, the host's own resources.
///
/// Which product each of those is, is `adapters`' business and not this crate's
/// — naming one here is how the seam stops being a seam.
///
/// It is a trait so `testkit` can fake an unhealthy container runtime. Without
/// it, every health failure path is untestable — which is the whole reason
/// Doctor's grid is worth having.
pub trait HealthProbe {
    /// The one shape every probe returns, so no crate reaches across a boundary
    /// to read another's status.
    fn probe(&self) -> HealthReport;
}

// The payload types below are named by the traits and defined where the domain
// lives. Declared here as opaque re-exports once `core-model` carries them.
pub use placeholders::*;

#[doc(hidden)]
mod placeholders {
    //! **Not the design.** These five names are what the traits above take and
    //! return, and they belong in `core-model` beside the rest of the domain.
    //! They are not written yet because M1 step 1 owns the Job record and its
    //! neighbours, and fixing their shape here would settle that by accident.
    //!
    //! They exist as uninhabited types so the traits compile and the seam is
    //! readable. Replace them, do not grow them.

    macro_rules! not_yet {
        ($($name:ident),* $(,)?) => {$(
            #[derive(Debug)]
            pub enum $name {}
        )*};
    }

    not_yet!(
        DroneSpawnConfig,
        DroneHandle,
        WorktreeSpec,
        WorktreePath,
        ModelRequest,
        ModelResponse,
        HealthReport,
    );
}
