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

/// One thing a person already has, as `get_kit_inventory` lists it — `#1491`.
///
/// **No credential.** A connected server crosses as the program's own file name
/// or the host it is at, and never as what comes after either — no argument
/// list, no query string, no userinfo, no environment. What crosses could not
/// start the server it names, and allowing one stays its own act on a Kit row
/// a person added themselves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupItem {
    pub name: String,
    /// The item's own words for itself, where its file carries them — or, for
    /// a connected server, the program or host it is at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// Where it came from, as a person would type it.
    pub source: String,
}

/// Something of the right shape in the right place that would not read.
/// **Carried rather than dropped**: a skill Armada cannot describe is still a
/// skill the person has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupUnreadable {
    pub source: String,
    pub why: String,
}

/// What became of one kind. **Tagged**, so a surface matches one field rather
/// than reading an empty list as *you have none of these*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub enum WhatWasRead {
    Read {
        items: Vec<SetupItem>,
        unreadable: Vec<SetupUnreadable>,
    },
    NotRead {
        why: String,
    },
}

/// One kind of thing, and what became of it. `kind` is Armada's word for it —
/// `skills`, `plugins`, `agent_file`, `sub_agents`, `commands`, `mcp_servers`,
/// `allowlist`, `models` — and never a harness's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupKindRow {
    pub kind: String,
    pub read: WhatWasRead,
}

/// `get_kit_inventory`' answer: the setup a person already works with, read
/// from the harness's own home and shown beside what Armada itself holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitInventory {
    /// The harness, in its own name. **The one vendor word on this wire**, and
    /// it arrives as data an adapter produced rather than as a literal Bridge
    /// holds — so a second harness draws on the same screen.
    pub harness: String,
    pub home: String,
    /// Whether that home is there at all.
    pub present: bool,
    /// Every kind, in a fixed order, including the ones nothing reads yet.
    pub kinds: Vec<SetupKindRow>,
}
