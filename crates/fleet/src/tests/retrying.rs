//! A failed mechanical Check going back to the Drone, and the budget that
//! bounds how often.
//!
//! # The case these are against
//!
//! A Job failed one Check on a real one-line regression and reached
//! `completed_failed` on its first attempt, holding a live Drone with the whole
//! context needed to fix it. Every test here asserts one half of the answer:
//! that the failure reaches the Drone, and that it stops reaching it.
//!
//! # Two turns per attempt, and why the tests count them
//!
//! `turn` rules on one submission. A hand-back leaves the Job `running` with the
//! step re-entered, so the next submission is the next attempt — which is what
//! makes a spent budget testable at all. A test that calls `turn` once and
//! asserts a failure would pass against a system with no retries in it.

use api::Queries;
use core_model::{EscalationTrigger, JobStatus, StepId, StepState, StepVerdict};
use testkit::{FakeWorkProduct, Gate, Sketch};

use crate::adrift::Adrift;
use crate::gate::Ruling;
use crate::resume::Redirection;
use crate::tests::admitted::{dispatched, started};
use crate::tests::daemon::{a_fleet_holding, a_proposal, diff_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// One step gated on a command of the test's choosing, with `budget`
/// hand-backs before the failure stands.
fn gated_on(run: &str, budget: u32) -> config::ResolvedWorkflow {
    testkit::retried(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::Check {
                name: "suite",
                run,
                expect_exit_code: 0,
                when: &[],
            }],
            judged_on: &[],
            scope: None,
            gaming: None,
        }],
        budget,
    )
}

/// A command that fails and says something a Drone could act on.
const UNHAPPY: &str =
    "/bin/sh -c 'echo assertion failed: redispatch_job is not in the router 1>&2; exit 101'";

/// **The whole issue, as one case.** A Check fails, the budget has room, and
/// the Drone is told rather than terminated.
#[tokio::test]
async fn a_failed_check_inside_the_budget_goes_back_to_the_drone() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 2),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let ruling_turn = fleet.turn().await.unwrap();

    let ruled = ruling_turn.ruled().expect("the gate ruled");
    assert!(
        matches!(ruled, Ruling::HandedBack { .. }),
        "the correct verdict with somewhere to go: {ruled:?}"
    );
    assert!(
        !ruled.ends_the_drone(),
        "the Drone that wrote it is what makes the retry cheap"
    );

    let after = fleet.load(job.id()).await.unwrap();
    assert_eq!(
        after.status(),
        JobStatus::Running,
        "nothing about the Job failed: the step is being worked again"
    );
    let step = after.step(&StepId::new("implement")).expect("the row");
    assert_eq!(
        step.state(),
        StepState::Running,
        "the step passed through `retrying` and re-entered `running`"
    );
    assert_eq!(
        step.last_verdict(),
        Some(StepVerdict::Failed(
            core_model::StepLevelTrigger::of(EscalationTrigger::GateFailure).unwrap()
        )),
        "activity and verdict are separate fields: it is being worked, and the \
         gate said no"
    );
}

/// **The other half.** The budget is spent, the hand-backs stop, and the Job
/// waits for a person rather than ending — `#208`. The assertion below used to
/// cite the open question that asked where a spent budget lands, and said in as
/// many words that it was written to be changed by the answer.
#[tokio::test]
async fn a_spent_budget_stops() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 1),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let ruling_turn = fleet.turn().await.unwrap();
    let first = ruling_turn.ruled().expect("a first ruling");
    assert!(
        matches!(first, Ruling::HandedBack { .. }),
        "one hand-back is what a budget of one buys: {first:?}"
    );

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let ruling_turn = fleet.turn().await.unwrap();
    let second = ruling_turn.ruled().expect("a second ruling");
    assert!(
        matches!(second, Ruling::Failed { .. }),
        "the second run has nothing left to spend: {second:?}"
    );
    assert!(second.tell().is_none(), "there is nobody left to tell");

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::AwaitingRepair,
        "a spent budget holds the Job for a person rather than ending it"
    );
}

/// A step that asks for no budget behaves exactly as every step did before one
/// existed. **Absent is none**, and this is the assertion that says so from the
/// Job's side rather than the parser's.
#[tokio::test]
async fn a_step_with_no_budget_fails_on_its_first_failed_check() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 0),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let ruling_turn = fleet.turn().await.unwrap();

    let ruled = ruling_turn.ruled().expect("the gate ruled");
    assert!(matches!(ruled, Ruling::Failed { .. }), "{ruled:?}");
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::AwaitingRepair
    );
}

/// **The slot goes back the moment the budget is spent**, and a redirect is
/// refused rather than answered.
///
/// `#208` landed the other way: the Drone was kept alive at `awaiting_repair`
/// so that telling it what it was missing would cost no respawn. What it also
/// kept was the working slot, for as long as a person took to read the failure
/// — which is the trade `awaiting_review` made and gave up on 30 Aug 2026,
/// after one Job held the only slot for four hours and fifty-six minutes while
/// somebody read it. **A repair somebody may take a day over is that wait under
/// another name**, so the session ends here and the answer is a restart.
///
/// The worktree survives, which is what makes the restart worth having: every
/// step that passed is still on the branch.
#[tokio::test]
async fn a_spent_budget_frees_the_slot_and_leaves_a_restart_as_the_answer() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 0),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::AwaitingRepair
    );
    assert!(
        fleet.working_on().await.is_empty(),
        "the next queued Job may have the slot now, not once somebody answers"
    );

    let said = Redirection::saying("the assertion is in tests/parse.rs").expect("something in it");
    assert!(
        matches!(
            fleet.redirect(job.id(), &said).await,
            Err(Adrift::NoDroneToRedirect { .. })
        ),
        "there is no session left to inject a turn into"
    );

    let restarted = fleet
        .restart_step(job.id(), None)
        .await
        .expect("a fresh Drone onto the worktree the last one left");
    assert_eq!(
        restarted.status(),
        JobStatus::Queued,
        "the act asks for a Drone and the turn is what starts one — `#456`"
    );
    assert_eq!(
        started(&fleet, job.id()).await.unwrap().status(),
        JobStatus::Running,
        "the cap has room, so the next turn spawns"
    );
}

/// **A gate failure stops the step it failed, and says why on the row.** #179.
///
/// The Job reached `completed_failed` with its step still reading `running` and
/// `last_verdict` null, so the only record that the step had failed was the
/// Check run — nothing on the step, which is the field every derived reading
/// keys on. #156 made `gate_undecided` stop its step; the failure path was not
/// changed with it, and this is the same fix on the fourth ruling.
///
/// The order is the one thing that matters: a step is frozen beneath every
/// status but two, so a stop attempted after the Job moved would be refused and
/// the verdict never written. That is unchanged by the Job now holding at
/// `awaiting_repair` rather than ending — `running -> awaiting_repair` carries
/// the `no_step_running` guard the old edge carried, so a caller that got the
/// order wrong is refused rather than silently leaving this behind. #208.
#[tokio::test]
async fn a_failed_check_stops_the_step_and_writes_its_verdict() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 0),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let ruling_turn = fleet.turn().await.unwrap();

    let ruled = ruling_turn.ruled().expect("the gate ruled");
    assert!(matches!(ruled, Ruling::Failed { .. }), "{ruled:?}");

    let ended = fleet.load(job.id()).await.unwrap();
    assert_eq!(ended.status(), JobStatus::AwaitingRepair);
    let step = ended
        .step(&StepId::new("implement"))
        .expect("the row is there");
    assert_eq!(
        step.state(),
        StepState::Stopped,
        "a step left running beneath a held Job is one nothing can find a way out of"
    );
    assert_eq!(
        step.last_verdict(),
        Some(StepVerdict::Failed(
            core_model::StepLevelTrigger::of(EscalationTrigger::GateFailure)
                .expect("a step-level trigger")
        )),
        "the same trigger a hand-back writes: the same tier failed, and what \
         differs is whether there was budget left to answer it"
    );
}

/// The same, after the budget is spent rather than absent. **Both paths into
/// `Ruling::Failed` leave the same record**, which is what #179 could not be
/// closed without: a step stopped on one route and left running on the other
/// would render as two different failures.
#[tokio::test]
async fn a_spent_budget_stops_the_step_the_same_way() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 1),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let ruling_turn = fleet.turn().await.unwrap();
    let second = ruling_turn.ruled().expect("a second ruling");
    assert!(matches!(second, Ruling::Failed { .. }), "{second:?}");

    let ended = fleet.load(job.id()).await.unwrap();
    assert_eq!(ended.status(), JobStatus::AwaitingRepair);
    let step = ended
        .step(&StepId::new("implement"))
        .expect("the row is there");
    assert_eq!(step.state(), StepState::Stopped);
    assert!(
        matches!(step.last_verdict(), Some(StepVerdict::Failed(_))),
        "the record says the step failed, in the field that says so"
    );
}

/// **The turn carries what the Check printed**, which is the difference between
/// a Drone that can fix the failure and one that has to reproduce it first.
#[tokio::test]
async fn the_drone_is_told_what_the_check_printed() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 2),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let ruling_turn = fleet.turn().await.unwrap();

    let ruled = ruling_turn.ruled().expect("the gate ruled");
    let told = ruled.tell().expect("a hand-back tells the Drone").text();

    assert!(
        told.contains("redispatch_job is not in the router"),
        "the output is the point: {told}"
    );
    assert!(told.contains("`suite`"), "and which check said it: {told}");
    assert!(
        told.contains("it exited 101"),
        "with what it produced against what was expected: {told}"
    );
    assert!(
        told.contains("do not weaken"),
        "a hand-back is the moment a weakened assertion becomes tempting: {told}"
    );
}

/// **No counter, ever.** A Drone one attempt from the end has the strongest
/// possible incentive to satisfy the bar rather than do the work, so nothing in
/// the turn says which attempt this is or how many are left.
#[tokio::test]
async fn the_turn_says_nothing_about_how_much_budget_is_left() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 1),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let ruling_turn = fleet.turn().await.unwrap();

    let ruled = ruling_turn.ruled().expect("the gate ruled");
    let told = ruled.tell().expect("a hand-back tells the Drone").text();
    for word in ["attempt", "retry", "retries", "last", "remaining"] {
        assert!(!told.contains(word), "`{word}` is in the turn: {told}");
    }
}

/// **A Check that could not be run is not handed back.** The command is not
/// there, so nothing the Drone writes changes the answer, and three attempts
/// would produce the same failure three times.
#[tokio::test]
async fn a_check_that_never_ran_is_not_handed_back_however_much_budget_there_is() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on("/nowhere/no-such-command --please", 5),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();

    let ruling_turn = fleet.turn().await.unwrap();

    let ruled = ruling_turn.ruled().expect("the gate ruled");
    assert!(
        matches!(ruled, Ruling::Failed { .. }),
        "a budget must not be spent on a command that is not installed: {ruled:?}"
    );
}

/// **Each run's Checks are kept, not overwritten.** `store::attempt` files
/// per-run records under the count of a step's entries into `running`, and this
/// is the assertion that a hand-back actually moves that count — without it the
/// second run's rows would replace the first's and the record would read as one
/// attempt.
#[tokio::test]
async fn both_runs_of_a_retried_step_are_on_the_record() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 1),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let every = fleet
        .store()
        .lock()
        .await
        .step_checks_every_attempt(job.id())
        .expect("the rows");
    assert_eq!(
        every.len(),
        2,
        "one group per run of the step, oldest first: {every:?}"
    );
    assert_eq!(every[0].attempt.number(), 1);
    assert_eq!(every[1].attempt.number(), 2);
    assert_ne!(
        every[0].record[0].output_path, every[1].record[0].output_path,
        "and each run's output is its own file, so the first run's row does not \
         point at the second run's log"
    );
}

/// **The rows a run tree draws.** `state` and `last_verdict` are both the
/// latest, so before this a step that passed on its second try and one that
/// passed on its first were the same message — and which attempt a person was
/// looking at was reachable from no field on `get_job`.
#[tokio::test]
async fn get_job_says_how_many_times_a_step_ran_and_what_each_run_came_to() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 1),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let detail = fleet.get_job(ipc::JobId::from(job.id())).await.unwrap();
    let implement = detail
        .steps
        .iter()
        .find(|step| step.step_id.as_str() == "implement")
        .expect("the one step");
    assert_eq!(
        implement.attempts.len(),
        2,
        "one entry per run, oldest first: {:?}",
        implement.attempts
    );
    assert_eq!(implement.attempts[0].attempt, 1);
    assert_eq!(
        implement.attempts[0].outcome.as_wire(),
        "retrying",
        "the first run was handed back"
    );
    assert!(
        implement.attempts[0].why.is_some(),
        "and it says which trigger handed it back: {:?}",
        implement.attempts[0]
    );
    assert_eq!(implement.attempts[1].attempt, 2);
    assert!(
        implement.attempts[1].ended_at.is_some(),
        "the second run is over too: {:?}",
        implement.attempts[1]
    );
}

/// A hand-back is written into the Job's own log as a step move, which is what
/// lets a person see a budget being spent rather than inferring it from a Job
/// that took longer than it should have.
#[tokio::test]
async fn the_log_says_the_step_was_handed_back() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on(UNHAPPY, 2),
        1,
    );

    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let moves = fleet
        .store()
        .lock()
        .await
        .events_for(job.id())
        .expect("the log");
    assert!(
        moves.iter().any(|event| matches!(
            event.moved(),
            store::Moved::Step {
                to: StepState::Retrying,
                ..
            }
        )),
        "the row that tells a machine hand-back from a person's restart: {moves:?}"
    );
}
