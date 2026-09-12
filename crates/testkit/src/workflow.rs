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

use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::FrozenWorkflow;

/// One mechanical check on a fixture step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate<'a> {
    /// A named Check, its command, and the code a passing run of it exits
    /// with. The name is declared in the fixture's Manifest automatically, so
    /// the resolve cannot fail on a name the test forgot to declare.
    Check {
        name: &'a str,
        run: &'a str,
        /// **Written onto the Manifest's Check, not onto the step that names
        /// it.** One field here because a fixture states one fact; where that
        /// fact is written is what moved, and the tests that expect a non-zero
        /// code — a signalled run, a deliberately failing reproduction — were
        /// what proved the two spellings could not both stand.
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

/// How patient one step is: how long its Drone may say nothing before Fleet
/// pokes it, and how many nudges it gets.
///
/// **Per step rather than per fixture**, which is where this parts company with
/// [`retried`]. That note says a whole-fixture argument holds until a test
/// wants two steps with two different values — and that test is exactly what a
/// per-step patience is for, because one number for every kind of work is the
/// thing `#60` ended.
///
/// **Two `Option`s and not two numbers.** `None` writes no key at all, which is
/// the step deferring to what Fleet is running with; the halves fall back
/// separately, so a fixture can say one and leave the other alone.
#[derive(Debug, Clone, Copy)]
pub struct Patience<'a> {
    pub step: &'a str,
    pub quiet_after_seconds: Option<u32>,
    pub poke_limit: Option<u32>,
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
    built(steps, 0, &[])
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
    built(steps, retry_limit, &[])
}

/// The same fixture with some steps naming a model of their own, as
/// `(step id, model)`.
///
/// **A whole-fixture argument rather than a ninth field on [`Sketch`]**, for
/// [`retried`]'s reason: ninety-odd fixture literals would each have to state a
/// `None` about a dial they do not use. A step absent from `models` declares
/// none, which is what lets one fixture show the annotated step and the one
/// falling back to the Job's side by side — the only thing worth asserting
/// about the dial.
///
/// The roster the parser checks against is built from these pairs, so a fixture
/// cannot name a model the machine would refuse. Whether an unknown one *is*
/// refused is `config`'s own test, over the real parser and a real roster.
pub fn modelled(steps: &[Sketch<'_>], models: &[(&str, &str)]) -> ResolvedWorkflow {
    built(steps, 0, models)
}

/// The same fixture with a Commands registry, and some Checks requiring from
/// it.
///
/// `commands` is `(name, run)`; `requires` is `(check name, [command names])`
/// and the inner order is the order the Manifest writes, which is the order
/// Fleet runs.
///
/// **Whole-fixture arguments rather than a fifth field on [`Gate::Check`]**,
/// for [`retried`]'s reason and more sharply: sixty-odd `Gate::Check` literals
/// would each have to state an empty list about a key they do not use. A
/// prerequisite is the whole subject of the handful of tests that want one and
/// is noise on every other.
pub fn requiring(
    steps: &[Sketch<'_>],
    commands: &[(&str, &str)],
    requires: &[(&str, &[&str])],
) -> ResolvedWorkflow {
    assembled(
        steps,
        0,
        &[],
        commands,
        requires,
        &[],
        &[],
        Sends::TheLastStep,
        Held::NoStep,
    )
}

/// The same fixture with some steps saying how long their own Drone may be
/// quiet, and how often it is nudged. A step absent from `patience` declares
/// neither, which is every other fixture in the workspace.
pub fn patient(steps: &[Sketch<'_>], patience: &[Patience<'_>]) -> ResolvedWorkflow {
    assembled(
        steps,
        0,
        &[],
        &[],
        &[],
        patience,
        &[],
        Sends::TheLastStep,
        Held::NoStep,
    )
}

/// How a named Check is run against a subset of the tree, as the fixture's
/// Manifest writes it.
///
/// **A whole-fixture argument rather than a fifth field on [`Gate::Check`]**,
/// for [`requiring`]'s reason: a narrowing is the whole subject of the handful
/// of tests that want one and is noise on every other, and a Check absent from
/// the list writes no `narrow:` at all — which is the Check that runs whole.
#[derive(Debug, Clone, Copy)]
pub struct Narrows<'a> {
    /// The Check this narrowing belongs to.
    pub check: &'a str,
    /// The command a narrowed run starts from.
    pub run: &'a str,
    /// How one value is spelled as an argument. `{}` is the value.
    pub each: &'a str,
    /// Which changed paths feed it. Empty writes no `from:` key.
    pub from: &'a [&'a str],
    /// The directory whose child names the value. `None` writes no `under:`
    /// key, which is the verbatim case.
    pub under: Option<&'a str>,
    /// Values the narrowing never produces. Empty writes no `except:` key.
    pub except: &'a [&'a str],
}

/// The same fixture with some of its Checks declaring how they narrow.
pub fn narrowing(steps: &[Sketch<'_>], narrows: &[Narrows<'_>]) -> ResolvedWorkflow {
    assembled(
        steps,
        0,
        &[],
        &[],
        &[],
        &[],
        narrows,
        Sends::TheLastStep,
        Held::NoStep,
    )
}

fn built(steps: &[Sketch<'_>], retry_limit: u32, models: &[(&str, &str)]) -> ResolvedWorkflow {
    assembled(
        steps,
        retry_limit,
        models,
        &[],
        &[],
        &[],
        &[],
        Sends::TheLastStep,
        Held::NoStep,
    )
}

/// Which step of a fixture sends the work out.
///
/// **Not `testkit::Delivering`**, which is the script a `FakeVcs` answers a
/// push and a rebase from. This is the workflow's own declaration.
///
/// **A whole-fixture argument rather than a field on [`Sketch`]**, for
/// [`retried`]'s reason, and with a second: `TheLastStep` is what every fixture
/// in the workspace was written under, when the last step's advance was what
/// landed a Job's work. A test about delivery says otherwise through
/// [`delivering`]; nothing else has to state anything.
#[derive(Debug, Clone, Copy)]
enum Sends<'a> {
    TheLastStep,
    /// The step named, or no step at all — which is what a workflow producing
    /// something read rather than merged declares.
    Named(Option<&'a str>),
}

impl Sends<'_> {
    fn is(self, steps: &[Sketch<'_>], n: usize) -> bool {
        match self {
            Sends::TheLastStep => n + 1 == steps.len(),
            Sends::Named(at) => at == Some(steps[n].id),
        }
    }
}

/// Which step of a fixture holds for a person rather than advancing itself.
///
/// **A whole-fixture argument rather than a field on [`Sketch`]**, for
/// [`Sends`]'s reason: `NoStep` is what every fixture in the workspace was
/// written under, and a field would make ninety-odd literals state a gate they
/// do not use.
///
/// **Separate from [`Sends`] even though every shipped `handoff` step declares
/// both.** They are two declarations: `delivers` says where the work goes out
/// and `advance_gate` says who answers for it, and a fixture that spelled them
/// as one could not produce a workflow that delivers and advances on its own —
/// which the schema allows and nothing forbids.
#[derive(Debug, Clone, Copy)]
enum Held<'a> {
    NoStep,
    Named(&'a str),
}

impl Held<'_> {
    fn is(self, step: &Sketch<'_>) -> bool {
        matches!(self, Held::Named(at) if at == step.id)
    }
}

/// The same fixture with the named step sending the work out **and holding for
/// a person** — the pair every shipped workflow's `handoff` step declares.
///
/// **Two declarations written together because one step carries both**, not
/// because either implies the other: [`delivering`] says where the work goes
/// out on its own, and this is the shape a Job a person merges actually has.
/// The gate is `human_always`, which the parser accepts beside judge checks and
/// beside none.
pub fn handing_off(steps: &[Sketch<'_>], at: &str) -> ResolvedWorkflow {
    assembled(
        steps,
        0,
        &[],
        &[],
        &[],
        &[],
        &[],
        Sends::Named(Some(at)),
        Held::Named(at),
    )
}

/// The same fixture with the named step sending the work out, and no other step
/// doing so. `None` is a workflow that delivers nothing at all.
pub fn delivering(steps: &[Sketch<'_>], at: Option<&str>) -> ResolvedWorkflow {
    assembled(
        steps,
        0,
        &[],
        &[],
        &[],
        &[],
        &[],
        Sends::Named(at),
        Held::NoStep,
    )
}

fn assembled(
    steps: &[Sketch<'_>],
    retry_limit: u32,
    models: &[(&str, &str)],
    commands: &[(&str, &str)],
    requires: &[(&str, &[&str])],
    patience: &[Patience<'_>],
    narrows: &[Narrows<'_>],
    delivers: Sends<'_>,
    held: Held<'_>,
) -> ResolvedWorkflow {
    let roster = Roster::of(models.iter().map(|(_, model)| *model));
    let def = WorkflowDef::parse(
        Path::new("fixture-workflow.yml"),
        &workflow_text(steps, retry_limit, models, patience, delivers, held),
        &roster,
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let manifest = Manifest::parse(
        Path::new("fixture-armada.yml"),
        &manifest_text(steps, commands, requires, narrows),
    )
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

fn workflow_text(
    steps: &[Sketch<'_>],
    retry_limit: u32,
    models: &[(&str, &str)],
    patience: &[Patience<'_>],
    delivers: Sends<'_>,
    held: Held<'_>,
) -> String {
    let mut text = String::from(
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\nsteps:\n",
    );
    for (n, step) in steps.iter().enumerate() {
        // A person's gate outranks both, because it names an actor rather
        // than a tier: `config`'s own parser accepts `human_always` beside
        // judge checks and beside none.
        let gate = match (held.is(step), step.judged_on.is_empty()) {
            (true, _) => "human_always",
            (false, true) => "auto",
            (false, false) => "auto_if_judge_passes",
        };
        // Every step has to say, and [`Sends`] is what says it.
        let delivers = delivers.is(steps, n);
        text.push_str(&format!(
            "  - id: {}\n    label: \"{}\"\n    advance_gate: {gate}\n    \
             delivers: {delivers}\n    retry_limit: {retry_limit}\n",
            step.id, step.label
        ));
        if let Some(evidence) = step.evidence_type {
            text.push_str(&format!("    evidence_type: {evidence}\n"));
        }
        if let Some((_, model)) = models.iter().find(|(id, _)| *id == step.id) {
            text.push_str(&format!("    model: {model}\n"));
        }
        // Each half on its own, and no key at all where the fixture says
        // nothing — which is what a step deferring to Fleet looks like in a
        // real file, and is refused by neither reader.
        if let Some(declared) = patience.iter().find(|held| held.step == step.id) {
            if let Some(seconds) = declared.quiet_after_seconds {
                text.push_str(&format!("    quiet_after_seconds: {seconds}\n"));
            }
            if let Some(limit) = declared.poke_limit {
                text.push_str(&format!("    poke_limit: {limit}\n"));
            }
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
                // `on_refusal: refuse` on every fixture criterion, so every
                // existing test exercising a veto keeps testing one:
                // `docs/concepts/judge.md`'s asking design makes `ask` the
                // default, and a fixture that wants that behaviour tests it
                // through a criterion of its own rather than through this one.
                text.push_str(&format!(
                    "          - criterion_id: {id}\n            question: \"{question}\"\n            on_refusal: refuse\n"
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
                // **The step names the Check and says nothing else about it.**
                // `expect_exit_code` is written on the Manifest's Check below,
                // which is where it lives now — and a fixture writing it in
                // both places would be the one shape the real parser refuses.
                Gate::Check { name, .. } => text.push_str(&format!(
                    "      - type: manifest_check\n        check: {name}\n"
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
fn manifest_text(
    steps: &[Sketch<'_>],
    commands: &[(&str, &str)],
    requires: &[(&str, &[&str])],
    narrows: &[Narrows<'_>],
) -> String {
    let mut declared: BTreeMap<&str, (&str, i64, &[&str])> = BTreeMap::new();
    for step in steps {
        for gate in step.gates {
            if let Gate::Check {
                name,
                run,
                expect_exit_code,
                when,
            } = gate
            {
                declared.insert(name, (run, *expect_exit_code, when));
            }
        }
    }
    let mut text = String::from("version: 1\nid: 01FIXTUREMANIFEST\n");
    if declared.is_empty() && commands.is_empty() {
        return text;
    }
    if !declared.is_empty() {
        text.push_str("checks:\n");
    }
    for (name, (run, expect_exit_code, when)) in declared {
        text.push_str(&format!("  {name}:\n    run: \"{run}\"\n"));
        // Absent rather than zero, for the reason every other key here is
        // absent rather than empty: zero is what an `armada.yml` saying nothing
        // already means, and writing it would put a line in every fixture to
        // state what silence states.
        if expect_exit_code != 0 {
            text.push_str(&format!("    expect_exit_code: {expect_exit_code}\n"));
        }
        // No key at all where the fixture declares no path, because that is
        // what an `armada.yml` without a `when` looks like — and an empty list
        // is refused by the parser this fixture runs through.
        if !when.is_empty() {
            let quoted: Vec<String> = when.iter().map(|p| format!("\"{p}\"")).collect();
            text.push_str(&format!("    when: [{}]\n", quoted.join(", ")));
        }
        // Same rule: absent rather than empty. A fixture naming a Command the
        // `commands` argument does not declare is refused by the real parser,
        // which is the point of writing YAML at all.
        if let Some((_, needed)) = requires.iter().find(|(check, _)| check == &name) {
            text.push_str(&format!("    requires: [{}]\n", needed.join(", ")));
        }
        // Same rule again: a Check the list does not name writes no `narrow:`,
        // which is the Check that runs whole.
        if let Some(narrow) = narrows.iter().find(|narrow| narrow.check == name) {
            text.push_str(&format!(
                "    narrow:\n      run: \"{}\"\n      each: \"{}\"\n",
                narrow.run, narrow.each
            ));
            if !narrow.from.is_empty() {
                let quoted: Vec<String> = narrow.from.iter().map(|p| format!("\"{p}\"")).collect();
                text.push_str(&format!("      from: [{}]\n", quoted.join(", ")));
            }
            if let Some(under) = narrow.under {
                text.push_str(&format!("      under: \"{under}\"\n"));
            }
            if !narrow.except.is_empty() {
                text.push_str(&format!("      except: [{}]\n", narrow.except.join(", ")));
            }
        }
    }
    if !commands.is_empty() {
        text.push_str("commands:\n");
    }
    for (name, run) in commands {
        text.push_str(&format!("  {name}:\n    run: \"{run}\"\n"));
    }
    text
}
