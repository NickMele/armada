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
//! [`split`] is the one splitter in the workspace and is public for that
//! reason. `fleet::drifting` asks whether what a `run` line names is still in
//! the repository, which is the same resolution this crate does before it
//! spawns — so it reads this rather than writing a second one that would agree
//! until the first quoted word.
//!
//! # It decides nothing, but for one thing only this crate can read
//!
//! What comes back is a fact: a code, a signal, an expired budget, or a spawn
//! that never happened. Whether it satisfies a step is `verification`'s — two
//! crates, because the deciding half must be testable without a process and
//! the running half cannot be. [`one_test_ran`] is the one exception: telling
//! a name that matched nothing apart from a pass needs the runner's own
//! printed summary, which only this crate captures at all.

mod matched;
mod narrow;
mod run;
mod serving;

#[cfg(test)]
mod tests;

pub use matched::{one_test_ran, OneTestRan};
pub use narrow::{narrowed, one_test, Narrowed};
pub use run::{run, run_until, run_writing, run_writing_with_env, split, Attempt, Output, Writing};
pub use serving::Served;
