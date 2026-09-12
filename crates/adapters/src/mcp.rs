//! The configuration file a Drone is spawned against, holding one server — and
//! the repository's own, holding one entry beside whatever else is in it.
//!
//! **Two documents, opposite guarantees.** The first is Armada's: it is written
//! whole, it holds exactly one server, and a Drone is confined to it. The
//! second is the repository's: Armada adds one entry and preserves every other
//! byte, because somebody else wrote that file. [`publish_the_agents_door`] is
//! the second and everything above it is the first.
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
/// Not the CLI's full vocabulary: see
/// `docs/spikes/010-can-a-drone-be-identified.md` for the measured set and
/// why the rest do not fit a Fleet-spawned Drone. A misspelling of this word
/// used to be a string the compiler could not see; now it is a variant that
/// does not exist.
///
/// **Two, and they are for two different readers.** A Drone is handed an
/// address by the Fleet that spawned it, so it gets [`Transport::Http`]. An
/// agent in a terminal is handed nothing and has to find a Fleet for itself,
/// so what a repository's configuration names is a program — see
/// [`publish_the_agents_door`].
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Transport {
    Http,
    Stdio,
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

/// Where a repository tells an agent working in it which servers to open.
///
/// **The repository's own file, and that is what makes it safe.** A Drone is
/// spawned under `--strict-mcp-config` against the document
/// [`only_the_evidence_server`] writes, so it never reads this one — see
/// `harness`. Fleet control is therefore published in the one place a Drone
/// cannot inherit it from.
pub const REPOSITORY_CONFIG: &str = ".mcp.json";

/// A server an agent starts for itself, as the agent CLI's schema reads one.
///
/// **A program and not an address.** Fleet's port is leased rather than fixed,
/// so a URL written into a repository is a number that goes stale the next time
/// Fleet starts — and the port it goes stale *onto* may belong to something
/// else entirely by then. The program named here reads the runtime file at the
/// moment a session opens and decides what to do about what it finds.
#[derive(Serialize)]
struct StdioServer<'a> {
    #[serde(rename = "type")]
    transport: Transport,
    command: &'a str,
    args: &'a [&'a str],
}

/// What publishing did. **Said out loud rather than inferred**, so a boot can
/// report a file it changed and stay quiet about one it did not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Published {
    /// The entry was not there, or named something else. The file was written.
    Written,
    /// The file already says exactly this. Nothing was touched — no mtime
    /// moved and no working tree went dirty on a Fleet restart.
    AlreadyThere,
}

/// Register the agent's door in `at`, keeping everything else in the file.
///
/// `command` and `args` are the caller's, because the program is `armada`'s own
/// verb and this crate may not know it. What this crate owns is the file's
/// name, the schema's spellings and the merge that does not destroy what a
/// person wrote.
///
/// **It is a selection and not a grant.** Publishing the door here tells an
/// agent standing in this repository where to knock; it confers nothing,
/// because the listener is on loopback with no authentication and every process
/// on this machine was already inside that boundary.
pub fn publish_the_agents_door(
    at: &Path,
    command: &str,
    args: &[&str],
) -> Result<Published, io::Error> {
    let existing = match fs::read(at) {
        Ok(bytes) => Some(bytes),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => None,
        Err(cause) => return Err(cause),
    };
    let written = ipc::merging::merged_into(
        existing.as_deref(),
        SERVERS,
        ipc::door::SERVER,
        &StdioServer {
            transport: Transport::Stdio,
            command,
            args,
        },
    )
    .map_err(|why| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} was left alone: {why}", at.display()),
        )
    })?;

    if existing.as_deref() == Some(written.as_bytes()) {
        return Ok(Published::AlreadyThere);
    }
    if let Some(parent) = at.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(at, written)?;
    // **No `restrict` here, unlike the document above.** That one names the
    // address a Drone reports evidence to and is Armada's alone; this one is a
    // file in somebody's repository, which they edit, review and may commit.
    Ok(Published::Written)
}

/// The key both documents hold their servers under.
///
/// **A second spelling of the `rename` above, because an attribute takes a
/// literal and not a constant.** The pair is what a test asserts rather than
/// what a reader is asked to notice: the Drone's document is built by `serde`
/// and an agent's is merged into a file, so the two reach the same key by
/// different routes and could drift apart silently.
const SERVERS: &str = "mcpServers";
