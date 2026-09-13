//! A review step's Drone is told what the review it hands in is for, and what Fleet refuses.
//! A step that asks for anything else is told nothing about a review. #903.

use core_model::StepId;
use testkit::Sketch;

use crate::briefing::first_turn;
use crate::crossing::Crossed;
use crate::tests::briefing::a_job;

fn the_brief_at(evidence_type: &str) -> String {
    let workflow = testkit::frozen(&[Sketch {
        id: "handoff",
        label: "Review the change",
        evidence_type: Some(evidence_type),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    first_turn(
        &a_job(),
        &workflow,
        &StepId::new("handoff"),
        &Crossed::nothing(),
    )
    .expect("a brief assembles")
    .as_str()
    .to_string()
}

#[test]
fn a_review_step_tells_its_drone_what_the_review_is_for_and_what_is_refused() {
    let said = the_brief_at("review");
    assert!(said.contains("Armada's review of the"), "{said}");
    assert!(said.contains("submit_evidence's review field"), "{said}");
    assert!(said.contains("every changed file belongs to one"), "{said}");
    assert!(said.contains("@@ header"), "{said}");
    assert!(
        said.contains("Fleet checks the review against the diff"),
        "{said}"
    );
}

#[test]
fn a_step_that_asks_for_a_note_is_told_nothing_about_a_review() {
    let said = the_brief_at("facts_note");
    assert!(!said.contains("submit_evidence's review field"), "{said}");
}
