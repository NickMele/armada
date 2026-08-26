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

//! # `no_std`, except under test
//!
//! Conditional for the one reason `core-model`'s is: the unit test harness
//! needs `std` to link. Every shipped build of this crate is `no_std` and
//! depends on nothing, which is what `cargo tree -p adapter-traits` shows.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod secret;
mod worktree;

pub use secret::Secret;
pub use worktree::{derived, Worktree, WorktreeSpec, WorktreeSpecRefused};

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

/// Version control, as Fleet uses it.
///
/// # It has no push method, and that is not the only thing missing
///
/// **No push.** A Drone commits locally inside its own worktree; push, pull
/// request and merge are Fleet's, with credentials Fleet holds. A capability
/// absent from the type cannot be reached by a Drone that reasons its way
/// around a denial. What a Drone itself is handed is a narrower type again,
/// which is not this trait.
///
/// **No removal, and no way to ask for one.** Removal is driven by Job
/// retention, never by a process ending — so the caller that would decide is
/// the retention pass, and there is no retention in M1. Rather than shipping a
/// method nothing may call yet, the method does not exist: a worktree survives
/// every terminal state because nothing in the workspace can delete one, not
/// because everybody remembered not to. Worktrees accumulate and a person
/// removes them by hand, which is the evidence M1 is collecting. The method
/// arrives with the pass that is allowed to call it.
///
/// **No "does it already exist" query.** Creation answers that itself, by
/// refusing; a separate probe would be a check-then-act that two Jobs can
/// interleave.
pub trait Vcs {
    /// Errors this implementation can raise. Named by the implementation, so a
    /// caller matching a branch collision against a disk failure is matching on
    /// something real rather than on a rendered sentence.
    type Error;

    /// Create a Job's worktree on its own new branch.
    ///
    /// Called at approval, **before any Drone exists**. A failure here is a
    /// failed Job with nothing spawned, which is why the error type is the
    /// caller's to match on rather than something to log.
    ///
    /// The branch is derived from the Job id and is **new**: an existing branch
    /// of that name is refused, never checked out. v1 discovered why on a real
    /// machine — a worktree added onto an existing branch interleaves two Jobs'
    /// commits, and git's own refusal names the operation rather than the
    /// reason.
    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, Self::Error>;
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
    //! **Not the design.** These names are what the traits above take and
    //! return, and they belong in `core-model` beside the rest of the domain.
    //! They are not written yet because M1 step 1 owns the Job record and its
    //! neighbours, and fixing their shape here would settle that by accident.
    //!
    //! They exist as uninhabited types so the traits compile and the seam is
    //! readable. Replace them, do not grow them.
    //!
    //! **`WorktreeSpec` and `WorktreePath` were two of them and are gone.** M1
    //! step 7 replaced them with real types in [`worktree`](super::worktree),
    //! not in `core-model` — the domain crate takes no dependency on a
    //! filesystem layout, and this crate must derive the path without one.

    macro_rules! not_yet {
        ($($name:ident),* $(,)?) => {$(
            #[derive(Debug)]
            pub enum $name {}
        )*};
    }

    not_yet!(
        DroneSpawnConfig,
        DroneHandle,
        ModelRequest,
        ModelResponse,
        HealthReport,
    );
}
