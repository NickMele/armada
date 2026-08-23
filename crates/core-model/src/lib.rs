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
//! # Why it is empty
//!
//! M0 step 7 creates this crate and names its dependency rule; it does not say
//! what types it holds, and the things that would fill it are owned elsewhere —
//! the Job record and its transition machine are M1 step 1, and the escalation
//! trigger enum is a Notion database that is still the source of truth. The
//! acceptance test is written against this crate before it is filled, on
//! purpose: a test written against a finished skeleton gets shaped by the code
//! it finds.
