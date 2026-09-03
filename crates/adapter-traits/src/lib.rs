//! The seams: the traits an adapter implements, and nothing that implements
//! them.
//!
//! One trait per boundary, and `Secret<T>`. Version control splits into three
//! of them — [`Vcs`] creates a worktree at approval, [`WorkProduct`] reads one
//! at the gate, [`Delivery`] publishes what a finished one holds — because each
//! is held by a different caller and none may reach another's methods. The implementations live
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

mod commit;
mod delivery;
mod event;
mod harness;
mod judge;
mod secret;
mod work_product;
mod worktree;

pub use commit::{CommitTime, Committed};
pub use delivery::{
    how_the_base_was_found, Base, BroughtUpToDate, Delivery, Landed, NotDelivered, Opened, Pushed,
    Review, Standing,
};
pub use event::{CallDetail, DroneEvent, Speaker};
pub use harness::{
    AmbientServers, DroneHandle, DroneSpawnConfig, Environment, Grant, Launch, McpConfig, Model,
    Prompt, Prompting, SpawnConfigRefused, Toolbelt,
};
pub use judge::{Ask, JudgeCall, ModelClient};
pub use secret::Secret;
pub use work_product::{
    Change, Changed, ChangedFile, Counted, CountedFile, Footprint, LineCount, Patch, WorkProduct,
};
pub use worktree::{derived, Worktree, WorktreeSpec, WorktreeSpecRefused};

/// The agent harness: what a Drone is started as, and what its output means.
///
/// One headless agent CLI is the only implementation, and the trait exists so
/// it is not the only *possible* one — and, more immediately, so `testkit` can
/// drive a whole Job from a scripted stream. Which CLI is a question only
/// `adapters` is allowed to answer.
///
/// # Neither method starts a process
///
/// [`render`](AgentHarness::render) produces a [`Launch`]; Fleet starts it,
/// detached, through the one type in the workspace that can start anything. A
/// harness that spawned could spawn attached, and every confinement property
/// would then only be checkable by spawning the thing being confined. As it
/// stands they are assertions on a value.
///
/// **A second implementation is not cheap, and the trait is shaped to say so.**
/// The argument list *is* the permission model: what a Drone may run, what is
/// refused however broadly the allowlist grants, whether ambient servers are
/// excluded. A second harness re-expresses every one of those in its own
/// vocabulary, and the failure mode when it does not is the worst one in the
/// system — a missing capability does not fail, it waits. So
/// [`DroneSpawnConfig`] has no escape-hatch constructor: a second
/// implementation does not compile until it has answered every capability
/// question the first one answers.
pub trait AgentHarness {
    /// Errors this harness can raise. Named by the implementation so a caller
    /// matching on them is matching on something real.
    type Error;

    /// How this harness would start a Drone against a prepared worktree.
    ///
    /// The configuration is typed and carries no raw argument builder and no
    /// escape-hatch constructor, so a caller cannot quietly drop the isolation
    /// the configuration exists to guarantee.
    fn render(&self, config: &DroneSpawnConfig) -> Result<Launch, Self::Error>;

    /// Read one line of the Drone's output.
    ///
    /// **Total, and never empty.** There is no `Result`: a line that does not
    /// decode comes back as [`DroneEvent::Unreadable`], because a decoder that
    /// can return an error is a decoder whose caller can drop the line. A line
    /// this vocabulary has no name for comes back as
    /// [`DroneEvent::Unrecognised`] for the same reason.
    ///
    /// **One line can be more than one event.** A turn carrying two tool calls
    /// is two, and answering with only the first would drop work the Drone did.
    fn read(&self, line: &str) -> alloc::vec::Vec<DroneEvent>;
}

/// Version control, as Fleet uses it.
///
/// # It has no push method, and that is not the only thing missing
///
/// **No push.** Push, pull request and merge are [`Delivery`]'s, held by Fleet
/// with the operator's credentials. A capability absent from the type cannot be
/// reached by a Drone that reasons its way around a denial. What a Drone itself
/// is handed is a narrower type again, which is neither trait — and it carries
/// no `git` at all, which is why the commit below is here.
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
    /// Errors creating a worktree can raise. Named by the implementation, so a
    /// caller matching a branch collision against a disk failure is matching on
    /// something real rather than on a rendered sentence.
    type Error;

    /// Errors committing can raise, and **a second type rather than more
    /// variants of the first.** Creation fails at approval with no Drone and no
    /// work; a commit fails after a Job's Checks have passed and its work is on
    /// disk. Neither list is a list of things to do about the other.
    type CommitError;

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

    /// Commit everything in a Job's worktree onto the branch checked out in it.
    ///
    /// Called when a Job's **last** step advances, by Fleet, which is this
    /// trait's only holder. It is not on [`WorkProduct`]: the gate holds that
    /// one, and a gate that could commit could satisfy `diff_nonempty` on the
    /// Drone's behalf.
    ///
    /// Everything git can see is taken, untracked files included and ignored
    /// files excluded — the same set [`WorkProduct::changed_files`] counts, so
    /// what made `diff_nonempty` pass is what lands.
    fn commit_all(
        &self,
        worktree: &Worktree,
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError>;
}

/// Credential access, brokered. A Drone never holds a secret directly, and what
/// comes back is a [`Secret`], which cannot be printed.
pub trait Secrets {
    type Error;

    fn resolve(&self, key: &str) -> Result<Secret<alloc::string::String>, Self::Error>;
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
    //!
    //! **`DroneSpawnConfig` and `DroneHandle` are gone too**, replaced by M1
    //! step 8 in [`harness`](super::harness), for the same reason and in the
    //! same place: what a Drone is confined to is derived from a worktree and a
    //! toolbelt, and neither is a domain fact.
    //!
    //! **`ModelRequest` and `ModelResponse` are gone**, replaced by
    //! [`judge`](super::judge)'s `Ask` and `JudgeCall`: what a one-shot call
    //! carries is a rendering question, not a domain one.

    macro_rules! not_yet {
        ($($name:ident),* $(,)?) => {$(
            #[derive(Debug)]
            pub enum $name {}
        )*};
    }

    not_yet!(HealthReport);
}
