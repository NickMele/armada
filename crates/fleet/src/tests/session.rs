//! What goes down the pipe to a live Drone, and what comes back when it is
//! gone.
//!
//! The turn's shape is the harness's, measured in spike 4: one JSON object per
//! line, `{"type":"user","message":{"role":"user","content":…}}`, with stdin
//! held open. It is asserted here rather than only at the far end, because a
//! turn that is one byte wrong is a Drone that never hears from Fleet again and
//! a Job that escalates as stalled for a reason nobody can see.

use config::{ResolvedStep, ResolvedWorkflow};
use testkit::Sketch;
use verification::{OutcomeTurn, Ran, Verified};

use crate::briefing::Declaring;
use crate::session::Turn;

fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: None,
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "review",
            label: "Review",
            evidence_type: None,
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

fn step(of: &ResolvedWorkflow) -> &ResolvedStep {
    of.steps().first().expect("a workflow has a first step")
}

/// The mechanical tier having run everything the step declared, which for these
/// fixtures is nothing at all. Built from a real `Ran` rather than from two
/// numbers, because `Verified` has no constructor that takes them — the counts
/// are read off the set a step actually produced.
fn every_check_ran(of: &ResolvedWorkflow) -> Verified {
    Verified::of(&Ran::of(step(of), &[]).expect("a step declaring no check"))
}

fn encoded(turn: &Turn) -> String {
    ipc::encode(turn).expect("a turn is plain data and always encodes")
}

#[test]
fn a_turn_is_one_line_of_json_and_nothing_else() {
    let line = encoded(&Turn::first("do the work"));
    assert!(!line.contains('\n'), "a turn is one line: {line}");
    assert!(line.starts_with(r#"{"type":"user""#), "{line}");
    assert!(line.contains(r#""role":"user""#), "{line}");
    assert!(line.contains("do the work"), "{line}");
}

#[test]
fn a_turn_carrying_a_newline_stays_one_line() {
    // A six-layer prompt is full of them, and a turn broken across two lines is
    // a turn the harness reads as two malformed objects.
    let line = encoded(&Turn::first("first layer\n\nsecond layer"));
    assert!(!line.contains('\n'), "{line}");
    assert!(line.contains("\\n"), "the newline is escaped, not dropped");
}

#[test]
fn the_gate_s_outcome_and_the_first_turn_take_the_same_shape() {
    // One path, not two. The channel that carries the task is the channel that
    // carries what the gate decided, so there is no second encoding to keep
    // correct — and no second one to get wrong on a Job that has already run.
    let workflow = workflow();
    let outcome = OutcomeTurn::advanced(
        step(&workflow),
        workflow.steps().get(1),
        every_check_ran(&workflow),
    );

    let injected = encoded(&Turn::outcome(&outcome, None));
    assert!(injected.starts_with(r#"{"type":"user""#), "{injected}");

    // The first line rather than the whole text: an outcome turn is several
    // paragraphs and every break in it is escaped on the way down the pipe,
    // which is the property the case above is about.
    let first_line = outcome
        .text()
        .lines()
        .next()
        .expect("an outcome says something");
    assert!(injected.contains(first_line), "{injected}");
}

#[test]
fn a_turn_carries_no_counter_and_no_check_name() {
    // The Drone is never told what the Checks are, and never told how many
    // attempts are left: a counter is a bar, and a Drone one attempt from
    // escalation has the strongest possible incentive to satisfy the bar rather
    // than do the work. Whatever `OutcomeTurn` says, this is the shape of the
    // envelope around it and it adds nothing.
    let workflow = workflow();
    let outcome = OutcomeTurn::advanced(step(&workflow), None, every_check_ran(&workflow));
    let injected = encoded(&Turn::outcome(&outcome, None));

    let added = injected.len() - outcome.text().len();
    assert!(
        added < 80,
        "the envelope is the harness's shape and nothing of Armada's: {injected}"
    );
}

/// **One boundary, one turn.** The step being started asks for a plan, and the
/// ask travels with the verdict rather than as a second injected message —
/// which would spend a second turn boundary to deliver the half the Drone acts
/// on first.
#[test]
fn the_ask_the_next_step_makes_rides_on_the_same_turn_as_the_verdict() {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: None,
        gates: &[],
        judged_on: &[],
        scope: Some(testkit::Scoped {
            diff_check: true,
            at_step_start: true,
            exclude: &[],
            references: &[],
        }),
        gaming: None,
    }]);
    let next = step(&workflow);
    let asked = Declaring::at(next).expect("a scoped step asks");
    let outcome = OutcomeTurn::advanced(next, Some(next), every_check_ran(&workflow));

    let injected = encoded(&Turn::outcome(&outcome, Some(&asked)));
    assert!(!injected.contains('\n'), "still one line: {injected}");
    assert!(injected.contains("BEFORE YOU START"), "{injected}");
    assert!(
        injected.contains("is verified"),
        "the verdict is still there: {injected}"
    );
}

/// A step that asks for nothing leaves the turn byte for byte what it was. The
/// cold switch reaches the pipe as well as the prompt.
#[test]
fn a_step_that_asks_for_no_plan_leaves_the_outcome_turn_alone() {
    let workflow = workflow();
    let outcome = OutcomeTurn::advanced(
        step(&workflow),
        workflow.steps().get(1),
        every_check_ran(&workflow),
    );
    let none = workflow.steps().get(1).expect("a second step");
    assert_eq!(Declaring::at(none), None, "the fixture asks for nothing");
    assert_eq!(
        encoded(&Turn::outcome(&outcome, Declaring::at(none).as_ref())),
        encoded(&Turn::outcome(&outcome, None))
    );
}
