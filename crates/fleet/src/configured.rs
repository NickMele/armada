//! One Manifest as Fleet resolved it, for the caller that asks what a Job here
//! will be held to.
//!
//! **A Fleet serves one repository.** A `manifest_id` naming another is
//! refused rather than resolved against whichever Manifest answered first —
//! `Queries::resolve_job`'s rule, for the other id a caller can name.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ManifestConfig, ManifestId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::wire::manifest_summary;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_manifest` — what this repository's `armada.yml` declares.
    pub(crate) fn manifest_config(&self, named: ManifestId) -> Result<ManifestConfig, Refusal> {
        let held = self.manifest();
        if named.as_str() != held.id().as_str() {
            return Err(self.refusal(Adrift::NoSuchManifest {
                named: named.as_str().to_string(),
                held: held.id().as_str().to_string(),
            }));
        }
        Ok(ManifestConfig {
            id: ManifestId::from(held.id()),
            summary: manifest_summary(held, &self.host().records_root),
            base: held.base().map(str::to_string),
            checks_as_written: held.checks_as_written().to_vec(),
            // Named where the Check has a name. A mechanical Check — a
            // non-empty diff, a file written — is not one `armada.yml`
            // declares, so it has none to give and is not in this list.
            proved_after_a_merge: held
                .proved_after_a_merge()
                .iter()
                .filter_map(|check| check.name().map(str::to_string))
                .collect(),
            commands: held.command_names(),
            servers: held.server_names(),
            ports: held.port_names(),
            exclude_paths: held
                .exclude_paths()
                .iter()
                .map(|path| path.as_str().to_string())
                .collect(),
            cost_cap_micros: held.cost_cap_micros(),
            turn_cap: held.turn_cap(),
            quiet_after_seconds: held.quiet_after_seconds(),
            poke_limit: held.poke_limit(),
        })
    }
}
