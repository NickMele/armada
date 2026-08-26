//! The composition root, and the only one.
//!
//! Kept thin so linking stays the only slow build step. There is no `api-bin`
//! and no thirteenth crate.
//!
//! It also carries `fleet-bin doctor --json`, the short-lived probe process
//! Bridge spawns on demand — which is how Doctor sees what a long-running
//! daemon cannot report about itself.
//!
//! # Where the composition actually is
//!
//! `src/main.rs`. This library exists for the parts of the same binary that are
//! worth testing without starting a process, and [`setup`](mod@setup) is the
//! first of them: reading a repository's `armada.yml` and its one workflow, and
//! resolving the second against the first, is the part of starting Fleet that
//! can be wrong on disk. Everything after it needs a port, a store and a
//! process, and reads better where it runs than behind a function that exists
//! to be called once.

pub mod setup;

#[cfg(test)]
mod tests;

pub use setup::{Setup, SetupRefused};
