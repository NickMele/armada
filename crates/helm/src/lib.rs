//! Armada's command layer, as a library.
//!
//! The binary is a thin `main` over this; the tests drive it directly with fake
//! seams. Nothing here is a public contract — the package is `publish = false`,
//! and `ARCHITECTURE.md` §1.3 keeps the promised surface to the CLI and its
//! `--json` payloads.

#![deny(unsafe_code)]

pub mod app;
pub mod args;
pub mod ask;
pub mod bridge;
pub mod mcp;
pub mod redact;
pub mod render;
pub mod verbs;
