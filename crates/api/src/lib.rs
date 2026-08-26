//! HTTP and WebSocket transport, on one listener.
//!
//! **This crate does not depend on `fleet`**, which is what makes the daemon
//! core drivable in tests with zero network. The two run as one process and
//! talk over an in-process channel; the seam is real even though the process
//! boundary is not.
//!
//! axum, with its built-in WebSocket upgrade as an extractor in the same
//! router, so there is no second port and no assembly step. gRPC and `tonic`
//! were rejected on measured cost: +43 crates, and a hard build failure on a
//! clean machine with no `protoc`.
//!
//! Queries and commands are request-response and go over HTTP. Only unsolicited
//! pushes need the socket — who initiates is the whole rule.
//!
//! # What holds the seam open
//!
//! [`Daemon`] is stated here and implemented in `fleet`, so the dependency
//! points one way and `cargo tree -p api` names no `fleet`. The trait speaks
//! `ipc` DTOs and nothing else: **no `core_model` type appears anywhere in this
//! crate**, so there is no shape here through which a domain field could reach
//! the wire unredacted.
//!
//! # Five operations, and a stream
//!
//! `list_jobs`, `propose_job`, `approve_dispatch`, `kill_drone` and the event
//! stream — the five M1 needs, named with `crates/ipc/operations.toml`'s own
//! keys. The other twenty-nine, the `/v0` lifeboat and version-skew handling
//! belong to Ship and are neither built nor stubbed here.

mod daemon;
mod routes;
mod stream;

#[cfg(test)]
mod tests;

pub use daemon::{Daemon, Refusal};
pub use routes::{router, Route, Served, SERVED};
pub use stream::{Broadcaster, Next, Subscription, BACKLOG};
