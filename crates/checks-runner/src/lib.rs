//! Running a Manifest's Checks and Commands.
//!
//! Called with explicitly injected data — a lease id, a worktree path — and
//! **never a live scheduler handle**. That is the whole reason this is a crate.
//!
//! v1's equivalent reached 1,816 lines inside its core, coupled to the
//! scheduler, the lease table and process-group plumbing, and could not be
//! lifted out. The seam is here so the runner can be driven by a test that has
//! no scheduler at all.
//!
//! # What is built so far
//!
//! [`run`], which executes one Check's command in one worktree under a budget
//! and reports how it ended. Commands — the ungating half of a Manifest's two
//! registries — are not here: nothing may invoke one yet, and a runner for
//! something with no caller is a surface that gets used by accident.
//!
//! # It decides nothing
//!
//! What comes back is a fact: a code, a signal, an expired budget, or a spawn
//! that never happened. Whether that fact satisfies a step is `verification`'s,
//! and the two are separate crates because the deciding half must be testable
//! without a process and the running half cannot be.

mod run;

#[cfg(test)]
mod tests;

pub use run::{run, Attempt, Output};
