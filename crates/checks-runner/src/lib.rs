//! Running a Manifest's Checks and Commands.
//!
//! Called with explicitly injected data — a lease id, a worktree path — and
//! **never a live scheduler handle**. That is the whole reason this is a crate:
//! v1's equivalent reached 1,816 lines coupled to the scheduler, the lease
//! table and process-group plumbing, and could not be lifted out. The seam is
//! here so a test with no scheduler can drive the runner.
//!
//! # What is built so far
//!
//! [`run`] executes one Check's command in one worktree under a budget and
//! reports how it ended; [`narrowed`] assembles the command a Check runs
//! against one Drone's own change. One crate because they must agree about
//! quoting — [`narrow`] says why. Commands are not here: nothing may invoke one
//! yet, and a runner with no caller gets used by accident. [`Served`] is a
//! lifetime rather than a third thing — `evidence.serve` is a command line held
//! up while a second runs against it — and it is here because the three process
//! rules are [`run`]'s: no shell, a group of its own, the whole group ended.
//!
//! # It decides nothing
//!
//! What comes back is a fact: a code, a signal, an expired budget, or a spawn
//! that never happened. Whether it satisfies a step is `verification`'s — two
//! crates, because the deciding half must be testable without a process and
//! the running half cannot be.

mod narrow;
mod run;
mod serving;

#[cfg(test)]
mod tests;

pub use narrow::{narrowed, Narrowed};
pub use run::{run, Attempt, Output};
pub use serving::Served;
