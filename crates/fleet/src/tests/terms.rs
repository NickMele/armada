//! What a step's own terms say to the Drone working it.
//!
//! **The other half of [`briefing`](mod@crate::tests::briefing)**, split with
//! the module it tests: these are the four blocks a step puts to whoever works
//! it, and their text does not depend on which turn carried it. The cases
//! against a *moment* — a boundary that re-asks, a restart whose sentence has
//! to match the record — stay there, because what they are about is whether the
//! right thing was sent at all.
//!
//! `Checking`'s cases are `crate::tests::dry_run`'s and were before this split:
//! the block offers a tool, and what it says is only worth asserting beside
//! what happens when the tool is called.
//!
//! Every case reads the text out of an assembled turn rather than off the type,
//! because a block a Drone never receives is the failure worth catching and a
//! block asserted in isolation cannot see it.

use core_model::{RepoPath, StepId};
use testkit::{Gate, Scoped, Sketch};

use crate::briefing::first_turn;
use crate::crossing::Crossed;
use crate::terms::Redeclaring;
use crate::tests::briefing::{a_job, turn_at};

/// A step that asks for no scope gets no block, because there is no call it
/// could make — telling every Drone about a tool most of them cannot use is how
/// an instruction stops being read.
#[test]
fn a_step_with_no_scope_is_told_nothing_about_declaring_one() {
    for step in ["implement", "summarise"] {
        assert!(!turn_at(step).contains("BEFORE YOU START"));
    }
}

/// A step that asks for one says so, and says what a plan that turned out wrong
/// is fixed by. **The obligation is in the prompt** rather than only in the
/// tool description, for the reason the reporting clause is: spike 6 measured
/// that a description alone does not make a Drone call a tool.
#[test]
fn a_scoped_step_is_told_to_declare_before_it_starts() {
    let workflow = testkit::frozen(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: Some(testkit::Scoped {
            diff_check: true,
            at_step_start: true,
            exclude: &["secrets"],
            references: &[],
        }),
        gaming: None,
    }]);
    let said = first_turn(
        &a_job(),
        &workflow,
        &StepId::new("implement"),
        &Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string();

    assert!(said.contains("BEFORE YOU START"));
    assert!(
        said.contains("call the tool"),
        "a wrong plan has a way out: {said}"
    );
    assert!(said.contains("secrets"), "the denylist is named: {said}");
    assert!(
        !said.contains("mcp__") && !said.to_lowercase().contains("declare_scope"),
        "described rather than named, like the Evidence tool: {said}"
    );
}

// --------------------------------------------- what a drifting Drone is told

/// One step, watched or not, for the drift notice to be built from.
fn watching(live: bool) -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: Some(Scoped {
            diff_check: true,
            at_step_start: live,
            exclude: &[],
            references: &[],
        }),
        gaming: None,
    }])
}

fn drift_notice(workflow: &config::ResolvedWorkflow, paths: &[&str]) -> Option<Redeclaring> {
    Redeclaring::at(
        workflow.steps().first().expect("a first step"),
        &paths
            .iter()
            .map(|path| RepoPath::new(*path))
            .collect::<Vec<RepoPath>>(),
    )
}

/// **The mechanism nothing could reach.** Drift has been compared against the
/// plan on every turn since the scope tool existed, and every finding went to
/// the Job's log — which no Drone reads. The notice names the file and names
/// the call, because "you edited outside your scope" does not make re-declaring
/// obvious to anything reading it.
#[test]
fn a_drone_that_drifted_is_told_which_file_and_which_call() {
    let workflow = watching(true);
    let notice = drift_notice(&workflow, &["crates/ipc/src/lib.rs"]).expect("a watched step");

    let said = notice.text();
    assert!(said.contains("crates/ipc/src/lib.rs"), "{said}");
    assert!(
        said.contains("call the scope tool again"),
        "the call that fixes it, not just the finding: {said}"
    );
    assert!(
        !said.contains("mcp__") && !said.to_lowercase().contains("declare_scope"),
        "described rather than named, like every other tool: {said}"
    );
}

/// **Not an accusation and not a stop-work order.** Drift is a signal because
/// investigation legitimately moves the work, so a Drone that reads this and
/// carries on has done nothing wrong. A notice that read like the thrashing
/// directive would have a Drone down tools over a file it was right to touch.
#[test]
fn the_drift_notice_asks_for_nothing_but_the_call() {
    let workflow = watching(true);
    let notice = drift_notice(&workflow, &["src/lib.rs"]).expect("a watched step");

    let said = notice.text();
    assert!(said.contains("Nothing has failed"), "{said}");
    assert!(said.contains("not being asked to stop"), "{said}");
    assert!(
        !said.contains("Stop and report"),
        "that is the thrashing directive, and this is not it: {said}"
    );
    assert!(
        !said.contains("failed to") && !said.contains("should have"),
        "nothing here is put to the Drone as a fault: {said}"
    );
}

/// The cold switch, on the block this time. A step whose plan is measured only
/// at the gate has no live plan to correct, and a Drone told to call a tool it
/// was never asked to call goes looking for one.
#[test]
fn a_step_whose_edits_are_not_watched_has_no_drift_notice_to_send() {
    assert_eq!(drift_notice(&watching(false), &["src/lib.rs"]), None);
}

/// Nothing new is nothing to say. The once-per-path rule is
/// `Working::drifting`'s and this rides it rather than keeping a second memory
/// of what a Drone has already been told.
#[test]
fn nothing_seen_outside_the_plan_is_nothing_to_send() {
    assert_eq!(drift_notice(&watching(true), &[]), None);
}

/// A step that declares an artifact, and the turn its Drone is given.
fn turn_delivering(target: &str) -> String {
    let workflow = testkit::frozen(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("facts_note"),
        gates: &[Gate::ArtifactExists { target }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    first_turn(
        &a_job(),
        &workflow,
        &StepId::new("implement"),
        &Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string()
}

/// **The path is named, because a check nobody was told about fails every
/// time.** The step's product is a file now; a Drone that is not told which
/// file does the work and loses it.
#[test]
fn a_step_that_must_write_a_file_is_told_which_file() {
    let said = turn_delivering(".armada/artifacts/plan.md");
    assert!(said.contains(".armada/artifacts/plan.md"), "{said}");
    assert!(said.contains("WHAT THIS PART DELIVERS"), "{said}");
}

/// **The deliverable and the scratch directory are two things and the turn
/// says which.** This is the Job measured on 2026-08-29: the Drone put its
/// plan under `.armada/<job-id>/`, which this repository ignores, so the file
/// never entered the diff and the Judge refused the step for a root cause
/// written on page one of it.
#[test]
fn the_deliverable_is_not_offered_the_scratch_directory() {
    let said = turn_delivering(".armada/artifacts/plan.md");
    let scratch = format!(".armada/{}/", a_job().id().as_str());
    let delivers = said
        .split("WHAT THIS PART DELIVERS")
        .nth(1)
        .expect("the block");

    assert!(
        !delivers.contains(&scratch),
        "the deliverable block points at the scratch path: {delivers}"
    );
    assert!(
        said.contains("not one of them"),
        "the scratch block does not say a deliverable is excluded: {said}"
    );
    // A plan is no longer one of the scratch examples, because on a `plan` step
    // it read as an instruction to file the deliverable out of the Judge's
    // sight.
    assert!(
        !said.contains("A plan, a checklist"),
        "a plan is still offered as scratch: {said}"
    );
}

/// A step that declares no artifact gets no block. **An empty block reads as an
/// answered one**, and every step whose product is the diff would otherwise be
/// asked for a file nothing looks for.
#[test]
fn a_step_that_declares_no_artifact_is_not_asked_for_one() {
    for step in ["implement", "summarise"] {
        let said = turn_at(step);
        assert!(!said.contains("WHAT THIS PART DELIVERS"), "{step}: {said}");
    }
}

/// **The block says the path is the one that is read**, which is the fact a
/// Drone acts on. Fleet opens exactly this path, so nothing goes looking for a
/// file written well somewhere else — "write it here" is not a filing
/// convention and the wording must not read as one.
#[test]
fn the_deliverable_block_says_this_path_is_the_one_that_is_read() {
    let said = turn_delivering(".armada/artifacts/plan.md");
    assert!(
        said.contains("This exact path is the one that is read"),
        "{said}"
    );
    assert!(
        said.contains("an empty file or no file stops this part"),
        "{said}"
    );
    assert!(
        said.contains("a file somewhere else is not this part's work"),
        "{said}"
    );
}
