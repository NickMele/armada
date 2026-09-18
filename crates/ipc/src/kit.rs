//! Kit's MCP servers on the wire, and what one Manifest says about each —
//! `#1275`.
//!
//! **`resolves` crosses rather than being computed on the far side.** It is
//! `core_model::a_drone_resolves` over the two tiers, and the same call decides
//! the document a Drone is spawned against — a surface that recomputed it could
//! draw a server as reaching a Drone that no Drone is handed.

use serde::{Deserialize, Serialize};

use crate::enums::{Actor, ManifestReach, ReachesDrones};
use crate::ids::Instant;

/// Where a server is, in the two shapes the agent CLI's schema accepts.
///
/// **Tagged by `transport`**, so a reader matches one field and never guesses
/// from which of two optional keys turned up.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum ServerAddress {
    Stdio { command: String, args: Vec<String> },
    Http { url: String },
}

impl From<&core_model::ServerAddress> for ServerAddress {
    fn from(address: &core_model::ServerAddress) -> ServerAddress {
        match address {
            core_model::ServerAddress::Stdio { command, args } => ServerAddress::Stdio {
                command: command.clone(),
                args: args.clone(),
            },
            core_model::ServerAddress::Http { url } => ServerAddress::Http { url: url.clone() },
        }
    }
}

impl ServerAddress {
    /// The domain's own, or `None` where a blank command or an address that is
    /// not `http` makes it no address at all.
    pub fn to_domain(&self) -> Option<core_model::ServerAddress> {
        match self {
            ServerAddress::Stdio { command, args } => {
                core_model::ServerAddress::program(command, args)
            }
            ServerAddress::Http { url } => core_model::ServerAddress::address(url),
        }
    }
}

/// One row of `get_kit_servers`: a server, both tiers, and what a Drone
/// dispatched against the Manifest asked about is handed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitServerRow {
    pub name: String,
    pub address: ServerAddress,
    /// Kit's own default, across every Manifest.
    pub drones: ReachesDrones,
    /// This Manifest's word, left out where it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ManifestReach>,
    /// Whether a Drone dispatched here gets it. The resolution, not a hint.
    pub resolves: bool,
    pub added_at: Instant,
    pub by: Actor,
}

/// `get_kit_servers`' answer: every server in Kit, by name.
///
/// **A named wrapper rather than a bare list**, as every other `list_*` answer
/// on this seam is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitServers {
    pub servers: Vec<KitServerRow>,
}

/// The request half of `add_kit_server`. A name Kit already holds replaces
/// that server's address and leaves both tiers of its reach alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddKitServer {
    pub name: String,
    pub address: ServerAddress,
}

/// The request half of `forget_kit_server`. A name Kit does not hold is a 409.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetKitServer {
    pub name: String,
}

/// The request half of `set_kit_server_reach` — Kit's own tier, for every
/// Manifest that does not say otherwise.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetKitServerReach {
    pub name: String,
    pub drones: ReachesDrones,
}

/// The request half of `set_manifest_server_reach` — the Manifest's own tier.
///
/// **`null` is the take-back, and it is sent rather than implied**, for
/// [`SetModel`](crate::SetModel)'s reason: a Bridge that dropped the field
/// would throw away a person's word about who may reach a server and answer
/// 200.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetManifestServerReach {
    pub name: String,
    #[serde(deserialize_with = "stated")]
    pub reach: Option<ManifestReach>,
}

/// An `Option` whose key must be present. `deserialize_with` turns off serde's
/// rule that a missing `Option` is `None`, which is the whole point.
fn stated<'de, D>(input: D) -> Result<Option<ManifestReach>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<ManifestReach>::deserialize(input)
}
