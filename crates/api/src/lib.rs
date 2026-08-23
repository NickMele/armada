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
