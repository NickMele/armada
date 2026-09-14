//! What a Drone's own run leaves out, and what its brief says about it. #849.

use std::path::Path;
use std::sync::Arc;

use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::StepId;

use crate::terms::Checking;
use crate::tests::dry_run::{a_fleet_checking, ask, router, started, Held};
use crate::tests::tmp::TempDir;

/// `implement` and `tests` each gating on every Check, then `handoff`, against
/// a Manifest with `suite` everywhere, `slow` gate only and `e2e` handoff only.
fn declared(slow: &str) -> ResolvedWorkflow {
    let workflow = "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\nsteps:\n\
         - id: implement\n  label: Implement\n  evidence: {submitted: {type: diff}}\n  \
         delivers: false\n  advance_gate: auto\n  mechanical_checks: [{ type: every_manifest_check }]\n\
         - id: tests\n  label: Tests\n  evidence: {submitted: {type: diff}}\n  \
         delivers: false\n  advance_gate: auto\n  mechanical_checks: [{ type: every_manifest_check }]\n\
         - id: handoff\n  label: Handoff\n  delivers: true\n  advance_gate: auto\n";
    let manifest = format!(
        "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  suite:\n    run: /usr/bin/true\n  \
         slow:\n    run: \"{slow}\"\n    runs_at: gate\n  \
         e2e:\n    run: /usr/bin/false\n    runs_at: handoff\n"
    );
    let def = WorkflowDef::parse(
        Path::new("shaped.yml"),
        workflow,
        &Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the workflow did not parse: {refused}"));
    let manifest = Manifest::parse(Path::new("armada.yml"), &manifest)
        .unwrap_or_else(|refused| panic!("the manifest did not parse: {refused}"));
    ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared")
}

/// **The third line of the definition of done, in miniature.** `slow` would
/// fail if it ran; the run passes and never names it.
#[tokio::test]
async fn a_gate_only_check_is_not_in_a_drones_run() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        declared("/usr/bin/false"),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = ask(&app, &fleet, &home).await;

    assert!(!said.is_error, "{}", said.text);
    assert!(said.text.contains("suite"), "{}", said.text);
    assert!(
        !said.text.contains("slow") && !said.text.contains("FAILED"),
        "the gate-only Check neither ran nor appeared: {}",
        said.text
    );
}

/// The brief names what asking leaves out and where each runs instead, so a
/// clean run does not read as the whole bar.
#[test]
fn the_brief_says_which_checks_a_run_leaves_out_and_where_they_run() {
    let workflow = declared("/usr/bin/true");
    let frozen = workflow.frozen();

    let implement = frozen.step(&StepId::new("implement")).expect("the step");
    let told = Checking::at(frozen, implement)
        .expect("an offer")
        .text()
        .to_string();
    assert!(
        told.contains("Asking does not run `slow`: it runs only when you submit."),
        "{told}"
    );
    assert!(
        told.contains("`e2e` is not checked on this part at all"),
        "{told}"
    );

    let tests = frozen.step(&StepId::new("tests")).expect("the step");
    let told = Checking::at(frozen, tests)
        .expect("an offer")
        .text()
        .to_string();
    assert!(told.contains("Asking does not run `e2e` either"), "{told}");
    assert!(!told.contains("not checked on this part at all"), "{told}");
}
