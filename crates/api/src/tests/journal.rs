//! One Job's own log, over the same in-memory pipe the other two sockets use.
//!
//! What these have to prove is the half a reader cannot check: that a viewer
//! gets what was written before it connected **and** what is written after,
//! from one connection and with no Drone anywhere in the picture — which is the
//! whole case this route exists for.
//!
//! The reader is a fake rather than a file. What `fleet::journal` does with a
//! real `.armada/logs/` is proved over there, beside the file; what is proved
//! here is the socket's own behaviour, which is the part `api` owns.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use futures_util::StreamExt;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use ipc::{Instant, JobId, JournalMessage, LogNote, NoteLevel, NotedField, Quiet, StepId, Voice};
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tower::{Service, ServiceExt};

use crate::journal::{Journal, Reading, Window};
use crate::tests::connected;
use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::{run_id, A_PROPOSAL};
use crate::{router, Broadcaster, Served};

/// A log a test appends to, standing in for the file.
///
/// It answers on the same terms the file reader does — everything after a
/// cursor, and where the next pass starts — so what the socket is driven
/// through here is the same contract Fleet implements.
#[derive(Default)]
struct FakeLog {
    written: Mutex<Vec<LogNote>>,
    unreadable: Mutex<bool>,
}

impl FakeLog {
    fn wrote(&self, note: LogNote) {
        self.written.lock().expect("not poisoned").push(note);
    }

    fn broke(&self) {
        *self.unreadable.lock().expect("not poisoned") = true;
    }
}

impl Journal for FakeLog {
    fn read(&self, _handle: &str, from: u64) -> Reading {
        let written = self.written.lock().expect("not poisoned");
        let seen = from as usize;
        Reading {
            notes: written.iter().skip(seen).cloned().collect(),
            from: written.len() as u64,
            skipped: 0,
            unreadable: *self.unreadable.lock().expect("not poisoned"),
        }
    }

    /// The settled read, on the same terms: everything written, counted, with
    /// nothing in front of it. What a real window leaves out is proved in
    /// `fleet`, beside the file; what is proved here is the route's own answer.
    fn window(&self, _handle: &str) -> Window {
        let written = self.written.lock().expect("not poisoned");
        Window {
            notes: written.clone(),
            from_note: 1,
            total_notes: written.len() as u32,
            undecodable: 0,
            bytes: written.len() as u64,
            unreadable: *self.unreadable.lock().expect("not poisoned"),
            path: String::from(".armada/logs/a-proposal.jsonl"),
        }
    }
}

fn note(msg: &str) -> LogNote {
    LogNote {
        at: Instant::carried("2026-09-04T09:00:00.000Z"),
        by: Voice::Fleet,
        level: NoteLevel::Info,
        msg: msg.to_string(),
        step: None,
        drone: None,
        fields: Vec::new(),
    }
}

fn wired(log: Arc<FakeLog>) -> Router {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    router(Served::sharing(daemon, run_id(), events).reading(log))
}

/// A Job the fake holds, so the route is not a 404.
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

async fn read(socket: &mut WebSocketStream<DuplexStream>) -> JournalMessage {
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket answers")
        .expect("the socket is open")
        .expect("a frame");
    let Message::Text(json) = frame else {
        panic!("the socket is text: {frame:?}");
    };
    ipc::decode("a journal message", json.as_bytes()).expect("a journal message")
}

fn said(message: JournalMessage) -> LogNote {
    let JournalMessage::Note(note) = message else {
        panic!("a note, not {message:?}");
    };
    note
}

/// The claim, and the issue's own words: a Job with no agent on it shows what
/// Fleet is doing to it, live.
#[tokio::test]
async fn a_job_with_no_drone_gets_what_fleet_did_and_then_what_it_does_next() {
    let log = Arc::new(FakeLog::default());
    log.wrote(note("worktree cut"));
    let app = wired(Arc::clone(&log));
    let job = a_job(&app).await;

    let mut socket = connected(app, &format!("/jobs/{}/log", job.as_str()), 8192).await;

    let JournalMessage::Opened(opened) = read(&mut socket).await else {
        panic!("the first message says what this connection is");
    };
    assert_eq!(opened.protocol_version, ipc::PROTOCOL_VERSION);
    assert_eq!(opened.job_id, job);
    assert_eq!(opened.skipped, 0);

    // Written before anybody connected, and still read. **This is the case the
    // transcript socket cannot serve** — no Drone has been spawned, so there is
    // nothing writing and nothing to observe.
    assert_eq!(said(read(&mut socket).await).msg, "worktree cut");

    log.wrote(note("preparation began"));
    assert_eq!(said(read(&mut socket).await).msg, "preparation began");
}

/// Attribution and payload cross with the note. **Every entry names who and
/// every entry opens** — the two rules the component was written around, and
/// the two a stream of grey prose would break.
#[tokio::test]
async fn a_note_carries_its_voice_its_level_and_the_fields_it_opens_to() {
    let log = Arc::new(FakeLog::default());
    log.wrote(LogNote {
        level: NoteLevel::Warn,
        step: Some(StepId::carried("implement")),
        fields: vec![NotedField {
            name: "paths".to_string(),
            value: "crates/api/src/routes.rs".to_string(),
        }],
        ..note("the step edited outside its declared scope")
    });
    let app = wired(Arc::clone(&log));
    let job = a_job(&app).await;

    let mut socket = connected(app, &format!("/jobs/{}/log", job.as_str()), 8192).await;
    let JournalMessage::Opened(_) = read(&mut socket).await else {
        panic!("it opens");
    };

    let note = said(read(&mut socket).await);
    assert_eq!(note.by, Voice::Fleet, "the wire says who, never the reader");
    assert_eq!(note.level, NoteLevel::Warn);
    assert_eq!(note.step, Some(StepId::carried("implement")));
    assert_eq!(note.fields.len(), 1, "the row has a payload to open");
}

/// A log Fleet cannot read ends the stream with a sentence. **A socket that
/// simply stops is indistinguishable from one that broke**, and here it would
/// be indistinguishable from a Job nothing is happening to — which is the
/// reading this whole route exists to stop being wrong.
#[tokio::test]
async fn a_log_that_will_not_read_says_so_rather_than_going_quiet() {
    let log = Arc::new(FakeLog::default());
    log.broke();
    let app = wired(Arc::clone(&log));
    let job = a_job(&app).await;

    let mut socket = connected(app, &format!("/jobs/{}/log", job.as_str()), 8192).await;
    let JournalMessage::Opened(_) = read(&mut socket).await else {
        panic!("it opens");
    };
    let JournalMessage::Closed(closed) = read(&mut socket).await else {
        panic!("a closing message, not silence");
    };
    assert_eq!(closed.because, Quiet::Unreadable);
}

/// A Job id that names nothing is refused **before** the upgrade, so a caller
/// reads the 404 at the moment they asked rather than opening a socket that
/// says nothing.
#[tokio::test]
async fn an_unknown_job_is_a_404_before_the_upgrade() {
    let app = wired(Arc::new(FakeLog::default()));
    assert_eq!(
        handshake(app, "/jobs/01NOSUCHJOB/log").await,
        StatusCode::NOT_FOUND
    );
}

/// A listener built with no reader answers a fault naming what is missing.
/// **Never an empty stream** — that reads as a Job nothing is happening to,
/// which is the one thing this route must never say by accident.
///
/// **The Job has to exist for this to be the answer.** The `:job_id` segment is
/// resolved before the handler runs, so an id naming nothing is the 404 above
/// whatever the listener was built with.
#[tokio::test]
async fn a_fleet_with_no_reader_says_so_rather_than_opening_an_empty_stream() {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    let app = router(Served::sharing(daemon, run_id(), events));
    let job = a_job(&app).await;
    assert_eq!(
        handshake(app, &format!("/jobs/{}/log", job.as_str())).await,
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

/// The claim `get_job_log` makes: the same log, answered once, over HTTP.
///
/// **The half an agent can use.** It cannot be interrupted mid-turn and so
/// cannot follow the socket above; what it needs is the lines already written,
/// and this is that read.
#[tokio::test]
async fn the_settled_read_answers_the_log_once_and_says_it_is_a_window() {
    let log = Arc::new(FakeLog::default());
    log.wrote(note("worktree cut"));
    log.wrote(note("preparation began"));
    let app = wired(Arc::clone(&log));
    let job = a_job(&app).await;

    let answered = fetched(&app, &format!("/jobs/{}/log/read", job.as_str())).await;
    assert_eq!(answered.0, StatusCode::OK);
    let read: ipc::JobLog = ipc::decode("a Job log", &answered.1).expect("a Job log comes back");

    assert_eq!(read.job_id, job);
    assert_eq!(read.notes.len(), 2);
    assert_eq!(read.notes[0].msg, "worktree cut");
    assert_eq!(read.notes[1].msg, "preparation began");
    assert_eq!(read.from_note, 1);
    assert_eq!(read.total_notes, 2);
    assert!(
        read.whole,
        "the answer says whether it is one, and is not asked"
    );
    assert!(!read.unreadable);
    assert_eq!(read.path, ".armada/logs/a-proposal.jsonl");
}

/// **The stream is not reopened, and nothing about it moves.** The settled read
/// is a second route on the same reader; the socket next door still opens, and
/// still hands the same notes.
#[tokio::test]
async fn the_settled_read_leaves_the_stream_exactly_where_it_was() {
    let log = Arc::new(FakeLog::default());
    log.wrote(note("worktree cut"));
    let app = wired(Arc::clone(&log));
    let job = a_job(&app).await;

    let answered = fetched(&app, &format!("/jobs/{}/log/read", job.as_str())).await;
    assert_eq!(answered.0, StatusCode::OK);

    let mut socket = connected(app, &format!("/jobs/{}/log", job.as_str()), 8192).await;
    let JournalMessage::Opened(_) = read(&mut socket).await else {
        panic!("the socket still opens");
    };
    assert_eq!(said(read(&mut socket).await).msg, "worktree cut");
}

/// A log that will not read answers with what it had and says so, rather than
/// refusing. **A refusal would throw away the notes that did read**, on the one
/// call somebody makes when something is already wrong.
#[tokio::test]
async fn a_settled_read_of_a_broken_log_says_so_rather_than_refusing() {
    let log = Arc::new(FakeLog::default());
    log.wrote(note("worktree cut"));
    log.broke();
    let app = wired(Arc::clone(&log));
    let job = a_job(&app).await;

    let answered = fetched(&app, &format!("/jobs/{}/log/read", job.as_str())).await;
    assert_eq!(answered.0, StatusCode::OK, "not a refusal");
    let read: ipc::JobLog = ipc::decode("a Job log", &answered.1).expect("a Job log comes back");
    assert!(read.unreadable);
    assert!(!read.whole, "and so it is not the whole of it");
    assert_eq!(read.notes.len(), 1, "what did read is still carried");
}

/// A Job id that names nothing is the extractor's 404, exactly as it is on the
/// socket. The reader is never reached.
#[tokio::test]
async fn the_settled_read_of_an_unknown_job_is_a_404() {
    let app = wired(Arc::new(FakeLog::default()));
    let answered = fetched(&app, "/jobs/01NOSUCHJOB/log/read").await;
    assert_eq!(answered.0, StatusCode::NOT_FOUND);
}

/// A listener built with no reader says so here too. **Never an empty window**
/// — a Job whose log nothing can read is not a Job nothing happened to.
#[tokio::test]
async fn a_settled_read_with_no_reader_is_a_fault_and_not_an_empty_window() {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    let app = router(Served::sharing(daemon, run_id(), events));
    let job = a_job(&app).await;
    let answered = fetched(&app, &format!("/jobs/{}/log/read", job.as_str())).await;
    assert_eq!(answered.0, StatusCode::INTERNAL_SERVER_ERROR);
}

/// **The issue's own claim, end to end**: an agent investigating a Job reads
/// what Fleet did to it, in one call, without following a stream.
///
/// It goes through the door rather than the route, because the door is what an
/// agent has. Nothing hand-lists the tool — it is offered because the
/// inventory's `agent_access` column says so.
#[tokio::test]
async fn an_agent_reads_what_fleet_did_to_a_job_in_one_call() {
    let log = Arc::new(FakeLog::default());
    log.wrote(note("worktree cut"));
    log.wrote(note("preparation ran `pnpm install`"));
    let app = wired(Arc::clone(&log));
    let job = a_job(&app).await;

    let offered = called(&app, r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).await;
    assert!(
        offered.contains("get_job_log"),
        "the tool set is generated from the inventory: {offered}"
    );

    let answered = called(
        &app,
        &format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{
               "name":"get_job_log","arguments":{{"job_id":"{}"}}}}}}"#,
            job.as_str()
        ),
    )
    .await;

    assert!(answered.contains("preparation ran"), "{answered}");
    assert!(answered.contains("worktree cut"), "{answered}");
    // And it is one answer, not a subscription: nothing here is a cursor to
    // come back with.
    assert!(answered.contains("total_notes"), "{answered}");
}

/// One JSON-RPC message through the agent's door, as the text that came back.
async fn called(app: &Router, body: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(crate::door::DOOR_PATH)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("a body")
        .to_bytes();
    String::from_utf8(body.to_vec()).expect("text")
}

/// One GET against the router, as its status and its body.
async fn fetched(app: &Router, path: &str) -> (StatusCode, axum::body::Bytes) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("a body")
        .to_bytes();
    (status, body)
}

/// What a real handshake was answered with, where it was refused.
///
/// A hand-built request that is not a valid upgrade is refused by the extractor
/// with a 426 and never reaches the question either case above asks — so both
/// go through a genuine client rather than a `oneshot`.
async fn handshake(app: Router, path: &str) -> StatusCode {
    let (client_side, server_side) = tokio::io::duplex(8192);
    tokio::spawn(async move {
        let service = service_fn(move |request| app.clone().call(request));
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(server_side), service)
            .with_upgrades()
            .await;
    });
    let opened =
        tokio_tungstenite::client_async(format!("ws://fleet.invalid{path}"), client_side).await;
    let Err(tokio_tungstenite::tungstenite::Error::Http(answer)) = opened else {
        panic!("the handshake is refused before any socket opens");
    };
    answer.status()
}
