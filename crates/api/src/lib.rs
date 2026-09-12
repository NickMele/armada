//! HTTP and WebSocket transport, on one listener.
//!
//! **This crate does not depend on `fleet`**, which is what makes the daemon
//! core drivable in tests with zero network. The two run as one process and
//! talk over an in-process channel; the seam is real even though the process
//! boundary is not. [`Queries`], [`Commands`] and [`Tools`] are stated here and
//! implemented in `fleet`, so the dependency points one way and
//! `cargo tree -p api` names no `fleet`; [`Daemon`] is the three together and
//! the bound [`router`] takes. They speak `ipc` DTOs and nothing else: **no
//! `core_model` type appears anywhere in this crate**, so there is no shape
//! here through which a domain field could reach the wire unredacted.
//!
//! **axum, with its built-in WebSocket upgrade as an extractor in the same
//! router**, so there is no second port and no assembly step. gRPC and `tonic`
//! were rejected on measured cost: +43 crates, and a hard build failure on a
//! clean machine with no `protoc`. Queries and commands are request-response
//! and go over HTTP; only unsolicited pushes need the socket, and who initiates
//! is the whole rule.
//!
//! **A subset of the inventory, and a stream.** [`SERVED`] is the operations M1
//! needs, named with `crates/ipc/operations.toml`'s own keys. The rest of the
//! inventory, the `/v0` lifeboat and version-skew handling belong to Ship and
//! are neither built nor stubbed here. [`MCP_PATH`] is on the listener and on
//! neither seam above: a Drone's Evidence tool, deliberately absent from
//! [`SERVED`]. See `mcp`.

mod answers;
/// The four reads that narrow the Board rather than drawing it.
mod attention;
mod commands;
mod daemon;
/// The agent's door: the HTTP surface, spoken as MCP. **Who may open it is
/// `#698`; what it is scoped to is here.**
mod door;
/// The reads that belong to no Job: the roster, the machine, the spend and
/// what crossed the stream.
mod fleetwide;
/// One Job's own log, followed off the file Fleet already writes.
/// **The third voice in the activity log.**
mod following;
mod journal;
mod mcp;
mod observing;
mod queries;
/// The `:job_id` a route carries, resolved before a handler can reach it.
mod reference;
/// The run sheet's routes: a person's run of one Manifest entry in a Job's
/// worktree, and what the runs left.
mod rehearsing;
mod routes;
/// Everything a handler is given. **Next door to the table**, which is what
/// the gate rule reads.
mod served;
mod servers;
mod sockets;
mod stream;
/// A person's run's output, on a socket of its own per run — never `/events`.
mod watching_run;

#[cfg(test)]
mod tests;

pub use daemon::{
    Commands, Daemon, FramePart, FrameSpan, PermissionAnswer, Queries, Refusal, Tools,
};
pub use door::{offered, Scope, DOOR_PATH};
pub use following::{Follow, Followed, LiveOutput};
pub use journal::{Journal, Reading, FOLLOW};
pub use mcp::{Caller, MCP_PATH};
pub use observing::{Feed, Observed, Seen, Turns, Watch, WATCHING};
pub use reference::Resolved;
pub use routes::{router, Route, SERVED};
pub use served::Served;
pub use stream::{Broadcaster, Next, Subscription, BACKLOG, TALLIED};
pub use watching_run::{
    ObservedRun, ObservedServer, RunChunk, RunFeed, RunSeen, RunWatch, RUN_BACKLOG,
};
