//! Armada's pure core.
//!
//! Decisions are functions over data (`ARCHITECTURE.md` §1.2). Nothing here
//! spawns, writes, kills or labels — those live in `manifest`, and the three
//! injected seams reach them. Parsing a string is a pure function; reading the
//! file it came from is not, and is somebody else's job.
//!
//! Phase 1 populated two things: the `armada.yml` contract ([`config`]) and the
//! failure vocabulary every verb answers in ([`error`]). Phase 2 adds the
//! ownership core — the two identities, workspace resolution, port blocks, the
//! claim/lease reducer, the reaping decisions, the scope lens and the `--json`
//! envelope — plus [`ctx`], the three seams everything above reaches the
//! outside world through.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod compose;
pub mod config;
pub mod ctx;
pub mod dispatch;
pub mod envelope;
pub mod error;
pub mod failure;
pub mod fleet;
pub mod glob;
pub mod helm;
pub mod id;
pub mod lease;
pub mod lifecycle;
pub mod ports;
pub mod reap;
pub mod recent;
pub mod registry;
pub mod run;
pub mod scan;
pub mod schedule;
pub mod scope;
pub mod select;
pub mod service;
pub mod template;
pub mod verify;
pub mod workspace;
