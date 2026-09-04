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

use testkit::{FakeWorkProduct, Gate, Sketch};

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
    worktree_directory(&home, job.id());
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
