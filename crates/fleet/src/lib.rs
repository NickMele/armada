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
//! The OS lifecycle: the runtime file Bridge finds Fleet by
//! ([`runtime`](mod@runtime)), the identity check that makes that file's pid
//! mean something ([`process`](mod@process)), and the detached spawn every
//! Drone gets ([`detach`](mod@detach)).
//!
//! And the gate: the Evidence tool a Drone reports through
//! ([`evidence`](mod@evidence)), and what Fleet does about a submission
//! ([`gate`](mod@gate)) — run the step's Checks in the worktree, decide, and
//! either advance the step or end the Job. **Fleet decides, not the Drone**,
//! and nothing in either module accepts a fact from a Drone that gates its own
//! step.
//!
//! [`session`](mod@session) is the seam the outcome leaves through, and it has
//! no implementation: injecting a turn needs a live session, spawning a Drone
//! is a later step, and a fake of the trait cannot be written either while
//! `DroneHandle` is uninhabited. The mechanism itself is measured — see that
//! module.
//!
//! Scheduling and the Job-shape classifier are later steps, and neither is
//! stubbed here.
//!
//! `api::Daemon` is not implemented yet either, which is why `fleet-bin` binds
//! the listener and does not serve on it. A trait implementation that answered
//! every request with a fault would be a placeholder a client cannot tell from
//! a fault, and `api`'s own route table says why that is worse than nothing.

pub mod detach;
pub mod evidence;
pub mod gate;
pub mod process;
pub mod runtime;
pub mod session;

#[cfg(test)]
mod tests;

pub use detach::Detached;
pub use evidence::{Call, EvidenceInbox, EvidenceTool, Landed, Recorded};
pub use gate::{apply, rule_on, AtStep, CheckBudget, CheckOutput, Ruling};
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use runtime::{
    machine_path, provisional_address, Presence, Published, PublishError, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME, PROVISIONAL_PORT,
};
pub use session::LiveSession;
