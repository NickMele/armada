//! The cases that need git's own opinion, and nothing else.
//!
//! See [`repo`] for why these run against a real repository while everything
//! above this crate runs against a fake.

pub mod repo;
mod work_product;
mod worktree;
