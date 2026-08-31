//! The configuration file a Drone is spawned against, holding one server.
//!
//! # Why this is written rather than assembled at the call site
//!
//! Because the guarantee is about what the document does **not** contain, and a
//! document assembled from a map somebody passes in is a document a caller can
//! add a second entry to. [`only_the_evidence_server`] takes one address and
//! there is no parameter through which a second server could arrive.
//!
//! Paired with `--strict-mcp-config`, which `harness` puts on every argument
//! list: the file says which server, the flag says *only* that server. v1
//! passed neither and its Drone came up holding the operator's seven connected
//! servers, ninety-five tools and the accounts behind them.
//!
//! # What is not here
//!
//! **The server itself.** Answering a tool call means turning JSON-RPC bytes
//! into a typed call, and gate rule five scopes that to the crates where bytes
//! enter the process. `fleet::evidence` is everything from the typed call
//! inward and says the same thing from the other side. What answers on the
//! address this document names is `api`'s Evidence endpoint, which is where the
//! JSON-RPC is read and where `ipc` does the reading.

use std::fs;
use std::io;
use std::path::Path;

use serde::Serialize;

/// The name Armada's own server is registered under, and the prefix every tool
/// it exposes therefore carries.
///
/// Fixed rather than configurable: it is half of the tool name the Drone is
/// told to call, and a configurable half is a prompt and a toolbelt that can
/// disagree.
pub const EVIDENCE_SERVER: &str = "armada";

/// The document, as the harness reads it.
///
/// A struct rather than a map, so the shape is fixed at compile time and the
/// only server in it is the one field below.
#[derive(Serialize)]
struct StrictConfig<'a> {
    #[serde(rename = "mcpServers")]
    servers: OnlyServer<'a>,
}

#[derive(Serialize)]
struct OnlyServer<'a> {
    armada: HttpServer<'a>,
}

#[derive(Serialize)]
struct HttpServer<'a> {
    #[serde(rename = "type")]
    transport: Transport,
    url: &'a str,
}

/// The `type` the agent CLI's `--mcp-config` schema accepts for a server this
/// crate ever constructs.
///
/// One variant, not the CLI's full vocabulary: see
/// `docs/spikes/010-can-a-drone-be-identified.md` for the measured set and
/// why the rest do not fit a Fleet-spawned Drone. A misspelling of this word
/// used to be a string the compiler could not see; now it is a variant that
/// does not exist.
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Transport {
    Http,
}

/// Write the file, holding Armada's Evidence server and nothing else.
///
/// `at` is where the file goes and `url` is where Fleet is serving. The path is
/// outside the worktree — a Drone that could read its own MCP configuration
/// could read the address it reports to, and a Drone that could write it could
/// name a different server.
pub fn only_the_evidence_server(at: &Path, url: &str) -> Result<(), io::Error> {
    let document = ipc::encode(&StrictConfig {
        servers: OnlyServer {
            armada: HttpServer {
                transport: Transport::Http,
                url,
            },
        },
    })
    .map_err(|why| io::Error::new(io::ErrorKind::InvalidData, why.to_string()))?;

    if let Some(parent) = at.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(at, document)?;
    restrict(at)
}

/// Owner-only, because the file names the address a Drone's evidence is
/// accepted at. World-readable would put that address in front of every process
/// on the machine, which is the same reason nothing brokered goes in argv.
#[cfg(unix)]
fn restrict(at: &Path) -> Result<(), io::Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(at, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict(_at: &Path) -> Result<(), io::Error> {
    Ok(())
}
