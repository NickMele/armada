//! A Drone asking whether its work passes, and what that does and does not do.
//!
//! # The proportion is the claim
//!
//! One case where a Drone gets an answer, and five where nothing else happens:
//! the step does not move, the gate runs the Checks again for itself and
//! reaches its own verdict, the convergence clocks do not count the wait, and
//! the two bounds refuse a call each. **A dry run that could advance a step
//! would be the Drone marking its own work**, so most of what this file proves
//! is an absence.
//!
//! # The Checks are real commands and one of them changes its mind
//!
//! `/bin/test ! -e <marker>` passes while a file is not there and fails once it
//! is. That is the only way to write the case that matters: a dry run in which
//! everything passed, followed by a gate in which the same Check does not — and
//! a step that ends `completed_failed` regardless of what the Drone was told a
//! moment earlier.
//!
//! # The clock is pushed rather than waited on
//!
//! [`Held`] is `silence`'s clock, for its reason: a threshold in minutes is not
//! one a test can sit through. What is different here is that it is pushed
//! **while a Check is running**, from the test's own task, which is the only
//! way to ask whether the vigil counts that time against the Drone.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use config::ResolvedWorkflow;
use core_model::{JobStatus, StepId, Timestamp};
use http_body_util::BodyExt;
use ipc::RunId;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch};
use tower::ServiceExt;

use crate::briefing::first_turn;
use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::daemon::Fleet;
use crate::dry_run::{DryRuns, NotRun};
use crate::silence::Liveness;
use crate::terms::Checking;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::checked_by_the_one;

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

/// The Check command that passes until `marker` exists.
///
/// `/bin/test` rather than a shell, because `checks_runner` splits a `run`
/// string on whitespace and executes the program directly — which is the whole
/// reason a Manifest Check cannot pipe.
fn passes_until(marker: &std::path::Path) -> String {
    format!("/bin/test ! -e {}", marker.display())
}

// ------------------------------------------------- a Drone gets its answer

/// **The claim of the whole issue, over the wire a Drone actually uses.** A
/// tool call arrives as JSON-RPC on the router that ships, Fleet runs the
/// step's Checks in the Drone's worktree, and what comes back names each one,
/// says what it did, and says where to read more.
#[tokio::test]
async fn a_drone_asking_for_the_checks_is_told_what_each_one_did() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/false"),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    let job = started(&fleet, &home).await;

    let said = ask(&app).await;
    assert!(!said.is_error, "{}", said.text);
    assert!(
        said.text.contains("suite") && said.text.contains("FAILED"),
        "the Check that did not pass is named and so is what it did: {}",
        said.text
    );
    assert!(
        said.text.contains("it exited 1"),
        "the failure's own sentence, not a bare word: {}",
        said.text
    );
    assert!(
        said.text.contains("diff_nonempty"),
        "the built-in is a row like any other: {}",
        said.text
    );
    assert!(
        said.text.contains("implement.1.dry.0.log"),
        "and where to read the rest of it: {}",
        said.text
    );
    assert!(
        said.text.contains("not a verdict"),
        "said in the answer and not only in the briefing: {}",
        said.text
    );

    // The path it named is a file that is there, holding what the Check
    // printed — a pointer to nothing would be worse than no pointer.
    let log = crate::check_output::checks_dir(&home.path().to_string_lossy(), &job)
        .join("implement.1.dry.0.log");
    assert!(
        log.is_file(),
        "{} was named and does not exist",
        log.display()
    );
    assert!(
        std::fs::read_to_string(&log)
            .expect("the log reads")
            .contains("--- stdout ---"),
        "both streams, each behind its marker"
    );
    // **And not where the gate writes.** A dry run writes no row, so a file at
    // the gate's own path would be output the record does not point at. Both
    // carry the attempt, so this holds per run rather than once per step —
    // #63 made a step workable twice and the path is the whole key.
    assert!(
        !crate::check_output::checks_dir(&home.path().to_string_lossy(), &job)
            .join("implement.1.0.log")
            .exists(),
        "a dry run wrote over the gate's log"
    );
}

/// **Adding a Check to a step is the whole of adding it.** #200 named three
/// more Manifest Checks on every step that produces a diff, and the worry was
/// that `run_checks` reached the two it already knew — a Drone that cannot see
/// what failed cannot fix it, and the allowlist denies it `pnpm` as it denies
/// it `cargo`.
///
/// Nothing in `dry_run` names a Check. It walks whatever the frozen step
/// declares, so a step declaring five is answered with five rows, and the
/// failing one among them is named whichever position it sits in.
#[tokio::test]
async fn every_check_the_step_declares_gets_a_row_however_many_there_are() {
    let home = TempDir::new();
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "build",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::Check {
                name: "typecheck",
                run: "/usr/bin/false",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::Check {
                name: "bridge_build",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::Check {
                name: "storybook",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let fleet = Arc::new(a_fleet_checking(
        &home,
        workflow,
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = ask(&app).await;
    assert!(!said.is_error, "{}", said.text);
    for named in ["build", "typecheck", "bridge_build", "storybook"] {
        assert!(
            said.text.contains(named),
            "`{named}` was declared and is not in the report: {}",
            said.text
        );
    }
    assert!(
        said.text.contains("typecheck") && said.text.contains("it exited 1"),
        "the one that failed is the one reported failing: {}",
        said.text
    );
}

// ------------------------------------------------------- and nothing moves

/// **The case the whole design turns on.** Every Check passes in the dry run,
/// the world then changes underneath it, and the gate reaches its own verdict
/// on its own run — so the pass the Drone was shown satisfied nothing.
#[tokio::test]
async fn a_dry_run_that_passed_does_not_satisfy_the_gate() {
    let home = TempDir::new();
    let marker = home.path().join("broken");
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step(&passes_until(&marker)),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    let job = started(&fleet, &home).await;

    let said = ask(&app).await;
    assert!(
        said.text.contains("PASSED") && !said.text.contains("FAILED"),
        "every check passed in the dry run: {}",
        said.text
    );
    let record = fleet.load(&job).await.expect("the Job");
    assert_eq!(
        record.status(),
        JobStatus::Running,
        "a dry run moved the Job"
    );
    assert_eq!(
        record
            .step(&StepId::new("implement"))
            .expect("the step")
            .state(),
        core_model::StepState::Running,
        "a dry run moved the step"
    );
    assert!(
        fleet
            .store()
            .lock()
            .await
            .step_checks(&job)
            .expect("the rows read")
            .is_empty(),
        "a dry run wrote a Check row, which is the record claiming a run that \
         decided something"
    );

    // The same Check, now failing. Nothing the Drone did caused this and that
    // is the point: what the gate rules on is its own run and never the one
    // the Drone was shown.
    std::fs::write(&marker, "").expect("the marker writes");
    submit(&app).await;
    fleet.turn().await.expect("the gate runs");
    assert_eq!(
        fleet.load(&job).await.expect("the Job").status(),
        JobStatus::AwaitingRepair,
        "the gate re-ran the checks and reached its own verdict"
    );
}

// ------------------------------------------------------- and the clocks wait

/// **A Drone waiting on a Check Fleet is running is not silent and is not
/// thrashing.** The clock is pushed a thousand times past the silence threshold
/// while the run is in flight, and the vigil says nothing — then the step's own
/// wall clock is read afterwards and the time is not on it.
///
/// Without the suspension this is the tripwire firing on the honest case, which
/// is exactly what offering a Drone a `cargo build` would otherwise create.
#[tokio::test]
async fn the_clocks_do_not_count_the_time_a_check_takes() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 2"),
        Arc::clone(&clock),
        3,
    ));
    let job = started(&fleet, &home).await;

    let running = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        async move { checked_by_the_one(&fleet).await }
    });
    wait_until_checking(&fleet).await;

    // Far past the silence threshold, the pokes and the wall clock together.
    clock.on(QUIET_AFTER.as_secs() * 1_000);
    for _ in 0..8 {
        let turned = fleet.turn().await.expect("a turn");
        assert!(
            turned.quiet().is_none(),
            "the vigil counted a Check Fleet was running against the Drone: {:?}",
            turned.quiet()
        );
        assert!(
            turned.wandering().is_none(),
            "the thrashing chain fired while Fleet was answering the Drone: {:?}",
            turned.wandering()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fleet.load(&job).await.expect("the Job").status(),
        JobStatus::Running,
        "the Job escalated while Fleet was running its own checks"
    );

    let report = running.await.expect("the run finished").expect("a report");
    assert_eq!(report.ran.len(), 2);

    // And the time is given back rather than merely not read: a suspension that
    // only skipped the check would leave the wall clock over its ceiling for
    // the rest of the step.
    let now = fleet.now();
    let running_for = fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .expect("a Drone is working")
        .running_for(&now);
    assert!(
        running_for < WALL_CLOCK,
        "the step's wall clock kept the {} seconds Fleet spent on its own \
         checks: {running_for:?}",
        QUIET_AFTER.as_secs() * 1_000
    );
}

// ------------------------------------------------------------ what bounds it

/// **The correctness bound.** Two runs at once are two builds in one worktree,
/// contending for one target directory, and neither answer would be about the
/// work — so the second is refused rather than queued.
#[tokio::test]
async fn a_second_run_while_one_is_going_is_refused() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 2"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;

    let running = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        async move { checked_by_the_one(&fleet).await }
    });
    wait_until_checking(&fleet).await;

    let refused = checked_by_the_one(&fleet).await;
    assert!(
        matches!(refused, Err(NotRun::AlreadyRunning)),
        "{refused:?}"
    );
    running.await.expect("the run finished").expect("a report");
}

/// The gate runs these same Checks in this same worktree, so a Drone that has
/// already submitted is told to wait rather than given a second run alongside
/// the one that decides.
#[tokio::test]
async fn a_drone_that_has_already_submitted_is_told_to_wait() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/true"),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    submit(&app).await;
    assert_eq!(fleet.evidence_waiting(), 1, "the gate has not run yet");
    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("the gate is about to");
    assert!(matches!(refused, NotRun::AlreadySubmitted), "{refused:?}");
    assert!(
        refused.to_string().contains("later turn"),
        "and is told where the answer comes from: {refused}"
    );
}

/// **The cost bound.** The clock suspension above removes the pressure that
/// would otherwise have limited this, so a count does — and the refusal says
/// what to do instead, because a Drone told only "no" asks again.
#[tokio::test]
async fn a_step_that_has_spent_its_allowance_is_refused_and_told_why() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(
        &home,
        one_step("/usr/bin/true"),
        Arc::new(Held::started()),
        1,
    );
    started(&fleet, &home).await;

    checked_by_the_one(&fleet).await.expect("the first run");
    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("the second is refused");
    assert!(
        matches!(refused, NotRun::Spent { allowed: 1 }),
        "{refused:?}"
    );
    let said = refused.to_string();
    assert!(
        said.contains("submit"),
        "a Drone out of runs is told what to do instead: {said}"
    );
}

/// A step declaring no mechanical Check is refused rather than answered with an
/// empty report — a report with no rows reads as a run that found nothing
/// wrong, which is the vacuous pass one layer out.
#[tokio::test]
async fn a_step_with_no_checks_is_refused_rather_than_answered_with_nothing() {
    let home = TempDir::new();
    let unchecked = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let fleet = a_fleet_checking(&home, unchecked, Arc::new(Held::started()), 3);
    started(&fleet, &home).await;

    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("nothing to run");
    assert!(
        matches!(refused, NotRun::StepHasNoChecks { .. }),
        "{refused:?}"
    );
}

/// A call arriving when no Job is being worked is a tool error the Drone can
/// read, never a queued run against whatever comes next.
#[tokio::test]
async fn a_call_with_nothing_working_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(
        &home,
        one_step("/usr/bin/true"),
        Arc::new(Held::started()),
        3,
    );

    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("nothing is working");
    assert!(matches!(refused, NotRun::NothingIsWorking), "{refused:?}");
}

// -------------------------------------------------- and the Drone is told

/// **A tool nothing points at is the defect this capability is about.** The
/// first turn offers it, in the same block shape a scope declaration is asked
/// for in — and it names no tool, for the reason `Declaring` does not.
#[test]
fn the_first_turn_offers_the_dry_run_and_says_it_is_not_a_pass() {
    let workflow = one_step("/usr/bin/true");
    let said = first_turn(
        &crate::tests::briefing::a_job(),
        workflow.frozen(),
        &StepId::new("implement"),
        &crate::crossing::Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string();

    assert!(said.contains("FINDING OUT WHERE YOU STAND"), "{said}");
    assert!(
        said.contains("not a verdict"),
        "a Drone that read a green dry run as a finished part would be worse \
         off for having been offered it: {said}"
    );
    assert!(
        said.contains("Submitting is still the only way to report"),
        "{said}"
    );
    assert!(
        !said.contains("mcp__") && !said.contains("run_checks"),
        "described rather than named, like the other two tools: {said}"
    );
    assert!(
        !said.contains("/usr/bin/true"),
        "the offer is not the Check. Nothing here is written from a resolved \
         command: {said}"
    );
}

/// And a step with nothing to run is not offered it. A Drone pointed at a call
/// that will be refused reads the refusal as a broken system, which is the
/// silent denial this whole issue is about arriving from the other side.
#[test]
fn a_step_with_no_checks_is_not_offered_the_dry_run() {
    let unchecked = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let step = unchecked
        .frozen()
        .steps()
        .iter()
        .find(|step| step.id() == &StepId::new("implement"))
        .expect("the step")
        .clone();
    assert!(Checking::at(&step).is_none());
}

// ------------------------------------------------------------------ plumbing

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

/// One step whose named Check covers `packages/**` — paths the fixture Fleet's
/// worktree, which holds `src/parse.rs`, does not touch.
fn one_scoped_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "storybook",
                run: "/usr/bin/false",
                expect_exit_code: 0,
                when: &["packages/**"],
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// **The rehearsal has to agree with the gate.** The report's own closing
/// sentence promises the Drone the same Checks, run by Fleet — so a dry run
/// that spent a Check the gate will skip would be telling a Drone its work
/// failed something no gate is going to ask.
#[tokio::test]
async fn a_dry_run_skips_the_same_check_the_gate_would() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_scoped_step(),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    let _job = started(&fleet, &home).await;

    let said = ask(&app).await;
    assert!(!said.is_error, "{}", said.text);
    // The command exits 1. A row that is not FAILED is a Check that never ran.
    assert!(
        said.text.contains("storybook") && said.text.contains("SKIPPED"),
        "the Check is named and said to have been skipped: {}",
        said.text
    );
    assert!(
        !said.text.contains("FAILED"),
        "nothing failed, so nothing may say so: {}",
        said.text
    );
    // And the closing line does not report a pass nobody earned.
    assert!(
        said.text.contains("cover paths this step did not touch"),
        "the summary says what was not run: {}",
        said.text
    );
    assert!(
        !said.text.contains("2 of 2 passed"),
        "one of the two was never run: {}",
        said.text
    );
}
