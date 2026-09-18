//! Kit's MCP servers, resolved for one Manifest — `#1275`.
//!
//! `crate::permitting::repository` is the model: a rule a person set, kept in
//! Fleet's own store, read back by the surface that lists it and by the next
//! spawn. This one is read by [`crate::spawning`], which writes what it returns
//! into the document a Drone is started against.
//!
//! **Kit widens no Drone by itself.** `core_model::a_drone_resolves` is the
//! whole rule and nothing here restates it, so what is drawn and what is
//! written are one answer.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{a_drone_resolves, Actor, KitServer, ManifestReach, ReachesDrones, ServerName};
use ipc::WireError;

use crate::daemon::Fleet;

/// Not job-scoped, so it earns its own code beside the refusals it raises —
/// `crate::permitting::repository`'s reason.
const KIT_REFUSED: &str = "fleet.kit_refused";

/// What a change to Kit can refuse on.
#[derive(Debug)]
pub enum NotInKit {
    /// The name is blank, too long, or holds a character a JSON key and a tool
    /// name do not spell the same way.
    NotAName {
        given: String,
    },
    /// A blank command, or an address that is not `http` or `https`.
    NotAnAddress,
    /// The Evidence server's own name. **Refused rather than shadowed**: a
    /// Drone's one way to report is `mcp__armada__*`, and a second `armada`
    /// would make every brief's tool name ambiguous.
    ArmadasOwnName,
    /// Kit holds no server under this name.
    NoSuchServer {
        name: String,
    },
    NotRecorded {
        cause: String,
    },
}

impl std::fmt::Display for NotInKit {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotInKit::NotAName { given } => write!(
                out,
                "`{given}` is not a name a server may carry: letters, digits, `-` and `_`, up to 64"
            ),
            NotInKit::NotAnAddress => write!(
                out,
                "a server is a program with a name, or an `http`/`https` address"
            ),
            NotInKit::ArmadasOwnName => write!(
                out,
                "`{}` is Armada's own server, and a second one would leave a Drone's brief naming two",
                adapters::evidence_server()
            ),
            NotInKit::NoSuchServer { name } => write!(out, "Kit holds no server called `{name}`"),
            NotInKit::NotRecorded { cause } => write!(out, "{cause}"),
        }
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
    /// Every server in Kit, with what this Manifest said about each.
    ///
    /// **Empty where the store will not read**, which is what
    /// `Fleet::repository_allowed` answers too: a Kit that cannot be read is a
    /// Drone with the Evidence server alone, never a Drone with everything.
    pub(crate) async fn kit_here(
        &self,
        served: &crate::repositories::Served,
    ) -> Vec<(KitServer, Option<ManifestReach>)> {
        let store = self.store().lock().await;
        let servers = store.kit_servers().unwrap_or_default();
        let said = store
            .manifest_server_reaches(served.manifest().id())
            .unwrap_or_default();
        servers
            .into_iter()
            .map(|server| {
                let reach = said
                    .iter()
                    .find(|(name, _)| name == &server.name)
                    .map(|(_, reach)| *reach);
                (server, reach)
            })
            .collect()
    }

    /// The servers a Drone dispatched against this Manifest is handed.
    pub(crate) async fn servers_for_a_drone(
        &self,
        served: &crate::repositories::Served,
    ) -> Vec<KitServer> {
        self.kit_here(served)
            .await
            .into_iter()
            .filter(|(server, reach)| a_drone_resolves(server, *reach))
            .map(|(server, _)| server)
            .collect()
    }

    /// `get_kit_servers` — every server in Kit, both tiers, and what a Drone
    /// dispatched here resolves.
    pub async fn kit_servers_listed(
        &self,
        served: &crate::repositories::Served,
    ) -> ipc::KitServers {
        ipc::KitServers {
            servers: self
                .kit_here(served)
                .await
                .into_iter()
                .map(|(server, reach)| ipc::KitServerRow {
                    name: server.name.as_str().to_string(),
                    address: (&server.address).into(),
                    drones: server.drones.into(),
                    manifest: reach.map(Into::into),
                    resolves: a_drone_resolves(&server, reach),
                    added_at: ipc::Instant::carried(server.added_at.as_str()),
                    by: server.by.into(),
                })
                .collect(),
        }
    }

    /// `add_kit_server` — put a server in Kit, reaching no Drone.
    ///
    /// A name Kit already holds replaces that server's address and leaves both
    /// tiers of its reach alone: correcting a command is not re-granting it.
    pub async fn add_kit_server(&self, adding: ipc::AddKitServer) -> Result<(), NotInKit> {
        let name = self.server_named(&adding.name)?;
        let address = adding.address.to_domain().ok_or(NotInKit::NotAnAddress)?;
        let server = KitServer::added(name, address, self.now(), Actor::Human);
        self.store()
            .lock()
            .await
            .put_kit_server(&server)
            .map_err(|cause| NotInKit::NotRecorded {
                cause: cause.to_string(),
            })
    }

    /// `forget_kit_server` — take a server out of Kit, and every Manifest's
    /// word about it with it.
    ///
    /// **Read by the next spawn**, so a Drone already running keeps what it
    /// was started with for the step it is on.
    pub async fn forget_kit_server(
        &self,
        forgetting: ipc::ForgetKitServer,
    ) -> Result<(), NotInKit> {
        let name = self.server_named(&forgetting.name)?;
        let gone = self
            .store()
            .lock()
            .await
            .forget_kit_server(&name)
            .map_err(|cause| NotInKit::NotRecorded {
                cause: cause.to_string(),
            })?;
        match gone {
            true => Ok(()),
            false => Err(NotInKit::NoSuchServer {
                name: forgetting.name,
            }),
        }
    }

    /// `set_kit_server_reach` — Kit's own tier, for every Manifest that has not
    /// said otherwise.
    pub async fn set_kit_server_reach(
        &self,
        setting: ipc::SetKitServerReach,
    ) -> Result<(), NotInKit> {
        let name = self.server_named(&setting.name)?;
        let drones: ReachesDrones = setting.drones.domain();
        let moved = self
            .store()
            .lock()
            .await
            .set_kit_server_reach(&name, drones)
            .map_err(|cause| NotInKit::NotRecorded {
                cause: cause.to_string(),
            })?;
        match moved {
            true => Ok(()),
            false => Err(NotInKit::NoSuchServer { name: setting.name }),
        }
    }

    /// `set_manifest_server_reach` — this Manifest's own word, or `null` to
    /// take it back and leave Kit's default answering.
    pub async fn set_manifest_server_reach(
        &self,
        served: &crate::repositories::Served,
        setting: ipc::SetManifestServerReach,
    ) -> Result<(), NotInKit> {
        let name = self.server_named(&setting.name)?;
        let at = self.now();
        let mut store = self.store().lock().await;
        let held = store
            .kit_servers()
            .unwrap_or_default()
            .into_iter()
            .any(|server| server.name == name);
        if !held {
            return Err(NotInKit::NoSuchServer { name: setting.name });
        }
        store
            .set_manifest_server_reach(
                served.manifest().id(),
                &name,
                setting.reach.map(|reach| reach.domain()),
                &at,
                Actor::Human,
            )
            .map_err(|cause| NotInKit::NotRecorded {
                cause: cause.to_string(),
            })
    }

    /// A name, through the same constructor a stored one is read back by, and
    /// never Armada's own.
    fn server_named(&self, given: &str) -> Result<ServerName, NotInKit> {
        let name = ServerName::named(given).ok_or_else(|| NotInKit::NotAName {
            given: given.to_string(),
        })?;
        match name.as_str() == adapters::evidence_server() {
            true => Err(NotInKit::ArmadasOwnName),
            false => Ok(name),
        }
    }

    /// [`NotInKit`] as a 409 with no Job to name.
    pub(crate) fn kit_refusal(&self, why: NotInKit) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            KIT_REFUSED,
            why.to_string(),
            self.run_id(),
        ))
    }
}
