//! The transport, driven with **no network**.
//!
//! Nothing here binds a port, resolves a name or opens a socket. The HTTP half
//! calls the router as the `tower` service it is; the WebSocket half runs a
//! real upgrade over an in-memory pipe. That is possible only because `api`
//! does not depend on `fleet` — the step's claim about the dependency graph and
//! the step's claim about testability are the same claim, and these tests are
//! what makes the second one checkable.

mod fake;
mod served;
mod stream;
