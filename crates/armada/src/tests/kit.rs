//! Kit's Workflows, as `Setup::at` reads them from Kit's home. #425.
//!
//! Beside `setup.rs` rather than in it: that file is about a repository's own
//! files, and these are about the one directory that is not the repository's.

use config::WorkflowSource;

use crate::setup::{kit, Setup, KIT_HOME, KIT_WORKFLOWS};
use crate::tests::setup::{a_repository, a_workflow, bug, roster, sources};
use crate::tests::TempDir;

/// **One file replaces a carried definition by id**, from Kit or from the
/// repository, and the repository's replaces Kit's.
#[test]
fn kit_replaces_what_armada_carries_and_the_repository_replaces_kit() {
    let dir = a_repository();
    let kit = TempDir::new();
    kit.write(&format!("{KIT_WORKFLOWS}/bug.yml"), &a_workflow("bug"));
    kit.write(
        &format!("{KIT_WORKFLOWS}/hotfix.yml"),
        &a_workflow("hotfix"),
    );
    dir.write(".armada/workflows/hotfix.yml", &a_workflow("hotfix"));

    let setup = Setup::at(dir.path(), kit.path(), &roster()).expect("three places merge");
    let held = sources(&setup);
    for expected in [
        ("bug", WorkflowSource::Kit),
        ("feature", WorkflowSource::Armada),
        ("hotfix", WorkflowSource::Repository),
    ] {
        assert!(held.contains(&expected), "{expected:?} in {held:?}");
    }
    assert_eq!(
        bug(&setup).steps().len(),
        1,
        "Kit's one-step bug, not Armada's"
    );
}

/// **A Kit definition that does not fit is left out and named, and Fleet
/// starts anyway** — one that will not parse, and one gating on a Check this
/// repository lacks. The owner's decision. Armada's `bug` answers for the id.
#[test]
fn a_kit_definition_that_does_not_fit_is_left_out_and_named() {
    let dir = a_repository();
    let kit = TempDir::new();
    kit.write(
        &format!("{KIT_WORKFLOWS}/broken.yml"),
        "version: 1\nworkflow_id: broken\n",
    );
    kit.write(
        &format!("{KIT_WORKFLOWS}/bug.yml"),
        "version: 1\nworkflow_id: bug\nname: bug\nstructure: linear\nsteps:\n  - id: only\n    \
         label: Only\n    evidence: {submitted: {type: diff}}\n    delivers: true\n    \
         advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: build }\n",
    );

    let setup = Setup::at(dir.path(), kit.path(), &roster()).expect("starts anyway");
    assert_eq!(bug(&setup).source(), WorkflowSource::Armada);
    let said: Vec<String> = setup.left_out().iter().map(ToString::to_string).collect();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said.iter().any(|line| line.contains("broken.yml")),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|line| line.starts_with("workflow `bug` from Kit") && line.contains("build")),
        "{said:?}"
    );
}

/// Kit's home is made where it was not, with somewhere for Workflows to go, and
/// making it again is no act at all.
#[test]
fn kits_home_is_made_with_a_place_for_workflows() {
    let home = TempDir::new();
    let first = kit(home.path()).expect("made");
    assert_eq!(first, home.path().join(KIT_HOME));
    assert!(first.join(KIT_WORKFLOWS).is_dir());
    assert_eq!(kit(home.path()).expect("already there"), first);
}
