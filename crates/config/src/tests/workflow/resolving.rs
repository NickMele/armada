//! The cross-file check: a WorkflowDef met with a Manifest.
//!
//! **Two files whose agreement nothing else asks about.** Everything either
//! file says about itself is settled by the time these run, so what is left is
//! only what neither can answer alone — whether a name a step wrote is
//! declared, and what a Check carries with it once it is.

use core_model::ResolvedCheck;

use super::{bug_with, manifest, parse, BUG};
use crate::error::ResolveError;
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::named;

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
        "  - id: lint\n    label: Lint\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: lint, expect_exit_code: 0 }\n",
    )
    .expect("the definition itself is well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("`lint` is not declared");
    let ResolveError::ChecksNotDeclared { unknown, .. } = &error;
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
        "  - id: tidy\n    label: Tidy\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: fmt, expect_exit_code: 0 }\n",
    )
    .expect("well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("`fmt` is a Command");
    let ResolveError::ChecksNotDeclared { unknown, .. } = &error;
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
        "  - id: lint\n    label: Lint\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: lint, expect_exit_code: 0 }\n      - { type: manifest_check, check: typecheck, expect_exit_code: 0 }\n",
    )
    .expect("well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("two names miss");
    let ResolveError::ChecksNotDeclared { unknown, .. } = &error;
    let names: Vec<&str> = unknown.iter().map(|u| u.check.as_str()).collect();
    assert_eq!(names, ["lint", "typecheck"]);
}

#[test]
fn a_step_with_no_checks_needs_nothing_from_the_manifest() {
    let def = parse(
        "version: 1\nworkflow_id: fixture\nname: prototype\nstructure: linear\nsteps:\n  - id: frame\n    label: Frame\n    evidence_type: facts_note\n    advance_gate: auto\n",
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
