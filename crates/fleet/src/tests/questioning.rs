//! A Drone asking a person a question, and the answer coming back.
//!
//! # What these prove, and what they deliberately do not
//!
//! The ask, the four refusals, the answer reaching the pipe, and — the two that
//! matter most — that neither vigil counts a waiting Drone as a stopped one. **A
//! Drone blocked on an unanswered question must not escalate as `stalled` or
//! `thrashing`**: the wait is a person's and has no budget, so a Job escalated
//! for having a question outstanding is a Job that stopped for doing the right
//! thing.
//!
//! Not proved here: that the Drone *acts* on the answer — nothing in this
//! workspace reads what a Drone said. `Fleet::ask_question` is called with the
//! Job id the peer lookup would have produced, so what is skipped is the socket
//! and nothing about the binding.
//!
//! # Over 500 lines, and left as one file
//!
//! `crate::tests::headroom`'s argument. The ask, the refusals, the answer and
//! the two vigils are one act proved at four depths, and a split would put "a
//! label never offered is refused" in a different file from "the Drone waiting
//! for that answer is not read as stalled" — the same Drone, one turn apart.
//! Sixty of the lines are the pushed clock and the production thresholds, which
//! are the fixture; `crate::tests::silence`'s clock is private to it.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use api::Queries;
use config::ResolvedWorkflow;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, StepId, StepState, Target, Timestamp,
};
use ipc::mcp::{AskQuestion, AskedOption};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::daemon::Fleet;
use crate::questioning::{NotAnswered, NotAsked};
use crate::silence::Liveness;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

/// How long a Drone may say nothing before the vigil speaks. The production
/// number, so the two cases below are measured against what ships.
const QUIET_AFTER: Duration = Duration::from_secs(120);

/// A clock that ticks a second per reading, and jumps when a test says so.
/// `crate::tests::silence`'s, carried rather than shared because that module's
/// is private to it.
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

    /// Move the clock on. **Never backwards.**
    fn on(&self, seconds: u64) {
        self.pushed.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T{:02}:{:02}:{:02}.000Z",
            (at / 3_600) % 24,
            (at / 60) % 60,
            at % 60
        ))
    }
}

/// A well-formed question: two answers, each saying what it commits to.
fn a_question() -> AskQuestion {
    AskQuestion {
        question: "Should the store schema change be its own Job?".to_string(),
        options: vec![
            AskedOption {
                label: "Its own Job".to_string(),
                consequence: "dispatch a migration Job first and make the rest depend on it"
                    .to_string(),
            },
            AskedOption {
                label: "Fold it in".to_string(),
                consequence: "the first Job that needs the column adds it".to_string(),
            },
        ],
    }
}

fn one_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn called() -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }]
}

/// A Drone that says one thing and then reads whatever is written to it. The
/// pipe stays open, which is what an answer needs.
fn a_drone_that_listens() -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            "echo BUSY; while IFS= read -r line; do echo ANSWERED; done",
        ],
    )
    .reading("BUSY", called())
    .reading("ANSWERED", called())
}

/// A Drone that takes the write and never speaks again — the shape a Drone
/// waiting on an answer has, from the outside.
fn a_drone_that_waits() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called())
}

/// A Fleet on one step. **The Judge fails every call**: nothing here may ask a
/// model anything, and the thrashing test below turns on that — a look that ran
/// would fail loudly rather than silently pass.
fn a_fleet_with(home: &TempDir, harness: FakeHarness) -> Fixture {
    watching(home, harness, Arc::new(Held::started()))
}

/// The same Fleet, with a clock a test can push and **both vigils armed**.
///
/// The default fittings carry `NEVER_QUIET` and `UNTRIPPABLE`, which is right
/// for every fixture whose Drone is legitimately silent for a short run — and
/// wrong here, because it would make the two cases below pass whether or not
/// the guards exist. So the thresholds are the production ones: 120 seconds of
/// silence and two pokes, and the wall clock and call count `armada::serve`
/// ships. **Both were verified to fire on this fixture with the guards
/// removed**, which is the only thing that makes them evidence.
fn watching(home: &TempDir, harness: FakeHarness, clock: Arc<Held>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.clock = clock;
    fittings.liveness = Liveness::of(QUIET_AFTER, 2);
    fittings.norms = StepNorms::of(60, Duration::from_secs(1_500), Duration::from_secs(120));
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a question"));
    Fleet::assembled(fittings)
}

async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    settled(fleet).await;
    job.id().clone()
}

/// Wait until nothing more is arriving.
async fn settled(fleet: &Fixture) {
    let mut steady = 0;
    let mut last = usize::MAX;
    for _ in 0..400 {
        let now = heard(fleet).await;
        steady = if now == last { steady + 1 } else { 0 };
        if steady == 10 && now > 0 {
            return;
        }
        last = now;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never stopped talking");
}

async fn heard(fleet: &Fixture) -> usize {
    let held = fleet.the_only_slot().await;
    let slot = held.lock().await;
    slot.as_ref().map(|at| at.heard().len()).unwrap_or_default()
}

/// Whether the slot is holding an unanswered question.
async fn is_waiting(fleet: &Fixture, job: &JobId) -> bool {
    let held = fleet.slot_of(job).await.expect("a slot for this Job");
    let slot = held.lock().await;
    slot.as_ref()
        .map(|at| at.asked().is_some())
        .unwrap_or_default()
}

/// **The requirement, as a case.** A working Drone asks, the question is held,
/// and nothing about the Job or the step moved.
#[tokio::test]
async fn a_question_is_held_and_nothing_moves() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_waits());
    let job = started(&fleet, &home).await;

    let asked = fleet
        .ask_question(&job, a_question())
        .await
        .expect("a working Drone may ask");

    assert_eq!(
        asked.question(),
        "Should the store schema change be its own Job?"
    );
    let record = fleet.load(&job).await.unwrap();
    assert_eq!(
        record.status(),
        JobStatus::Running,
        "a Job with a question outstanding is still running — nothing mints a status for waiting"
    );
    assert_eq!(
        record.step(&StepId::new(IMPLEMENT)).unwrap().state(),
        StepState::Running,
        "and the step is still running, which is the whole `job.judging` argument applied here"
    );
    assert!(is_waiting(&fleet, &job).await);
}

/// The wire says so too: `get_job` carries the question, its options and the
/// instant, and a client can draw the controls from it without asking anything
/// else.
#[tokio::test]
async fn the_question_reaches_the_wire_whole() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_waits());
    let job = started(&fleet, &home).await;
    fleet.ask_question(&job, a_question()).await.unwrap();

    let served = fleet
        .question_awaited(&job)
        .await
        .expect("a question outstanding is served");
    assert_eq!(served.options.len(), 2);
    assert_eq!(served.options[0].label, "Its own Job");
    assert!(
        !served.options[0].consequence.is_empty(),
        "a label with no consequence is a button whose effect has to be guessed"
    );
    assert_eq!(served.step_id.as_str(), IMPLEMENT);
}

/// **The Board row says so, which is what puts the Job under Needs you.**
///
/// `who_is_acting` on `running` is `Drone`, so without this flag a Job waiting
/// on a person sits under Running and a question on a Job nobody has open is
/// invisible. `packages/screens/src/board.ts` reads it, and
/// `docs/concepts/job-board.md` is the rule; this is the half of that pair Rust
/// can prove, because Bridge has no test runner.
#[tokio::test]
async fn the_board_row_says_the_drone_is_waiting_and_stops_saying_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_listens());
    let job = started(&fleet, &home).await;

    let before = Queries::list_jobs(&fleet).await.expect("a board");
    assert!(
        before.jobs.iter().all(|row| !row.asking),
        "a Drone that has asked nothing is not waiting on anybody"
    );

    let asked = fleet.ask_question(&job, a_question()).await.unwrap();
    let waiting = Queries::list_jobs(&fleet).await.expect("a board");
    assert!(
        waiting
            .jobs
            .iter()
            .any(|row| row.id.as_str() == job.as_str() && row.asking),
        "the row a Board draws has to carry it — the question itself does not"
    );

    fleet
        .answer_question(&job, asked.id().as_str(), "Fold it in")
        .await
        .unwrap();
    let after = Queries::list_jobs(&fleet).await.expect("a board");
    assert!(
        after.jobs.iter().all(|row| !row.asking),
        "answered once, so the row stops claiming it — there is no badge to go stale"
    );
}

/// **One at a time.** A Drone that could stack questions would be holding a
/// conversation, and a queue is a thing a person answers out of order.
#[tokio::test]
async fn a_second_question_is_refused_while_one_is_outstanding() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_waits());
    let job = started(&fleet, &home).await;
    fleet.ask_question(&job, a_question()).await.unwrap();

    let refused = fleet
        .ask_question(&job, a_question())
        .await
        .expect_err("the first is still outstanding");
    assert!(matches!(refused, NotAsked::AlreadyAsking { .. }));
    assert!(
        refused.to_string().contains("Should the store schema"),
        "and it quotes the one being waited on: {refused}"
    );
}

/// A question on a Job a person has already stopped is a second thing to answer
/// about a Job they have taken over.
#[tokio::test]
async fn a_job_that_is_not_running_takes_no_question() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_waits());
    let job = started(&fleet, &home).await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::Stalled),
            Actor::Fleet,
        )
        .await
        .unwrap();

    let refused = fleet
        .ask_question(&job, a_question())
        .await
        .expect_err("an escalated Job is a person's");
    assert!(matches!(refused, NotAsked::NotRunning { .. }));
    assert!(!is_waiting(&fleet, &job).await, "and nothing was held");
}

/// The answer goes down the pipe and the question stops being outstanding.
#[tokio::test]
async fn an_answer_reaches_the_drone_and_closes_the_question() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_listens());
    let job = started(&fleet, &home).await;
    let asked = fleet.ask_question(&job, a_question()).await.unwrap();

    let told = fleet
        .answer_question(&job, asked.id().as_str(), "Fold it in")
        .await
        .expect("a label the Drone offered");

    assert_eq!(told.chose, "Fold it in");
    assert!(
        !is_waiting(&fleet, &job).await,
        "answered once — the question is gone, not marked"
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "and the Job never left running, so there is no move to wait for"
    );
}

/// **A label the Drone never offered is refused rather than passed through.**
/// There is no free-text answer, and this is where that is enforced: prose
/// reaches a Drone through `redirect_drone` and through nothing else.
#[tokio::test]
async fn a_label_that_was_not_offered_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_listens());
    let job = started(&fleet, &home).await;
    let asked = fleet.ask_question(&job, a_question()).await.unwrap();

    let refused = fleet
        .answer_question(&job, asked.id().as_str(), "do whatever you think best")
        .await
        .expect_err("that is not one of the answers");
    assert!(matches!(refused, NotAnswered::NotOffered { .. }));
    assert!(
        refused.to_string().contains("Its own Job"),
        "and the refusal names what was offered: {refused}"
    );
    assert!(is_waiting(&fleet, &job).await, "the question still stands");
}

/// **A window left open across an answer names a question that is gone.**
/// Refused rather than applied to whatever is outstanding now: a label that
/// matched the newer question by coincidence would dispatch work nobody chose.
#[tokio::test]
async fn an_answer_to_a_question_that_is_not_the_one_outstanding_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_listens());
    let job = started(&fleet, &home).await;
    let first = fleet.ask_question(&job, a_question()).await.unwrap();
    fleet
        .answer_question(&job, first.id().as_str(), "Fold it in")
        .await
        .unwrap();
    let second = fleet.ask_question(&job, a_question()).await.unwrap();
    assert_ne!(first.id().as_str(), second.id().as_str());

    let refused = fleet
        .answer_question(&job, first.id().as_str(), "Fold it in")
        .await
        .expect_err("that question has been answered");
    assert!(matches!(refused, NotAnswered::Superseded { .. }));
    assert!(
        is_waiting(&fleet, &job).await,
        "and the one that is outstanding is untouched"
    );
}

/// Answering a Job whose Drone asked nothing.
#[tokio::test]
async fn a_job_with_nothing_outstanding_takes_no_answer() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_listens());
    let job = started(&fleet, &home).await;

    let refused = fleet
        .answer_question(&job, "01WHATEVER", "Fold it in")
        .await
        .expect_err("nothing asked");
    assert!(matches!(refused, NotAnswered::NothingIsAsking { .. }));
}

/// **The one that matters most.** A Drone waiting on an answer says nothing for
/// as long as the person takes, and the vigil must not read that as silence:
/// three pokes and the Job escalates as `stalled` for having asked a question.
///
/// The clock is pushed past the real threshold once per poke the step has, plus
/// one — which is exactly what `crate::tests::silence` does to reach the
/// escalation. So what this proves is the guard and not a timing accident.
#[tokio::test]
async fn a_waiting_drone_is_never_poked_and_never_stalls() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(&home, a_drone_that_waits(), Arc::clone(&clock));
    let job = started(&fleet, &home).await;
    fleet.ask_question(&job, a_question()).await.unwrap();

    // Four thresholds: three pokes and the escalation that follows them.
    for _ in 0..4 {
        clock.on(QUIET_AFTER.as_secs() + 60);
        for _ in 0..20 {
            let turned = fleet.turn().await.expect("a turn");
            assert!(
                turned.each.iter().all(|worked| worked.quiet.is_none()),
                "a waiting Drone is not a quiet one"
            );
        }
    }

    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "waiting on a person is not `stalled`"
    );
    assert!(
        is_waiting(&fleet, &job).await,
        "and the question is still there to be answered"
    );
}

/// The other vigil, and the reason it needs its own guard: the wall clock runs
/// while a person thinks, so without this the tripwire fires the moment the
/// answer lands — buying a Judge call on a step that has just been unblocked.
///
/// **The Judge fails every call in this fixture**, so a look that ran would show
/// up as a stage rather than passing silently.
#[tokio::test]
async fn a_waiting_drone_is_never_read_as_thrashing() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(&home, a_drone_that_waits(), Arc::clone(&clock));
    let job = started(&fleet, &home).await;
    fleet.ask_question(&job, a_question()).await.unwrap();

    for _ in 0..4 {
        clock.on(QUIET_AFTER.as_secs() * 20);
        for _ in 0..20 {
            let turned = fleet.turn().await.expect("a turn");
            assert!(
                turned.each.iter().all(|worked| worked.wandering.is_none()),
                "the chain looked at a Drone that is not working at all"
            );
        }
    }

    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "and nothing escalated it as `thrashing`"
    );
}
