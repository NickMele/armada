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
use verification::OutcomeTurn;

use crate::session::Turn;

fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: None,
            gates: &[],
        },
        Sketch {
            id: "review",
            label: "Review",
            evidence_type: None,
            gates: &[],
        },
    ])
}

fn step(of: &ResolvedWorkflow) -> &ResolvedStep {
    of.steps().first().expect("a workflow has a first step")
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
    let outcome = OutcomeTurn::advanced(step(&workflow), workflow.steps().get(1));

    let injected = encoded(&Turn::outcome(&outcome));
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
    let outcome = OutcomeTurn::advanced(step(&workflow), None);
    let injected = encoded(&Turn::outcome(&outcome));

    let added = injected.len() - outcome.text().len();
    assert!(
        added < 80,
        "the envelope is the harness's shape and nothing of Armada's: {injected}"
    );
}
