//! Armada's pure core.
//!
//! Decisions are functions over data (`ARCHITECTURE.md` §1.2). Nothing here
//! spawns, writes, kills or labels — those live in `manifest`, and the three
//! injected seams reach them. Parsing a string is a pure function; reading the
//! file it came from is not, and is somebody else's job.
//!
//! Phase 1 populated two things: the `char.yml` contract ([`config`]) and the
//! failure vocabulary every verb answers in ([`error`]). Phase 2 adds the
//! ownership core — the two identities, workspace resolution, port blocks, the
//! claim/lease reducer, the reaping decisions, the scope lens and the `--json`
//! envelope — plus [`ctx`], the three seams everything above reaches the
//! outside world through.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod ctx;
pub mod dispatch;
pub mod envelope;
pub mod error;
pub mod glob;
pub mod id;
pub mod lease;
pub mod ports;
pub mod reap;
pub mod registry;
pub mod run;
pub mod schedule;
pub mod scope;
pub mod select;
pub mod template;
pub mod workspace;
