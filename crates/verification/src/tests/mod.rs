//! What advances a step, and everything that does not.
//!
//! Split in two because the two halves refuse different things. [`submission`]
//! is about the call a Drone makes — what it may say, and what it has no field
//! to say. [`gate`] is about the decision Fleet then takes, and most of its
//! cases are cases where nothing advances. [`judge`] is the third tier and its
//! cases each name the constitutional rule they hold. [`product`] is what a
//! Judge is shown on a step whose work product is not a diff, which is the one
//! place a Drone's own writing reaches it. [`request`] is rule 2's other half —
//! what the Judge is measured *against* — and its cases are written against the
//! two designed criteria that were dropped for wanting it. [`converging`] is the
//! mid-step look, which gates nothing and holds the same two rules anyway.
//! [`quoted`] is the reading under a citation — whether the words a refusal
//! puts in quotation marks are in what the call was shown, or in nothing.
//! [`answered`] is the Check tier of a brief — the two outcomes a Judge can
//! actually be shown, and the bound on how much of a run travels with them.

mod answered;
mod converging;
mod gaming;
mod gate;
mod judge;
mod product;
mod quoted;
mod request;
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
                    when: &[],
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

/// A one-step workflow whose only Check declares which paths it covers.
///
/// It is a second fixture rather than a third gate on [`workflow`] because the
/// interesting assertion is about a step where *every* Check is skippable —
/// which is the step that advances having verified nothing.
pub(crate) fn scoped() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "storybook",
            run: "true",
            expect_exit_code: 0,
            when: &["packages/**"],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}
