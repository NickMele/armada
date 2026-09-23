//! A Command that stays running, held by Fleet — `docs/concepts/fleet.md`,
//! *Servers*. What the run sheet and *Where things are* draw a server from.
//!
//! **Lifecycle facts on `/events`, output on a socket of its own.** A server
//! prints for as long as it runs, and on the one drop-oldest stream carrying
//! every Job it would evict the state the Board is drawn from — the run
//! socket's reason, one subject over.

use serde::{Deserialize, Serialize};

use crate::event::Missed;
use crate::ids::{Instant, JobId, ManifestId};
use crate::underway::{OutputClosed, OutputLines};
use crate::version::ProtocolVersion;

/// Where a server is. **Only `serving` carries a usable address**: a link
/// pressed while `starting` opens a port nothing answers on yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerPhase {
    /// Its `run` is going, or `serve` is up and `ready` has not passed.
    Starting,
    Serving,
    /// Over. `stopped` says whether something stopped it or it fell over.
    Exited,
}

/// Who asked for it. A Drone asks through Fleet's MCP tool and never its
/// shell, which is how Fleet knows the server exists at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartedBy {
    Person,
    Drone,
}

/// An address a server offers, resolved against the span it runs under.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLink {
    pub url: String,
    /// The button's label. Absent: draw the URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A declared port a server's fields name, and the number it resolved to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPort {
    pub name: String,
    pub port: u16,
}

/// Which checkout a server serves, and whether what it serves is still what
/// that checkout stands at.
///
/// **A name and a port are not enough.** Two checkouts of one repository
/// declare the same server under the same name, the address answers either
/// way, and nothing on it says whose build came back — `#1577`. Every answer
/// that names a server carries this.
///
/// **Since 18.1.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerCheckout {
    /// The checkout the server runs in, absolute: a Job's worktree, or a
    /// repository's main checkout.
    pub path: String,
    /// The branch checked out there. Absent for the main checkout, which a
    /// person moves between branches and Fleet does not read per server —
    /// `path` and `commit` are what say which build is answering there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The commit the checkout stood at when the server started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// How many commits have landed in that checkout since the server started
    /// — what it serves is that far behind what the repository says.
    ///
    /// **Absent is not zero.** Zero is Fleet having looked and found nothing
    /// landed; absent is Fleet never having been told, which is every server
    /// on a checkout no Job of this Fleet has merged into.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
}

/// One instance of one server. `list_servers`' row, and the payload of all
/// three `server.*` events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerState {
    /// This instance's own id. What `stop_server` and `observe_server` take.
    pub id: String,
    /// The Command's name in the Manifest.
    pub name: String,
    /// Absent: started with no Job, in the main checkout, on its span.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
    /// The repository it runs in, by its Manifest — a Job's own, or the main
    /// checkout's. What tells two repositories' main-checkout servers apart.
    /// Absent only from a Fleet that predates it. **Since 13.17.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// Which checkout answers on this server's address. **Since 18.1.**
    pub checkout: ServerCheckout,
    pub phase: ServerPhase,
    /// The `serve` line as it ran, `${port.NAME}` resolved.
    pub serve: String,
    /// The declared ports its fields name, in the order they first appear.
    /// `localhost` and the first of these is where it is.
    pub ports: Vec<ServerPort>,
    pub links: Vec<ServerLink>,
    pub started_by: StartedBy,
    pub started_at: Instant,
    /// When `ready` passed, or `serve` started where there is no `ready`.
    /// Uptime is counted from here; nothing ticks on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serving_since: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// Absent while it runs, and where it ended with no code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// How it ended, in a sentence. **A server that exits on its own has
    /// failed, whatever its code**, and this says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<String>,
    /// A person pressed Stop, or its Job or Fleet ended. `false` on an
    /// `exited` server is one that stopped on its own.
    pub stopped: bool,
    /// The log, relative to `ManifestSummary::records_root`.
    pub log: String,
}

/// `list_servers`: every server Fleet holds, and the last instance of each
/// that ended — so one that fell over is still there to read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerList {
    pub servers: Vec<ServerState>,
}

/// `start_server`'s body. **A name, never a command line and never a port**:
/// what runs is what the Manifest declares, on the span it runs under. Which
/// repository's main checkout is `?manifest_id=`, as on every checkout route;
/// a Job's repository is the Job's own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartServer {
    pub name: String,
    /// The Job whose worktree and span it runs in. Absent: a checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
    /// A checkout of that repository to serve, by its path — a worktree
    /// somebody cut by hand to look at another branch. Absent: the main
    /// checkout. Ignored where `job_id` names a Job, whose worktree is its
    /// own. **Since 18.2.**
    ///
    /// **A path and never a port**, the field above's rule: what it changes is
    /// which span `${port.NAME}` resolves against, not the number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkout: Option<String>,
}

/// `stop_server`'s body: which instance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedServer {
    pub id: String,
}

/// One declared server, as the run sheet lists it beside the Commands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerEntry {
    pub name: String,
    /// What runs first, as declared. Absent: nothing to build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// The `serve` line, as declared.
    pub serve: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ready: Option<String>,
    /// As declared, `${port.NAME}` unresolved.
    pub links: Vec<ServerLink>,
    pub destructive: bool,
    /// This Job's instance: running, or the last one that ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<ServerState>,
}

/// One message on a server's socket, `observe_server` — the run socket's four,
/// spelled the same way.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum ServerMessage {
    Opened(ServerOpened),
    Lines(OutputLines),
    Missed(Missed),
    Closed(OutputClosed),
}

/// The first message: whose server, and what the opening read left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerOpened {
    pub protocol_version: ProtocolVersion,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
    /// The log, relative to `ManifestSummary::records_root`.
    pub path: String,
    /// Still running when this opened. `false`: the history is all of it.
    pub live: bool,
    pub skipped: u64,
}
