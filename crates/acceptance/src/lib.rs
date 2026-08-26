//! The acceptance test's home. The test itself is in `tests/bug_job.rs`, and
//! the bench it runs on is in `tests/bench/`.
//!
//! # This crate is green, and that is new
//!
//! For the whole of M0 — Foundations it did not compile, deliberately.
//! `tests/bug_job.rs` was written **before** the code it tests, so that the
//! compiler's error list would be the list of things the remaining crates had
//! to provide, in the shape the test asked for rather than the shape they
//! happened to take. A test written against a finished skeleton gets shaped by
//! the code it finds, which is the failure that build order existed to prevent.
//!
//! That is over. The code the test named exists, the test was reconciled
//! against the vocabulary that was actually built, and it passes.
//!
//! **Two things still say otherwise and are wrong.** `cargo xtask
//! verify-foundations` carries a rule asserting that `cargo test -p acceptance`
//! exits non-zero, and a Stop hook ends the session if it does not. Both were
//! written for a milestone in which failing was the requirement. Neither was
//! changed by the step that made this green, because a gate weakened to admit
//! the thing it was watching for is not a gate — what they should assert
//! instead is a person's decision and is recorded on that step's issue.
//!
//! # What the test is, in one line
//!
//! One hermetic run of a Bug Job — no process, no repository, no network — plus
//! the invariants that make the run mean something. What it proves and what it
//! does not are written at the top of `tests/bug_job.rs`, which is where
//! somebody reading the claim will look.
