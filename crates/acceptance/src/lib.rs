//! The acceptance test's home. The test itself is in `tests/bug_job.rs`.
//!
//! # This crate does not compile, and that is the milestone's definition of done
//!
//! `tests/bug_job.rs` is written **before** the code it tests, and it names an
//! API that does not exist: `Job::transition`, `Fleet`, the four adapter traits
//! and their fakes. Nothing here is stubbed to make it build.
//!
//! That is deliberate. The test is the specification of what a working Job is,
//! and a test written against a finished skeleton gets shaped by the code it
//! finds — which is the failure the M0 build order exists to prevent. Written
//! first, the compiler's error list *is* the list of things the remaining crates
//! must provide, in the shape the test asked for rather than the shape they
//! happened to take.
//!
//! **Green here is a build failure.** `cargo xtask verify-foundations` asserts
//! that `cargo test -p acceptance` exits non-zero and reports which kind of
//! failure it was, so "does not compile yet" can never be mistaken for
//! "compiles and passes". A Stop hook ends the session if it goes green.
//!
//! Until then the honest reading of a compile error here is: *these are the
//! things that do not exist yet, named by the test that needs them.*
