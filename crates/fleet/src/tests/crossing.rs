//! What a Drone that was not there is handed, and the two silences that are
//! not the same silence.
//!
//! A Drone belongs to a step, so the one on part two never saw part one. The
//! block that tells it what part one produced is drawn in
//! `docs/contracts/agent-prompt.md`, and the reason it went eight months
//! unwritten is in `briefing`'s own header: a block rendered empty reads to a
//! Drone as a block that was answered.
//!
//! So the cases below are mostly absences. There is no earlier part; there is
//! an earlier part and no record of it; there is a record and no file. Only one
//! of the four renders the block the contract draws, and the other three are
//! where the defect lives.
//!
//! The wording is not pinned line by line — `crate::tests::briefing` says why.
//! What is pinned is which facts reach a Drone and which do not.

use core_model::{EvidenceType, FrozenWorkflow, StepEvidence, StepId};
use testkit::{Gate, Sketch};

use crate::briefing::first_turn;
use crate::crossing::{Cleared, Crossed, Produced};
use crate::tests::briefing::a_job;

const CLAIMED: &str = "The reader's bound is inclusive where the caller expects exclusive.";

/// Two parts, and the first one writes a file. The Bug workflow's shape after
/// `#138`: a step another step reads from declares an `artifact_exists` naming
/// what it wrote.
fn note_then_fix() -> FrozenWorkflow {
    testkit::frozen(&[
        Sketch {
            id: "root_cause",
            label: "Find the cause",
            evidence_type: Some("facts_note"),
            gates: &[Gate::ArtifactExists {
                target: ".armada/artifacts/root_cause.md",
            }],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "fix",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// The same two parts, where the first one's product is the diff and it writes
/// no file of its own. Most steps are this shape.
fn fix_then_summarise() -> FrozenWorkflow {
    testkit::frozen(&[
        Sketch {
            id: "root_cause",
            label: "Find the cause",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "fix",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

fn recorded(step: &str) -> Vec<(StepId, StepEvidence)> {
    vec![(
        StepId::new(step),
        StepEvidence {
            evidence_type: EvidenceType::FactsNote,
            claimed: String::from(CLAIMED),
            shown_by: String::from("`read_to` stops at `end` rather than before it — read.rs:41"),
            not_claimed: String::from("The writer has the same bound and is untouched."),
        },
    )]
}

/// The turn a Drone standing at `at` gets, carrying `crossed`.
fn turn(workflow: &FrozenWorkflow, at: &str, crossed: Crossed) -> String {
    first_turn(&a_job(), workflow, &StepId::new(at), &crossed)
        .expect("a prompt")
        .as_str()
        .to_string()
}

/// Everything the boundary hands across, assembled the way a caller does.
fn crossing(workflow: &FrozenWorkflow, at: &str, recorded: &[(StepId, StepEvidence)]) -> Crossed {
    Crossed::nothing().and_produced(Produced::before(workflow, &StepId::new(at), recorded))
}

// --------------------------------------------- what the part before produced

/// **The claim in the contract's own shape.** `docs/contracts/agent-prompt.md`
/// and `docs/concepts/drone.md` both draw `What part 1 produced:` with the
/// claim quoted under it, and this is that block rendered from the record
/// rather than from a fixture's idea of it.
#[test]
fn a_drone_that_did_not_see_the_part_before_is_told_what_it_claimed() {
    let workflow = note_then_fix();
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")),
    );

    assert!(said.contains("What part 1 produced"), "{said}");
    assert!(said.contains(CLAIMED), "{said}");
}

/// **Quoted *and* pointed at, which is the whole of what `#138` bought.** The
/// claim is a sentence a Drone typed; the file is what was checked. A brief
/// that only quoted would hand the next Drone a claim about a document, which
/// is the thing `#138` closed.
#[test]
fn the_file_the_part_before_wrote_is_named_as_well_as_quoted() {
    let workflow = note_then_fix();
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")),
    );

    assert!(said.contains(".armada/artifacts/root_cause.md"), "{said}");
    assert!(
        said.contains("summarises it and does not replace it"),
        "the quotation must not read as the whole of it: {said}"
    );
}

/// **A path is only named where the definition names one.** Most steps produce
/// a diff and no file, and a brief inventing a path would send a Drone looking
/// for something nothing wrote.
#[test]
fn a_part_that_wrote_no_file_points_at_the_branch_instead() {
    let workflow = fix_then_summarise();
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")),
    );

    assert!(said.contains(CLAIMED), "{said}");
    assert!(
        said.contains("Its work is on the branch you are in"),
        "{said}"
    );
    assert!(!said.contains(".armada/artifacts/"), "{said}");
}

/// **`shown_by` does not cross.** `#138` is explicit that the brief points at a
/// path Fleet resolved rather than at a string the previous Drone typed, and
/// `shown_by` is that string.
#[test]
fn what_the_part_before_said_about_its_own_artifact_does_not_cross() {
    let workflow = note_then_fix();
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")),
    );

    assert!(!said.contains("read.rs:41"), "{said}");
}

// ------------------------------------------------- the two different silences

/// **The first part renders no block, and that is not an empty block.** The
/// rail directly above says "You are on part 1" and marks nothing done, so
/// there is nothing an absence here could be read as answering.
#[test]
fn the_first_part_of_a_job_is_told_nothing_about_a_part_before_it() {
    let workflow = note_then_fix();
    let said = turn(
        &workflow,
        "root_cause",
        crossing(&workflow, "root_cause", &[]),
    );

    assert!(!said.contains("produced"), "{said}");
    assert!(said.contains("You are on part 1"), "{said}");
}

/// **The second silence is the one that misleads, so it is spoken.** A step
/// that was overridden forward has a rail entry marked done and no evidence
/// behind it. Said rather than left out, which is what
/// `verification::GamingBrief` does with the same absence on the Judge's side.
#[test]
fn a_part_before_that_recorded_nothing_says_so_rather_than_rendering_blank() {
    let workflow = note_then_fix();
    let said = turn(&workflow, "fix", crossing(&workflow, "fix", &[]));

    assert!(said.contains("What part 1 produced"), "{said}");
    assert!(
        said.contains("There is no record of what it claimed"),
        "{said}"
    );
    assert!(
        said.contains(".armada/artifacts/root_cause.md"),
        "the file it was asked for is still where to look: {said}"
    );
}

// ------------------------------------------ the verdict that advanced the step

/// **The gate's outcome does not survive being moved, so it is re-tensed.**
/// `OutcomeTurn::advanced` says "Go on to Implement", which is a continuation
/// addressed to the Drone that did the work. This Drone is not continuing, and
/// what it needs from the verdict is that the part is closed.
#[test]
fn a_part_the_checks_cleared_is_handed_over_as_closed_and_not_as_a_continuation() {
    let workflow = note_then_fix();
    let passed = &workflow.steps()[0];
    let said = turn(
        &workflow,
        "fix",
        Crossed::nothing().and_cleared(Cleared::checked(passed)),
    );

    assert!(said.contains("THE PART BEFORE THIS ONE"), "{said}");
    assert!(said.contains("passed the checks that gate it"), "{said}");
    assert!(said.contains("not yours to do again"), "{said}");
    assert!(
        !said.contains("Go on to"),
        "a fresh Drone is not going on to anything: {said}"
    );
}

/// A human gate is a person deciding, and the block says that instead of
/// claiming a check passed that nobody ran — which is the distinction
/// `OutcomeTurn::approved` exists to keep.
#[test]
fn a_part_a_person_took_says_a_person_took_it() {
    let workflow = note_then_fix();
    let passed = &workflow.steps()[0];
    let said = turn(
        &workflow,
        "fix",
        Crossed::nothing().and_cleared(Cleared::reviewed(passed)),
    );

    assert!(said.contains("was read by a person and accepted"), "{said}");
    assert!(!said.contains("passed the checks"), "{said}");
}

/// **No count, on the one turn most likely to grow one.** The rule
/// `verification::OutcomeTurn` carries about attempts holds here: a Drone given
/// an arithmetic has an incentive to satisfy it.
#[test]
fn nothing_that_crosses_a_boundary_is_a_number_a_drone_could_be_judged_on() {
    let workflow = note_then_fix();
    let passed = &workflow.steps()[0];
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")).and_cleared(Cleared::checked(passed)),
    );

    assert!(!said.contains("attempt"), "{said}");
    assert!(!said.contains("skipped"), "{said}");
}

// ------------------------------------------------ what carries and what does not

/// **A boundary that carries nothing renders nothing**, which is what every
/// spawn does until `#140` teaches it otherwise. The Drone gets exactly the
/// brief it got before this module existed.
#[test]
fn a_boundary_carrying_nothing_adds_no_block_at_all() {
    let workflow = note_then_fix();
    let bare = turn(&workflow, "fix", Crossed::nothing());

    assert!(!bare.contains("What part 1 produced"), "{bare}");
    assert!(!bare.contains("THE PART BEFORE THIS ONE"), "{bare}");
}

/// **Two carried items compose, and a third will.** `#207` adds a redirect that
/// arrived while no Drone was there, and it adds a method rather than reshaping
/// the value every caller passes.
#[test]
fn the_two_things_a_boundary_carries_are_both_in_one_turn() {
    let workflow = note_then_fix();
    let passed = &workflow.steps()[0];
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")).and_cleared(Cleared::checked(passed)),
    );

    assert!(said.contains("What part 1 produced"), "{said}");
    assert!(said.contains("THE PART BEFORE THIS ONE"), "{said}");
    assert!(
        said.find("What part 1 produced") < said.find("THE PART BEFORE THIS ONE"),
        "the rail establishes that a part before exists before anything says it is closed: {said}"
    );
    assert!(
        said.find("THE PART BEFORE THIS ONE") < said.find("STEP: Implement"),
        "and both precede the part this Drone is actually being asked to do: {said}"
    );
}

/// **A step that is not in the workflow reaches nothing.** There is no way
/// through this type to a later step's evidence either: the only step it
/// resolves is the one immediately before, by index.
#[test]
fn a_step_the_workflow_does_not_declare_produces_no_block() {
    let workflow = note_then_fix();
    assert_eq!(
        Produced::before(&workflow, &StepId::new("nowhere"), &recorded("root_cause")),
        None,
    );
}
