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
//!
//! The last group drives a real step boundary and reads the brief off the
//! transcript. What is in doubt there is not the rendering, which the cases
//! above cover, but that the record survives a process ending and reaches the
//! Drone that starts the next part.

use crate::evidence::Call;
use core_model::{EvidenceType, FrozenWorkflow, StepEvidence, StepId};
use testkit::{Gate, Sketch};
use verification::NotClaimed;

use crate::briefing::first_turn;
use crate::crossing::{Cleared, Crossed, Produced};
use crate::tests::briefing::a_job;
use crate::tests::briefing::{a_diff_call, told_across_the_boundary};
use crate::tests::tmp::TempDir;

const CLAIMED: &str = "The reader's bound is inclusive where the caller expects exclusive.";
const LEFT_ALONE: &str = "The writer has the same bound and is untouched.";

/// Two parts, the second of which is the one every case below stands on.
///
/// **`writes` is the only difference between the two shapes**, and it is a
/// parameter rather than a second literal so that the difference is the thing a
/// reader sees. A first part declaring an `artifact_exists` is the Bug
/// workflow's shape after `#138`; a first part declaring none is most steps,
/// whose product is the diff.
fn two_parts(writes: &[Gate]) -> FrozenWorkflow {
    testkit::frozen(&[
        Sketch {
            id: "root_cause",
            label: "Find the cause",
            evidence_type: Some("facts_note"),
            gates: writes,
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

fn note_then_fix() -> FrozenWorkflow {
    two_parts(&[Gate::ArtifactExists {
        target: ".armada/artifacts/root_cause.md",
    }])
}

fn fix_then_summarise() -> FrozenWorkflow {
    two_parts(&[])
}

fn recorded(step: &str) -> Vec<(StepId, StepEvidence)> {
    recorded_leaving(step, LEFT_ALONE)
}

/// The same record, with what the earlier part said its claim does not cover.
///
/// **A parameter because empty is a case and not an oversight.** A Drone that
/// left nothing behind writes nothing here, which `docs/contracts/agent-copy.md`
/// makes legal, and the two are rendered differently.
fn recorded_leaving(step: &str, left_alone: &str) -> Vec<(StepId, StepEvidence)> {
    vec![(
        StepId::new(step),
        StepEvidence {
            evidence_type: EvidenceType::FactsNote,
            claimed: String::from(CLAIMED),
            shown_by: String::from("`read_to` stops at `end` rather than before it — read.rs:41"),
            not_claimed: String::from(left_alone),
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

// -------------------------------------------- and what it deliberately did not

/// **The one field the deliverable cannot supply, which is why it crosses.**
/// The owner ruled on 31 Aug 2026. `claimed` summarises a file sitting in this
/// worktree and the block names the path, so a Drone that wants the whole of it
/// opens it. `not_claimed` is nowhere but here.
#[test]
fn what_the_part_before_left_alone_crosses_because_nothing_else_holds_it() {
    let workflow = note_then_fix();
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")),
    );

    assert!(said.contains("What part 1 did not claim"), "{said}");
    assert!(said.contains(LEFT_ALONE), "{said}");
}

/// **It must not read as a to-do list**, which is the whole risk of carrying
/// it. A gap the earlier part left on purpose is the thing a later part is
/// likeliest to go and close unasked — and a part doing the next part's work is
/// what the field exists to prevent, arriving by the back door.
#[test]
fn what_was_left_alone_is_framed_as_context_and_never_as_work() {
    let workflow = note_then_fix();
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")),
    );

    assert!(said.contains("not a list of work this part owes"), "{said}");
    assert!(
        said.contains("a gap it left on purpose, or something it changed that nobody asked for"),
        "both kinds of thing the field holds, so neither is read as the other: {said}"
    );
}

/// **The quotation follows the file it is about**, so that "what is quoted
/// above summarises it" can only mean the claim. A second quotation between the
/// two would make that sentence ambiguous about which one it refers to.
#[test]
fn what_was_left_alone_comes_after_the_file_the_claim_summarises() {
    let workflow = note_then_fix();
    let said = turn(
        &workflow,
        "fix",
        crossing(&workflow, "fix", &recorded("root_cause")),
    );

    assert!(
        said.find("summarises it and does not replace it") < said.find("What part 1 did not claim"),
        "{said}"
    );
}

/// **Empty is legal and absent is not** — `docs/contracts/agent-copy.md` says
/// so of the field itself, and this is the same rule one level down. A label
/// with nothing under it turns a part that left nothing behind into a part that
/// declined to answer.
#[test]
fn a_part_that_left_nothing_behind_gets_no_label_with_nothing_under_it() {
    let workflow = note_then_fix();
    for left_alone in ["", "   \n  "] {
        let said = turn(
            &workflow,
            "fix",
            crossing(
                &workflow,
                "fix",
                &recorded_leaving("root_cause", left_alone),
            ),
        );

        assert!(said.contains(CLAIMED), "the claim still crosses: {said}");
        assert!(!said.contains("did not claim"), "{said}");
        assert!(!said.contains("not a list of work"), "{said}");
    }
}

/// A part whose evidence is not on the record at all has no claim and nothing
/// left alone, and neither is invented for it.
#[test]
fn a_part_before_that_recorded_nothing_left_nothing_alone_either() {
    let workflow = note_then_fix();
    let said = turn(&workflow, "fix", crossing(&workflow, "fix", &[]));

    assert!(!said.contains("did not claim"), "{said}");
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

// ------------------------------------ and the same thing across a real boundary

/// The same submission by a Drone that left something behind and said so.
///
/// **A second fixture rather than a parameter on the first**, because an empty
/// `not_claimed` is a case the boundary has to render differently rather than a
/// value a caller happens to pass. `docs/contracts/agent-copy.md` makes empty
/// legal, and the two fixtures are the two states.
fn a_diff_call_leaving<'a>(left_alone: &'a str) -> Call<'a> {
    Call {
        not_claimed: NotClaimed(left_alone),
        ..a_diff_call()
    }
}

/// **The one field a fresh Drone cannot get any other way, across a real
/// boundary.** Part one wrote a file and the brief names it, so everything
/// `claimed` summarises is a `cat` away. What part one decided to leave alone
/// is not in that file and is not in the diff — it exists in the submission and
/// nowhere else, and the process that held it has ended.
///
/// Read off the transcript rather than off the assembled block. What is in
/// doubt is not that `crossing` renders it — `crate::tests::crossing` asserts
/// that — but that the record survives the boundary and reaches the far end of
/// the pipe.
#[tokio::test]
async fn a_step_boundary_carries_what_the_part_before_deliberately_left_alone() {
    let home = TempDir::new();
    let left_alone = "The writer has the same bound and is untouched.";
    let (_fleet, sent) =
        told_across_the_boundary(&home, true, a_diff_call_leaving(left_alone)).await;

    assert!(
        sent[0].contains("What part 1 did not claim"),
        "the Drone on part 2 was not told what part 1 left alone: {}",
        sent[0]
    );
    assert!(
        sent[0].contains(left_alone),
        "and not told it in part 1's own words: {}",
        sent[0]
    );
    assert!(
        sent[0].contains("not a list of work this part owes"),
        "a Drone that reads it as a to-do list does the next part's work: {}",
        sent[0]
    );
}

/// **And a part that left nothing behind is not a part that declined to
/// answer.** `a_diff_call` submits an empty `not_claimed`, which
/// `docs/contracts/agent-copy.md` makes legal, and a label with nothing under
/// it would report the first of those as the second.
#[tokio::test]
async fn a_step_boundary_carrying_nothing_left_alone_renders_no_label_for_it() {
    let home = TempDir::new();
    let (_fleet, sent) = told_across_the_boundary(&home, true, a_diff_call()).await;

    assert!(
        sent[0].contains("What part 1 produced"),
        "the claim still crosses: {}",
        sent[0]
    );
    assert!(
        !sent[0].contains("did not claim"),
        "and nothing is opened that has nothing under it: {}",
        sent[0]
    );
}
