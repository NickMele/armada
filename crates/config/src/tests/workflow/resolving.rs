//! The cross-file check: a WorkflowDef met with a Manifest.
//!
//! **Two files whose agreement nothing else asks about.** Everything either
//! file says about itself is settled by the time these run, so what is left is
//! only what neither can answer alone — whether a name a step wrote is
//! declared, and what a Check carries with it once it is.

use core_model::{ResolvedCheck, StepId};

use super::{bug_with, manifest, parse, BUG, MANIFEST};
use crate::error::{Disagreement, ResolveError, UnknownCheck};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{named, roster};
use crate::workflow::WorkflowDef;

/// The names that resolved to nothing, or a panic naming what came back
/// instead. A refusal of the other shape here is a test that would otherwise
/// pass by matching nothing.
fn missing(error: &ResolveError) -> &[UnknownCheck] {
    match error {
        ResolveError::ChecksNotDeclared { unknown, .. } => unknown,
        other => panic!("expected an undeclared name and got: {other}"),
    }
}

/// The places the two files say different things, same shape and same reason.
fn disagreements(error: &ResolveError) -> &[Disagreement] {
    match error {
        ResolveError::StepsDisagreeWithTheManifest { disagreements, .. } => disagreements,
        other => panic!("expected a disagreement and got: {other}"),
    }
}

#[test]
fn a_resolved_workflow_carries_the_command_not_the_name() {
    // The lookup that could miss happens once, here. Nothing at step time
    // performs one at all, which is why an absent Check is unrepresentable
    // downstream rather than checked for.
    let def = parse(BUG).expect("the worked example");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest()).expect("every name declared");
    assert_eq!(
        resolved.steps()[1].checks(),
        [
            ResolvedCheck::ManifestCheck {
                name: "build".to_string(),
                run: "cargo build --workspace".to_string(),
                expect_exit_code: 0,
                when: None,
                requires: Vec::new(),
                narrow: None,
            },
            ResolvedCheck::ManifestCheck {
                name: "test".to_string(),
                run: "cargo nextest run --workspace".to_string(),
                expect_exit_code: 0,
                when: None,
                requires: Vec::new(),
                narrow: None,
            },
            ResolvedCheck::DiffNonempty,
        ]
    );
    assert_eq!(resolved.name(), "bug");
}

#[test]
fn a_step_naming_a_check_the_manifest_lacks_is_refused_before_dispatch() {
    let def = bug_with(
        "  - id: lint\n    label: Lint\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: lint, expect_exit_code: 0 }\n",
    )
    .expect("the definition itself is well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("`lint` is not declared");
    let unknown = missing(&error);
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].step.as_str(), "lint");
    assert_eq!(unknown[0].check, "lint");
    assert!(!unknown[0].is_a_command);
    assert_eq!(unknown[0].declared, ["build", "test"]);

    let message = error.to_string();
    assert!(message.contains("armada.yml"), "{message}");
    assert!(message.contains("workflows/bug.yml"), "{message}");
    assert!(
        message.contains("Declared Checks are `build`, `test`"),
        "{message}"
    );
}

#[test]
fn naming_a_command_where_a_check_belongs_is_told_which_mistake_it_was() {
    let def = bug_with(
        "  - id: tidy\n    label: Tidy\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: fmt, expect_exit_code: 0 }\n",
    )
    .expect("well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("`fmt` is a Command");
    let unknown = missing(&error);
    assert!(unknown[0].is_a_command);
    assert!(
        error
            .to_string()
            .contains("declared as a Command, not a Check"),
        "{error}"
    );
}

#[test]
fn every_unresolved_name_is_reported_not_only_the_first() {
    let def = bug_with(
        "  - id: lint\n    label: Lint\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: lint, expect_exit_code: 0 }\n      - { type: manifest_check, check: typecheck, expect_exit_code: 0 }\n",
    )
    .expect("well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("two names miss");
    let unknown = missing(&error);
    let names: Vec<&str> = unknown.iter().map(|u| u.check.as_str()).collect();
    assert_eq!(names, ["lint", "typecheck"]);
}

#[test]
fn a_step_with_no_checks_needs_nothing_from_the_manifest() {
    let def = parse(
        "version: 1\nworkflow_id: fixture\nname: prototype\nstructure: linear\nsteps:\n  - id: frame\n    label: Frame\n    evidence_type: facts_note\n    delivers: false\n    advance_gate: auto\n",
    )
    .expect("well formed");
    let bare = Manifest::parse(&named("armada.yml"), "version: 1\nid: tooling\n").expect("bare");
    let resolved = ResolvedWorkflow::resolve(&def, &bare).expect("nothing to resolve");
    assert!(resolved.steps()[0].checks().is_empty());
}

/// The same workflow's Manifest, with `build` scoped to the Rust tree.
const SCOPED_MANIFEST: &str = r#"
version: 1
id: armada
checks:
  build:
    run: cargo build --workspace
    when: ["crates/**", "Cargo.toml"]
  test:
    run: cargo nextest run --workspace
commands:
  fmt:
    run: cargo fmt --all
"#;

#[test]
fn a_checks_when_is_lifted_off_the_manifest_and_frozen_onto_the_step() {
    // **The pattern is the Manifest's and the step does not restate it.** The
    // repository declares once what a Check covers and every workflow inherits
    // it — there is no step-level key to override it with, which is what stops
    // the same glob drifting across `bug`, `feature`, `refactor` and `revert`.
    let def = parse(BUG).expect("the worked example");
    let manifest = Manifest::parse(&named("armada.yml"), SCOPED_MANIFEST).expect("a scoped check");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");

    let checks = resolved.steps()[1].checks();
    assert_eq!(
        checks[0].when().map(core_model::Covers::written),
        Some("crates/**, Cargo.toml".to_string())
    );
    // The one that declares nothing carries nothing, and covers everything.
    assert_eq!(checks[1].when(), None);
    assert!(checks[1].covers(&["anything/at/all".to_string()]));
    // Frozen: the resolved value is a copy, so an edit to `armada.yml` after
    // this point changes the next Job and not this one.
    assert!(checks[0].covers(&["crates/store/src/read.rs".to_string()]));
    assert!(!checks[0].covers(&["packages/components/src/Badge.tsx".to_string()]));
}

#[test]
fn a_built_in_check_declares_no_paths_and_always_runs() {
    let def = parse(BUG).expect("the worked example");
    let manifest = Manifest::parse(&named("armada.yml"), SCOPED_MANIFEST).expect("a scoped check");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");
    let diff = &resolved.steps()[1].checks()[2];
    assert_eq!(diff, &ResolvedCheck::DiffNonempty);
    assert_eq!(diff.when(), None);
    assert!(!diff.needs_changed_paths());
    assert!(diff.covers(&[]));
}

/// The same two Checks, one of which passes by failing. `repro` is the shape
/// the key exists for — a deliberately failing test, which `bug.json`'s
/// designed sample already declares `expect_exit_code: 1` on.
const FAILING_MANIFEST: &str = r#"
version: 1
id: armada
checks:
  build:
    run: cargo build --workspace
  test:
    run: cargo nextest run --workspace
    expect_exit_code: 1
commands:
  fmt:
    run: cargo fmt --all
"#;

/// **A step that writes nothing takes the Manifest's code.** The move the whole
/// change is for: the repository that wrote the command is the one that knows a
/// clean run of it is `1`, and every workflow naming the Check inherits that
/// without restating it.
#[test]
fn a_step_that_says_nothing_about_an_exit_code_takes_the_checks_own() {
    let def = parse(
        "version: 1\nworkflow_id: bug\nname: bug\nstructure: linear\nsteps:\n  - id: repro\n    label: Reproduce\n    evidence_type: diff\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: test }\n      - { type: manifest_check, check: build }\n",
    )
    .expect("a step may leave the key out");
    let manifest = Manifest::parse(&named("armada.yml"), FAILING_MANIFEST).expect("a manifest");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");
    let expects: Vec<Option<i64>> = resolved.steps()[0]
        .checks()
        .iter()
        .map(ResolvedCheck::expects)
        .collect();
    assert_eq!(
        expects,
        [Some(1), Some(0)],
        "the Check that declares one carries it, and the one that does not is zero"
    );
}

/// **The seven shipped workflows still parse and still resolve.** Every one of
/// them writes `expect_exit_code: 0` today and the Manifest's default is zero,
/// so the pair agrees and nothing changes for a Job in flight. This is the
/// claim that lets the workflow files be switched over separately.
#[test]
fn a_step_restating_the_manifests_code_is_carried_unchanged() {
    let def = parse(BUG).expect("the worked example");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest()).expect("the old spelling resolves");
    assert_eq!(resolved.steps()[1].checks()[0].expects(), Some(0));
}

/// **And a step restating it wrongly is refused before dispatch.** The key
/// survives so old workflows parse, not so that a workflow can override a
/// repository about its own command — two files disagreeing about what a
/// passing run looks like has no reading that is not a guess, and the guess
/// would make the Check unpassable for every Job dispatched here.
#[test]
fn a_step_contradicting_the_manifests_exit_code_is_refused() {
    let def = parse(
        "version: 1\nworkflow_id: bug\nname: bug\nstructure: linear\nsteps:\n  - id: repro\n    label: Reproduce\n    evidence_type: diff\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: test, expect_exit_code: 0 }\n",
    )
    .expect("the definition itself is well formed");
    let manifest = Manifest::parse(&named("armada.yml"), FAILING_MANIFEST).expect("a manifest");
    let error = ResolvedWorkflow::resolve(&def, &manifest).expect_err("the two disagree");
    assert_eq!(
        disagreements(&error),
        [Disagreement::ExitCode {
            step: core_model::StepId::new("repro".to_string()),
            check: "test".to_string(),
            step_expects: 0,
            manifest_expects: 1,
        }]
    );
    let message = error.to_string();
    assert!(message.contains("delete it from the step"), "{message}");
}

/// **`every_manifest_check` is the whole registry, whatever is in it.** The
/// point of the spelling: the same step resolves to two Checks here and to
/// however many the next repository declares, and a Check added to `armada.yml`
/// is gated on without a workflow being edited.
#[test]
fn a_step_may_gate_on_every_check_the_manifest_declares() {
    let def = parse(
        "version: 1\nworkflow_id: feature\nname: feature\nstructure: linear\nsteps:\n  - id: implement\n    label: Implement\n    evidence_type: diff\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: every_manifest_check }\n      - { type: diff_nonempty }\n",
    )
    .expect("a step may gate on all of them");
    let manifest = Manifest::parse(&named("armada.yml"), FAILING_MANIFEST).expect("a manifest");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");
    let checks = resolved.steps()[0].checks();
    // Sorted, because `checks:` is a map and has no order somebody wrote.
    assert_eq!(
        checks
            .iter()
            .map(ResolvedCheck::label)
            .collect::<Vec<&str>>(),
        ["build", "test", "diff_nonempty"]
    );
    // Each carries what the Manifest says about it, exactly as a named one
    // does — the command, and the code that means it passed.
    assert_eq!(checks[0].run(), Some("cargo build --workspace"));
    assert_eq!(checks[1].expects(), Some(1));
    assert_eq!(checks[2], ResolvedCheck::DiffNonempty);
    // And the step keeps saying what it asked for, beside what that came to.
    assert!(resolved.steps()[0].gates_on_every_check());
}

/// **A step that declares no mechanical check is still ungated.** The
/// load-bearing half of the design: absence must not become *all*, or the four
/// shipped workflows whose steps produce a document nothing compiles would gate
/// on this repository's Rust build.
#[test]
fn declaring_no_checks_is_not_declaring_every_check() {
    let def = parse(
        "version: 1\nworkflow_id: design\nname: design\nstructure: linear\nsteps:\n  - id: draft\n    label: Draft\n    evidence_type: document\n    delivers: false\n    advance_gate: auto\n",
    )
    .expect("well formed");
    let manifest = Manifest::parse(&named("armada.yml"), FAILING_MANIFEST).expect("a manifest");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("nothing to resolve");
    assert!(resolved.steps()[0].checks().is_empty());
    // The Manifest declares two, so the empty list here is not the registry
    // being empty — it is the step never having asked, and the record says
    // which of the two an empty list means.
    assert!(!resolved.steps()[0].gates_on_every_check());
}

/// **A repository declaring no Checks expands to nothing, and is not refused.**
/// `docs/concepts/manifest.md` sanctions an ungated workspace as a state rather
/// than a mistake, and *run what this repository declares* reads literally: a
/// repository declaring nothing runs nothing. Refusing would have made a
/// documented state unusable with any shipped workflow.
///
/// **What was wrong was the silence.** The step is frozen saying it asked for
/// every Check, so the empty list beside it reads as *the repository declared
/// none* rather than as *this step declared no gate*.
#[test]
fn gating_on_every_check_where_the_manifest_declares_none_expands_to_nothing_and_says_so() {
    let def = parse(
        "version: 1\nworkflow_id: feature\nname: feature\nstructure: linear\nsteps:\n  - id: implement\n    label: Implement\n    evidence_type: diff\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: every_manifest_check }\n",
    )
    .expect("well formed");
    let bare = Manifest::parse(&named("armada.yml"), "version: 1\nid: tooling\n").expect("bare");
    let resolved = ResolvedWorkflow::resolve(&def, &bare).expect("an ungated repository is legal");
    let step = &resolved.steps()[0];
    assert!(step.checks().is_empty(), "there was nothing to expand to");
    assert!(
        step.gates_on_every_check(),
        "and the record still says the step asked for all of them"
    );
}

/// A Manifest whose declaration order is not its alphabetical order, so the
/// two are told apart by what comes back rather than by luck.
const OUT_OF_ORDER_MANIFEST: &str = r#"
version: 1
id: armada
checks:
  zebra:
    run: run zebra
  build:
    run: run build
  apple:
    run: run apple
"#;

/// **The expansion is the file's order, not the alphabet's.** `checks:` is
/// where a repository sequences its gate — `fleet::checking` starts four at a
/// time off this order — and until a step could gate on every Check, the
/// workflow file was the only place that sequence was written. Expanding
/// alphabetically would have reordered every repository's gate silently, and
/// this repository's own `armada.yml` argues the sequence in its own words,
/// with measured seconds.
///
/// `check_names` is the other view of the same set and stays sorted, because a
/// message listing what is declared is scanned by a person for a name they
/// expected. Both are asserted here so neither can quietly become the other.
#[test]
fn every_manifest_check_expands_in_the_order_the_manifest_writes_not_the_alphabet() {
    let def = parse(
        "version: 1\nworkflow_id: feature\nname: feature\nstructure: linear\nsteps:\n  - id: implement\n    label: Implement\n    evidence_type: diff\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: every_manifest_check }\n",
    )
    .expect("well formed");
    let manifest =
        Manifest::parse(&named("armada.yml"), OUT_OF_ORDER_MANIFEST).expect("a manifest");
    assert_eq!(
        manifest.checks_as_written(),
        ["zebra", "build", "apple"],
        "the file's order survives the parse"
    );
    assert_eq!(
        manifest.check_names(),
        ["apple", "build", "zebra"],
        "and the sorted view is still sorted, for the message that reads it"
    );

    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");
    assert_eq!(
        resolved.steps()[0]
            .checks()
            .iter()
            .map(ResolvedCheck::label)
            .collect::<Vec<&str>>(),
        ["zebra", "build", "apple"],
        "the gate runs them as the file wrote them"
    );
}

/// A step that named its Checks one at a time never claims it asked for all of
/// them, however many the Manifest happens to declare.
#[test]
fn naming_checks_by_hand_does_not_read_as_gating_on_every_one() {
    let def = parse(BUG).expect("the worked example");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest()).expect("every name declared");
    assert_eq!(resolved.steps()[1].checks().len(), 3);
    assert!(!resolved.steps()[1].gates_on_every_check());
}

/// A workflow with a `visual` step, met with a Manifest that declares no
/// harness.
const SHOWS: &str = r#"
version: 1
workflow_id: shows
name: shows
structure: linear
steps:
  - id: implement
    label: Implement
    evidence_type: diff
    delivers: false
    advance_gate: auto
  - id: show
    label: Show what it looks like
    evidence_type: visual
    delivers: true
    advance_gate: auto
"#;

/// A harness section, in the shape a repository declaring one writes it.
const AND_A_HARNESS: &str = r#"
evidence:
  serve: pnpm dev --port 6006
  ready: curl -sf http://localhost:6006
  run: pnpm exec playwright test {}
  frames: .playwright/frames
"#;

/// **A step asking to be shown, held to a repository that can show it.**
///
/// Neither file can answer this alone: `evidence_type: visual` parses against
/// no Manifest, and an `evidence:` section is declared with no workflow in
/// sight. So it is checked here, at resolve, and refused before anything is
/// dispatched.
///
/// **Refusing at dispatch is the whole argument.** The alternative is a Job
/// that reaches the step with a worktree cut and a Drone spawned and then
/// captures nothing — which reads as a broken harness rather than as a file
/// that never declared one, and sends a person to fix the wrong thing.
#[test]
fn a_step_that_shows_its_work_is_refused_where_nothing_can_show_it() {
    let def = WorkflowDef::parse(&named("workflows/shows.yml"), SHOWS, &roster())
        .expect("a workflow with a visual step");

    let refused = ResolvedWorkflow::resolve(&def, &manifest())
        .expect_err("no harness is declared, so the step cannot run");

    let ResolveError::ShowsWithNoHarness { steps, .. } = refused else {
        panic!("expected a harness refusal and got: {refused}");
    };
    assert_eq!(
        steps.iter().map(StepId::as_str).collect::<Vec<&str>>(),
        ["show"],
        "the step is named, so the message says which one to look at"
    );
}

/// And the same workflow resolves once the repository says how it shows its
/// work. **Absence is the refusal and not the type**, which is what keeps
/// `visual` a step a repository can opt into rather than one nobody can write.
#[test]
fn a_repository_that_declares_a_harness_may_have_steps_that_show_their_work() {
    let def = WorkflowDef::parse(&named("workflows/shows.yml"), SHOWS, &roster())
        .expect("a workflow with a visual step");
    let manifest = Manifest::parse(&named("armada.yml"), &format!("{MANIFEST}{AND_A_HARNESS}"))
        .expect("a manifest with a harness");

    let resolved = ResolvedWorkflow::resolve(&def, &manifest)
        .expect("the harness is declared, so it resolves");
    assert_eq!(resolved.steps().len(), 2);
}
