//! A person's Bridge preferences — `crate::limits`'s shape one table over,
//! without the admission overlay: nothing here is applied to a running
//! Drone, so there is no "shipped, overlaid by saved" to compute.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Preferences, SavePreference};
use store::LoadJobError;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The store's shape, as the wire spells it. A plain field-for-field copy
/// rather than a `From` impl: one call site, and `ipc` already depends on
/// nothing here that would make an `impl` worth naming.
fn as_wire(preferences: store::Preferences) -> Preferences {
    Preferences {
        where_things_are_open: preferences.where_things_are_open,
    }
}

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
    /// Every preference, read straight from the store. **No cache**, unlike
    /// `limits_in_force`: this is read once per connection rather than on
    /// every admission turn, so there is nothing here worth keeping in memory
    /// between saves.
    pub(crate) async fn preferences_in_force(&self) -> Result<Preferences, Adrift> {
        let store = self.store().lock().await;
        store
            .preferences()
            .map(as_wire)
            .map_err(|fault| Adrift::Reading(LoadJobError::Database(fault)))
    }

    /// Save one preference, then answer with every preference now in force.
    pub(crate) async fn save_preference_now(
        &self,
        save: SavePreference,
    ) -> Result<Preferences, Adrift> {
        let mut store = self.store().lock().await;
        store
            .save_preference(&save.name, save.value)
            .map(as_wire)
            .map_err(Adrift::Writing)
    }
}
