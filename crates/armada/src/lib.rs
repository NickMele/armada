//! The composition root, and the only one.
//!
//! Kept thin so linking stays the only slow build step. There is no `api-bin`
//! and no thirteenth crate.
//!
//! It will also carry `armada doctor --json`, the short-lived probe process
//! Bridge spawns on demand — how Doctor sees what a long-running daemon cannot
//! report about itself. **Not built**: `cli` knows four verbs, and `doctor` is
//! not one of them yet. `docs/concepts/doctor.md` specifies it.
//!
//! # Where the composition actually is
//!
//! [`serve`](mod@serve), which is the one verb that needs a port, a store and a
//! process. `src/main.rs` reads the command line and dispatches; everything a
//! verb does is a module here, so it can be driven by a test that starts
//! nothing.
//!
//! [`setup`](mod@setup) reads a repository's `armada.yml` and its one workflow
//! and resolves the second against the first — the part of starting Fleet that
//! can be wrong on disk. [`agent`](mod@agent) is the same shape for the
//! machine: which binary a Drone is started as, and which model.
//! [`declared`](mod@declared) and [`clean`](mod@clean) are the three verbs that
//! need no daemon at all. [`watching`](mod@watching) is what makes the
//! Manifest's live keys live — `#430` — and it is here because the composition
//! root owns the runtime and nothing below it may spawn a task.

pub mod agent;
pub mod clean;
pub mod cli;
pub mod declared;
pub mod say;
pub mod serve;
pub mod setup;
pub mod watching;

#[cfg(test)]
mod tests;

pub use agent::{
    agent_binary, judge_model, model_choices, proposer_model, NoSuchAgent, AGENT_BINARY,
    JUDGE_MODEL, MODEL, PROPOSER_MODEL,
};
pub use setup::{Setup, SetupRefused};
