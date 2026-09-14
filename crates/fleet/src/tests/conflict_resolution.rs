//! A Job at its handoff gate whose pull request conflicts with main is sent
//! back for a Drone that can edit files. `sent_to_clear_conflicts` stays
//! generic over who asked (`#663`), though the only road left to it is
//! Fleet's own sweep (`#1131`) — the cases naming `Actor::Human` exercise the
//! primitive directly, since Bridge's press is gone.
//!
//! The workflow is `daemon`'s two-step fixture — `implement`, which writes
//! code, then `summarise`, which delivers and holds for a person.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{BroughtUpToDate, KeptCurrent, Landing, Rendering, Standing};
use core_model::{Actor, JobId, JobStatus, JobStep, StepId, StepState};
use testkit::{Delivered, FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fitted_with, manifest, note_evidence, one,
    two_steps_gated_on_a_person, worktree_directory,
};
use crate::tests::restarting::{on_it, the_handle, until_spoken};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const FIRST_BASE: &str = "8c2ce68100000000000000000000000000000000";

/// The two-step fixture, gated on `summarise`, over a harness that echoes back
/// what it is told, because these cases read a Drone's own opening brief.
fn a_fleet_echoing(home: &TempDir, vcs: FakeVcs) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::that_echoes_its_first_turn(),
    );
    fittings.starting().workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// The two-step shape with a Judge on `implement`, which `config` accepts only
/// gated `auto_if_judge_passes` — so it is written out rather than borrowed.
fn judged_then_held_for_a_person() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\n\
         steps:\n  - id: implement\n    label: \"Implement\"\n    \
         evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      \
         - type: diff_nonempty\n    judge_checks:\n      - criteria:\n          - \
         criterion_id: c1\n            question: does it fix the off-by-one?\n            \
         on_refusal: refuse\n    delivers: false\n    advance_gate: auto_if_judge_passes\n  - \
         id: summarise\n    label: \"Summarise\"\n    evidence: {submitted: {type: facts_note}}\n    \
         delivers: true\n    advance_gate: human_always\n",
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

/// The same, sweeping the forge every turn, with a Judge on `implement` this
/// case keeps a handle on.
fn a_fleet_sweeping(home: &TempDir, judge: &Arc<FakeJudge>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::that_echoes_its_first_turn(),
    );
    fittings.starting().workflows = one(judged_then_held_for_a_person());
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.judge = judge.clone();
    Fleet::assembled(fittings)
}

/// A Job driven to its handoff gate, with a pull request open.
async fn a_job_at_its_handoff_gate(home: &TempDir, fleet: &Fixture) -> JobId {
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job);
    dispatched(fleet, &job_id).await.expect("it dispatches");

    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.expect("implement's gate runs");
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("summarise's gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "summarise is the fixture's own human gate: {:?}",
        turned.ruled()
    );
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job reads").status(),
        JobStatus::AwaitingReview,
        "holding for a person, pull request already open"
    );
    job_id
}

/// Main moved to `onto` under the open pull request, and it conflicts — both
/// for the sweep's attempt and for the catch-up a spawn makes.
fn main_moved_with_a_conflict(fleet: &Fixture, onto: &str) {
    fleet.vcs().move_ref_to("main", onto);
    fleet.vcs().now_kept_current(KeptCurrent::Conflicted {
        onto: onto.to_string(),
        files: vec![String::from("src/parse.rs")],
    });
    fleet.vcs().now_landed(Landing::Open {
        url: String::from("https://forge.invalid/armada/pull/1"),
        rendering: Rendering::FromASupersededBase {
            pinned: String::from("67cb1b9e"),
            written_on: String::from("8c2ce681"),
        },
    });
    fleet.vcs().now_behind(
        Standing::Behind { commits: 1 },
        Some(BroughtUpToDate::Conflicted {
            base: String::from("main"),
            files: vec![String::from("src/parse.rs")],
        }),
    );
}

/// Turn until a Drone is on `step`. **Turned for, not assumed**: under a full
/// suite's load one turn does not always see the spawn through.
async fn until_a_drone_is_on(fleet: &Fixture, job_id: &JobId, step: &str) {
    for _ in 0..8 {
        fleet.turn().await.expect("a turn");
        let job = fleet.load(job_id).await.expect("the Job reads");
        let on = job.step(&StepId::new(step));
        if job.status() == JobStatus::Running
            && on.map(JobStep::state) == Some(StepState::Running)
            && on.and_then(JobStep::assigned_drone).is_some()
        {
            return;
        }
    }
    let status = fleet.load(job_id).await.ok().map(|job| job.status());
    panic!("no Drone reached `{step}`: {status:?}");
}

/// The Drone clears the markers, `implement`'s gate passes, and the walk
/// forward holds at `summarise` again.
async fn cleared_and_back_at_review(fleet: &Fixture, job_id: &JobId) {
    fleet.vcs().now_behind(Standing::UpToDate, None);
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("implement's gate runs again");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the markers are cleared, so the Checks pass: {:?}",
        turned.ruled()
    );
    until_a_drone_is_on(fleet, job_id, "summarise").await;
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("summarise's gate runs again");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "back at the same human gate: {:?}",
        turned.ruled()
    );
}

/// Every loop return on the Job, by whose hand.
async fn returns_by(fleet: &Fixture, job_id: &JobId) -> Vec<Actor> {
    let events = fleet
        .store()
        .lock()
        .await
        .events_for(job_id)
        .expect("the log reads");
    events
        .iter()
        .filter(|event| {
            matches!(
                event.moved(),
                store::Moved::Step {
                    returned_by: Some(_),
                    ..
                }
            )
        })
        .map(store::RecordedEvent::actor)
        .collect()
}

/// **`#1131`'s definition of done.** Nobody presses anything: the sweep finds
/// the conflict at the gate and sends the Job back as Fleet, the Drone is
/// handed the markers and told nothing was wrong with the work, its Checks gate
/// the pass and the Judge is not asked again, and the Job comes back to review
/// with the merge pushed — once for this base, however often the sweep runs.
#[tokio::test]
async fn the_sweep_sends_a_conflicted_job_at_its_gate_back_as_fleet_once_per_base() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let fleet = a_fleet_sweeping(&home, &judge);
    let job_id = a_job_at_its_handoff_gate(&home, &fleet).await;
    let judged_before = judge.asked().len();
    assert!(judged_before > 0, "implement's Judge ran on the first pass");

    main_moved_with_a_conflict(&fleet, FIRST_BASE);
    until_a_drone_is_on(&fleet, &job_id, "implement").await;
    assert_eq!(
        returns_by(&fleet, &job_id).await,
        vec![Actor::Fleet],
        "sent back once, and by Fleet"
    );

    let said = until_spoken(
        &home,
        &the_handle(&fleet, &job_id).await,
        &on_it(&fleet, &job_id).await,
    )
    .await;
    assert!(
        said.contains("conflict markers in them") && said.contains("src/parse.rs"),
        "the Drone is handed the markers and the file: {said}"
    );
    assert!(
        said.contains("Nothing was found wrong with the work"),
        "and told the pass is for the conflicts alone: {said}"
    );
    assert!(
        !said.contains("Nothing here is yours to fix")
            && !said.contains("read the work after this part passed"),
        "never told to leave it, nor handed the gate's findings to redo: {said}"
    );

    cleared_and_back_at_review(&fleet, &job_id).await;
    assert_eq!(
        judge.asked().len(),
        judged_before,
        "the change was judged already; clearing its conflicts is gated by its Checks"
    );

    for _ in 0..3 {
        fleet.turn().await.expect("the sweep runs again");
    }
    let job = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(job.status(), JobStatus::AwaitingReview, "back at review");
    assert_eq!(
        returns_by(&fleet, &job_id).await,
        vec![Actor::Fleet],
        "the same base is not sent round again"
    );
    assert_eq!(fleet.vcs().times_kept_current(), 1);
    let pushes = fleet
        .vcs()
        .delivered()
        .iter()
        .filter(|did| matches!(did, Delivered::Pushed { .. }))
        .count();
    assert_eq!(
        pushes, 2,
        "the first delivery, and the merge on the ordinary push"
    );
}

/// **Clean clears do not add up.** A long-lived Job clears two conflicts, both
/// pushed, then one pass comes back still conflicting: that is one in a row.
#[tokio::test]
async fn two_clean_clears_then_one_conflicted_pass_is_sent_again() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let fleet = a_fleet_sweeping(&home, &judge);
    let job_id = a_job_at_its_handoff_gate(&home, &fleet).await;
    let base = |n: u32| format!("{n:040x}");

    for n in 1..=2 {
        main_moved_with_a_conflict(&fleet, &base(n));
        until_a_drone_is_on(&fleet, &job_id, "implement").await;
        cleared_and_back_at_review(&fleet, &job_id).await;
    }
    main_moved_with_a_conflict(&fleet, &base(3));
    until_a_drone_is_on(&fleet, &job_id, "implement").await;
    still_conflicting_back_at_review(&fleet, &job_id).await;

    main_moved_with_a_conflict(&fleet, &base(4));
    until_a_drone_is_on(&fleet, &job_id, "implement").await;
    assert_eq!(returns_by(&fleet, &job_id).await, vec![Actor::Fleet; 4]);
}

/// The same pass, but the base moved again while it ran: the delivering step's
/// own catch-up conflicts, so nothing is pushed and the pull request still
/// conflicts when the Job holds at review.
async fn still_conflicting_back_at_review(fleet: &Fixture, job_id: &JobId) {
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("implement's gate runs again");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "{:?}",
        turned.ruled()
    );
    until_a_drone_is_on(fleet, job_id, "summarise").await;
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("summarise's gate runs again");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "{:?}",
        turned.ruled()
    );
}

/// **The loop ends.** Each pass comes back unpushed, so the count never starts
/// again, and the conflict after the third such pass escalates as `loop_cap`
/// rather than going round again.
#[tokio::test]
async fn a_third_pass_that_still_conflicts_escalates() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let fleet = a_fleet_sweeping(&home, &judge);
    let job_id = a_job_at_its_handoff_gate(&home, &fleet).await;
    let base = |n: u32| format!("{n:040x}");

    for n in 1..=crate::conflict_resolution::CLEARING_SENDS {
        main_moved_with_a_conflict(&fleet, &base(n));
        until_a_drone_is_on(&fleet, &job_id, "implement").await;
        still_conflicting_back_at_review(&fleet, &job_id).await;
    }

    main_moved_with_a_conflict(&fleet, &base(0xff));
    fleet
        .turn()
        .await
        .expect("the sweep reads the conflict past the last send");

    let job = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(job.status(), JobStatus::Escalated, "a person decides now");
    assert_eq!(
        job.step(&StepId::new("summarise")).map(JobStep::state),
        Some(StepState::Stopped),
        "stopped at the gate it was waiting at"
    );
    let fleet_sends = crate::conflict_resolution::CLEARING_SENDS as usize;
    assert_eq!(
        returns_by(&fleet, &job_id).await,
        vec![Actor::Fleet; fleet_sends],
        "and not sent back past the bound"
    );
}

/// **`#663`'s primitive, run as a person would have reached it.** Sends the
/// branch back to `implement`, a Drone there reads the conflict as its opening
/// brief, and the walk forward carries the Job back through `summarise`, which
/// pushes the fix. Nothing reaches `summarise`'s own Drone about the conflict.
#[tokio::test]
async fn a_conflicted_pull_request_at_the_handoff_gate_is_resolved_without_reaching_its_drone() {
    let home = TempDir::new();
    let fleet = a_fleet_echoing(&home, FakeVcs::new());
    let job_id = a_job_at_its_handoff_gate(&home, &fleet).await;

    fleet.vcs().now_behind(
        Standing::Behind { commits: 1 },
        Some(BroughtUpToDate::Conflicted {
            base: String::from("main"),
            files: vec![String::from("src/parse.rs")],
        }),
    );

    let sent_back = fleet
        .sent_to_clear_conflicts(&job_id, Actor::Human)
        .await
        .expect("the gate offers this, and the branch is behind");
    assert_eq!(
        sent_back.status(),
        JobStatus::Queued,
        "queued, and the turn is what spawns the fresh Drone — #428's own shape"
    );
    assert_eq!(
        sent_back
            .step(&StepId::new("summarise"))
            .map(JobStep::state),
        Some(StepState::AwaitingHuman),
        "the gate itself has not moved"
    );
    assert_eq!(returns_by(&fleet, &job_id).await, vec![Actor::Human]);

    fleet.turn().await.expect("the fresh Drone is spawned");
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job reads")
            .step(&StepId::new("implement"))
            .map(JobStep::state),
        Some(StepState::Running),
        "implement is being redone, not summarise"
    );

    let said = until_spoken(
        &home,
        &the_handle(&fleet, &job_id).await,
        &on_it(&fleet, &job_id).await,
    )
    .await;
    assert!(
        said.contains("conflict markers in them") && said.contains("src/parse.rs"),
        "the Drone on `implement` was not told what to resolve: {said}"
    );

    fleet.vcs().now_behind(Standing::UpToDate, None);
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("implement's gate runs again");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the conflict is resolved, so this pass clears the gate: {:?}",
        turned.ruled()
    );

    until_a_drone_is_on(&fleet, &job_id, "summarise").await;
    let re_entered_drone = on_it(&fleet, &job_id).await;
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    let turned = fleet
        .turn()
        .await
        .expect("summarise's gate runs a second time");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "back at the same human gate it left: {:?}",
        turned.ruled()
    );

    let said_to_summarise =
        until_spoken(&home, &the_handle(&fleet, &job_id).await, &re_entered_drone).await;
    assert!(
        !said_to_summarise.contains("conflict"),
        "the handoff Drone was told about a conflict, which #660 is the whole \
         argument against: {said_to_summarise}"
    );
    assert!(
        fleet
            .vcs()
            .delivered()
            .iter()
            .any(|done| matches!(done, Delivered::Pushed { .. })),
        "the merged branch was pushed, updating the pull request in place"
    );
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job reads").status(),
        JobStatus::AwaitingReview,
        "back where the person left it, with the branch now current"
    );
}

/// A call with no open pull request has nothing to resolve.
#[tokio::test]
async fn a_call_with_no_open_pull_request_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_echoing(
        &home,
        FakeVcs::new().delivering(testkit::Delivering {
            push: adapter_traits::Pushed::NoRemote,
            review: adapter_traits::Opened::NothingPushed,
            ..testkit::Delivering::default()
        }),
    );
    let job_id = a_job_at_its_handoff_gate(&home, &fleet).await;

    let refused = fleet
        .sent_to_clear_conflicts(&job_id, Actor::Human)
        .await
        .expect_err("no remote, so no pull request was ever opened");
    assert!(matches!(
        refused,
        crate::adrift::Adrift::NothingToResolve { .. }
    ));
}

/// A call off the review gate is refused the way every other answer there is.
#[tokio::test]
async fn a_call_off_the_gate_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_echoing(&home, FakeVcs::new());
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("it dispatches");

    let refused = fleet
        .sent_to_clear_conflicts(&job_id, Actor::Human)
        .await
        .expect_err("running, not awaiting_review");
    assert!(matches!(
        refused,
        crate::adrift::Adrift::NotUnderReview { .. }
    ));
}
