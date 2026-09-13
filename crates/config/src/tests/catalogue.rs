//! The merge of a workflow's three sources, over text and nothing else.
//!
//! The carried half needs the adapter's roster, because the carried
//! definitions name its models, so it is `tests/carried.rs`. What is here is
//! the rule itself: the more specific place wins, whatever order the
//! definitions arrive in.

use std::path::Path;

use core_model::WorkflowSource;

use crate::catalogue::{Catalogue, CatalogueRefused, WhyLeftOut, Written};
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

fn resolved(written: Vec<Written>) -> crate::catalogue::ResolvedCatalogue {
    Catalogue::of(written, &roster())
        .expect("a catalogue")
        .resolve(&a_manifest())
        .expect("nothing the repository wrote is refused")
}

fn label_of(held: &crate::catalogue::ResolvedCatalogue, id: &str) -> (String, WorkflowSource) {
    let workflow = held
        .workflows()
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
        assert_eq!(
            label_of(&resolved(written), "bug"),
            ("The repository's".to_string(), WorkflowSource::Repository),
            "arrival order flipped: {flipped}"
        );
    }
}

#[test]
fn an_id_only_one_place_declares_is_held_from_that_place() {
    let held = resolved(vec![
        kit("hotfix.yml", one_step("hotfix", "Hot", None)),
        repository("bug.yml", one_step("bug", "Bug", None)),
    ]);
    let sources: Vec<(&str, WorkflowSource)> = held
        .workflows()
        .iter()
        .map(|(id, workflow)| (id.as_str(), workflow.source()))
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

/// **The repository's own is strict; Kit's is left out and named**, and the
/// repository still gets its catalogue.
#[test]
fn a_definition_that_will_not_parse_is_refused_in_the_repository_and_left_out_of_kit() {
    let broken = || "version: 1\nworkflow_id: broken\n".to_string();
    let refused = Catalogue::of([repository("broken.yml", broken())], &roster())
        .expect_err("the repository declared it");
    let CatalogueRefused::Refused(why) = refused else {
        panic!("a parse refusal, not {refused:?}");
    };
    assert_eq!(why.path(), Path::new("/repo/.armada/workflows/broken.yml"));

    let held = resolved(vec![
        kit("broken.yml", broken()),
        repository("bug.yml", one_step("bug", "Bug", None)),
    ]);
    assert_eq!(held.workflows().len(), 1);
    let [left] = held.left_out() else {
        panic!("one left out: {:?}", held.left_out());
    };
    assert_eq!((left.id(), left.source()), (None, WorkflowSource::Kit));
    assert!(matches!(left.why(), WhyLeftOut::Unparsed(_)));
    let said = left.to_string();
    assert!(
        said.contains("from Kit") && said.contains("/kit/workflows/broken.yml"),
        "{said}"
    );
}

/// **One from Kit naming a Check this repository lacks is left out**, and the
/// next place down answers for its id — here, nobody, so the id is absent.
#[test]
fn a_kit_definition_naming_an_undeclared_check_is_left_out_and_named() {
    let held = resolved(vec![kit(
        "bug.yml",
        one_step("bug", "Kit's", Some("build")),
    )]);
    assert!(held.workflows().is_empty());
    let [left] = held.left_out() else {
        panic!("one left out: {:?}", held.left_out());
    };
    assert_eq!(left.id().map(|id| id.as_str()), Some("bug"));
    assert!(matches!(left.why(), WhyLeftOut::Unresolved(_)));
    let said = left.to_string();
    assert!(
        said.contains("workflow `bug` from Kit") && said.contains("build"),
        "{said}"
    );
}

/// A repository's own that will not resolve still refuses: it declared it.
#[test]
fn a_repositorys_definition_naming_an_undeclared_check_is_still_refused() {
    let catalogue = Catalogue::of(
        [
            kit("bug.yml", one_step("bug", "Kit's", None)),
            repository("bug.yml", one_step("bug", "Own", Some("build"))),
        ],
        &roster(),
    )
    .expect("both parse");
    assert!(
        catalogue.resolve(&a_manifest()).is_err(),
        "not Kit's in its place"
    );
}

/// A replaced definition never runs, so a Check it names that this repository
/// does not declare is not checked, and nothing is left out for it.
#[test]
fn a_replaced_definition_is_not_resolved() {
    let held = resolved(vec![
        kit("bug.yml", one_step("bug", "Kit's", Some("build"))),
        repository("bug.yml", one_step("bug", "Ungated", None)),
    ]);
    assert_eq!(label_of(&held, "bug").0, "Ungated");
    assert!(held.left_out().is_empty());
}

#[test]
fn a_definition_resolved_on_its_own_is_the_repositorys() {
    let def = WorkflowDef::parse(&named("bug.yml"), &one_step("bug", "Bug", None), &roster())
        .expect("it parses");
    let resolved = ResolvedWorkflow::resolve(&def, &a_manifest()).expect("it resolves");
    assert_eq!(resolved.source(), WorkflowSource::Repository);
    assert_eq!(resolved.frozen().source(), WorkflowSource::Repository);
}
