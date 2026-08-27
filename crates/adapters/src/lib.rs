//! Everything that talks to something outside Armada.
//!
//! The agent CLI, git2, the keychain, the model client — and the health probes
//! for each, because a probe is the smallest instance of what this crate already
//! owns. git2 is here for worktrees regardless, so asking whether git works adds
//! no dependency.
//!
//! **This is the only crate permitted to name a vendor.** Outside it, a vendor's
//! name is the boundary having leaked, and the gate refuses it.
//!
//! A dedicated health crate was rejected: no measurement backs the seam, and
//! grouping by surface rather than by capability is the shape that grew v1's
//! core to 38,470 lines.

//! # What is built so far
//!
//! Version control ([`GitVcs`], the work-product reader beside it, and
//! [`reclaim`] — which gives one Job's worktree and branch back, and is the
//! only thing in the workspace that can), and the agent harness: [`HeadlessAgent`] renders a confined Drone into a program and
//! an argument list, [`transcript`](mod@transcript) reads what that Drone says
//! back, and [`mcp`](mod@mcp) writes the one-server configuration file the two
//! flags that confine it point at.
//!
//! **The harness does not spawn.** Starting a process, detached, is `fleet`'s —
//! see `adapter_traits::AgentHarness` for the three things that split buys, the
//! first of which is that every confinement property here is a value a test
//! reads rather than a process a test has to start.

mod commit;
mod error;
mod harness;
mod mcp;
mod reclaim;
mod transcript;
mod work_product;
mod worktree;

#[cfg(test)]
mod tests;

pub use error::{CommitWorkError, CreateWorktreeError, ReadWorkProductError};
pub use harness::{evidence_server, evidence_tool, HarnessRefused, HeadlessAgent};
pub use mcp::only_the_evidence_server;
pub use reclaim::{reclaim, BranchGone, Reclaimed, RepoUnreadable, WorktreeGone};
pub use worktree::GitVcs;
