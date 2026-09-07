//! What a Drone was told before it called: the block the first turn carries,
//! and the tool description that points at it.
//!
//! **A tool nothing points at is the defect this capability is about**, and a
//! tool offered on a step that would refuse the call is that defect from the
//! other side — a Drone pointed at a refusal reads it as a broken system. So
//! the offer appears and disappears with the Checks the step declares.
//!
//! The heading is one string two crates share with nothing in the type system
//! holding them together, which is why one case reads it back off the wire.

use std::sync::Arc;

use core_model::StepId;
use testkit::Sketch;

use crate::briefing::first_turn;
use crate::terms::Checking;
use crate::tests::dry_run::{a_fleet_checking, one_step, post, router, Held};
use crate::tests::tmp::TempDir;

/// **A tool nothing points at is the defect this capability is about.** The
/// first turn offers it, in the same block shape a scope declaration is asked
/// for in — and it names no tool, for the reason `Declaring` does not.
#[test]
fn the_first_turn_offers_the_dry_run_and_says_it_is_not_a_pass() {
    let workflow = one_step("/usr/bin/true");
    let said = first_turn(
        &crate::tests::briefing::a_job(),
        workflow.frozen(),
        &StepId::new("implement"),
        &crate::crossing::Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string();

    assert!(said.contains("FINDING OUT WHERE YOU STAND"), "{said}");
    assert!(
        said.contains("not a verdict"),
        "a Drone that read a green dry run as a finished part would be worse \
         off for having been offered it: {said}"
    );
    assert!(
        said.contains("Submitting is still the only way to report"),
        "{said}"
    );
    assert!(
        !said.contains("mcp__") && !said.contains("run_checks"),
        "described rather than named, like the other two tools: {said}"
    );
    assert!(
        !said.contains("/usr/bin/true"),
        "the offer is not the Check. Nothing here is written from a resolved \
         command: {said}"
    );
    assert!(
        said.contains("These are the checks that gate this part:\n\n  - suite\n  - diff_nonempty"),
        "both kinds, by label, in the order the step declares them — the same \
         words and the same order the report comes back in: {said}"
    );
}

/// The tool's description sends a Drone to the block above by its heading, and
/// a heading that moved would send it nowhere. **Two crates, one string, and
/// nothing in the type system holding them together** — `ipc` assembles its
/// tool list with no step in hand, so the names live in `fleet` and the
/// description points at where they are.
#[tokio::test]
async fn the_tool_points_at_the_block_that_names_the_checks() {
    let workflow = one_step("/usr/bin/true");
    let step = workflow
        .frozen()
        .steps()
        .iter()
        .find(|step| step.id() == &StepId::new("implement"))
        .expect("the step")
        .clone();
    let offer = Checking::at(&step).expect("an offer");
    let heading = offer
        .text()
        .lines()
        .next()
        .expect("the block's own heading");

    let home = TempDir::new();
    let fleet = a_fleet_checking(
        &home,
        one_step("/usr/bin/true"),
        Arc::new(Held::started()),
        3,
    );
    let listed = post(
        &router(&Arc::new(fleet)),
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    )
    .await;

    assert!(
        listed.contains(heading),
        "the description names the block `{heading}`: {listed}"
    );
}

/// And a step with nothing to run is not offered it. A Drone pointed at a call
/// that will be refused reads the refusal as a broken system, which is the
/// silent denial this whole issue is about arriving from the other side.
#[test]
fn a_step_with_no_checks_is_not_offered_the_dry_run() {
    let unchecked = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let step = unchecked
        .frozen()
        .steps()
        .iter()
        .find(|step| step.id() == &StepId::new("implement"))
        .expect("the step")
        .clone();
    assert!(Checking::at(&step).is_none());
}
