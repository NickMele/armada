//! `#663`'s own repro: a Job at its handoff gate, whose pull request is behind
//! main with conflicts, is resolved by a Drone that can edit files rather than
//! by the one that cannot.
//!
//! The workflow is `daemon`'s two-step fixture — `implement`, which writes
//! code, then `summarise`, which delivers and holds for a person. That is
//! `a_job_held_at_its_handoff_gate_serves_its_delivery`'s own shape, in
//! `crate::tests::landing`, carried one press further.

use adapter_traits::{BroughtUpToDate, Standing};
use core_model::{JobStatus, StepId, StepState};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fitted_with, note_evidence, one, two_steps_gated_on_a_person,
    worktree_directory,
};
use crate::tests::restarting::{on_it, the_handle, until_spoken};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The two-step fixture, gated on `summarise` — `a_fleet_gated_on_a_person`'s
/// own shape, over a harness that echoes back what it is told rather than the
/// default that only listens, because this suite has to read a Drone's own
/// opening brief.
fn a_fleet_echoing(home: &TempDir, vcs: FakeVcs) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::that_echoes_its_first_turn(),
    );
    fittings.workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.vcs = vcs;
    Fleet::assembled(fittings)
}

/// A Job driven to its handoff gate, with a pull request open — every case
/// here starts from the same place #660 was found in.
async fn a_job_at_its_handoff_gate(home: &TempDir, fleet: &Fixture) -> core_model::JobId {
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

/// **The whole of `#663`'s follow-up.** A press sends the branch back to
/// `implement` — not to `summarise`, which only summarises and cannot run git
/// or edit a file — a Drone there reads the conflict as its opening brief,
/// resolves it, and the ordinary forward walk carries the Job back through
/// `summarise` a second time, which is what pushes the fix and updates the
/// pull request. Nothing reaches `summarise`'s own Drone about the conflict at
/// any point.
#[tokio::test]
async fn a_conflicted_pull_request_at_the_handoff_gate_is_resolved_without_reaching_its_drone() {
    let home = TempDir::new();
    let fleet = a_fleet_echoing(&home, FakeVcs::new());
    let job_id = a_job_at_its_handoff_gate(&home, &fleet).await;

    // The shape #660 was in: main moved under the open pull request and the
    // rebase conflicts.
    fleet.vcs().now_behind(
        Standing::Behind { commits: 1 },
        Some(BroughtUpToDate::Conflicted {
            base: String::from("main"),
            files: vec![String::from("src/parse.rs")],
        }),
    );

    let sent_back = fleet
        .resolve_pull_request_conflict(&job_id)
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
            .map(core_model::JobStep::state),
        Some(StepState::AwaitingHuman),
        "the gate itself has not moved"
    );

    fleet.turn().await.expect("the fresh Drone is spawned");
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job reads")
            .step(&StepId::new("implement"))
            .map(core_model::JobStep::state),
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
        said.contains("conflict markers in them"),
        "the Drone on `implement` was not told there was anything to resolve: {said}"
    );
    assert!(
        said.contains("src/parse.rs"),
        "and it was not told which file: {said}"
    );

    // The Drone resolves it. From here on the branch is current — the same
    // fact a real `git rebase` would leave behind once the markers are gone.
    fleet.vcs().now_behind(Standing::UpToDate, None);
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("implement's gate runs again");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the conflict is resolved, so this pass clears the gate: {:?}",
        turned.ruled()
    );

    // The forward walk carries the Job back through `summarise`, which is
    // `delivers()`'s own step: a fresh Drone is spawned there for its own
    // ordinary reason — writing the facts note — and never told about a
    // conflict, because there was never one in its own brief to carry.
    fleet.turn().await.expect("summarise is re-entered");
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
            .any(|done| matches!(done, testkit::Delivered::PushedForcing { .. })),
        "the rebased branch was pushed with force-with-lease, updating the pull \
         request in place"
    );
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job reads").status(),
        JobStatus::AwaitingReview,
        "back where the person left it, with the branch now current"
    );
}

/// A press with no open pull request has nothing to resolve.
#[tokio::test]
async fn a_press_with_no_open_pull_request_is_refused() {
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
        .resolve_pull_request_conflict(&job_id)
        .await
        .expect_err("no remote, so no pull request was ever opened");
    assert!(matches!(
        refused,
        crate::adrift::Adrift::NothingToResolve { .. }
    ));
}

/// A press off the review gate is refused the way every other answer at that
/// gate already is.
#[tokio::test]
async fn a_press_off_the_gate_is_refused() {
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
        .resolve_pull_request_conflict(&job_id)
        .await
        .expect_err("running, not awaiting_review");
    assert!(matches!(
        refused,
        crate::adrift::Adrift::NotUnderReview { .. }
    ));
}
