//! A pull request's failed CI, acted on from the Job: re-run on the forge, or a Drone sent
//! back to find out why. #905.

use std::time::Duration;

use core_model::JobStatus;
use testkit::{Delivering, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, one, two_steps_gated_on_a_person,
    worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::Adrift;

type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";

fn a_fleet_at_a_gate(home: &TempDir, delivering: Delivering) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.vcs = FakeVcs::new().delivering(delivering);
    fittings.noticing = Noticing::every(Duration::from_secs(3600));
    Fleet::assembled(fittings)
}

/// Work the Job to its last step, which delivers on entry and then holds.
async fn at_the_gate(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    job.id().clone()
}

/// What the sweep would have read: two checks, one of them failed.
async fn one_check_failed(fleet: &Fixture) {
    fleet.sweeping().lock().await.pr_detail.insert(
        PULL_REQUEST.to_string(),
        ipc::PullRequestDetail {
            number: Some(1),
            title: None,
            mergeable: Some(true),
            reviews: Vec::new(),
            currency: None,
            checks: Some(ipc::PullRequestChecks {
                kind: "some_failed".to_string(),
                checks: 2,
                finished: None,
                failed: vec!["unit tests".to_string()],
            }),
        },
    );
}

#[tokio::test]
async fn re_running_asks_the_forge_and_moves_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_at_a_gate(&home, Delivering::default());
    let job = at_the_gate(&fleet, &home).await;

    let moved = fleet
        .rerun_failed_checks(&job)
        .await
        .expect("an open pull request");
    assert_eq!(moved.status(), JobStatus::AwaitingReview, "nothing moved");
}

#[tokio::test]
async fn a_rerun_the_forge_refuses_is_refused_in_its_words() {
    let home = TempDir::new();
    let fleet = a_fleet_at_a_gate(
        &home,
        Delivering {
            rerun: Err(adapter_traits::NotRerun {
                said: "no failed run".to_string(),
            }),
            ..Delivering::default()
        },
    );
    let job = at_the_gate(&fleet, &home).await;

    match fleet.rerun_failed_checks(&job).await {
        Err(Adrift::RerunRefused { said, .. }) => assert_eq!(said, "no failed run"),
        other => panic!("refused in the forge's words, got {other:?}"),
    }
}

#[tokio::test]
async fn investigating_with_nothing_failed_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_at_a_gate(&home, Delivering::default());
    let job = at_the_gate(&fleet, &home).await;

    assert!(matches!(
        fleet.investigate_failed_checks(&job).await,
        Err(Adrift::NothingToInvestigate { .. })
    ));
}

/// **The Job goes back with the failed checks as the next Drone's note**, and re-runs nothing.
#[tokio::test]
async fn investigating_sends_the_branch_back_naming_what_failed() {
    let home = TempDir::new();
    let fleet = a_fleet_at_a_gate(&home, Delivering::default());
    let job = at_the_gate(&fleet, &home).await;
    one_check_failed(&fleet).await;

    let sent = fleet
        .investigate_failed_checks(&job)
        .await
        .expect("a failed check at the gate");
    assert_eq!(sent.status(), JobStatus::Queued);
    let note = sent
        .redirect_waiting()
        .expect("the failed checks wait for the next Drone")
        .text()
        .to_string();
    assert!(note.contains("unit tests"), "{note}");
    assert!(note.contains("Do not re-run anything"), "{note}");
}
