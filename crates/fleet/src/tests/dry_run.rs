//! A Drone asking whether its work passes, and what that does and does not do.
//!
//! # The proportion is the claim
//!
//! One case where a Drone gets an answer, and most where nothing else happens:
//! the step does not move, the gate runs the Checks again for itself and
//! reaches its own verdict, the convergence clocks do not count the wait, and
//! the two bounds refuse a call each. **A dry run that could advance a step
//! would be the Drone marking its own work**, so most of what these prove is
//! an absence.
//!
//! Why there are four modules: `report` is what comes back, `absence` is what
//! it did not decide and did not cost, `refusing` is the calls that get no
//! report at all, and `offering` is what a Drone was told before it made one.
//! The Fleet, the Checks and the wire are here, because a run assembled
//! differently between them would leave four modules answering about four
//! different steps.
//!
//! # The clock ticks a second per reading and jumps when a test says so
//!
//! [`Held`] is `silence`'s clock, for its reason: a threshold in minutes is not
//! one a test can sit through. `absence` pushes it **while a Check is running**,
//! from the test's own task, which is the only way to ask whether the vigil
//! counts that time against the Drone.

mod absence;
mod offering;
mod refusing;
mod report;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use config::ResolvedWorkflow;
use core_model::Timestamp;
use http_body_util::BodyExt;
use ipc::RunId;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch};
use tower::ServiceExt;

use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::daemon::Fleet;
use crate::dry_run::DryRuns;
use crate::silence::Liveness;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The production silence threshold, so what these cases push past is the
/// number that ships.
const QUIET_AFTER: Duration = Duration::from_secs(120);
/// The production wall clock, likewise.
const WALL_CLOCK: Duration = Duration::from_secs(1_500);

/// A clock that ticks a second per reading and jumps when a test says so.
struct Held {
    ticks: AtomicU64,
    pushed: AtomicU64,
}

impl Held {
    fn started() -> Held {
        Held {
            ticks: AtomicU64::new(0),
            pushed: AtomicU64::new(0),
        }
    }

    fn on(&self, seconds: u64) {
        self.pushed.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-28T{:02}:{:02}:{:02}.000Z",
            (at / 3_600) % 24,
            (at / 60) % 60,
            at % 60
        ))
    }
}

/// One step, gated on a named Check and on a non-empty diff — the two kinds
/// there are, so a report has one row of each.
fn one_step(run: &str) -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "suite",
                run,
                expect_exit_code: 0,
                when: &[],
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// One tool call, as the transcript would carry it.
fn called() -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }]
}

/// A Drone that says one thing and is then quiet for longer than any case runs.
fn a_quiet_drone() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called())
}

/// A Fleet holding that one step, with the production thresholds and a Judge
/// that fails every call — **nothing here may ask a model anything.**
fn a_fleet_checking(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    clock: Arc<Held>,
    allowed: u32,
) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    let x = 1;\n"),
        a_quiet_drone(),
    );
    fittings.workflows = one(workflow);
    fittings.clock = clock;
    fittings.liveness = Liveness::of(QUIET_AFTER, 2);
    fittings.norms = StepNorms::of(60, WALL_CLOCK, Duration::from_secs(120));
    fittings.dry_runs = DryRuns::of(allowed);
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a dry run"));
    Fleet::assembled(fittings)
}

/// Approve the Job and hand back its id, with a worktree on disk and a Drone in
/// the slot.
async fn started(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("a proposed Job");
    worktree_directory(home, job.id());
    dispatched(&fleet, job.id()).await.expect("an approved Job");
    job.id().clone()
}

fn router(fleet: &Arc<Fixture>) -> Router {
    api::router(api::Served::sharing(
        Arc::clone(fleet),
        RunId::carried("01RUN"),
        fleet.events(),
    ))
}

struct Said {
    text: String,
    is_error: bool,
}

/// The dry-run tool call, exactly as a client makes one: no `arguments`
/// member at all, because the tool takes none.
async fn ask(app: &Router) -> Said {
    let body = post(
        app,
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"run_checks"}}"#,
    )
    .await;
    let at = body
        .find("\"text\":\"")
        .unwrap_or_else(|| panic!("a tool result carrying one text block: {body}"));
    let rest = &body[at + 8..];
    let end = rest.find("\",").expect("a closed text block");
    Said {
        text: rest[..end].to_string(),
        is_error: body.contains("\"isError\":true"),
    }
}

async fn submit(app: &Router) {
    post(
        app,
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"submit_evidence",
            "arguments":{"claimed":"The parser takes it.","shown_by":"src/parse.rs",
            "not_claimed":""}}}"#,
    )
    .await;
}

async fn post(app: &Router, body: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri(api::MCP_PATH)
        .header("content-type", "application/json")
        // **With a peer, because a Drone tool call is attributed by one.** A
        // router served by `axum::serve` carries this from the accepted
        // connection; a `oneshot` carries whatever the test puts on it, and a
        // request with none is refused rather than guessed at — see
        // `crate::peer`.
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:51000"
                .parse::<std::net::SocketAddr>()
                .expect("a loopback address"),
        ))
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "never a 4xx and never a 500"
    );
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body that reads")
        .to_bytes()
        .to_vec();
    String::from_utf8(bytes).expect("a JSON body")
}

/// Wait until Fleet has the mark on, so the clock is pushed and the second call
/// is made while a run really is in flight rather than before one started.
async fn wait_until_checking(fleet: &Fixture) {
    for _ in 0..400 {
        if fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .is_some_and(|at_work| at_work.is_checking())
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the run never started");
}
