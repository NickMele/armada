//! A Drone working and not getting anywhere, and the four stages between that
//! and an escalation.
//!
//! # The cases are the joints, not the happy path
//!
//! The chain's whole content is what it refuses to do early. So most of what is
//! below asserts an absence: a tripwire that escalates nothing, a look that
//! stops the chain, a Drone that reports and is therefore not thrashing. Only
//! one case reaches `thrashing`, which is the proportion the trigger's own
//! wording asks for.
//!
//! # These start a real child and a real shell
//!
//! The Drone is a shell that prints a line and then either answers what is
//! injected into it or does not. That is the whole difference between the last
//! two cases, and it is a property of a process rather than of a value.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{EscalationTrigger, JobStatus, StepState, StepVerdict, TransitionReason};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Scoped, Sketch};
use verification::{Convergence, NotConverging, MID_STEP_CONVERGENCE};

use crate::converging::{stops_the_step, ReportNow, Stage, StepNorms, Tripwire, Wandering};
use crate::daemon::Fleet;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;
use ipc::mcp::DeclareScope;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The answer a Judge gives when it has decided the step is going nowhere.
const THRASHING: &str = "state: thrashing\n\
                         expected: the parser accepts a trailing comma\n\
                         produced: the same panic on the same input\n\
                         consequence: every caller still crashes on the same file";

/// Norms nothing but the tool-call count can trip.
fn on_calls(calls: u32) -> StepNorms {
    StepNorms::of(
        calls,
        Duration::from_secs(86_400),
        Duration::from_secs(86_400),
    )
}

/// One step, gated on nothing, so nothing but the chain can move it.
fn one_step(scope: Option<Scoped<'static>>) -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope,
        gaming: None,
    }])
}

/// A Drone that prints one line and then never reads or says anything again.
///
/// The line decodes as `calls` tool calls, which is how a test puts a live
/// count where the tripwire reads it.
///
/// **It is deliberately not one `Ended` carrying a turn count.** That is what
/// this fixture used to do, and it could only trip the wire because the wire
/// was reading a number the harness does not publish until an invocation is
/// over — which is to say the test asserted the defect. A Drone mid-step has
/// made calls and has ended nothing.
/// Whether a pid is still running. `kill -0` rather than `libc::kill`, because
/// this crate denies `unsafe` and the one exception is the `setsid` in
/// `detach.rs` — a test is not a second reason to open that door.
fn alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn a_drone_that_will_not_answer(calls: u32) -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called(calls))
}

/// The same Drone, except that it answers every turn injected into it. Its
/// answer is a terminating event, because coming to rest is what `boundaries`
/// counts and what the forced report is read against.
fn a_drone_that_answers(calls: u32) -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            "echo BUSY; while IFS= read -r line; do echo RESTED; done",
        ],
    )
    .reading("BUSY", called(calls))
    .reading("RESTED", vec![ended()])
}

/// `calls` tool calls, as the transcript would carry them one at a time.
fn called(calls: u32) -> Vec<DroneEvent> {
    (0..calls)
        .map(|nth| DroneEvent::Called {
            tool: String::from("Read"),
            call: format!("call-{nth}"),
            detail: CallDetail::of("a file"),
        })
        .collect()
}

fn ended() -> DroneEvent {
    DroneEvent::Ended {
        turns: 0,
        cost_micros: 0,
        refusals: 0,
    }
}

/// A Fleet whose one step is watched by these norms and looked at by this
/// Judge, with that Drone on it.
fn a_watched_fleet(
    home: &TempDir,
    harness: FakeHarness,
    judge: Arc<FakeJudge>,
    norms: StepNorms,
    workflow: ResolvedWorkflow,
) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(workflow);
    fittings.judge = judge;
    fittings.norms = norms;
    Fleet::assembled(fittings)
}

/// The fixture the three stage-four cases share: a Judge that finds thrashing,
/// a turn norm five turns under what the Drone reports, and a grace short
/// enough to expire. What differs between them is the Drone.
fn a_chain_that_will_reach_the_trigger(home: &TempDir, harness: FakeHarness) -> Fixture {
    a_watched_fleet(
        home,
        harness,
        Arc::new(FakeJudge::saying(THRASHING)),
        StepNorms::of(5, Duration::from_secs(86_400), Duration::from_secs(2)),
        one_step(None),
    )
}

/// Approve the Job and hand back its id, with a worktree on disk.
async fn started(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.unwrap();
    job.id().clone()
}

/// Turn until the chain says something, or give up.
async fn next_stage(fleet: &Fixture, waiting_for: &str) -> Wandering {
    for _ in 0..400 {
        if let Some(wandering) = fleet.turn().await.expect("a turn").wandering {
            return wandering;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the chain never {waiting_for}");
}

/// Turn a fixed number of times and answer with everything the chain said.
async fn turns(fleet: &Fixture, how_many: usize) -> Vec<Wandering> {
    let mut said = Vec::new();
    for _ in 0..how_many {
        if let Some(wandering) = fleet.turn().await.expect("a turn").wandering {
            said.push(wandering);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    said
}

// -------------------------------------------------------------- stage one

/// **Cold by default.** A step inside its norms reaches no model at all, which
/// is what keeps the whole chain free on the Jobs it does not apply to.
#[tokio::test]
async fn a_step_inside_its_norms_is_never_looked_at() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(THRASHING));
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(3),
        Arc::clone(&judge),
        on_calls(500),
        one_step(None),
    );
    started(&fleet, &home).await;

    assert!(turns(&fleet, 12).await.is_empty());
    assert!(
        judge.asked().is_empty(),
        "a slow step is not a thrashing step, and nothing should have been spent"
    );
}

/// **The failure the whole chain exists to avoid.** A turn count over the norm
/// is a reason to look, never a reason to escalate — read the other way,
/// `thrashing` would mean "took a while" and would fire on every slow step.
#[tokio::test]
async fn a_mechanical_trigger_on_its_own_escalates_nothing() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("state: converging"));
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(90),
        Arc::clone(&judge),
        on_calls(5),
        one_step(None),
    );
    let job = started(&fleet, &home).await;

    let wandering = next_stage(&fleet, "looked at the step").await;
    assert!(matches!(
        wandering.stage,
        Stage::StillConverging {
            tripped: Tripwire::ToolCalls { taken: 90 },
            ..
        }
    ));
    assert_eq!(judge.asked().len(), 1, "one call, not one per turn");
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);
}

/// The second detector, on a step whose turn count says nothing. The clock is
/// the injected one, so what the ceiling is measured against is a value rather
/// than a machine.
#[tokio::test]
async fn a_step_running_past_its_wall_clock_is_looked_at() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("state: converging"));
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(1),
        Arc::clone(&judge),
        StepNorms::of(500, Duration::from_secs(3), Duration::from_secs(86_400)),
        one_step(None),
    );
    let job = started(&fleet, &home).await;

    let wandering = next_stage(&fleet, "looked at the step").await;
    assert!(
        matches!(
            wandering.stage,
            Stage::StillConverging {
                tripped: Tripwire::WallClock { .. },
                ..
            }
        ),
        "{:?}",
        wandering.stage
    );
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);
}

/// Work outside the declared plan is the third detector, and it is a signal
/// rather than a failure — `judge.md`: legitimate investigation sometimes moves
/// the work, so the look is what decides.
#[tokio::test]
async fn work_outside_the_plan_asks_the_judge_and_fails_nothing() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("state: justified_drift"));
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(1),
        Arc::clone(&judge),
        on_calls(500),
        one_step(Some(Scoped {
            diff_check: true,
            at_step_start: true,
            exclude: &[],
            references: &[],
        })),
    );
    let job = started(&fleet, &home).await;
    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["docs".to_string()],
        })
        .await
        .expect("the step declares a scope");

    let wandering = next_stage(&fleet, "looked at the drift").await;
    assert!(matches!(
        wandering.stage,
        Stage::StillConverging {
            tripped: Tripwire::OffPlan { .. },
            found: Convergence::JustifiedDrift,
        }
    ));
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);
}

// -------------------------------------------------------------- stage two

/// A look that finds the work converging ends the chain, and does not run
/// again — drift stays tripped for the rest of the step, and a look per turn
/// would be the schedule `judge.md` rules out.
#[tokio::test]
async fn a_look_that_finds_convergence_stops_the_chain() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("state: converging"));
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(40),
        Arc::clone(&judge),
        on_calls(5),
        one_step(None),
    );
    let job = started(&fleet, &home).await;

    next_stage(&fleet, "looked at the step").await;
    assert!(turns(&fleet, 20).await.is_empty(), "the chain said more");
    assert_eq!(judge.asked().len(), 1);
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);
}

/// **The Judge reads the work product.** A brief carrying a turn count would be
/// judging the Drone rather than what it made, and the prompt contract says the
/// count is not the finding.
#[tokio::test]
async fn the_look_reads_what_was_produced_and_never_the_drones_turns() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("state: converging"));
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(90),
        Arc::clone(&judge),
        on_calls(5),
        one_step(None),
    );
    started(&fleet, &home).await;
    next_stage(&fleet, "looked at the step").await;

    let asked = judge.asked();
    let question = asked.first().expect("one question");
    assert!(question.contains("panic!()"), "the diff is the subject");
    assert!(!question.contains("90"), "the turn count reached the brief");
    assert!(!question.contains("turn"), "{question}");
}

/// A call that could not be made is not a finding. Escalating on one would make
/// `thrashing` fire when the Judge is down.
#[tokio::test]
async fn a_look_that_could_not_be_made_escalates_nothing() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::that_fails("a Judge with no network"));
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(90),
        Arc::clone(&judge),
        on_calls(5),
        one_step(None),
    );
    let job = started(&fleet, &home).await;

    let wandering = next_stage(&fleet, "gave up on the look").await;
    assert!(matches!(wandering.stage, Stage::CouldNotLook { .. }));
    assert!(turns(&fleet, 10).await.is_empty(), "it asked again");
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);
}

// ------------------------------------------------------ stages three and four

/// **The distinction the trigger turns on.** A Drone that thrashes and then
/// reports when it is told to has not thrashed by the registry's definition,
/// and nothing here may escalate it.
///
/// The grace is the same two seconds the escalating case uses, so the only
/// thing standing between this Job and `thrashing` is that the Drone answered.
#[tokio::test]
async fn a_drone_that_reports_when_it_is_interrupted_has_not_thrashed() {
    let home = TempDir::new();
    let fleet = a_chain_that_will_reach_the_trigger(&home, a_drone_that_answers(90));
    let job = started(&fleet, &home).await;

    let wandering = next_stage(&fleet, "told the Drone to report").await;
    assert!(matches!(wandering.stage, Stage::AskedToReport { .. }));
    // The grace is what this pause is. A directive is consumed when the
    // Drone's current turn ends, so a report that has not arrived within a
    // scheduler tick is not a Drone that refused to answer.
    tokio::time::sleep(Duration::from_millis(250)).await;
    let said = turns(&fleet, 40).await;
    assert!(
        said.is_empty(),
        "the Drone came to rest when told to, which is not thrashing: {said:?}"
    );
    let job = fleet.load(&job).await.unwrap();
    assert_eq!(job.status(), JobStatus::Running);
    assert!(job
        .steps()
        .iter()
        .all(|step| step.state() != StepState::Stopped));
}

/// The one case that reaches the trigger: told to stop, and it did not.
#[tokio::test]
async fn a_drone_that_will_not_report_is_escalated_with_its_step_stopped() {
    let home = TempDir::new();
    let fleet = a_chain_that_will_reach_the_trigger(&home, a_drone_that_will_not_answer(90));
    let job_id = started(&fleet, &home).await;

    let asked = next_stage(&fleet, "told the Drone to report").await;
    assert!(matches!(asked.stage, Stage::AskedToReport { .. }));
    // Read the pid while the Drone is still held, so the kill below is measured
    // against a process that was demonstrably alive rather than assumed to be.
    let pid = fleet
        .slot()
        .lock()
        .await
        .as_ref()
        .expect("a Drone is working")
        .session()
        .pid();
    assert!(alive(pid), "the Drone is alive when it is told to report");
    let escalated = next_stage(&fleet, "escalated").await;
    assert!(matches!(escalated.stage, Stage::Escalated { .. }));

    let job = fleet.load(&job_id).await.unwrap();
    assert_eq!(job.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(&job_id).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Thrashing))
    );
    let step = job.step(&core_model::StepId::new("implement")).unwrap();
    assert_eq!(step.state(), StepState::Stopped);
    assert_eq!(
        step.last_verdict(),
        Some(StepVerdict::Failed(
            stops_the_step().expect("thrashing is step-level")
        )),
        "the step names why it stopped, which is what a restart later resumes"
    );

    // **The cap, and the one place Fleet stops a Drone itself.** Everything
    // else escalated is held so a person can redirect it, and holding costs
    // nothing because a Drone waiting on a person is idle. This one was told to
    // report, did neither, and is spending money on a step it is not
    // converging on — holding it would be paying to watch.
    for _ in 0..50 {
        if !alive(pid) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        !alive(pid),
        "a Drone confirmed to be thrashing is stopped, not held"
    );
    // And the worktree is untouched — a cap ends the spending, never the work.
    assert!(
        home.path().join(".armada/worktrees").exists(),
        "the worktree survives the cap"
    );
}

/// **Nothing here kills a Drone.** `helm.md`: a thrashing Drone is held with
/// its worktree intact, which is what leaves a redirect available rather than
/// only a restart.
#[tokio::test]
async fn the_escalation_holds_the_drone_rather_than_ending_it() {
    let home = TempDir::new();
    let fleet = a_chain_that_will_reach_the_trigger(&home, a_drone_that_will_not_answer(90));
    let job_id = started(&fleet, &home).await;
    next_stage(&fleet, "told the Drone to report").await;
    next_stage(&fleet, "escalated").await;

    assert_eq!(fleet.working_on().await, Some(job_id.clone()));
    // Once, and never again: the step is stopped and the chain has nowhere
    // further to go.
    assert!(turns(&fleet, 10).await.is_empty());
}

/// The finding is on the record. Without it the escalation says `thrashing`
/// and not what about it — the same defect an uncited refusal would be.
#[tokio::test]
async fn the_escalation_records_the_observable_that_did_not_move() {
    let home = TempDir::new();
    let fleet = a_chain_that_will_reach_the_trigger(&home, a_drone_that_will_not_answer(90));
    let job_id = started(&fleet, &home).await;
    next_stage(&fleet, "told the Drone to report").await;
    next_stage(&fleet, "escalated").await;

    let recorded = fleet
        .store()
        .lock()
        .await
        .step_checks(&job_id)
        .expect("the step's rows read back");
    let checks: Vec<(String, Option<String>)> = recorded
        .iter()
        .flat_map(|(_, rows)| rows.iter())
        .map(|check| (check.name.clone(), check.produced.clone()))
        .collect();
    assert_eq!(
        checks,
        vec![(
            MID_STEP_CONVERGENCE.to_string(),
            Some("the same panic on the same input".to_string())
        )]
    );
}

// --------------------------------------------------------------- the turn

/// What reaches the Drone is `expected` and `produced` and nothing else.
/// `consequence` is written for the person deciding what to do about it, which
/// is the same field selection the refusal reprompt makes.
#[test]
fn the_directive_carries_two_of_the_three_fields() {
    let told = ReportNow::about(&NotConverging::cited(
        "the parser accepts a trailing comma",
        "the same panic on the same input",
        "every caller still crashes on the same file",
    ));
    let text = told.text().to_string();
    assert!(
        text.starts_with("Stop and report your current state now."),
        "{text}"
    );
    assert!(
        text.contains("the parser accepts a trailing comma"),
        "{text}"
    );
    assert!(text.contains("the same panic on the same input"), "{text}");
    assert!(
        !text.contains("every caller still crashes"),
        "consequence is the person's field: {text}"
    );
    assert!(!text.contains("attempt"), "no counter, ever: {text}");
}

/// The registry types the row step-level, which is what lets it name which step
/// stopped. A change there reads as this failing rather than as a panic in the
/// daemon.
#[test]
fn thrashing_is_a_trigger_a_step_can_be_stopped_with() {
    assert_eq!(
        stops_the_step().map(|narrowed| narrowed.trigger()),
        Some(EscalationTrigger::Thrashing)
    );
}
