//! A person's review model reaches the step that writes Armada's review, beats the Job's
//! own chosen model there, and no other step. #903.

use core_model::JobId;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::permitting::NotPermitted;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_holding, a_proposal, diff_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// `implement` produces a diff, and `handoff` writes the review.
fn implement_then_review() -> config::ResolvedWorkflow {
    testkit::modelled(
        &[
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[Gate::DiffNonempty],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
            Sketch {
                id: "handoff",
                label: "Review the change",
                evidence_type: Some("review"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
        ],
        &[],
    )
}

async fn on_implement(home: &TempDir) -> (Fixture, JobId) {
    let fleet = a_fleet_holding(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        implement_then_review(),
        1,
    );
    let job = fleet
        .propose(a_proposal("a person picks the review model"))
        .await
        .expect("a proposal");
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.expect("it is approved");
    (fleet, job.id().clone())
}

async fn onto_the_review(fleet: &Fixture) {
    submitted_by_the_one(fleet, diff_evidence())
        .await
        .expect("evidence lands");
    fleet.turn().await.expect("implement advances");
}

/// **On the review step, the review model beats the Job's chosen model.**
#[tokio::test]
async fn the_review_step_spawns_on_the_review_model_over_the_jobs_chosen_one() {
    let home = TempDir::new();
    let (fleet, job) = on_implement(&home).await;

    fleet
        .set_model(&job, Some("another-model"))
        .await
        .expect("a model this Fleet offers");
    fleet
        .set_review_model(&job, Some("a-model"))
        .await
        .expect("a model this Fleet offers");
    assert_eq!(
        fleet.review_model_override_of(&job).await.as_deref(),
        Some("a-model")
    );

    onto_the_review(&fleet).await;
    let spawned = fleet.harness().configured();
    assert_eq!(spawned.len(), 2, "the review is a second Drone");
    assert_eq!(
        spawned[1].model().as_str(),
        "a-model",
        "the review model, over the Job's chosen another-model"
    );
}

/// With no review model chosen, the review step runs on the Job's chosen model as before.
#[tokio::test]
async fn without_a_review_model_the_review_step_runs_on_the_jobs_chosen_model() {
    let home = TempDir::new();
    let (fleet, job) = on_implement(&home).await;

    fleet
        .set_model(&job, Some("another-model"))
        .await
        .expect("chosen");
    onto_the_review(&fleet).await;
    assert_eq!(
        fleet.harness().configured()[1].model().as_str(),
        "another-model"
    );
}

/// A name `list_models` does not offer is refused, and nothing is recorded.
#[tokio::test]
async fn a_review_model_this_fleet_does_not_offer_is_refused() {
    let home = TempDir::new();
    let (fleet, job) = on_implement(&home).await;

    let refused = fleet.set_review_model(&job, Some("no-such-model")).await;
    assert!(
        matches!(refused, Err(NotPermitted::NoSuchModel { .. })),
        "{refused:?}"
    );
    assert_eq!(fleet.review_model_override_of(&job).await, None);
}
