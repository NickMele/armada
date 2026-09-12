//! A plain-language reading of one command a Drone reached for, for the person
//! deciding whether to allow it — `explain_command` in
//! `crates/ipc/operations.toml`.
//!
//! **It decides nothing.** The offers a command carries are unchanged by it,
//! and a person who never asks is answered exactly as before. What it removes
//! is the case where somebody allows a command they could not read, or rejects
//! one they would have allowed.

use serde::{Deserialize, Serialize};

/// What a model said one command does.
///
/// **The model is named because the reading is a claim, not a fact.** A person
/// weighing an explanation is entitled to know what read it, the same way a
/// Judge's verdict carries the model that reached it — and the cheap end of
/// the roster is what answers here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExplained {
    /// The reading itself, in prose a person can act on: what the command
    /// does, and what about it is worth a second look.
    pub explanation: String,
    /// Which model said it.
    pub model: String,
}
