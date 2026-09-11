//! One running Check's log, over the same in-memory pipe the other sockets use.
//!
//! What these prove is the half a reader cannot check from outside: that a
//! viewer gets what the Check printed before it connected **and** what it
//! prints after, from one connection, and is told in a sentence when the Check
//! has ended rather than left watching a file that will not grow again.
//!
//! The reader is a fake, for `crate::tests::journal`'s reason: what
//! `fleet::following` does with a real file is proved over there.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use futures_util::StreamExt;
use ipc::{JobId, OutputEnded, OutputMessage};
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tower::ServiceExt;

use crate::tests::connected;
use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::{run_id, A_PROPOSAL};
use crate::{router, Broadcaster, Follow, Followed, LiveOutput, Served};

const KEPT: &str = "implement.1.live.0.log";

/// A log a test appends to and then ends, standing in for the file.
#[derive(Default)]
struct Growing {
    written: Mutex<Vec<String>>,
    ended: Mutex<bool>,
}

impl Growing {
    fn wrote(&self, line: &str) {
        self.written
            .lock()
            .expect("not poisoned")
            .push(line.to_string());
    }

    fn ended(&self) {
        *self.ended.lock().expect("not poisoned") = true;
    }
}

impl Follow for Growing {
    fn read(&self, from: u64, _to_the_end: bool) -> Followed {
        let written = self.written.lock().expect("not poisoned");
        Followed {
            lines: written.iter().skip(from as usize).cloned().collect(),
            from: written.len() as u64,
            skipped: 0,
            unreadable: false,
        }
    }

    fn writing(&self) -> bool {
        !*self.ended.lock().expect("not poisoned")
    }
}

async fn wired(log: Arc<Growing>) -> (Router, JobId) {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    *daemon.live.lock().expect("not poisoned") = Some((
        KEPT.to_string(),
        LiveOutput {
            name: "test".to_string(),
            attempt: 1,
            path: format!(".armada/checks/01JOB/{KEPT}"),
            follow: log,
        },
    ));
    let app = router(Served::sharing(daemon, run_id(), events));
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
    (app, job.id)
}

async fn read(socket: &mut WebSocketStream<DuplexStream>) -> OutputMessage {
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the socket is text: {frame:?}");
    };
    ipc::decode("a Check log message", json.as_bytes()).expect("a Check log message")
}

fn lines(message: OutputMessage) -> Vec<String> {
    let OutputMessage::Lines(lines) = message else {
        panic!("lines, not {message:?}");
    };
    lines.lines
}

/// **The claim `#628` makes of this route.** What the Check printed before
/// anybody opened it, then what it prints next, then a sentence once it ends.
#[tokio::test]
async fn a_running_checks_log_arrives_as_it_is_written_and_says_when_the_check_ends() {
    let log = Arc::new(Growing::default());
    log.wrote("compiling armada v0.0.0");
    let (app, job) = wired(Arc::clone(&log)).await;

    let path = format!("/jobs/{}/checks/{KEPT}/observe", job.as_str());
    let mut socket = connected(app, &path, 8192).await;

    let OutputMessage::Opened(opened) = read(&mut socket).await else {
        panic!("the first message says whose log this is");
    };
    assert_eq!(opened.protocol_version, ipc::PROTOCOL_VERSION);
    assert_eq!(opened.job_id, job);
    assert_eq!(
        (opened.name.as_str(), opened.attempt),
        ("test", 1),
        "whose it is comes off the answer, not off the row that was pressed"
    );
    assert_eq!(
        lines(read(&mut socket).await),
        vec!["compiling armada v0.0.0"]
    );

    log.wrote("running 12 tests");
    assert_eq!(lines(read(&mut socket).await), vec!["running 12 tests"]);

    log.wrote("test result: ok");
    log.ended();
    let mut last = read(&mut socket).await;
    if let OutputMessage::Lines(tail) = last {
        assert_eq!(tail.lines, vec!["test result: ok"]);
        last = read(&mut socket).await;
    }
    let OutputMessage::Closed(closed) = last else {
        panic!("a closing sentence, not silence: {last:?}");
    };
    assert_eq!(closed.because, OutputEnded::Finished);
}
