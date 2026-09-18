//! Kit's MCP servers, and how far one reaches. `docs/concepts/kit.md`, `#1275`.
//!
//! Kit holds the person's servers; a Manifest extends or restricts the set for
//! one repository. [`a_drone_resolves`] is the whole of that resolution.
//!
//! **A server a person adds reaches no Drone until they say so.** There is no
//! constructor that starts one anywhere but [`ReachesDrones::No`] — which is
//! `docs/scope.md`'s one confinement said as a type: a Kit whose servers
//! arrived switched on would hand a Drone the operator's whole set again,
//! which is the v1 defect `--strict-mcp-config` exists to have ended.
//!
//! **No credential.** [`ServerAddress`] carries a command or a URL and no
//! environment, because an environment is where an API key would go and a key
//! in Fleet's store is a decision nobody has taken.

use alloc::string::String;
use alloc::vec::Vec;

use crate::envelope::{Actor, Timestamp};

/// The longest name a server may carry.
///
/// **It is half of a tool name** — the CLI spells a server's tools
/// `mcp__<server>__<tool>` — so a bound here bounds an allowlist entry too.
const LONGEST_NAME: usize = 64;

/// What a server is registered under, and the prefix its tools carry.
///
/// **Letters, digits, `-` and `_`, and nothing else.** It is a JSON object key
/// in the document a Drone is spawned against and half a tool name in its
/// allowlist; a space or a `.` is a name one of those two spells differently.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerName(String);

impl ServerName {
    /// `None` where the text is blank, longer than [`LONGEST_NAME`], or holds
    /// a character outside the set above. Trimmed first, so a name pasted with
    /// space around it is that name rather than a refusal.
    pub fn named(text: &str) -> Option<ServerName> {
        let trimmed = text.trim();
        let shaped = !trimmed.is_empty()
            && trimmed.len() <= LONGEST_NAME
            && trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        shaped.then(|| ServerName(String::from(trimmed)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a server is, as the agent CLI's own schema reads one.
///
/// **Two forms and no third**: `adapters::mcp` writes the document these
/// become, and a form it cannot write would be a server a person could add and
/// no Drone could start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerAddress {
    /// A program a Drone's own CLI starts and talks to over its standard
    /// streams.
    Stdio {
        /// The program. Never empty.
        command: String,
        /// Its arguments, in order, each one argument. **Never one string
        /// split later** — splitting is a shell's job, and there is no shell
        /// between here and the spawn.
        args: Vec<String>,
    },
    /// An address opened over HTTP, as Armada's own Evidence server is named.
    Http {
        /// `http://` or `https://`. Never a bare host.
        url: String,
    },
}

impl ServerAddress {
    /// A program and its arguments. `None` where the program is blank.
    pub fn program(command: &str, args: &[String]) -> Option<ServerAddress> {
        let trimmed = command.trim();
        (!trimmed.is_empty()).then(|| ServerAddress::Stdio {
            command: String::from(trimmed),
            args: args.to_vec(),
        })
    }

    /// An address. `None` where it is not `http` or `https`.
    pub fn address(url: &str) -> Option<ServerAddress> {
        let trimmed = url.trim();
        let addressed = trimmed.starts_with("http://") || trimmed.starts_with("https://");
        addressed.then(|| ServerAddress::Http {
            url: String::from(trimmed),
        })
    }

    /// The word the store and the wire spell this form as.
    pub fn transport(&self) -> &'static str {
        match self {
            ServerAddress::Stdio { .. } => "stdio",
            ServerAddress::Http { .. } => "http",
        }
    }
}

/// Whether Kit's own tier lets a Drone see a server.
///
/// **A named pair rather than a `bool`**: both readings of a bare `true` are
/// plausible — *this server is on*, and *this server may be turned on* — and
/// they differ by every Drone on the machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReachesDrones {
    /// No Drone gets it unless a Manifest says so. **Where every server
    /// starts**, and this type's default for that reason.
    #[default]
    No,
    /// Every Manifest that does not restrict it gets it.
    Yes,
}

impl ReachesDrones {
    pub const ALL: &'static [ReachesDrones] = &[ReachesDrones::No, ReachesDrones::Yes];

    pub fn as_wire(&self) -> &'static str {
        match self {
            ReachesDrones::No => "no",
            ReachesDrones::Yes => "yes",
        }
    }

    /// `None` where the text is neither spelling: a row written by something
    /// that did not share this enum.
    pub fn from_wire(value: &str) -> Option<ReachesDrones> {
        ReachesDrones::ALL
            .iter()
            .copied()
            .find(|r| r.as_wire() == value)
    }

    pub fn reaches(&self) -> bool {
        matches!(self, ReachesDrones::Yes)
    }
}

/// What one Manifest says about one of Kit's servers.
///
/// **Absent is not a third word.** A Manifest that has said nothing holds no
/// row at all, and [`a_drone_resolves`] reads that as Kit's default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestReach {
    /// This repository's Drones get it, whatever Kit's default is.
    Extended,
    /// This repository's Drones do not, whatever Kit's default is.
    Restricted,
}

impl ManifestReach {
    pub const ALL: &'static [ManifestReach] = &[ManifestReach::Extended, ManifestReach::Restricted];

    pub fn as_wire(&self) -> &'static str {
        match self {
            ManifestReach::Extended => "extended",
            ManifestReach::Restricted => "restricted",
        }
    }

    pub fn from_wire(value: &str) -> Option<ManifestReach> {
        ManifestReach::ALL
            .iter()
            .copied()
            .find(|r| r.as_wire() == value)
    }
}

/// One MCP server a person put in their Kit.
///
/// Keyed by [`name`](Self::name): a second server under a name Kit holds
/// replaces the first, because that name is the key in the document a Drone
/// reads and two entries could not both be there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KitServer {
    pub name: ServerName,
    pub address: ServerAddress,
    /// Kit's own tier. `No` on every server [`KitServer::added`] makes.
    pub drones: ReachesDrones,
    pub added_at: Timestamp,
    pub by: Actor,
}

impl KitServer {
    /// A server as a person just added it: in Kit, and reaching no Drone.
    ///
    /// **The only constructor, and it takes no reach**, so adding a server
    /// cannot widen what any Drone is handed. Turning it on is a second act,
    /// on its own operation.
    pub fn added(name: ServerName, address: ServerAddress, at: Timestamp, by: Actor) -> KitServer {
        KitServer {
            name,
            address,
            drones: ReachesDrones::No,
            added_at: at,
            by,
        }
    }
}

/// Whether a Drone dispatched against a Manifest is handed this server.
///
/// **The Manifest's word wins where it has one**, in either direction; where it
/// has said nothing, Kit's default answers. `fleet::spawning` calls this and
/// writes what it returns, so a surface drawing *what this Manifest resolves*
/// and the document a Drone is spawned against cannot disagree.
pub fn a_drone_resolves(server: &KitServer, manifest: Option<ManifestReach>) -> bool {
    match manifest {
        Some(ManifestReach::Extended) => true,
        Some(ManifestReach::Restricted) => false,
        None => server.drones.reaches(),
    }
}

#[cfg(test)]
mod tests;
