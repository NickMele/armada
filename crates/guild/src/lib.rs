//! The Guild module: **you**, portable.
//!
//! Voice, expectations, how you work, your skills, hooks, subagents and
//! workflows — machine-global in `~/.armada/guild/`, built by interviewing you,
//! seeded from what is already on the machine, and synced between your machines
//! ([`glossary.md`](../../../docs/glossary.md), `PLAN.md` §13).
//!
//! **No part of a guild ever enters this repository.** What ships here is the
//! machinery that builds one, plus the starter content in `templates/guild/`
//! that `guild init` copies out — and from the moment it is copied it is yours
//! and Armada does not touch it again.
//!
//! # Where the layers are
//!
//! Guild is one crate holding both halves of `ARCHITECTURE.md` §1.5's split,
//! which is the shape §1.9 says a module is expected to take as it grows:
//!
//! | Pure — decisions over data | Shell — the outside world |
//! |---|---|
//! | [`secrets`] · [`memory`] · [`layout`] · [`interview`] · [`project`] · [`validate`] · [`upstream`] | [`import`] · [`repo`] · [`remote`] · [`bundle`] · [`inventory`] · [`machine`] · [`permissions`] · [`starters`] · [`projector`] |
//!
//! Every subprocess in the shell half goes through `ctx.run` (§1.1), so `git`
//! and `tar` are argv a test can assert rather than behaviour a test has to
//! mock. **The filesystem is not faked** and deliberately so — the same
//! reasoning §1.1 gives for SQLite. Tests write into a `TempDir` and read it
//! back.
//!
//! # Guild may not name Manifest
//!
//! They are siblings (§1.9). Manifest describes a repository and Guild
//! describes a person, and a dependency in either direction would mean one of
//! those descriptions had leaked into the other. In practice that costs exactly
//! one thing: Guild is *told* where `~/.armada` is rather than working it out,
//! because `armada_home` lives in `armada-manifest`. See [`layout`].

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod bundle;
pub mod import;
pub mod interview;
pub mod inventory;
pub mod layout;
pub mod machine;
pub mod memory;
pub mod permissions;
pub mod project;
pub mod projector;
pub mod remote;
pub mod repo;
pub mod secrets;
pub mod starters;
pub mod upstream;
pub mod validate;
