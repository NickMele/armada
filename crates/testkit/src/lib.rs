//! The fake harness, and the fixtures that drive it.
//!
//! `testkit` exists because v1 shipped 586 commits, 2,181 passing tests and a
//! working screen, and the Jobs it completed did not do what they claimed. A
//! fake that only emits happy-path output reproduces exactly that: a fast green
//! suite proving nothing about the part that actually failed. So the fixtures
//! are weighted towards misbehaviour, and each one is a specification of what a
//! misbehaving Drone's output stream really looks like, written before the
//! detector that catches it.
//!
//! Fixtures live in `fixtures/ndjson/`, one file per failure mode, beside the
//! pinned real capture the format-drift contract test compares against.
//!
//! # Why it is empty
//!
//! The five fixtures are the gate's subjects and are not written by M0 step 7 —
//! the gate names them missing, which is the gate working. This crate is the
//! place they will land.
