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
//! Why there are seven modules: `report` is what comes back, `absence` is what
//! it did not decide and did not cost, `refusing` is the calls that get no
//! report at all, `offering` is what a Drone was told before it made one,
//! `narrowing` is the second of the two runs it can ask for, `later` is the
//! report arriving as a turn whatever the call did, and `waiting` is a Drone
//! at rest until it does. The Fleet, the
//! Checks and the wire are here, because a run assembled differently between
//! them would leave five modules answering about five different steps.
//!
//! # The clock ticks a second per reading and jumps when a test says so
//!
//! [`Held`] is `silence`'s clock, for its reason: a threshold in minutes is not
//! one a test can sit through. `absence` pushes it **while a Check is running**,
//! from the test's own task, which is the only way to ask whether the vigil
//! counts that time against the Drone.

mod absence;
mod later;
mod leaving_out;
mod naming;
mod narrowing;
mod offering;
mod refusing;
mod report;
mod streaming;
mod waiting;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use config::ResolvedWorkflow;
use core_model::{DroneId, JobId, Timestamp};
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
    a_fleet_over(home, workflow, clock, allowed, &["src/parse.rs"])
}

/// The same, over a worktree holding whichever paths the case is about.
///
/// **A parameter rather than a second fixture**, because what a narrowed run
/// does is a function of the diff: the same step and the same Checks answer
/// differently for a change under `crates` and a change under `docs`, and two
/// fixtures would be two steps to keep in step.
fn a_fleet_over(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    clock: Arc<Held>,
    allowed: u32,
    changed: &[&str],
) -> Fixture {
    a_fleet_driven(home, workflow, clock, allowed, changed, a_quiet_drone())
}

/// The same, with the Drone a case needs in place of the quiet one.
fn a_fleet_driven(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    clock: Arc<Held>,
    allowed: u32,
    changed: &[&str],
    drone: FakeHarness,
) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(changed).showing("+    let x = 1;\n"),
        drone,
    );
    fittings.starting().workflows = one(workflow);
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
    worktree_directory(home, &job);
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

/// The dry-run tool call, exactly as a client makes one, asking for the whole
/// run — and then the report the Drone is sent, or the refusal it was given.
async fn ask(app: &Router, fleet: &Fixture, home: &TempDir) -> Said {
    asking(app, fleet, home, false).await
}

/// The same call, saying which of the two runs is wanted.
async fn asking(app: &Router, fleet: &Fixture, home: &TempDir, only_what_changed: bool) -> Said {
    let drone = the_one_drone(fleet).await;
    let before = match &drone {
        Some((job, drone)) => told_checks(fleet, home, job, drone).await.len(),
        None => 0,
    };
    let body = post(app, &call(only_what_changed)).await;
    let answered = Said {
        text: text_of(&body),
        is_error: body.contains("\"isError\":true"),
    };
    let Some((job, drone)) = drone.filter(|_| !answered.is_error) else {
        return answered;
    };
    assert!(
        answered.text.contains("later turn"),
        "the call says where the report comes from: {}",
        answered.text
    );
    for _ in 0..2_000 {
        let told = told_checks(fleet, home, &job, &drone).await;
        // The report is the run's last turn; a result that landed while others
        // still ran is a turn of its own before it (#1062).
        if let Some(text) = told
            .into_iter()
            .skip(before)
            .find(|turn| !turn.contains("Still going: "))
        {
            return Said {
                text,
                is_error: false,
            };
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the report never arrived as a later turn");
}

/// The `run_checks` call, as JSON-RPC.
fn call(only_what_changed: bool) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"run_checks",
            "arguments":{{"only_what_changed":{only_what_changed}}}}}}}"#
    )
}

/// The one Job being worked and its Drone, where there is one.
async fn the_one_drone(fleet: &Fixture) -> Option<(JobId, DroneId)> {
    let job = fleet.working_on().await.first().cloned()?;
    let drone = fleet.load(&job).await.ok()?.assigned_drone().cloned()?;
    Some((job, drone))
}

/// Every Checks report written into this Drone's transcript, oldest first, as
/// the rows carry the text. The transcript is the record of what Fleet said.
async fn told_checks(fleet: &Fixture, home: &TempDir, job: &JobId, drone: &DroneId) -> Vec<String> {
    let handle = fleet.load(job).await.expect("the Job").handle();
    let path = crate::transcript::transcript_of(&home.path().to_string_lossy(), &handle, drone);
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|row| row.contains("\"occasion\":\"checks\""))
        .map(text_of)
        .collect()
}

/// The first `text` string in a JSON body, still escaped.
fn text_of(body: &str) -> String {
    let Some(at) = body.find("\"text\":\"") else {
        panic!("a body carrying a text field: {body}");
    };
    let mut escaped = false;
    body[at + 8..]
        .chars()
        .take_while(|c| {
            let closes = *c == '"' && !escaped;
            escaped = *c == '\\' && !escaped;
            !closes
        })
        .collect()
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
