//! `setup.seed`: what a new worktree's build directories start from. #1064.

use crate::amending::{amend, Edit};
use crate::error::Fault;
use crate::manifest::{BadSeedPath, Manifest};
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

const COMMANDS: &str = "version: 1\nid: a\ncommands:\n  warm:\n    run: cargo build\n  \
                        tests:\n    run: cargo test --no-run\n  install:\n    run: pnpm install\n";

fn with_setup(setup: &str) -> String {
    format!("{COMMANDS}setup:\n{setup}")
}

#[test]
fn a_seed_arrives_with_its_paths_and_its_warm_up_resolved_in_order() {
    let manifest = parse(&with_setup(
        "  requires: [install]\n  seed:\n    paths: [target/, build]\n    warm: [tests, warm]\n",
    ))
    .expect("a seed");
    let seed = manifest.seed().expect("declared");
    assert_eq!(
        seed.paths(),
        ["target", "build"],
        "a trailing `/` is dropped"
    );
    let warm: Vec<(&str, &str)> = seed
        .warmed_by()
        .iter()
        .map(|one| (one.name(), one.run()))
        .collect();
    assert_eq!(
        warm,
        [("tests", "cargo test --no-run"), ("warm", "cargo build")]
    );
    assert_eq!(manifest.prepared_by().len(), 1);
}

#[test]
fn a_repository_that_declares_no_seed_has_none() {
    let manifest = parse(&with_setup("  requires: [install]\n")).expect("setup alone");
    assert!(manifest.seed().is_none());
}

#[test]
fn a_seed_needs_no_requires_beside_it() {
    let manifest = parse(&with_setup(
        "  seed:\n    paths: [target]\n    warm: [warm]\n",
    ))
    .expect("a seed and nothing to prepare");
    assert!(manifest.prepared_by().is_empty());
    assert!(manifest.seed().is_some());
}

#[test]
fn a_setup_with_neither_key_is_still_refused_at_requires() {
    let refused = refusals(parse(&with_setup("  {}\n")));
    assert!(matches!(
        fault_at(&refused, "setup.requires"),
        Fault::Missing
    ));
}

#[test]
fn both_seed_keys_are_required() {
    let refused = refusals(parse(&with_setup("  seed:\n    paths: [target]\n")));
    assert!(matches!(
        fault_at(&refused, "setup.seed.warm"),
        Fault::Missing
    ));
    let refused = refusals(parse(&with_setup("  seed:\n    warm: [warm]\n")));
    assert!(matches!(
        fault_at(&refused, "setup.seed.paths"),
        Fault::Missing
    ));
}

#[test]
fn a_warm_up_resolves_against_the_commands_as_requires_does() {
    let refused = refusals(parse(&with_setup(
        "  seed:\n    paths: [target]\n    warm: [nope]\n",
    )));
    assert!(matches!(
        fault_at(&refused, "setup.seed.warm[0]"),
        Fault::NotADeclaredCommand { value, .. } if value == "nope"
    ));
}

#[test]
fn a_path_that_names_no_build_directory_is_refused_and_says_why() {
    let cases = [
        ("/tmp/target", BadSeedPath::Absolute),
        ("../target", BadSeedPath::Escapes),
        ("target/*", BadSeedPath::Globbed),
        (".", BadSeedPath::TheWholeTree),
        (".git/objects", BadSeedPath::NotTheRepositorys),
        (".armada", BadSeedPath::NotTheRepositorys),
    ];
    for (path, expected) in cases {
        let refused = refusals(parse(&with_setup(&format!(
            "  seed:\n    paths: [\"{path}\"]\n    warm: [warm]\n"
        ))));
        assert!(
            matches!(
                fault_at(&refused, "setup.seed.paths[0]"),
                Fault::NotASeedPath { why, .. } if *why == expected
            ),
            "`{path}` was not refused as {expected:?}: {refused:?}"
        );
    }
}

#[test]
fn one_directory_named_twice_is_refused() {
    let refused = refusals(parse(&with_setup(
        "  seed:\n    paths: [target, target/]\n    warm: [warm]\n",
    )));
    assert!(matches!(
        fault_at(&refused, "setup.seed.paths[1]"),
        Fault::NotASeedPath {
            why: BadSeedPath::Twice,
            ..
        }
    ));
}

#[test]
fn clearing_requires_in_a_form_keeps_the_seed() {
    let text =
        with_setup("  requires: [install]\n  seed:\n    paths: [target]\n    warm: [warm]\n");
    let done = amend(
        &named("armada.yml"),
        &text,
        &[Edit::SetupRequires(Vec::new())],
    )
    .expect("the edit applies");
    let loaded = parse(done.text()).expect("the edited file loads");
    assert!(loaded.prepared_by().is_empty());
    assert_eq!(loaded.seed().expect("still seeded").paths(), ["target"]);
}

/// The file every live Job in this repository is gated by.
#[test]
fn this_repositorys_own_manifest_seeds_target_warmed_by_a_build_and_a_test_build() {
    let own = parse(include_str!("../../../../../armada.yml")).expect("armada.yml loads");
    let seed = own.seed().expect("this repository declares a seed");
    assert_eq!(seed.paths(), ["target"]);
    let runs: Vec<&str> = seed.warmed_by().iter().map(|one| one.run()).collect();
    assert_eq!(
        runs,
        [
            "cargo build --workspace --locked",
            "cargo nextest run --workspace --no-run --locked"
        ]
    );
}
