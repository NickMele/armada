//! The daemon: the scheduler, Drone lifecycle, worktrees, the Job-shape
//! classifier, and the Evidence MCP the Drone reports through.
//!
//! Daemon OS lifecycle is its own module, orthogonal to scheduling — v1 mixed
//! them and its `schedule.rs` alone reached 2,929 lines.
//!
//! # Two things that are not details
//!
//! **`libc::setsid()` at every Drone spawn, always.** launchd signals a job's
//! whole process tree, so an undetached Drone dies at every Fleet restart,
//! silently and mid-Job.
//!
//! **Fleet never auto-kills.** Anything escalated is paused with its worktree
//! held as-is. Killing is exclusively a human action.
//!
//! Fleet is also the only writer of Job state, and it reaches the agent only
//! through `adapter-traits` — never the real CLI directly, anywhere.
//!
//! # What is built so far
//!
//! The OS lifecycle only: the runtime file Bridge finds Fleet by
//! ([`runtime`](mod@runtime)), the identity check that makes that file's pid
//! mean something ([`process`](mod@process)), and the detached spawn every
//! Drone gets ([`detach`](mod@detach)). Scheduling, worktrees, the classifier
//! and the Evidence MCP are later steps, and none of them is stubbed here.
//!
//! `api::Daemon` is not implemented yet either, which is why `fleet-bin` binds
//! the listener and does not serve on it. A trait implementation that answered
//! every request with a fault would be a placeholder a client cannot tell from
//! a fault, and `api`'s own route table says why that is worse than nothing.

pub mod detach;
pub mod process;
pub mod runtime;

#[cfg(test)]
mod tests;

pub use detach::Detached;
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use runtime::{
    machine_path, provisional_address, Presence, Published, PublishError, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME, PROVISIONAL_PORT,
};
