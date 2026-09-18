//! A Helm conversation's routes, over the same in-memory pipe the other
//! sockets use: the thread and then what is said after, on one connection; a
//! blank message refused by the transport; a fresh start closing the socket
//! saying why; a repository nobody serves refused before the upgrade; and the
//! record served whole.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use futures_util::StreamExt;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use ipc::{HelmAsked, HelmConversation, HelmMessage, HelmSilence, Instant};
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tower::{Service, ServiceExt};

use crate::tests::connected;
use crate::tests::fake::{FakeDaemon, SERVED_MANIFEST};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> (Arc<FakeDaemon>, Router) {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    let app = router(Served::sharing(Arc::clone(&daemon), run_id(), events));
    (daemon, app)
}

async fn read(socket: &mut WebSocketStream<DuplexStream>) -> HelmMessage {
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the socket is text: {frame:?}");
    };
    ipc::decode("a Helm message", json.as_bytes()).expect("a Helm message")
}

async fn posted(app: &Router, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("a body")
        .to_bytes()
        .to_vec();
    (status, body)
}

#[tokio::test]
async fn one_connection_carries_the_thread_and_then_what_is_asked() {
    let (daemon, app) = wired();
    *daemon.helm_thread.lock().expect("not poisoned") = vec![HelmMessage::Asked(HelmAsked {
        ts: Instant::carried("2026-09-13T08:00:00.000Z"),
        text: "asked before".to_string(),
    })];
    let mut socket = connected(app.clone(), "/helm/observe", 8192).await;

    let HelmMessage::Opened(opened) = read(&mut socket).await else {
        panic!("the first message says what this connection is");
    };
    assert_eq!(opened.protocol_version, ipc::PROTOCOL_VERSION);
    assert_eq!(opened.manifest_id.as_str(), SERVED_MANIFEST);
    assert!(!opened.replying);
    let HelmMessage::Asked(before) = read(&mut socket).await else {
        panic!("the thread so far");
    };
    assert_eq!(before.text, "asked before");

    let (status, body) = posted(&app, "/helm/ask", r#"{"text":"why did it stall?"}"#).await;
    assert_eq!(status, StatusCode::ACCEPTED, "the reply is the socket's");
    let conversation: HelmConversation =
        ipc::decode("a conversation", &body).expect("the conversation comes back");
    assert!(conversation.replying);

    let HelmMessage::Asked(asked) = read(&mut socket).await else {
        panic!("what the person asked, live");
    };
    assert_eq!(asked.text, "why did it stall?");
}

#[tokio::test]
async fn a_blank_message_is_refused_by_the_transport() {
    let (_daemon, app) = wired();
    let (status, _) = posted(&app, "/helm/ask", r#"{"text":"   "}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn starting_fresh_closes_the_socket_saying_why() {
    let (_daemon, app) = wired();
    let mut socket = connected(app.clone(), "/helm/observe?manifest_id=armada", 8192).await;
    assert!(matches!(read(&mut socket).await, HelmMessage::Opened(_)));

    let (status, _) = posted(&app, "/helm/start_fresh", "").await;
    assert_eq!(status, StatusCode::OK);

    let HelmMessage::Closed(closed) = read(&mut socket).await else {
        panic!("the thread this showed is gone, and the socket says so");
    };
    assert_eq!(closed.because, HelmSilence::StartedFresh);
    let after = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket ends");
    assert!(
        !matches!(after, Some(Ok(Message::Text(_)))),
        "nothing follows closed: {after:?}"
    );
}

#[tokio::test]
async fn a_repository_nobody_serves_is_refused_before_the_upgrade() {
    let (_daemon, app) = wired();
    let (client_side, server_side) = tokio::io::duplex(8192);
    tokio::spawn(async move {
        let service = service_fn(move |request| app.clone().call(request));
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(server_side), service)
            .with_upgrades()
            .await;
    });
    let opened = tokio_tungstenite::client_async(
        "ws://fleet.invalid/helm/observe?manifest_id=elsewhere",
        client_side,
    )
    .await;
    let Err(tokio_tungstenite::tungstenite::Error::Http(answer)) = opened else {
        panic!("a repository nobody serves has no conversation to open");
    };
    assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn the_record_is_served_whole_and_a_repository_nobody_serves_is_refused() {
    let (_daemon, app) = wired();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/helm/debug?manifest_id=armada")
                .body(Body::empty())
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("a body")
        .to_bytes()
        .to_vec();
    let record: ipc::HelmDebugInfo = ipc::decode("a record", &body).expect("the record comes back");
    assert_eq!(record.manifest_id.as_str(), SERVED_MANIFEST);
    assert_eq!(record.protocol_version, ipc::PROTOCOL_VERSION);
    assert!(!record.brief.is_empty(), "the brief as it was sent");

    let refused = app
        .oneshot(
            Request::builder()
                .uri("/helm/debug?manifest_id=elsewhere")
                .body(Body::empty())
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
