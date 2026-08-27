//! A [`ResolvedWorkflow`] for a test, built through the real parser.
//!
//! # Why this writes YAML rather than constructing the types
//!
//! It cannot construct them: `ResolvedWorkflow` has no constructor but
//! `resolve`, and `resolve` takes a parsed `WorkflowDef` and a parsed
//! `Manifest`. That is the whole point of the type — holding one is proof every
//! Check name resolved — and a fixture that reached around it would let a test
//! assert against a workflow no `armada.yml` could produce.
//!
//! So a fixture is two small documents run through the same two parsers Fleet
//! uses. A test that names a Check gets the Check's command lifted in for real,
//! and a fixture that would be refused at load is refused here too, loudly,
//! instead of quietly becoming a case the gate never sees in production.
//!
//! Neither document is written to disk. Both parsers take the text and a path
//! for the refusals to name, and the path here is a name rather than a file.

use std::collections::BTreeMap;
use std::path::Path;

use config::{Manifest, ResolvedWorkflow, WorkflowDef};
use core_model::FrozenWorkflow;

/// One mechanical check on a fixture step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate<'a> {
    /// A named Check, its command, and the code the step expects. The name is
    /// declared in the fixture's Manifest automatically, so the resolve cannot
    /// fail on a name the test forgot to declare.
    Check {
        name: &'a str,
        run: &'a str,
        expect_exit_code: i64,
    },
    /// The step produced a non-empty diff.
    DiffNonempty,
}

/// One step of a fixture workflow.
///
/// Public fields and no `Default`, so a test writes each one out. `gates` is
/// routinely empty, which is the common shape rather than the edge one.
#[derive(Debug, Clone, Copy)]
pub struct Sketch<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// What the step asks for. `None` is a step that declares no evidence type
    /// and therefore accepts whatever arrives.
    pub evidence_type: Option<&'a str>,
    pub gates: &'a [Gate<'a>],
}

/// Build a workflow and the Manifest its Checks resolve against.
///
/// Panics rather than returning a `Result`: a fixture that does not parse is a
/// mistake in the test, and a test that has to unwrap its own fixture reads as
/// though the parse were the subject.
pub fn resolved(steps: &[Sketch<'_>]) -> ResolvedWorkflow {
    let def = WorkflowDef::parse(Path::new("fixture-workflow.yml"), &workflow_text(steps))
        .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let manifest = Manifest::parse(Path::new("fixture-armada.yml"), &manifest_text(steps))
        .unwrap_or_else(|refused| panic!("the fixture manifest did not parse: {refused}"));
    ResolvedWorkflow::resolve(&def, &manifest)
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

/// The same fixture as a Job would freeze it.
///
/// Still built through both parsers — this is [`resolved`]'s output with the
/// two file paths dropped, which is exactly what creation copies onto a record.
pub fn frozen(steps: &[Sketch<'_>]) -> FrozenWorkflow {
    resolved(steps).frozen().clone()
}

fn workflow_text(steps: &[Sketch<'_>]) -> String {
    let mut text = String::from(
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\nsteps:\n",
    );
    for step in steps {
        text.push_str(&format!(
            "  - id: {}\n    label: \"{}\"\n    advance_gate: auto\n",
            step.id, step.label
        ));
        if let Some(evidence) = step.evidence_type {
            text.push_str(&format!("    evidence_type: {evidence}\n"));
        }
        if step.gates.is_empty() {
            continue;
        }
        text.push_str("    mechanical_checks:\n");
        for gate in step.gates {
            match gate {
                Gate::Check {
                    name,
                    expect_exit_code,
                    ..
                } => text.push_str(&format!(
                    "      - type: manifest_check\n        check: {name}\n        \
                     expect_exit_code: {expect_exit_code}\n"
                )),
                Gate::DiffNonempty => text.push_str("      - type: diff_nonempty\n"),
            }
        }
    }
    text
}

/// Every Check any step named, declared once. A name used twice with two
/// commands is a mistake in the test and the second wins loudly enough to see
/// in the resolved command.
fn manifest_text(steps: &[Sketch<'_>]) -> String {
    let mut declared: BTreeMap<&str, &str> = BTreeMap::new();
    for step in steps {
        for gate in step.gates {
            if let Gate::Check { name, run, .. } = gate {
                declared.insert(name, run);
            }
        }
    }
    let mut text = String::from("version: 1\nid: 01FIXTUREMANIFEST\n");
    if declared.is_empty() {
        return text;
    }
    text.push_str("checks:\n");
    for (name, run) in declared {
        text.push_str(&format!("  {name}:\n    run: \"{run}\"\n"));
    }
    text
}
