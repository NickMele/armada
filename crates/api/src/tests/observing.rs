//! One Job's turns, over the same in-memory pipe the event stream uses.
//!
//! What these have to prove is that a viewer joining a Job already running gets
//! the history and then the live rows from **one** connection, and that doing
//! so costs the Board nothing. The second half is the reason this socket exists
//! at all, so it is asserted rather than assumed.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use futures_util::StreamExt;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use ipc::{Instant, JobId, Saw, Silence, StreamMessage, TranscriptRow, TurnMessage};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tower::{Service, ServiceExt};

use crate::tests::connected;
use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::{run_id, A_PROPOSAL};
use crate::{router, Broadcaster, Served};

/// The daemon kept alongside the router, because a test drives both: it plants
/// the history and holds the Drone's end of the channel.
fn wired() -> (Arc<FakeDaemon>, Router) {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    let app = router(Served::sharing(Arc::clone(&daemon), run_id(), events));
    (daemon, app)
}

/// A Job the fake holds, so `observe_job` is not a 404.
async fn a_job(app: &Router) -> JobId {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("content-type", "application/json")
                .body(Body::from(A_PROPOSAL))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("a body")
        .to_bytes();
    let job: ipc::JobSummary = ipc::decode("a Job", &body).expect("a Job comes back");
    job.id
}

fn said(what: &str) -> TranscriptRow {
    TranscriptRow {
        ts: Instant::carried("2026-08-26T09:00:00.000Z"),
        // Which step a row belongs to is Fleet's to say. This crate carries
        // rows and reads none of them, so there is nothing here to name one.
        step: None,
        // Whose row it is is Fleet's to say too, for the same reason.
        by: ipc::Voice::Drone,
        saw: Saw::Said {
            text: what.to_string(),
        },
    }
}

async fn read(socket: &mut WebSocketStream<DuplexStream>) -> TurnMessage {
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the socket is text: {frame:?}");
    };
    ipc::decode("a turn message", json.as_bytes()).expect("a turn message")
}

/// What a row says, so an assertion does not restate the vocabulary.
fn prose(message: TurnMessage) -> String {
    let TurnMessage::Row(shown) = message else {
        panic!("a row, not {message:?}");
    };
    let Saw::Said { text } = &shown.row().saw else {
        panic!("prose");
    };
    text.clone()
}

/// The claim: one connection, the history and then the ones that follow.
#[tokio::test]
async fn one_connection_answers_with_the_history_and_then_the_live_rows() {
    let (daemon, app) = wired();
    let job = a_job(&app).await;
    // A Drone already running, having already said something.
    *daemon.history.lock().expect("not poisoned") = vec![said("already happened")];
    let feed = daemon.dispatching(&job);

    let mut socket = connected(app, &format!("/jobs/{}/observe", job.as_str()), 8192).await;

    let TurnMessage::Opened(opened) = read(&mut socket).await else {
        panic!("the first message says what this connection is");
    };
    assert_eq!(opened.protocol_version, ipc::PROTOCOL_VERSION);
    assert_eq!(opened.job_id, job);
    assert!(opened.live, "a Drone is writing");
    assert_eq!(opened.skipped, 0);

    assert_eq!(prose(read(&mut socket).await), "already happened");

    // Offered only now: the history having arrived is what says the socket is
    // subscribed, so this cannot race the subscription.
    feed.offer(said("happening now"));
    assert_eq!(prose(read(&mut socket).await), "happening now");

    // The Drone finishes. The viewer is told, rather than left on a socket that
    // simply stops — which is indistinguishable from one that broke.
    drop(feed);
    let TurnMessage::Closed(closed) = read(&mut socket).await else {
        panic!("a closing message, not silence");
    };
    assert_eq!(closed.because, Silence::DroneEnded);
}

/// What a Drone spent reaches the person watching it. **The only place on the
/// wire either number appears** — it was withheld because the Job's rail was
/// said to state both, and no rail states either.
#[tokio::test]
async fn the_last_row_carries_what_the_run_cost_and_how_many_turns_it_took() {
    let (daemon, app) = wired();
    let job = a_job(&app).await;
    let feed = daemon.dispatching(&job);

    let mut socket = connected(app, &format!("/jobs/{}/observe", job.as_str()), 8192).await;
    let TurnMessage::Opened(_) = read(&mut socket).await else {
        panic!("it opens");
    };

    // A quota move offered first, because a viewer must not be told about
    // dispatch gating: if it crossed, it would be the message read below.
    feed.offer(TranscriptRow {
        ts: Instant::carried("2026-08-27T14:11:00.000Z"),
        step: None,
        by: ipc::Voice::Drone,
        saw: Saw::QuotaMoved {
            window: "five_hour".to_string(),
            status: "warning".to_string(),
        },
    });
    feed.offer(TranscriptRow {
        ts: Instant::carried("2026-08-27T14:12:00.000Z"),
        step: None,
        by: ipc::Voice::Drone,
        saw: Saw::Ended {
            turns: 41,
            cost_micros: 1_530_000,
            refusals: 0,
        },
    });

    let TurnMessage::Row(shown) = read(&mut socket).await else {
        panic!("the row a Drone ends on is shown");
    };
    assert_eq!(
        shown.row().saw,
        Saw::Ended {
            turns: 41,
            cost_micros: 1_530_000,
            refusals: 0,
        },
        "forty-one turns and a dollar fifty-three, whole"
    );
}

/// A Job nothing ever wrote a transcript for. **Ordinary, not an error** — it
/// was never dispatched, or its Drone went with the Fleet that spawned it.
#[tokio::test]
async fn a_job_with_no_transcript_opens_says_so_and_closes() {
    let (_daemon, app) = wired();
    let job = a_job(&app).await;

    let mut socket = connected(app, &format!("/jobs/{}/observe", job.as_str()), 8192).await;

    let TurnMessage::Opened(opened) = read(&mut socket).await else {
        panic!("a connection opens even with nothing to show");
    };
    assert!(!opened.live, "nothing is writing this Job");
    let TurnMessage::Closed(closed) = read(&mut socket).await else {
        panic!("and it says there is nothing coming");
    };
    assert_eq!(closed.because, Silence::NothingWriting);
}

/// A Job id that names nothing is refused **before** the upgrade, so a caller
/// reads a 404 rather than a socket that opens and says nothing.
#[tokio::test]
async fn an_unknown_job_is_a_404_and_never_a_socket() {
    let (_daemon, app) = wired();
    // A real handshake, because the daemon is asked between the extractor and
    // the upgrade: a hand-built request that is not a valid upgrade is refused
    // by the extractor and never reaches the question this test asks.
    let (client_side, server_side) = tokio::io::duplex(8192);
    tokio::spawn(async move {
        let service = service_fn(move |request| app.clone().call(request));
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(server_side), service)
            .with_upgrades()
            .await;
    });
    let opened =
        tokio_tungstenite::client_async("ws://fleet.invalid/jobs/01NOSUCHJOB/observe", client_side)
            .await;
    let Err(tokio_tungstenite::tungstenite::Error::Http(answer)) = opened else {
        panic!("a Job that does not exist has no view to open, and says so over HTTP");
    };
    assert_eq!(answer.status(), StatusCode::NOT_FOUND);
}

/// A viewer that cannot keep up loses the oldest rows and is **told how many**.
/// A gap left unsaid reads as a Drone that went quiet, which is the one thing
/// this view exists to tell apart.
#[tokio::test]
async fn a_viewer_that_cannot_keep_up_is_told_what_it_lost() {
    let (daemon, app) = wired();
    let job = a_job(&app).await;
    let feed = daemon.dispatching(&job);

    // A pipe too small to absorb the burst, so the socket task blocks on its
    // send and the subscription behind it is what falls behind.
    let mut socket = connected(app, &format!("/jobs/{}/observe", job.as_str()), 64).await;
    let TurnMessage::Opened(_) = read(&mut socket).await else {
        panic!("it opens");
    };

    for n in 0..(crate::WATCHING * 2) {
        feed.offer(said(&format!("row {n}")));
    }
    drop(feed);

    let mut told = 0;
    loop {
        match read(&mut socket).await {
            TurnMessage::Missed(missed) => {
                told = missed.dropped;
                break;
            }
            TurnMessage::Closed(_) => break,
            _ => continue,
        }
    }
    assert!(told > 0, "the viewer is told how many rows it lost");
}

/// The definition of done: a Board client on `/events` sees no transcript row
/// and no extra resync while a viewer watches a Job.
#[tokio::test]
async fn a_board_on_the_event_stream_sees_nothing_of_this() {
    let (daemon, app) = wired();
    let job = a_job(&app).await;
    let feed = daemon.dispatching(&job);

    let mut board = connected(app.clone(), "/events", 8192).await;
    let StreamMessage::Resync(_) = board_read(&mut board).await else {
        panic!("a Board opens with a resync");
    };

    let mut viewer = connected(app, &format!("/jobs/{}/observe", job.as_str()), 8192).await;
    let TurnMessage::Opened(_) = read(&mut viewer).await else {
        panic!("the viewer opens");
    };
    for n in 0..(crate::BACKLOG * 4) {
        feed.offer(said(&format!("row {n}")));
    }

    // Far more rows than the event stream's whole capacity. A row that reached
    // this socket, or a resync forced by an eviction there, both arrive as a
    // frame — and none does.
    let quiet = tokio::time::timeout(Duration::from_millis(250), board.next()).await;
    assert!(
        quiet.is_err(),
        "the Board's stream carries no transcript row and no extra resync"
    );
}

async fn board_read(socket: &mut WebSocketStream<DuplexStream>) -> StreamMessage {
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
