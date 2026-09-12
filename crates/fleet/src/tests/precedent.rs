//! What a step's own history says before its next mid-step look sees new
//! work — Job `01M28RVVN200232YNHWF8CFFKH`'s tests step failed the same Check
//! three times and the look never saw the first two.

use core_model::StepId;
use testkit::{FakeWorkProduct, Gate, Sketch};

use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_holding, a_proposal, diff_evidence, worktree_directory};
use crate::tests::tools::submitted_by_the_one;

fn gated_on_a_check_that_always_fails(budget: u32) -> config::ResolvedWorkflow {
    testkit::retried(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::Check {
                name: "suite",
                run: "/bin/sh -c 'echo it exited 101 1>&2; exit 101'",
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

/// A first attempt has no history to be measured against.
#[tokio::test]
async fn a_first_attempt_has_no_precedent() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on_a_check_that_always_fails(2),
        1,
    );
    let job = fleet.propose(a_proposal("fix the route")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    let found = fleet
        .precedent_failures(job.id(), &StepId::new("implement"))
        .await;
    assert!(found.is_empty(), "{found:?}");
}

/// **What the Job needed and did not have.** A second attempt of the same
/// step can see what its own last attempt did not pass, grounding a
/// `converging` answer in the step's own history rather than a diff's shape.
#[tokio::test]
async fn a_second_attempt_sees_what_the_first_one_did_not_pass() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_on_a_check_that_always_fails(2),
        1,
    );
    let job = fleet.propose(a_proposal("fix the route")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled(), Some(Ruling::HandedBack { .. })),
        "{turned:?}"
    );

    let found = fleet
        .precedent_failures(job.id(), &StepId::new("implement"))
        .await;
    let [check] = found.as_slice() else {
        panic!("one failed Check from the first attempt: {found:?}");
    };
    assert_eq!(check.name, "suite");
    assert_eq!(check.produced.as_deref(), Some("it exited 101"));
}
