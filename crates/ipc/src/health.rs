//! What Fleet can say about its own health, and what it cannot.
//!
//! **Not Doctor.** Doctor's grid is ten modules probed from `adapters`,
//! `config` and `store`, and it is not built — `docs/concepts/doctor.md`. This
//! answers the rows Fleet itself holds, and names the rest as unprobed.

use serde::{Deserialize, Serialize};

/// One module, asked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// The module name as Doctor's grid spells it — `Fleet`, `SQLite`,
    /// `Manifest`, `System stats`.
    pub module: String,
    /// `pass`, `warn` or `fail`, Doctor's own three words. A string rather than
    /// a closed set: the vocabulary is a surface's, and nothing matches on it.
    pub outcome: String,
    /// What the probe read, in one line. Never empty — a `pass` with nothing
    /// beside it is a row a person cannot check.
    pub detail: String,
}

/// Every probe Fleet ran, and everything it could not run one for.
///
/// **`not_probed` is the honest half.** A report of four passing rows with no
/// mention of the modules nobody asked reads as a healthy machine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHealth {
    pub probes: Vec<Probe>,
    /// What this answer says nothing about, with why beside each.
    pub not_probed: Vec<Unprobed>,
    /// `settings.helm-action-authority-tier-1-redirect-enabled-vs-read-only`,
    /// resolved once when Fleet started. `#1127` — Bridge had no way to read
    /// what Fleet actually decided, only the setting's own description.
    pub helm_action_authority: HelmActionAuthority,
}

/// How far this machine lets Helm act, as the wire spells `fleet::helm::
/// Authority`. Its own type here rather than a shared one: `ipc` names
/// nothing from `fleet`, and a wire enum outlives the Rust type it mirrors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmActionAuthority {
    Acting,
    ReadOnly,
}

/// A set of Doctor probes Fleet cannot run.
///
/// **Grouped by who owns them, not listed by name.** The modules are named in
/// `docs/concepts/doctor.md`, and the crate boundary that keeps a vendor's
/// spellings inside `adapters` is the reason they are not restated here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unprobed {
    /// Which crate or surface owns the probes.
    pub owner: String,
    /// Why Fleet cannot run them, and where their list is. Read by a person,
    /// never matched on.
    pub because: String,
}
