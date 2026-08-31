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
        /// Which paths the fixture's Manifest says this Check covers. **Empty
        /// writes no `when:` key at all**, which is the Check that always runs
        /// and the shape every fixture written before `when` existed has.
        when: &'a [&'a str],
    },
    /// The step produced a non-empty diff.
    DiffNonempty,
    /// The step wrote the file at this worktree-relative path.
    ArtifactExists { target: &'a str },
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
    /// The narrow questions the step puts to the Judge, as `(criterion_id,
    /// question)`. Empty is the common case.
    ///
    /// A step with any of them is written with `advance_gate:
    /// auto_if_judge_passes`, because the parser refuses a file where the gate
    /// and the criteria disagree — the fixture cannot produce a combination an
    /// `armada.yml` could not.
    pub judged_on: &'a [(&'a str, &'a str)],
    /// What the step's evidence is scoped to. `None` is a step declaring no
    /// `evidence_scope`, which is the common shape and the one every fixture
    /// written before scopes existed has.
    pub scope: Option<Scoped<'a>>,
    /// The second look the step declares. `None` is a step that asks whether
    /// its evidence satisfies the criteria and never whether it was gamed,
    /// which is every step written before the gaming check existed.
    pub gaming: Option<Gaming<'a>>,
}

/// A step's `gaming_check`, as a fixture writes it.
///
/// **`flag_if` is written as the wire values a file carries**, not as the enum,
/// so a fixture naming a pattern the parser does not know is refused here the
/// way an `armada.yml` would be.
#[derive(Debug, Clone, Copy)]
pub struct Gaming<'a> {
    /// `<step_id>.evidence`, or `None` for a check with no baseline.
    pub baseline: Option<&'a str>,
    pub flag_if: &'a [&'a str],
}

/// A step's `evidence_scope`, as a fixture writes it.
///
/// `context_source` is fixed at `drone_declared` because it is the only value
/// a scope check is about: the paths the Manifest could default are not the
/// ones a Drone is measured against.
#[derive(Debug, Clone, Copy)]
pub struct Scoped<'a> {
    /// Whether the footprint is checked against the declaration at the gate.
    pub diff_check: bool,
    /// Whether the plan is declared at step start, which is what makes the
    /// live check possible.
    pub at_step_start: bool,
    pub exclude: &'a [&'a str],
    /// What this step's work is measured against, as `<step_id>.evidence`
    /// entries. Empty is the common shape — most steps are judged on their own
    /// product and nothing else.
    pub references: &'a [&'a str],
}

/// Build a workflow and the Manifest its Checks resolve against.
///
/// Panics rather than returning a `Result`: a fixture that does not parse is a
/// mistake in the test, and a test that has to unwrap its own fixture reads as
/// though the parse were the subject.
pub fn resolved(steps: &[Sketch<'_>]) -> ResolvedWorkflow {
    retried(steps, 0)
}

/// The same fixture with every step declaring the same retry budget.
///
/// **A whole-fixture argument rather than a seventh field on [`Sketch`]**, and
/// that is not laziness about the sixty-odd literals it would touch. A budget
/// is what a test about retries is about; on every other fixture it is noise,
/// and a field would make each of those state a number it does not care about.
/// A test needing two steps with two different budgets does not exist, and
/// would be the moment to make it a field.
pub fn retried(steps: &[Sketch<'_>], retry_limit: u32) -> ResolvedWorkflow {
    let def = WorkflowDef::parse(
        Path::new("fixture-workflow.yml"),
        &workflow_text(steps, retry_limit),
    )
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

fn workflow_text(steps: &[Sketch<'_>], retry_limit: u32) -> String {
    let mut text = String::from(
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\nsteps:\n",
    );
    for step in steps {
        let gate = match step.judged_on.is_empty() {
            true => "auto",
            false => "auto_if_judge_passes",
        };
        text.push_str(&format!(
            "  - id: {}\n    label: \"{}\"\n    advance_gate: {gate}\n    \
             retry_limit: {retry_limit}\n",
            step.id, step.label
        ));
        if let Some(evidence) = step.evidence_type {
            text.push_str(&format!("    evidence_type: {evidence}\n"));
        }
        if let Some(scope) = step.scope {
            if scope.at_step_start {
                text.push_str("    declare_plan_at: step_start\n");
            }
            text.push_str("    evidence_scope:\n      context_source: drone_declared\n");
            text.push_str(&format!("      scope_diff_check: {}\n", scope.diff_check));
            if !scope.exclude.is_empty() {
                text.push_str("      exclude_paths:\n");
                for path in scope.exclude {
                    text.push_str(&format!("        - \"{path}\"\n"));
                }
            }
            if !scope.references.is_empty() {
                text.push_str("      reference_docs:\n");
                for reference in scope.references {
                    text.push_str(&format!("        - \"{reference}\"\n"));
                }
            }
        }
        if !step.judged_on.is_empty() || step.gaming.is_some() {
            text.push_str("    judge_checks:\n      -\n");
        }
        if !step.judged_on.is_empty() {
            text.push_str("        criteria:\n");
            for (id, question) in step.judged_on {
                text.push_str(&format!(
                    "          - criterion_id: {id}\n            question: \"{question}\"\n"
                ));
            }
        }
        if let Some(gaming) = step.gaming {
            text.push_str("        gaming_check:\n");
            if let Some(baseline) = gaming.baseline {
                text.push_str(&format!("          baseline_ref: \"{baseline}\"\n"));
            }
            text.push_str("          flag_if:\n");
            for pattern in gaming.flag_if {
                text.push_str(&format!("            - {pattern}\n"));
            }
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
                Gate::ArtifactExists { target } => text.push_str(&format!(
                    "      - type: artifact_exists\n        target: \"{target}\"\n"
                )),
            }
        }
    }
    text
}

/// Every Check any step named, declared once. A name used twice with two
/// commands is a mistake in the test and the second wins loudly enough to see
/// in the resolved command.
fn manifest_text(steps: &[Sketch<'_>]) -> String {
    let mut declared: BTreeMap<&str, (&str, &[&str])> = BTreeMap::new();
    for step in steps {
        for gate in step.gates {
            if let Gate::Check {
                name, run, when, ..
            } = gate
            {
                declared.insert(name, (run, when));
            }
        }
    }
    let mut text = String::from("version: 1\nid: 01FIXTUREMANIFEST\n");
    if declared.is_empty() {
        return text;
    }
    text.push_str("checks:\n");
    for (name, (run, when)) in declared {
        text.push_str(&format!("  {name}:\n    run: \"{run}\"\n"));
        // No key at all where the fixture declares no path, because that is
        // what an `armada.yml` without a `when` looks like — and an empty list
        // is refused by the parser this fixture runs through.
        if !when.is_empty() {
            let quoted: Vec<String> = when.iter().map(|p| format!("\"{p}\"")).collect();
            text.push_str(&format!("    when: [{}]\n", quoted.join(", ")));
        }
    }
    text
}
