//! The event stream, upgraded and driven over an in-memory pipe.
//!
//! `tokio::io::duplex` is both halves of a connection with no connection: the
//! server side is handed to hyper, the client side to a real WebSocket client,
//! and the upgrade that runs between them is the same code path a Bridge takes.
//! **Nothing here binds a port**, which is the point — a loopback listener
//! would still be a listener, and would still fail in an environment that has
//! no networking at all.

use axum::Router;
use futures_util::StreamExt;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use ipc::{StreamMessage, PROTOCOL_VERSION};
use std::time::Duration;
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tower::Service;

use crate::tests::fake::{run_id, FakeDaemon};
use crate::{router, Broadcaster, Served};

/// A connected client, over a pipe. `buffer` is the pipe's capacity, which is
/// how a test makes a client slow without making it wait.
async fn connected(app: Router, buffer: usize) -> WebSocketStream<DuplexStream> {
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
    let (socket, _) = tokio_tungstenite::client_async("ws://fleet.invalid/events", client_side)
        .await
        .expect("the upgrade is an extractor on the same router");
    socket
}

async fn read(socket: &mut WebSocketStream<DuplexStream>) -> StreamMessage {
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the stream answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the stream is text: {frame:?}");
    };
    ipc::decode("stream message", json.as_bytes()).expect("a stream message")
}

/// Every connection opens with current state, then carries events.
#[tokio::test]
async fn a_connection_opens_with_a_resync_and_then_carries_events() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    let app = router(Served::by(daemon, run_id(), events.clone()));
    let mut socket = connected(app, 8192).await;

    let StreamMessage::Resync(resync) = read(&mut socket).await else {
        panic!("the first message is the resync — a reconnecting client must not have to ask");
    };
    assert_eq!(resync.protocol_version, PROTOCOL_VERSION);
    assert_eq!(resync.cursor.position(), 0);
    assert!(resync.jobs.jobs.is_empty());

    // Published only now: the resync having arrived is what says the socket is
    // subscribed, so this cannot race the subscription.
    let cursor = events.publish(a_transition("queued", "running"));

    let StreamMessage::Event(delivered) = read(&mut socket).await else {
        panic!("an event follows the resync");
    };
    assert_eq!(delivered.cursor, cursor);
    let ipc::Event::JobStateChanged(moved) = delivered.event else {
        panic!("a move, not a creation");
    };
    assert_eq!(moved.to.as_wire(), "running");
}

/// A client that cannot keep up loses the oldest events and is **told**, then
/// handed current state. The count alone would leave it patching a history it
/// cannot repair.
#[tokio::test]
async fn a_client_that_cannot_keep_up_is_told_and_resynced() {
    let events = Broadcaster::with_backlog(1);
    let daemon = FakeDaemon::new(events.clone());
    let app = router(Served::by(daemon, run_id(), events.clone()));
    // A pipe small enough that the socket stops draining almost at once.
    let mut socket = connected(app, 256).await;

    let StreamMessage::Resync(_) = read(&mut socket).await else {
        panic!("the first message is the resync");
    };
    for _ in 0..24 {
        events.publish(a_transition("queued", "running"));
    }

    let mut told = None;
    let mut resynced_after = false;
    for _ in 0..24 {
        match read(&mut socket).await {
            StreamMessage::Missed(missed) => told = Some(missed.dropped),
            StreamMessage::Resync(_) if told.is_some() => {
                resynced_after = true;
                break;
            }
            _ => continue,
        }
    }
    let dropped = told.expect("the bound dropped events and the client was not told");
    assert!(dropped > 0);
    assert!(
        resynced_after,
        "a drop is always followed by current state, or the client is quietly wrong"
    );
}

/// A daemon that cannot answer closes the socket rather than sending half a
/// picture. There is no error message on this stream.
#[tokio::test]
async fn a_daemon_that_cannot_answer_closes_the_socket() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    *daemon.mute.lock().expect("not poisoned") = true;
    let app = router(Served::by(daemon, run_id(), events));
    let mut socket = connected(app, 8192).await;

    let ended = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket resolves");
    let closed = match ended {
        None => true,
        Some(Ok(Message::Close(_))) => true,
        Some(Err(_)) => true,
        other => panic!("the socket sent something instead of closing: {other:?}"),
    };
    assert!(closed);
}

fn a_transition(from: &str, to: &str) -> ipc::Event {
    let json = format!(
        r#"{{"message":"event","cursor":0,"event":{{"kind":"job.state_changed",
            "job_id":"01JOB0","from":"{from}","to":"{to}","actor":"fleet",
            "at":"2026-08-26T09:00:00.000Z"}}}}"#
    );
    let message: StreamMessage = ipc::decode("stream message", json.as_bytes()).expect("a message");
    match message {
        StreamMessage::Event(delivered) => delivered.event,
        _ => panic!("an event"),
    }
}
