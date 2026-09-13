//! The merge of a workflow's three sources, over text and nothing else.
//!
//! The carried half needs the adapter's roster, because the carried
//! definitions name its models, so it is `tests/carried.rs`. What is here is
//! the rule itself: the more specific place wins, whatever order the
//! definitions arrive in.

use std::path::Path;

use crate::catalogue::{Catalogue, CatalogueRefused, WorkflowSource, Written};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{named, roster};
use crate::workflow::WorkflowDef;

/// A one-step definition whose step is labelled `label`, gated on the Check
/// `gate` names or on nothing.
fn one_step(id: &str, label: &str, gate: Option<&str>) -> String {
    let checks = gate.map_or(String::new(), |check| {
        format!(
            "    evidence: {{submitted: {{type: diff}}}}\n    mechanical_checks:\n      \
             - {{ type: manifest_check, check: {check} }}\n"
        )
    });
    format!(
        "version: 1\nworkflow_id: {id}\nname: {id}\nstructure: linear\nsteps:\n  - id: only\n    \
         label: \"{label}\"\n    delivers: true\n    advance_gate: auto\n{checks}"
    )
}

fn kit(file: &str, text: String) -> Written {
    Written::in_kit(named(&format!("/kit/workflows/{file}")), text)
}

fn repository(file: &str, text: String) -> Written {
    Written::in_repository(named(&format!("/repo/.armada/workflows/{file}")), text)
}

fn a_manifest() -> Manifest {
    Manifest::parse(Path::new("/repo/armada.yml"), "version: 1\nid: fixture\n")
        .expect("a Manifest declaring nothing")
}

fn label_of(catalogue: &Catalogue, id: &str) -> (String, WorkflowSource) {
    let held = catalogue
        .resolve(&a_manifest())
        .expect("every winner resolves");
    let workflow = held
        .values()
        .find(|workflow| workflow.id().as_str() == id)
        .expect("the id is held");
    (workflow.steps()[0].label().to_string(), workflow.source())
}

#[test]
fn the_repositorys_definition_wins_over_kits_in_either_order() {
    for flipped in [false, true] {
        let mut written = vec![
            kit("bug.yml", one_step("bug", "Kit's", None)),
            repository("bug.yml", one_step("bug", "The repository's", None)),
        ];
        if flipped {
            written.reverse();
        }
        let catalogue = Catalogue::of(written, &roster()).expect("one id in two places");
        assert_eq!(
            label_of(&catalogue, "bug"),
            ("The repository's".to_string(), WorkflowSource::Repository),
            "arrival order flipped: {flipped}"
        );
    }
}

#[test]
fn an_id_only_one_place_declares_is_held_from_that_place() {
    let catalogue = Catalogue::of(
        [
            kit("hotfix.yml", one_step("hotfix", "Hot", None)),
            repository("bug.yml", one_step("bug", "Bug", None)),
        ],
        &roster(),
    )
    .expect("two ids");
    let sources: Vec<(&str, WorkflowSource)> = catalogue
        .sources()
        .into_iter()
        .map(|(id, source)| (id.as_str(), source))
        .collect();
    assert_eq!(
        sources,
        [
            ("bug", WorkflowSource::Repository),
            ("hotfix", WorkflowSource::Kit)
        ]
    );
}

/// Two files in one place naming one id is refused, naming both — the rule a
/// repository's own directory always had, now asked per place.
#[test]
fn one_id_twice_in_one_place_is_refused_naming_both() {
    let refused = Catalogue::of(
        [
            kit("first.yml", one_step("shared", "One", None)),
            kit("second.yml", one_step("shared", "Two", None)),
        ],
        &roster(),
    )
    .expect_err("two Kit files agree on an id");
    let CatalogueRefused::DuplicateWorkflowId { id, first, second } = refused else {
        panic!("a duplicate, not {refused:?}");
    };
    assert_eq!(id, "shared");
    assert!(first.ends_with("first.yml") && second.ends_with("second.yml"));
}

#[test]
fn a_definition_that_will_not_parse_is_refused_wherever_it_sits() {
    let refused = Catalogue::of(
        [
            kit(
                "broken.yml",
                "version: 1\nworkflow_id: broken\n".to_string(),
            ),
            repository("broken.yml", one_step("broken", "Fine", None)),
        ],
        &roster(),
    )
    .expect_err("a Kit file with no steps, even under a repository's own");
    let CatalogueRefused::Refused(why) = refused else {
        panic!("a parse refusal, not {refused:?}");
    };
    assert_eq!(why.path(), Path::new("/kit/workflows/broken.yml"));
}

/// A replaced definition never runs, so a Check it names that this repository
/// does not declare is not a reason to refuse the repository.
#[test]
fn a_replaced_definition_is_not_resolved() {
    let catalogue = Catalogue::of(
        [
            kit("bug.yml", one_step("bug", "Kit's", Some("build"))),
            repository("bug.yml", one_step("bug", "Ungated", None)),
        ],
        &roster(),
    )
    .expect("both parse");
    assert_eq!(label_of(&catalogue, "bug").0, "Ungated");

    let alone = Catalogue::of(
        [kit("bug.yml", one_step("bug", "Kit's", Some("build")))],
        &roster(),
    )
    .expect("it parses");
    assert!(
        alone.resolve(&a_manifest()).is_err(),
        "and the same file, winning, is refused for the name"
    );
}

#[test]
fn a_definition_resolved_on_its_own_is_the_repositorys() {
    let def = WorkflowDef::parse(&named("bug.yml"), &one_step("bug", "Bug", None), &roster())
        .expect("it parses");
    let resolved = ResolvedWorkflow::resolve(&def, &a_manifest()).expect("it resolves");
    assert_eq!(resolved.source(), WorkflowSource::Repository);
}
