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

mod error;
mod worktree;

#[cfg(test)]
mod tests;

pub use error::CreateWorktreeError;
pub use worktree::GitVcs;
