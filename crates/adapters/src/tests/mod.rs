//! The cases that need git's own opinion, and the ones that need no process at
//! all.
//!
//! See [`repo`] for why the version-control cases run against a real repository
//! while everything above this crate runs against a fake.
//!
//! `harness` and `transcript` are the other half and are the opposite shape:
//! **nothing there starts a process.** What a Drone is confined to is a value
//! the harness renders, and what a Drone said is a line somebody already
//! captured — so the whole confinement posture is asserted without a
//! credential, a network, or an agent.

mod commit;
mod delivery;
mod harness;
mod issue_lookup;
mod judge;
mod landing;
mod mcp;
mod reclaim;
pub mod repo;
mod standing;
mod transcript;
mod work_product;
mod worktree;
