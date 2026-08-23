//! The composition root, and the only one.
//!
//! Kept thin so linking stays the only slow build step. There is no `api-bin`
//! and no thirteenth crate.
//!
//! It also carries `fleet-bin doctor --json`, the short-lived probe process
//! Bridge spawns on demand — which is how Doctor sees what a long-running
//! daemon cannot report about itself.
