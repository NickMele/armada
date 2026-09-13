//! `freeze:`, the switch that stops new work starting in one repository —
//! `docs/concepts/manifest.md`, *Dispatch freeze*.
//!
//! **A flag and nothing beside it.** The concept describes a toggle, and a note
//! of why would be a key nothing reads, which this parser refuses on principle.

use super::Manifest;
use crate::error::Refusal;
use crate::yaml::{self, Table};

/// Whether the file freezes dispatch. **Absent is `false`**, and so is a refused
/// value, because a file carrying any refusal does not load.
pub(super) fn read(top: &mut Table<'_>, out: &mut Vec<Refusal>) -> bool {
    top.optional("freeze")
        .and_then(|value| yaml::flag("freeze", value, out))
        .unwrap_or(false)
}

impl Manifest {
    /// Whether no new Drone may start on this repository's work.
    ///
    /// **Read through the live cell**, so a save lifting a freeze is answered at
    /// the next admission rather than at a restart. One file's answer, not the
    /// resolution: `fleet::freezing` folds a Job's gating Manifests.
    pub fn frozen(&self) -> bool {
        self.live.read().freeze
    }
}
