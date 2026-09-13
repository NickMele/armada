//! The repositories one Fleet serves, and adding one by folder or by URL —
//! `list_repositories`, `add_repository` and `clone_repository` in
//! `crates/ipc/operations.toml`.
//!
//! **A repository is listed whether or not it has a Manifest yet.** Setup
//! starts from a folder nobody wrote an `armada.yml` for, so `manifest` is
//! absent until one loads there, and Scan is named by `root` rather than by a
//! Manifest id it does not have.

use serde::{Deserialize, Serialize};

use crate::ManifestSummary;

/// A folder to serve, as a person chose it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddRepository {
    /// Absolute. Fleet resolves it before comparing, so a symlink to a served
    /// repository is refused as that repository.
    pub path: String,
}

/// A repository to clone, and the folder a person picked to clone it under.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloneRepository {
    /// What `git clone` takes, read with the credentials git already has.
    pub url: String,
    /// Absolute, and already there. The clone lands in a new folder under it,
    /// named from the URL.
    pub parent: String,
}

/// One repository Fleet serves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositorySummary {
    /// The repository's root, resolved. What `?repository=` names on Scan and
    /// its proposals.
    pub root: String,
    /// Where its Job records live, for the reason `ManifestSummary` carries it.
    pub records_root: String,
    /// Absent until an `armada.yml` at `root` loads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ManifestSummary>,
}

/// Every repository Fleet serves, the one it was started in first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryList {
    pub repositories: Vec<RepositorySummary>,
}
