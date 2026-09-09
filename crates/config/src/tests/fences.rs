//! `exclude_paths` across the three tiers that may state it.
//!
//! **The subject is what a step may now leave out.** Everything one file says
//! about itself is settled elsewhere — [`crate::tests::scope`] for a step's
//! block, [`crate::tests::manifest`] for the `drone:` section — so what is
//! asked here is only the thing neither can answer alone: which tier's list a
//! resolved step ends up carrying, and that the tier below it is untouched.

use core_model::RepoPath;

use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{fault_at, named, refusals, roster};
use crate::workflow::WorkflowDef;

/// A step that states its own list, and one that states none. Both carry an
/// `evidence_scope`, because a step with no block at all is fenced by nothing
/// and is the last test in this file.
const TWO_STEPS: &str = r#"
version: 1
workflow_id: scoped
name: scoped
structure: linear
steps:
  - id: implement
    label: Implement
    evidence_type: diff
    delivers: false
    advance_gate: auto
    evidence_scope:
      context_source: drone_declared
      exclude_paths:
        - docs
  - id: handoff
    label: Summarise
    evidence_type: facts_note
    delivers: true
    advance_gate: auto
    evidence_scope:
      context_source: drone_declared
"#;

const BARE_STEP: &str = r#"
version: 1
workflow_id: scoped
name: scoped
structure: linear
steps:
  - id: implement
    label: Implement
    evidence_type: diff
    delivers: true
    advance_gate: auto
"#;

fn workflow(text: &str) -> WorkflowDef {
    WorkflowDef::parse(&named("scoped.yml"), text, &roster()).expect("the definition loads")
}

/// An `armada.yml` with `drone:` saying whatever the caller passes, or no
/// `drone:` at all where it passes nothing.
fn manifest(drone: Option<&str>) -> Manifest {
    let section = drone.unwrap_or("");
    Manifest::parse(
        &named("armada.yml"),
        &format!("version: 1\nid: armada\n{section}"),
    )
    .expect("the manifest loads")
}

fn fences(workflow: &ResolvedWorkflow, step: usize) -> Vec<RepoPath> {
    workflow.steps()[step]
        .evidence_scope()
        .expect("the step carries a scope")
        .exclude_paths()
        .to_vec()
}

#[test]
fn a_step_that_states_none_inherits_the_repositorys_list() {
    let resolved = ResolvedWorkflow::resolve(
        &workflow(TWO_STEPS),
        &manifest(Some(
            "drone:\n  exclude_paths:\n    - vendor\n    - .cache\n",
        )),
    )
    .expect("nothing to resolve");

    assert_eq!(
        fences(&resolved, 1),
        [RepoPath::new("vendor"), RepoPath::new(".cache")]
    );
}

/// **The backward-compatibility test, and the one the seven shipped workflows
/// rest on.** A step stating a list keeps it whole while the repository states
/// a different one — not joined, not appended to, and not the repository's.
#[test]
fn a_step_that_states_its_own_beats_the_repository() {
    let resolved = ResolvedWorkflow::resolve(
        &workflow(TWO_STEPS),
        &manifest(Some("drone:\n  exclude_paths:\n    - vendor\n")),
    )
    .expect("nothing to resolve");

    assert_eq!(fences(&resolved, 0), [RepoPath::new("docs")]);
}

/// Where neither file says, the compiled-in list is the answer. Named here
/// rather than asserted against `crate::resolve`'s constant, so changing that
/// constant fails a test that says what changed.
#[test]
fn a_repository_that_states_none_leaves_generated_output_fenced() {
    let resolved = ResolvedWorkflow::resolve(&workflow(TWO_STEPS), &manifest(None))
        .expect("nothing to resolve");

    assert_eq!(
        fences(&resolved, 1),
        [RepoPath::new("node_modules"), RepoPath::new("target")]
    );
}

/// The other two keys in `drone:` are resolved by `fleet::Liveness::at` and one
/// section is not one value: a repository stating only its fences has not
/// thereby stated a patience, and vice versa.
#[test]
fn each_key_in_the_drone_section_falls_back_on_its_own() {
    let fenced_only = manifest(Some("drone:\n  exclude_paths:\n    - vendor\n"));
    assert_eq!(fenced_only.quiet_after_seconds(), None);
    assert_eq!(fenced_only.poke_limit(), None);

    let patient_only = manifest(Some("drone:\n  quiet_after_seconds: 300\n"));
    assert!(patient_only.exclude_paths().is_empty());

    let resolved =
        ResolvedWorkflow::resolve(&workflow(TWO_STEPS), &patient_only).expect("nothing to resolve");
    assert_eq!(
        fences(&resolved, 1),
        [RepoPath::new("node_modules"), RepoPath::new("target")]
    );
}

/// **Nothing fences a step that asks for no declaration.** There is no
/// `evidence_scope` block to put a list in, and inventing one would give a step
/// a denylist against a declaration it never collects.
#[test]
fn a_step_with_no_evidence_scope_is_left_alone() {
    let resolved = ResolvedWorkflow::resolve(
        &workflow(BARE_STEP),
        &manifest(Some("drone:\n  exclude_paths:\n    - vendor\n")),
    )
    .expect("nothing to resolve");

    assert!(resolved.steps()[0].evidence_scope().is_none());
}

/// The empty list is refused wherever it is written, so absence has exactly one
/// reading at every tier: this file defers to the one above it.
#[test]
fn a_repository_cannot_write_the_empty_list() {
    let refused = refusals(Manifest::parse(
        &named("armada.yml"),
        "version: 1\nid: armada\ndrone:\n  exclude_paths: []\n",
    ));

    assert!(matches!(
        fault_at(&refused, "drone.exclude_paths"),
        crate::error::Fault::Empty
    ));
}
