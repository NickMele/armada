//! **Every workflow Armada carries resolves in a repository that is not this
//! one.** #425.
//!
//! `shipped.rs` resolves the eight against this repository's own `armada.yml`,
//! which is the one Manifest guaranteed to agree with them. Carried, they meet
//! every other shape, so they are resolved here against three that share
//! nothing with it: one declaring no Checks, one whose Checks are all named
//! differently, and one declaring no `review_gate` and no `evidence:`.

use std::collections::BTreeSet;
use std::path::Path;

use config::{
    AdvanceGate, Catalogue, Manifest, ResolvedCheck, ResolvedWorkflow, Roster, WorkflowSource,
    Written,
};

/// See `shipped.rs`: the roster the running daemon resolves, not a list here.
fn roster() -> Roster {
    Roster::of(adapters::HeadlessAgent::models())
}

/// A repository declaring nothing but its id.
const NO_CHECKS: &str = "version: 1\nid: bare\n";

/// Checks sharing no name with this repository's, with a prerequisite, and a
/// review policy set the other way from the default.
const RENAMED: &str = "version: 1\nid: renamed\nreview_gate: auto_if_judge_passes\n\
                       checks:\n  unit:\n    run: make unit\n  e2e:\n    run: make e2e\n    \
                       requires:\n      - seed\n  vet:\n    run: make vet\n\
                       commands:\n  seed:\n    run: make seed\n";

/// Checks, and neither `review_gate` nor `evidence:`.
const NO_REVIEW_GATE: &str = "version: 1\nid: plain\nchecks:\n  check:\n    run: make check\n";

fn manifest(text: &str) -> Manifest {
    Manifest::parse(Path::new("/elsewhere/armada.yml"), text)
        .unwrap_or_else(|why| panic!("the fixture Manifest loads: {why}"))
}

/// The carried set, resolved against `manifest` alone.
fn carried_against(manifest: &Manifest) -> Vec<ResolvedWorkflow> {
    let (workflows, left_out) = Catalogue::of(config::carried(), &roster())
        .unwrap_or_else(|why| panic!("the carried set makes a catalogue: {why:?}"))
        .resolve(manifest)
        .unwrap_or_else(|why| panic!("nothing a repository wrote is refused: {why}"))
        .into_parts();
    let named: Vec<String> = left_out.iter().map(ToString::to_string).collect();
    assert!(
        named.is_empty(),
        "no carried definition is left out: {named:?}"
    );
    workflows.into_values().collect()
}

/// **What makes a definition carryable, asked of each step**: it gates on
/// every Check the repository declares, in its order, or on no Check by name.
fn gates_only_on_what_the_repository_declares(text: &str) -> Vec<ResolvedWorkflow> {
    let manifest = manifest(text);
    let resolved = carried_against(&manifest);
    assert_eq!(
        resolved.len(),
        8,
        "all eight, against {}",
        manifest.id().as_str()
    );
    for workflow in &resolved {
        assert_eq!(workflow.source(), WorkflowSource::Armada);
        for step in workflow.steps() {
            let names: Vec<&str> = step
                .checks()
                .iter()
                .filter_map(|check| match check {
                    ResolvedCheck::ManifestCheck { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            let expected: Vec<&str> = match step.gates_on_every_check() {
                true => manifest
                    .checks_as_written()
                    .iter()
                    .map(String::as_str)
                    .collect(),
                false => Vec::new(),
            };
            assert_eq!(
                names,
                expected,
                "`{}` step `{}`",
                workflow.id().as_str(),
                step.id().as_str()
            );
        }
    }
    resolved
}

#[test]
fn every_carried_workflow_resolves_against_a_manifest_declaring_no_checks() {
    let resolved = gates_only_on_what_the_repository_declares(NO_CHECKS);
    let asking: usize = resolved
        .iter()
        .flat_map(|workflow| workflow.steps())
        .filter(|step| step.gates_on_every_check())
        .count();
    assert_eq!(
        asking, 5,
        "five steps still say they asked, which is all an empty registry leaves"
    );
}

#[test]
fn every_carried_workflow_resolves_against_checks_named_differently() {
    let resolved = gates_only_on_what_the_repository_declares(RENAMED);
    let bug = resolved
        .iter()
        .find(|workflow| workflow.id().as_str() == "bug")
        .expect("bug is carried");
    let e2e = bug
        .steps()
        .iter()
        .flat_map(|step| step.checks())
        .find(|check| check.name() == Some("e2e"))
        .expect("the repository's own e2e, on the step that gates on every Check");
    assert_eq!(e2e.requires()[0].run(), "make seed");
}

/// **No carried step defers to the policy**, so a repository that never set
/// one changes nothing here. Asserted because a carried step that did would
/// read `review_gate`'s default in every repository nobody configured — and
/// that should be a decision somebody takes, not an edit that slips through.
#[test]
fn every_carried_workflow_resolves_against_a_manifest_declaring_no_review_gate() {
    let resolved = gates_only_on_what_the_repository_declares(NO_REVIEW_GATE);
    let deferring: Vec<String> = resolved
        .iter()
        .flat_map(|workflow| {
            workflow
                .steps()
                .iter()
                .filter(|step| step.advance_gate() == AdvanceGate::ManifestRuleReviewGate)
                .map(move |step| format!("{}.{}", workflow.id().as_str(), step.id().as_str()))
        })
        .collect();
    assert!(deferring.is_empty(), "{deferring:?}");
}

/// **The carried set is this repository's files, all of them.** A ninth
/// definition added under `.armada/workflows/` and not to the carried list
/// would be one this repository runs and no other repository gets.
#[test]
fn armada_carries_every_definition_this_repository_ships_as_written() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.armada/workflows");
    let on_disk: BTreeSet<(String, String)> = std::fs::read_dir(&dir)
        .expect("the shipped definitions are there")
        .map(|entry| entry.expect("a directory entry").path())
        .map(|path| {
            let file = path
                .file_name()
                .expect("a file")
                .to_string_lossy()
                .to_string();
            (file, std::fs::read_to_string(&path).expect("readable"))
        })
        .collect();
    let carried: BTreeSet<(String, String)> = config::carried()
        .iter()
        .map(|one| {
            let file = one
                .path()
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .to_string();
            (file, one.text().to_string())
        })
        .collect();
    assert_eq!(carried, on_disk);
}

/// **Kit over Armada, and the repository over both**, with the carried set as
/// the bottom layer — the one precedence `src/tests/catalogue.rs` cannot build,
/// because only the carried set is Armada's and it names the adapter's models.
#[test]
fn one_file_in_kit_or_the_repository_replaces_a_carried_definition_by_id() {
    let one_step = |id: &str| {
        format!(
            "version: 1\nworkflow_id: {id}\nname: {id}\nstructure: linear\nsteps:\n  - id: only\n    \
             label: Only\n    delivers: true\n    advance_gate: auto\n"
        )
    };
    let mut written = config::carried();
    written.push(Written::in_kit(
        "/kit/workflows/bug.yml".into(),
        one_step("bug"),
    ));
    written.push(Written::in_kit(
        "/kit/workflows/revert.yml".into(),
        one_step("revert"),
    ));
    written.push(Written::in_repository(
        "/repo/.armada/workflows/revert.yml".into(),
        one_step("revert"),
    ));
    let held = Catalogue::of(written, &roster())
        .expect("the three places merge")
        .resolve(&manifest(NO_CHECKS))
        .expect("every winner resolves");
    let sources: Vec<(&str, WorkflowSource)> = held
        .workflows()
        .iter()
        .map(|(id, workflow)| (id.as_str(), workflow.source()))
        .filter(|(id, _)| ["bug", "feature", "revert"].contains(id))
        .collect();
    assert_eq!(
        sources,
        [
            ("bug", WorkflowSource::Kit),
            ("feature", WorkflowSource::Armada),
            ("revert", WorkflowSource::Repository),
        ]
    );
}
