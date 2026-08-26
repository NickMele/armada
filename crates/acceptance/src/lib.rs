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
//! **The two mechanisms that said otherwise are retired.** `cargo xtask
//! verify-foundations` rule one now asserts that `cargo test -p acceptance`
//! passes and ran something, and the Stop hook that ended a session on a green
//! run is deleted. Neither was weakened to admit what it was watching for; the
//! reasoning they carried — a milestone that can fake itself green proves
//! nothing — is what rule one still enforces, in the direction the falsehood
//! now runs. The account of all of it is `docs/practices/acceptance-tests.md`.
//!
//! # What the test is, in one line
//!
//! One hermetic run of a Bug Job — no process, no repository, no network — plus
//! the invariants that make the run mean something. What it proves and what it
//! does not are written at the top of `tests/bug_job.rs`, which is where
//! somebody reading the claim will look.
