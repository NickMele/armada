//! Which model each step's Drone is started as.
//!
//! **This could not be asked before a step was its own process.** One session
//! spanned a whole Job and a session cannot change model partway, so
//! `spawn_config` built every Drone with `job.model()` and there was nothing to
//! vary. Now that a step is its own process the two spawns of one Job can
//! differ, and the thing worth asserting is that they do — and that a step
//! naming nothing still gets the Job's, which is what every workflow written
//! before this relies on.
//!
//! The assertion is on the `DroneSpawnConfig` rather than on argv: which flag a
//! vendor's CLI spells a model with is `adapters`' question and is asserted
//! there.

use core_model::JobId;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::permitting::NotPermitted;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_holding, a_proposal, diff_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// Two steps: `implement` produces a diff and names no model, `summarise`
/// reports and names one. The shape every shipped workflow has — the
/// annotation is on the step that wants something other than the Job's.
fn a_diff_step_then_a_reporting_step() -> config::ResolvedWorkflow {
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
                id: "summarise",
                label: "Summarise",
                evidence_type: Some("facts_note"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
        ],
        &[("summarise", "the-reporting-model")],
    )
}

/// **Both halves, in one Job, because they are one claim.** A test that only
/// showed the override would pass against a Fleet that had stopped reading the
/// Job's model at all, and a test that only showed the fallback would pass
/// against the code as it stood before #141.
#[tokio::test]
async fn each_step_is_spawned_as_the_model_its_own_step_named() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_diff_step_then_a_reporting_step(),
        1,
    );
    let job = fleet
        .propose(a_proposal("two steps, two models"))
        .await
        .expect("a proposal");
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.expect("it is approved");

    let after_first = fleet.harness().configured();
    assert_eq!(after_first.len(), 1, "one step, one Drone");
    assert_eq!(
        after_first[0].model().as_str(),
        "a-model",
        "`implement` names no model, so it is run as the Job was proposed"
    );

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("evidence lands");
    fleet.turn().await.expect("the first step advances");

    let after_second = fleet.harness().configured();
    assert_eq!(after_second.len(), 2, "the second step is a second Drone");
    assert_eq!(
        after_second[1].model().as_str(),
        "the-reporting-model",
        "`summarise` names one, and the step boundary is where it takes effect"
    );
}

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The two-step Job, on its first step with a Drone already spawned.
async fn on_its_first_step(home: &TempDir) -> (Fixture, JobId) {
    let fleet = a_fleet_holding(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_diff_step_then_a_reporting_step(),
        1,
    );
    let job = fleet
        .propose(a_proposal("a person picks the model"))
        .await
        .expect("a proposal");
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.expect("it is approved");
    (fleet, job.id().clone())
}

/// Evidence for the first step, and the turn that spawns the second.
async fn onto_the_second_step(fleet: &Fixture) {
    submitted_by_the_one(fleet, diff_evidence())
        .await
        .expect("evidence lands");
    fleet.turn().await.expect("the first step advances");
}

/// **The next spawn is where a choice lands**, and it beats the model the
/// step named for itself. The Drone already running is untouched.
#[tokio::test]
async fn a_chosen_model_wins_over_the_steps_own_at_the_next_spawn() {
    let home = TempDir::new();
    let (fleet, job) = on_its_first_step(&home).await;

    fleet
        .set_model(&job, Some("another-model"))
        .await
        .expect("a model this Fleet offers");
    assert_eq!(
        fleet.model_override_of(&job).await.as_deref(),
        Some("another-model")
    );
    let before = fleet.harness().configured();
    assert_eq!(before.len(), 1, "nothing respawned for the choice");
    assert_eq!(before[0].model().as_str(), "a-model");

    onto_the_second_step(&fleet).await;
    let after = fleet.harness().configured();
    assert_eq!(after.len(), 2);
    assert_eq!(
        after[1].model().as_str(),
        "another-model",
        "over `summarise`'s own the-reporting-model"
    );
}

/// Clearing a choice puts the later step back on the model it named.
#[tokio::test]
async fn clearing_a_chosen_model_restores_the_steps_own() {
    let home = TempDir::new();
    let (fleet, job) = on_its_first_step(&home).await;

    fleet
        .set_model(&job, Some("another-model"))
        .await
        .expect("chosen");
    fleet.set_model(&job, None).await.expect("cleared");
    assert_eq!(fleet.model_override_of(&job).await, None);

    onto_the_second_step(&fleet).await;
    assert_eq!(
        fleet.harness().configured()[1].model().as_str(),
        "the-reporting-model"
    );
}

/// A name `list_models` does not offer is refused, and nothing is recorded.
#[tokio::test]
async fn a_model_this_fleet_does_not_offer_is_refused() {
    let home = TempDir::new();
    let (fleet, job) = on_its_first_step(&home).await;

    let refused = fleet.set_model(&job, Some("no-such-model")).await;

    match refused {
        Err(NotPermitted::NoSuchModel { named, offered }) => {
            assert_eq!(named, "no-such-model");
            assert_eq!(offered, vec!["a-model", "another-model"]);
        }
        other => panic!("an unknown model is refused: {other:?}"),
    }
    assert_eq!(fleet.model_override_of(&job).await, None);
}
