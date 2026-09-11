//! A Drone reaching for a command it was not granted, and a person answering.
//!
//! What these prove: the Job's setting makes the first answer; Refuse and hold
//! refuses and the fold that classifies the ending sees the refusal; Ask me
//! holds the call until a person answers and the answer is what the call
//! returns; an allow is remembered; and the Board row says the Job waits on a
//! person. Not proved here: the hold running out, which is four real minutes.
//! `Fleet::permission` is called with the Job id the peer lookup would have
//! produced, as `crate::tests::questioning` calls `ask_question`.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use api::{PermissionAnswer, Queries};
use config::ResolvedWorkflow;
use core_model::{JobId, JobStatus, WhenBlocked};
use ipc::mcp::{Incoming, PermissionAsked};
use ipc::{CommandAnswer, CommandInFlight};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::Fleet;
use crate::permitting::NotPermitted;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

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

/// What the harness sends the permission tool, read the way the transport
/// reads it — so the arguments are the ones the tool would have parsed.
fn asked(tool: &str, command: &str, call: &str) -> PermissionAsked {
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"permission","arguments":{{"tool_name":"{tool}","input":{{"command":"{command}"}},"tool_use_id":"{call}"}}}}}}"#
    );
    match ipc::mcp::read(body.as_bytes()) {
        Incoming::Permission { asked, .. } => asked,
        other => panic!("not a permission call: {other:?}"),
    }
}

/// A Drone whose first line is the call it reached for, and which reads
/// whatever is written to it afterwards.
fn a_drone_that_reached_for(call: &str) -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            "echo BUSY; while IFS= read -r line; do echo ANSWERED; done",
        ],
    )
    .reading(
        "BUSY",
        vec![DroneEvent::Called {
            tool: String::from("Bash"),
            call: String::from(call),
            detail: CallDetail::of("npm publish"),
        }],
    )
    .reading(
        "ANSWERED",
        vec![DroneEvent::Called {
            tool: String::from("Read"),
            call: String::from("after"),
            detail: CallDetail::of("a file"),
        }],
    )
}

fn a_fleet_with(home: &TempDir, harness: FakeHarness) -> Fixture {
    let mut fittings = fitted_with(home, FakeWorkProduct::changed(&["src/parse.rs"]), harness);
    fittings.workflows = one(one_step());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a command"));
    Fleet::assembled(fittings)
}

async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("publish the package"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    settled(fleet).await;
    job.id().clone()
}

/// Wait until nothing more is arriving.
async fn settled(fleet: &Fixture) {
    let mut steady = 0;
    let mut last = usize::MAX;
    for _ in 0..400 {
        let now = heard(fleet).await.len();
        steady = if now == last { steady + 1 } else { 0 };
        if steady == 10 && now > 0 {
            return;
        }
        last = now;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never stopped talking");
}

async fn heard(fleet: &Fixture) -> Vec<DroneEvent> {
    let held = fleet.the_only_slot().await;
    let slot = held.lock().await;
    slot.as_ref().map(|at| at.heard()).unwrap_or_default()
}

fn refused(heard: &[DroneEvent], call: &str) -> bool {
    heard
        .iter()
        .any(|event| matches!(event, DroneEvent::Refused { call: refused, .. } if refused == call))
}

/// Wait until a person is being asked, and hand back what they would see.
async fn until_waiting(fleet: &Fixture, job: &JobId) -> CommandInFlight {
    for _ in 0..400 {
        if let Some(waiting) = fleet.command_awaited(job).await {
            return waiting;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("nobody was ever asked");
}

/// **Where every Job starts.** The call is refused at once, with the command
/// named, and the refusal is in what the ending folds — the stream carries no
/// refusal line for it, so this is the only way `blocked_by_policy` can fire.
#[tokio::test]
async fn refuse_and_hold_refuses_and_the_fold_sees_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;

    let answer = fleet
        .permission(&job, &asked("Bash", "npm publish", "c1"))
        .await;

    let PermissionAnswer::Deny(words) = answer else {
        panic!("a new Job refuses and holds: {answer:?}");
    };
    assert!(words.contains("not granted `npm publish`"), "{words}");
    assert!(
        refused(&heard(&fleet).await, "c1"),
        "the fold sees Fleet's refusal"
    );
    assert!(
        fleet.command_awaited(&job).await.is_none(),
        "and nobody is asked"
    );
}

/// **The requirement, as a case.** Ask me holds the call, a person allows it,
/// and the allow is what the call returns — the Drone carries on in the same
/// session. The allow is remembered, so the next reach for it asks nobody.
#[tokio::test]
async fn ask_me_holds_the_call_until_a_person_allows_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", "npm publish", "c1");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        let waiting = until_waiting(&fleet, &job).await;
        assert_eq!(waiting.detail, "npm publish");
        assert_eq!(
            waiting.offers.len(),
            3,
            "allow for this job, always, reject"
        );
        fleet
            .answer_command(&job, "c1", CommandAnswer::AllowForJob)
            .await
    });

    answered.expect("the call is waiting, and the answer is one it offers");
    assert_eq!(answer, PermissionAnswer::Allow);
    assert!(fleet.command_awaited(&job).await.is_none(), "answered once");
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "nothing moved: the Drone never left its call"
    );
    let allowed = fleet.store().lock().await.allowed_commands(&job).unwrap();
    assert_eq!(allowed.len(), 1);
    assert_eq!(allowed[0].run, "npm publish");
    assert_eq!(
        fleet
            .permission(&job, &asked("Bash", "npm publish --tag next", "c2"))
            .await,
        PermissionAnswer::Allow,
        "the allow covers the same command with more arguments, and nobody is asked"
    );
}

/// **Allow all asks nobody.** A command and a tool outside the toolbelt are
/// both allowed at once, nothing is written down as allowed, and no refusal
/// reaches the fold.
#[tokio::test]
async fn allow_all_allows_the_call_and_asks_nobody() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AllowAll)
        .await
        .unwrap();

    assert_eq!(
        fleet
            .permission(&job, &asked("Bash", "npm publish", "c1"))
            .await,
        PermissionAnswer::Allow
    );
    assert_eq!(
        fleet
            .permission(&job, &asked("WebFetch", "https://example.com", "c2"))
            .await,
        PermissionAnswer::Allow
    );
    assert!(fleet.command_awaited(&job).await.is_none(), "nobody asked");
    assert!(!refused(&heard(&fleet).await, "c1"));
    assert!(fleet
        .store()
        .lock()
        .await
        .allowed_commands(&job)
        .unwrap()
        .is_empty());
}

/// A person saying no is the call's answer, and it is a refusal the fold sees.
#[tokio::test]
async fn a_rejected_command_is_refused_and_the_fold_sees_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", "npm publish", "c1");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(&job, "c1", CommandAnswer::Reject)
            .await
    });

    answered.unwrap();
    let PermissionAnswer::Deny(words) = answer else {
        panic!("a rejection is a deny: {answer:?}");
    };
    assert!(words.contains("said no to `npm publish`"), "{words}");
    assert!(refused(&heard(&fleet).await, "c1"));
    assert!(fleet
        .store()
        .lock()
        .await
        .allowed_commands(&job)
        .unwrap()
        .is_empty());
}

/// **The Board row says so, which is what puts the Job under Needs you**, and
/// stops saying it once a person has answered.
#[tokio::test]
async fn the_board_says_a_job_waits_on_a_command() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", "npm publish", "c1");

    let (_, waiting) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        let board = Queries::list_jobs(&fleet).await.expect("a board");
        let waiting = board
            .jobs
            .iter()
            .any(|row| row.id.as_str() == job.as_str() && row.asking);
        fleet
            .answer_command(&job, "c1", CommandAnswer::AllowForJob)
            .await
            .unwrap();
        waiting
    });

    assert!(waiting, "a Job waiting on a command waits on a person");
    let after = Queries::list_jobs(&fleet).await.expect("a board");
    assert!(
        after.jobs.iter().all(|row| !row.asking),
        "answered, so no longer"
    );
}

/// **An answer naming another call joins to nothing**, and the question stands
/// for the answer that does.
#[tokio::test]
async fn an_answer_naming_another_call_is_refused_and_the_question_stands() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", "npm publish", "c1");

    let (answer, wrong) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        let wrong = fleet
            .answer_command(&job, "c9", CommandAnswer::AllowForJob)
            .await;
        assert!(fleet.command_awaited(&job).await.is_some(), "still waiting");
        fleet
            .answer_command(&job, "c1", CommandAnswer::AllowForJob)
            .await
            .unwrap();
        wrong
    });

    assert!(matches!(wrong, Err(NotPermitted::NothingToAnswer { .. })));
    assert_eq!(answer, PermissionAnswer::Allow);
}

/// **Only a command can be allowed from here**, so a tool is refused even on a
/// Job that asks, and nobody is asked about it.
#[tokio::test]
async fn a_tool_that_is_not_a_command_is_refused_even_when_asking() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();

    let answer = fleet
        .permission(&job, &asked("WebFetch", "https://example.com", "c1"))
        .await;

    let PermissionAnswer::Deny(words) = answer else {
        panic!("a tool is never allowed from here: {answer:?}");
    };
    assert!(words.contains("not granted `WebFetch`"), "{words}");
    assert!(fleet.command_awaited(&job).await.is_none());
}

/// **One at a time.** A second call while a person is being asked about the
/// first is told to wait for that answer, and is not itself a refusal.
#[tokio::test]
async fn a_second_command_waits_behind_the_first() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", "npm publish", "c1");

    let (_, second) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        let second = fleet
            .permission(&job, &asked("Bash", "cargo publish", "c2"))
            .await;
        fleet
            .answer_command(&job, "c1", CommandAnswer::AllowForJob)
            .await
            .unwrap();
        second
    });

    let PermissionAnswer::Deny(words) = second else {
        panic!("the second waits behind the first: {second:?}");
    };
    assert!(
        words.contains("whether you may run `npm publish`"),
        "{words}"
    );
    assert!(
        !refused(&heard(&fleet).await, "c2"),
        "nothing about it is final"
    );
}
