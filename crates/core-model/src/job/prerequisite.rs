//! A Command a Check needs run before it, as the Job froze it.
//!
//! **The Manifest names a Command; the Job carries the name and the line.**
//! `checks.<name>.requires` is a list of names from the Manifest's own Commands
//! registry, so what actually runs is written in exactly one place — but a
//! failure has to name both halves, because the name is what a person edits and
//! the line is what they re-run to see it fail again. That is
//! `config::Preparation`'s reasoning, one registry over, and the two types stay
//! apart only because one is resolved at load and frozen onto a Job and the
//! other is resolved at load and read live.

use alloc::string::String;

/// One Command a Check requires, name and command line together.
///
/// Frozen onto the Job beside the Check's own `run` and `when`, and for their
/// reason: an edit to `armada.yml` mid-Job would otherwise change what runs
/// before a gate that has already measured work. A prerequisite is part of what
/// produced the exit code, so it moves the gate exactly as `run` would.
///
/// There is no way to build one but by resolving a `checks.<name>.requires`
/// entry against a Command the same Manifest declares, or by reading one back
/// off a row that was written that way — so a caller holding one is holding a
/// name that resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prerequisite {
    name: String,
    run: String,
}

impl Prerequisite {
    /// Build one from a name that has already been resolved against a declared
    /// Command. **`config::ResolvedWorkflow` and `store` are the only callers**
    /// — the first resolves it, the second reads back what the first wrote.
    pub fn resolved(name: String, run: String) -> Prerequisite {
        Prerequisite { name, run }
    }

    /// The Command's name, as `requires` wrote it. **What a person edits**, and
    /// what a failure is attributed to.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The command line, taken from `commands.<name>.run` when the workflow was
    /// resolved. What a person runs by hand to see it fail again.
    pub fn run(&self) -> &str {
        &self.run
    }
}
