//! What advances a step, and everything that does not.
//!
//! Split in two because the two halves refuse different things. [`submission`]
//! is about the call a Drone makes — what it may say, and what it has no field
//! to say. [`gate`] is about the decision Fleet then takes, and most of its
//! cases are cases where nothing advances. [`judge`] is the third tier and its
//! cases each name the constitutional rule they hold. [`converging`] is the
//! mid-step look, which gates nothing and holds the same two rules anyway.

mod converging;
mod gaming;
mod gate;
mod judge;
mod submission;

use config::{ResolvedStep, ResolvedWorkflow};
use testkit::{Gate, Sketch};

/// A two-step workflow: one gated on a Check and a non-empty diff, one gated on
/// nothing. Both shapes in one fixture, because the interesting assertions are
/// about the pair.
pub(crate) fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[
                Gate::Check {
                    name: "build",
                    run: "true",
                    expect_exit_code: 0,
                },
                Gate::DiffNonempty,
            ],
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
    ])
}

/// The gated step of [`workflow`].
pub(crate) fn gated(workflow: &ResolvedWorkflow) -> &ResolvedStep {
    &workflow.steps()[0]
}

/// The ungated step of [`workflow`]. Two of the four sample steps look like
/// this, which is why it is a fixture and not a special case.
pub(crate) fn ungated(workflow: &ResolvedWorkflow) -> &ResolvedStep {
    &workflow.steps()[1]
}
