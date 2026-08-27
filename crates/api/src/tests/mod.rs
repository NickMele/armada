//! The transport, driven with **no network**.
//!
//! Nothing here binds a port, resolves a name or opens a socket. The HTTP half
//! calls the router as the `tower` service it is; the WebSocket half runs a
//! real upgrade over an in-memory pipe. That is possible only because `api`
//! does not depend on `fleet` — the step's claim about the dependency graph and
//! the step's claim about testability are the same claim, and these tests are
//! what makes the second one checkable.

mod fake;
mod mcp;
mod observing;
mod served;
mod stream;

use axum::Router;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::io::DuplexStream;
use tokio_tungstenite::WebSocketStream;
use tower::Service;

/// A connected client, over a pipe. `buffer` is the pipe's capacity, which is
/// how a test makes a client slow without making it wait.
///
/// Shared by both sockets on the listener: the global stream and one Job's
/// turns are the same upgrade on the same router, and a second copy of this
/// would be a second claim about that.
async fn connected(app: Router, path: &str, buffer: usize) -> WebSocketStream<DuplexStream> {
    let (client_side, server_side) = tokio::io::duplex(buffer);
    tokio::spawn(async move {
        let service = service_fn(move |request| app.clone().call(request));
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(server_side), service)
            .with_upgrades()
            .await;
    });
    // The host is never resolved — the stream is already open. It exists
    // because a WebSocket handshake is an HTTP request and one must say so.
    let (socket, _) =
        tokio_tungstenite::client_async(format!("ws://fleet.invalid{path}"), client_side)
            .await
            .expect("the upgrade is an extractor on the same router");
    socket
}
