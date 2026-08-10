//! charkit's pure core.
//!
//! Decisions are functions over data (`ARCHITECTURE.md` §1.2). Nothing here
//! spawns, writes, kills or labels — those live in `adapters`, and the three
//! injected seams reach them. Parsing a string is a pure function; reading the
//! file it came from is not, and is somebody else's job.
//!
//! Phase 1 populates two things and deliberately nothing else: the `char.yml`
//! contract ([`config`]) and the failure vocabulary every verb answers in
//! ([`error`]). There is no runtime.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod error;
