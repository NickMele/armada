//! A Drone reaching for a command it was not granted, and a person answering.
//!
//! What these prove: the Job's setting makes the first answer; Refuse and hold
//! refuses and the fold that classifies the ending sees the refusal; Ask me
//! holds the call until a person answers and the answer is what the call
//! returns; an allow is remembered; and the Board row says the Job waits on a
//! person. `Fleet::permission` is called with the Job id the peer lookup would
//! have produced, as `crate::tests::questioning` calls `ask_question`.
//!
//! **The hold running out is proved on a planted hold.** [`PermissionHold`] is
//! a fitting, so the last three cases hold a question for milliseconds and
//! outlive it — what ships is four real minutes, and no case waits it.
//!
//! **Past 500 lines and staying one file.** The hold cases are the answer cases
//! read at a later moment: splitting them off would put the fixture, the fake
//! Drone and the `after` event in two places.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent, WorktreeSpec};
use api::{PermissionAnswer, Queries};
use config::ResolvedWorkflow;
use core_model::{Actor, AllowedCommand, JobId, JobStatus, Reach, Timestamp, WhenBlocked};
use ipc::mcp::{Incoming, PermissionAsked};
use ipc::{CommandAnswer, CommandInFlight};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::{Fittings, Fleet};
use crate::permitting::{Answered, NotPermitted, PermissionHold, Refusing};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

pub(super) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

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
pub(super) fn asked(tool: &str, command: &str, call: &str) -> PermissionAsked {
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
pub(super) fn a_drone_that_reached_for(call: &str) -> FakeHarness {
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

pub(super) fn a_fleet_with(home: &TempDir, harness: FakeHarness) -> Fixture {
    Fleet::assembled(the_fittings(home, harness))
}

/// The same Fleet, with the model call scripted. **The judge is handed in and
/// kept by the caller**, so a case about a reading can ask what the model was
/// actually asked — and answer that it was not asked at all.
pub(super) fn a_fleet_judged_by(
    home: &TempDir,
    harness: FakeHarness,
    judge: Arc<FakeJudge>,
) -> Fixture {
    let mut fittings = the_fittings(home, harness);
    fittings.judge = judge;
    Fleet::assembled(fittings)
}

/// [`a_fleet_with`], holding a question for the time named rather than the
/// fixture's thirty seconds — what lets a case outlive a hold.
fn a_fleet_holding_for(home: &TempDir, harness: FakeHarness, hold: Duration) -> Fixture {
    let mut fittings = the_fittings(home, harness);
    fittings.permission_hold = PermissionHold::of(hold);
    Fleet::assembled(fittings)
}

pub(super) fn the_fittings(
    home: &TempDir,
    harness: FakeHarness,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fitted_with(home, FakeWorkProduct::changed(&["src/parse.rs"]), harness);
    fittings.workflows = one(one_step());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a command"));
    fittings
}

pub(super) async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
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

pub(super) async fn heard(fleet: &Fixture) -> Vec<DroneEvent> {
    let held = fleet.the_only_slot().await;
    let slot = held.lock().await;
    slot.as_ref().map(|at| at.heard()).unwrap_or_default()
}

fn refused(heard: &[DroneEvent], call: &str) -> bool {
    heard
        .iter()
        .any(|event| matches!(event, DroneEvent::Refused { call: refused, .. } if refused == call))
}

/// How many lines the Drone has read from Fleet: the fake answers every one
/// with `ANSWERED`, scripted to this call. **Nothing writes to a Drone's input
/// on the in-the-call path**, so a count that rises is a permission turn and
/// not an answer that went back down the held call. Counted rather than looked
/// for, because dispatch wrote the opening brief the same way.
async fn lines_read(fleet: &Fixture) -> usize {
    heard(fleet)
        .await
        .iter()
        .filter(|event| matches!(event, DroneEvent::Called { call, .. } if call == "after"))
        .count()
}

/// Wait until the Drone has read another line, or say it never did.
async fn until_told(fleet: &Fixture, before: usize) -> bool {
    for _ in 0..400 {
        if lines_read(fleet).await > before {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    false
}

/// Long enough for a turn that was sent to have been read, which `until_told`
/// takes a poll or two to see. What "and not also as a turn" is asserted after.
const GRACE: Duration = Duration::from_millis(150);

/// Short enough that a case can outlive it, and long enough that the question
/// is on the slot and published before it runs out.
const BRIEFLY: Duration = Duration::from_millis(50);

/// Wait until a person is being asked, and hand back what they would see.
pub(super) async fn until_waiting(fleet: &Fixture, job: &JobId) -> CommandInFlight {
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
    fleet
        .set_when_blocked(&job, WhenBlocked::RefuseAndHold)
        .await
        .unwrap();

    let answer = fleet
        .permission(&job, &asked("Bash", "npm publish", "c1"))
        .await;

    let PermissionAnswer::Deny(words) = answer else {
        panic!("a Job set to refuse and hold does exactly that: {answer:?}");
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

/// **The default, as a case.** A Job nobody has touched the setting on holds
/// the call for a person rather than refusing it outright — issue #686.
#[tokio::test]
async fn a_new_job_asks_first_by_default() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    let asking = asked("Bash", "npm publish", "c1");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(&job, "c1", Answered::of(CommandAnswer::Reject, None))
            .await
    });

    answered.expect("a person was asked, unprompted by any setting");
    assert_eq!(
        answer,
        PermissionAnswer::Deny(Refusing::Rejected { note: None }.to_the_drone("npm publish"))
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
            .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
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

/// **A command taken back is refused at the next question**, with nothing
/// respawned for it, and a second take-back finds nothing to take.
#[tokio::test]
async fn a_command_taken_back_is_no_longer_allowed() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::RefuseAndHold)
        .await
        .unwrap();
    fleet
        .store()
        .lock()
        .await
        .allow_command(
            &job,
            &AllowedCommand {
                run: "npm publish".to_string(),
                reach: Reach::Job,
                allowed_at: Timestamp::from_rfc3339("2026-09-11T12:00:00.000Z"),
                by: Actor::Human,
            },
        )
        .unwrap();
    assert_eq!(
        fleet
            .permission(&job, &asked("Bash", "npm publish", "c1"))
            .await,
        PermissionAnswer::Allow
    );
    let listed: Vec<String> = fleet
        .allowed_of(&job)
        .await
        .into_iter()
        .map(|allow| allow.run)
        .collect();
    assert_eq!(listed, vec!["npm publish"], "what Job detail lists");

    fleet
        .remove_allowed_command(&job, "npm publish")
        .await
        .expect("a person allowed it");
    assert!(fleet.allowed_of(&job).await.is_empty(), "and lists no more");

    let answer = fleet
        .permission(&job, &asked("Bash", "npm publish", "c2"))
        .await;
    let PermissionAnswer::Deny(words) = answer else {
        panic!("taken back, so refused and held: {answer:?}");
    };
    assert!(words.contains("not granted `npm publish`"), "{words}");
    assert!(matches!(
        fleet.remove_allowed_command(&job, "npm publish").await,
        Err(NotPermitted::NothingAllowed { .. })
    ));
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
            .answer_command(&job, "c1", Answered::of(CommandAnswer::Reject, None))
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
            .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
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
            .answer_command(&job, "c9", Answered::of(CommandAnswer::AllowForJob, None))
            .await;
        assert!(fleet.command_awaited(&job).await.is_some(), "still waiting");
        fleet
            .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
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
            .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
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

/// **The hold runs out, and the Drone is told to wait rather than refused.**
/// The question stays on the slot, so the person whose answer was late still
/// has one to answer.
#[tokio::test]
async fn a_hold_that_runs_out_tells_the_drone_to_wait() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_for(&home, a_drone_that_reached_for("c1"), BRIEFLY);
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let before = lines_read(&fleet).await;

    let answer = fleet
        .permission(&job, &asked("Bash", "npm publish", "c1"))
        .await;

    assert_eq!(
        answer,
        PermissionAnswer::Deny(Refusing::Asked.to_the_drone("npm publish")),
        "asked, and nobody answered in time"
    );
    let PermissionAnswer::Deny(words) = &answer else {
        unreachable!("the assertion above")
    };
    assert!(
        words.contains("has been asked whether you may run"),
        "{words}"
    );
    assert!(words.contains("Stop and wait"), "{words}");
    let still = fleet
        .command_awaited(&job)
        .await
        .expect("the question stands for a person who is late");
    assert_eq!(still.call, "c1");
    assert_eq!(still.detail, "npm publish");
    assert_eq!(
        lines_read(&fleet).await,
        before,
        "nothing was said to the Drone"
    );
    assert!(fleet
        .store()
        .lock()
        .await
        .allowed_commands(&job)
        .unwrap()
        .is_empty());
}

/// **The late answer reaches the Drone as a turn**, which is what the deny
/// above told it to wait for: the Drone reads it, the allow is written down,
/// the question is settled and nothing left the session.
#[tokio::test]
async fn a_later_answer_reaches_the_drone_as_a_turn() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_for(&home, a_drone_that_reached_for("c1"), BRIEFLY);
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let before = lines_read(&fleet).await;

    let answer = fleet
        .permission(&job, &asked("Bash", "npm publish", "c1"))
        .await;
    assert_eq!(
        answer,
        PermissionAnswer::Deny(Refusing::Asked.to_the_drone("npm publish"))
    );

    fleet
        .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
        .await
        .expect("the question is still on the slot, late as the answer is");

    assert!(
        until_told(&fleet, before).await,
        "the Drone reads the answer as a turn"
    );
    assert!(
        fleet.command_awaited(&job).await.is_none(),
        "and the question is settled"
    );
    let allowed = fleet.store().lock().await.allowed_commands(&job).unwrap();
    assert_eq!(allowed.len(), 1);
    assert_eq!(allowed[0].run, "npm publish");
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "the Drone never left its session"
    );
}

/// **Answered inside the hold, and not again as a turn.** The call carries the
/// answer, and past the end of a hold the Drone has still been told nothing —
/// the other half of "never both" is [`a_later_answer_reaches_the_drone_as_a_turn`],
/// where the call has already given up.
#[tokio::test]
async fn an_answer_inside_the_hold_goes_down_the_call_only() {
    let home = TempDir::new();
    let hold = Duration::from_millis(500);
    let fleet = a_fleet_holding_for(&home, a_drone_that_reached_for("c1"), hold);
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", "npm publish", "c1");
    let before = lines_read(&fleet).await;

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
            .await
    });

    answered.expect("the call is still held");
    assert_eq!(answer, PermissionAnswer::Allow);
    tokio::time::sleep(hold + GRACE).await;
    assert_eq!(
        lines_read(&fleet).await,
        before,
        "answered in the call, so no turn follows it"
    );
}

/// **An answer racing the end of the hold arrives once**: down the call or as a
/// turn, never both and never neither. Which side wins is the machine's to
/// decide — the assertion is that one of them did, and that the question is
/// settled either way.
#[tokio::test]
async fn an_answer_at_the_end_of_the_hold_is_delivered_once() {
    for round in 0..5 {
        let home = TempDir::new();
        let fleet = a_fleet_holding_for(&home, a_drone_that_reached_for("c1"), BRIEFLY);
        let job = started(&fleet, &home).await;
        fleet
            .set_when_blocked(&job, WhenBlocked::AskMe)
            .await
            .unwrap();
        let asking = asked("Bash", "npm publish", "c1");
        let before = lines_read(&fleet).await;

        // Across the end of the hold rather than at one point either side of
        // it: the rounds run from well inside it to well past it, so both
        // deliveries are taken without either being predicted.
        let fired_at = Duration::from_millis(30 + 10 * round);
        let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
            until_waiting(&fleet, &job).await;
            tokio::time::sleep(fired_at).await;
            fleet
                .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
                .await
        });

        answered.unwrap_or_else(|why| panic!("round {round}: the answer was not taken: {why}"));
        if answer == PermissionAnswer::Allow {
            tokio::time::sleep(GRACE).await;
            assert_eq!(
                lines_read(&fleet).await,
                before,
                "round {round}: the call carried it, and a turn carried it again"
            );
        } else {
            assert_eq!(
                answer,
                PermissionAnswer::Deny(Refusing::Asked.to_the_drone("npm publish")),
                "round {round}: the hold ended, so the call says to wait"
            );
            assert!(
                until_told(&fleet, before).await,
                "round {round}: the call gave up and no turn reached the Drone"
            );
        }
        assert!(
            fleet.command_awaited(&job).await.is_none(),
            "round {round}: settled, whichever side carried it"
        );
    }
}

/// The branch tip's `armada.yml`, scripted as the fake's [`FakeVcs::commits`]
/// script is: the on-disk copy in the Job's worktree is never read for it.
pub(super) const THE_TIP: &str = "version: 1\nid: 01FIXTUREMANIFEST\n";

/// What a Drone's own uncommitted edit to `armada.yml` looks like, left dirty
/// in the worktree by the time a person answers a held command.
pub(super) const DRONES_EDIT: &str = "version: 1\nid: 01FIXTUREMANIFEST\n# the drone's own edit\n";

/// A Fleet whose version control is scripted, on a Job whose worktree already
/// holds a dirty `armada.yml` — the shape `#6` is about: a Drone edited the
/// file and never committed it, and then a person picks "Always allow in this
/// repository" for a command it reached for.
pub(super) async fn dirty_manifest_job(home: &TempDir) -> (Fixture, JobId, std::path::PathBuf) {
    let mut fittings = the_fittings(home, a_drone_that_reached_for("c1"));
    fittings.vcs = FakeVcs::new().with_tip_content("armada.yml", THE_TIP);
    let fleet = Fleet::assembled(fittings);
    let job = fleet
        .propose(a_proposal("publish the package"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).expect("a legal spec");
    let armada_yml = std::path::Path::new(&spec.worktree_path()).join("armada.yml");
    std::fs::write(&armada_yml, DRONES_EDIT).expect("the drone's own edit");
    dispatched(&fleet, job.id()).await.unwrap();
    settled(&fleet).await;
    (fleet, job.id().clone(), armada_yml)
}

/// **`#836`: Always allow commits nothing.** The branch tip's `armada.yml`
/// and the Drone's own uncommitted edit are both exactly as they were, and
/// the allow is recorded for the repository rather than declared in the
/// file — `crate::tests::always_allow` covers what that record is and where a
/// later Job reads it.
#[tokio::test]
async fn always_allow_leaves_the_tip_and_the_drones_edit_untouched() {
    let home = TempDir::new();
    let (fleet, job, armada_yml) = dirty_manifest_job(&home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let before_the_allow = fleet.vcs().committed().len();
    let asking = asked("Bash", "npm publish", "c1");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(&job, "c1", Answered::of(CommandAnswer::AlwaysAllow, None))
            .await
    });

    answered.expect("the call is waiting, and always allow is one of the answers it offers");
    assert_eq!(answer, PermissionAnswer::Allow);

    assert_eq!(
        fleet.vcs().committed().len(),
        before_the_allow,
        "an always-allow commits nothing, since #836"
    );
    assert_eq!(
        std::fs::read_to_string(&armada_yml).expect("the file"),
        DRONES_EDIT,
        "the drone's own uncommitted edit is exactly as it was"
    );

    assert!(
        fleet
            .store()
            .lock()
            .await
            .allowed_commands(&job)
            .unwrap()
            .is_empty(),
        "the row lives in the repository-wide table, not this Job's own"
    );
}
