//! The workflows a case is run against, and the two dials no fixture trips.
//!
//! **Through the parser, never built by hand.** Each fixture below is either a
//! `testkit::Sketch` or the YAML a real `.armada/workflows/` file would hold,
//! and both end at `ResolvedWorkflow::resolve` — a workflow no real file could
//! have produced would prove nothing about the gate that reads it.
//!
//! **[`UNTRIPPABLE`] and [`NEVER_QUIET`] are out of reach on purpose.** A
//! step's norms and a Drone's silence threshold are asserted by the two modules
//! those belong to; everywhere else they must not fire, because a fixture Drone
//! says nothing at all for the whole of a short run and a real one is allowed
//! two minutes of it.

use std::collections::BTreeMap;
use std::time::Duration;

use config::Manifest;
use testkit::{Gate, Sketch};

use crate::converging::StepNorms;
use crate::silence::Liveness;

/// Two steps: one gated on a non-empty diff, one gated on nothing.
///
/// The second is the shape the gate's own comment calls common rather than
/// edge — a step with no checks advances on evidence alone, and two of the four
/// sample workflows lean on it.
pub(super) fn two_steps() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
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

/// The same two steps, with **neither of them sending the work out** — a
/// workflow whose product is read rather than merged, which four of the eight
/// shipped workflows are.
///
/// What it buys the cases that use it is a step boundary with no delivery on
/// it. The delivering step is sent out as it is entered, and on a two-step
/// fixture the one boundary there is *is* that entry — so a case about what a
/// boundary does to a branch would otherwise be reading a commit, a push and a
/// pull request out of the same delta.
pub fn two_steps_delivering_nothing() -> config::ResolvedWorkflow {
    testkit::delivering(
        &[
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[Gate::DiffNonempty],
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
        ],
        None,
    )
}

/// Two steps, each gated on a non-empty diff. What the boundary needs: a
/// second step whose `diff_nonempty` can fail, which `two_steps` cannot express
/// because its second step is gated on nothing.
pub fn two_steps_both_gated_on_a_diff() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "verify",
            label: "Verify",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// The same two steps, with a person on the gate of the one named, and
/// optionally one question to the Judge on that same step.
///
/// **Written as a file rather than through `testkit::Sketch`**, which fixes a
/// step's gate from whether it declares a criterion and so cannot express a
/// human gate at all. It goes through both parsers for the reason every other
/// fixture does: a workflow a real `armada.yml` could not produce would prove
/// nothing about the gate — and a human gate carrying a Judge is exactly the
/// pairing the parser's agreement rule is asked about.
/// `delivers` names the step that sends the work out, and `None` is a workflow
/// that sends nothing — see [`two_steps_delivering_nothing`] for why a case
/// about a boundary wants the second.
pub fn two_steps_gated_on_a_person(
    gate_on: &str,
    question: Option<&str>,
    delivers: Option<&str>,
) -> config::ResolvedWorkflow {
    let gate = |step: &str| match step == gate_on {
        true => "human_always",
        false => "auto",
    };
    let judge = match question {
        None => String::new(),
        // **The check names no model**, and does not need to: what is under
        // test is what a human gate does with a verdict, not which model
        // produced it. It named one until `judge_checks[].model` came under
        // the roster — a vendor's alias in a fixture here, where the roster
        // offers nothing, and a spelling in `fleet` that gate rule six keeps
        // inside `adapters`.
        Some(question) => format!(
            "    judge_checks:\n      - criteria:\n          - \
             criterion_id: c1\n            question: {question}\n"
        ),
    };
    let sends = |step: &str| delivers == Some(step);
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\n\
             steps:\n  - id: implement\n    label: \"Implement\"\n    evidence_type: diff\n    \
             mechanical_checks:\n      - type: diff_nonempty\n{judge}    \
             delivers: {}\n    advance_gate: {}\n  - \
             id: summarise\n    label: \"Summarise\"\n    evidence_type: facts_note\n    \
             delivers: {}\n    advance_gate: {}\n",
            sends("implement"),
            gate("implement"),
            sends("summarise"),
            gate("summarise"),
        ),
        // The fixture names no model, so there is nothing for a roster to
        // offer. See `config::Roster`.
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

/// One workflow, keyed by its own id — what `fittings` holds when a case
/// wants nothing more than the fixture.
pub fn one(
    workflow: config::ResolvedWorkflow,
) -> BTreeMap<core_model::WorkflowId, config::ResolvedWorkflow> {
    let mut held = BTreeMap::new();
    held.insert(workflow.id().clone(), workflow);
    held
}

/// A one-step workflow with its own `workflow_id`, gated on nothing but the
/// evidence arriving — for a case that needs two workflows Fleet can tell
/// apart by id, and cannot tell apart by step id, since each step is named
/// for the workflow that declares it.
pub fn workflow_named(id: &str) -> config::ResolvedWorkflow {
    workflow_named_and(id, false)
}

/// The same, gated on a non-empty diff — for a case that needs a Check that
/// can fail.
pub fn workflow_named_gated_on_diff(id: &str) -> config::ResolvedWorkflow {
    workflow_named_and(id, true)
}

fn workflow_named_and(id: &str, gated: bool) -> config::ResolvedWorkflow {
    let step_id = format!("only_in_{id}");
    let mechanical = if gated {
        "    mechanical_checks:\n      - type: diff_nonempty\n"
    } else {
        ""
    };
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: {id}\nname: {id}\nstructure: linear\nsteps:\n  - id: \
             {step_id}\n    label: \"{step_id}\"\n    evidence_type: diff\n{mechanical}    \
             delivers: true\n    advance_gate: auto\n"
        ),
        // The fixture names no model, so there is nothing for a roster to
        // offer. See `config::Roster`.
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

pub fn manifest() -> Manifest {
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("a manifest that parses")
}

/// Norms no fixture trips. A step's turn count, its wall clock and the grace
/// after a forced report are all put out of reach, so every test but
/// `converging`'s own behaves exactly as it did before the chain existed.
pub const UNTRIPPABLE: StepNorms = StepNorms::of(
    u32::MAX,
    Duration::from_secs(86_400),
    Duration::from_secs(86_400),
);

/// A silence threshold no fixture reaches, for the same reason: every test but
/// `silence`'s own has a Drone that says nothing for the whole of a short run,
/// which is what a real one does for two minutes and no test can wait for.
pub const NEVER_QUIET: Liveness = Liveness::of(Duration::from_secs(86_400), 2);
