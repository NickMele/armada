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
//! And the Drone itself: [`drone`](mod@drone) builds the environment a Drone
//! gets — from an explicit list, never from Fleet's own — starts it detached
//! against a harness's rendering, and says what a dead Drone means for its Job.
//! [`session`](mod@session) is what speaks to a live one: the first turn and
//! every injected turn go down the same pipe, and every write is checked.
//!
//! **The harness renders and Fleet starts.** Nothing in `adapters` spawns, so
//! `Detached` stays the only way a process begins here, and every confinement
//! property is a value a test can read rather than a process a test must run.
//!
//! And the loop that joins them: [`daemon`](mod@daemon) is what Fleet is —
//! one working slot, the seams it is assembled from, and the five operations
//! `api::Daemon` names — and [`dispatch`](mod@dispatch) is what happens to a
//! Job while it is in that slot. [`serving`](mod@serving) is the trait
//! implementation, so the operations answer from a real Fleet.
//!
//! **Two things enter the process here and nowhere else.** [`clock`](mod@clock)
//! is the one place a clock is read, and [`mint`](mod@mint) the one place an id
//! is invented — every other crate in the workspace refuses both, and the
//! refusal needs somewhere to bottom out.
//!
//! The Job-shape classifier and a second working slot are later milestones, and
//! neither is stubbed here. Nor is `fleet-bin` wired to serve: constructing a
//! Fleet needs an `armada.yml` and a workflow resolved from somewhere, and
//! **which paths those are has not been decided by any step**. A composition
//! root that guessed would put that decision where nobody would look for it.

pub mod adrift;
pub mod briefing;
pub mod clock;
pub mod daemon;
pub mod detach;
pub mod dispatch;
pub mod drafting;
pub mod drone;
pub mod evidence;
pub mod gate;
pub mod mint;
pub mod process;
pub mod runtime;
pub mod serving;
pub mod session;
pub mod watch;
pub mod working;

#[cfg(test)]
mod tests;

pub use adrift::{Adrift, NotSubmitted};
pub use clock::{Clock, SystemClock};
pub use daemon::{Fittings, Fleet, Host, Reconciled, Turned};
pub use detach::Detached;
pub use drone::{
    aftermath, environment, Aftermath, DroneNotStarted, Ending, HostPaths, Left, Started,
};
pub use evidence::{Call, EvidenceInbox, EvidenceTool, Landed, Recorded};
pub use gate::{apply, rule_on, AtStep, CheckBudget, CheckOutput, Ruling};
pub use mint::{Mint, UlidMint};
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use runtime::{
    machine_path, provisional_address, Presence, PublishError, Published, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME, PROVISIONAL_PORT,
};
pub use session::{DroneSession, LiveSession, Turn};
pub use watch::Watching;
