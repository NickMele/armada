//! Which repository a request acts on, where its path does not say.
//!
//! **Query parameters, so no body a Bridge already sends changes shape.** An
//! absent one is the repository Fleet was started in.

use serde::Deserialize;

/// `?manifest_id=` on a route that acts on one repository's Manifest.
#[derive(Deserialize)]
pub(crate) struct InManifest {
    #[serde(default)]
    manifest_id: Option<String>,
}

impl InManifest {
    pub(crate) fn manifest(self) -> Option<ipc::ManifestId> {
        self.manifest_id.map(ipc::ManifestId::carried)
    }
}

/// `?repository=` on Scan and its proposals: a root `list_repositories`
/// names, since a repository nobody set up has no Manifest id.
#[derive(Deserialize)]
pub(crate) struct InRepository {
    #[serde(default)]
    repository: Option<String>,
}

impl InRepository {
    pub(crate) fn repository(self) -> Option<String> {
        self.repository
    }
}
