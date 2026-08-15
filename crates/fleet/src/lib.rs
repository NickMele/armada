//! The Fleet module's imperative shell.
//!
//! [`armada_core::fleet`] decides; this crate performs. What lives here is the
//! filesystem under `~/.armada/`, the `git worktree` calls, the subprocess a
//! Drone is, and the append-only inbox — every one of them reached through the
//! seams on `Ctx` (`ARCHITECTURE.md` §1.1), so a test asserts the exact argv and
//! feeds a recorded answer back.
//!
//! **Fleet is the lowest module that can see both Guild and Manifest**
//! (`ARCHITECTURE.md` §1.9). Guild and Manifest are siblings and neither may
//! name the other, so merging what you know with what a repository knows can
//! only happen here — and this crate names both for that reason rather than for
//! convenience.
//!
//! **Nothing may depend on Fleet except Helm**, and `cargo xtask boundaries`
//! enforces it.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod drone;
pub mod home;
pub mod inbox;
pub mod jobs;
pub mod manifest;
pub mod own;
pub mod worktree;
