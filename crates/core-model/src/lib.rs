//! The vocabulary every other crate agrees on: the Job record, its states and
//! transitions, the escalation triggers, the workflow definition, evidence.
//!
//! # What may not enter this crate
//!
//! No runtime, no I/O, no vendor. `core-model` is the one crate every other
//! crate depends on, so a dependency added here is a dependency added
//! everywhere — that is why `cargo tree` on this crate is a gate rule and not
//! a preference. No `tokio`, no `git2`, no `reqwest`, reachable at any depth.
//!
//! Serialisation lives here only as derives on types defined here. Reading
//! untyped JSON belongs to `store` and `ipc`, which are the two places bytes
//! enter the process.
//!
//! # What is here, and what is not
//!
//! The log envelope is here, because a line shape retrofitted after five crates
//! are already logging is a rewrite of all five — and `actor` cannot be
//! reconstructed afterwards at all.
//!
//! The Job record and its transition machine are not. M1 step 1 owns them by
//! name, and the acceptance test is written against this crate before it is
//! filled on purpose: a test written against a finished skeleton gets shaped by
//! the code it finds.

#![no_std]

extern crate alloc;

mod envelope;

pub use envelope::{
    env_keys, Actor, AuditLine, Component, Envelope, FieldValue, Level, Timestamp, Ulid,
};
