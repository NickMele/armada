//! Every workflow definition this repository ships parses, and resolves
//! against this repository's own `armada.yml`.
//!
//! An integration test rather than a unit one because the subject is the files
//! in `.armada/workflows/` and the file beside them, not the parser: a
//! definition that stops loading is a Fleet that cannot dispatch, and nothing
//! else in the workspace reads them.
//!
//! The four ways a definition has gone wrong so far were all silent until a Job
//! hit them — a key the parser defers, a gate disagreeing with its judge checks,
//! a `question` where the parser reads `criteria[]`, and a Judge asked something
//! on a step that produces nothing.
//!
//! # The fifth way, which had no test until #200
//!
//! **Parsing is not resolving.** A step may name a Check spelled correctly,
//! shaped correctly and declared nowhere, and [`config::WorkflowDef::parse`]
//! takes it — the cross-file question is [`config::ResolvedWorkflow`]'s and is
//! asked at dispatch. So the one edit this pair invites, adding a Check to a
//! step and forgetting the Manifest, was caught by nothing here and by
//! everything at the worktree.
//!
//! The second test below asks it. It is deliberately the *shipped* Manifest and
//! not a fixture: a fixture declaring `build` and `test` would keep passing
//! while `armada.yml` lost them.

use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every shipped definition, as `(path, text)`, sorted so a failure names the
/// same file on every machine.
fn shipped() -> Vec<(PathBuf, String)> {
    let dir = root().join(".armada/workflows");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the shipped definitions are there")
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("a readable definition");
            (path, text)
        })
        .collect()
}

#[test]
fn every_shipped_workflow_definition_parses() {
    let mut seen = 0;
    for (path, text) in shipped() {
        if let Err(why) = config::WorkflowDef::parse(&path, &text) {
            panic!("{} is refused:\n{why}", path.display());
        }
        seen += 1;
    }
    assert!(seen >= 7, "seven workflows ship, and {seen} were read");
}

#[test]
fn every_shipped_workflow_resolves_against_this_repositorys_manifest() {
    let manifest_path = root().join("armada.yml");
    let manifest = config::Manifest::load(&manifest_path)
        .unwrap_or_else(|why| panic!("{} is refused:\n{why}", manifest_path.display()));

    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text)
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        if let Err(why) = config::ResolvedWorkflow::resolve(&def, &manifest) {
            panic!("{} does not resolve:\n{why}", path.display());
        }
    }
}

/// The Checks the Bridge half is verified by, named where a reader looking for
/// them will look.
///
/// **Not a restatement of `armada.yml` for its own sake.** #200 is a Manifest
/// that declared only Rust Checks and workflows that therefore named only Rust
/// Checks, and the failure was that both halves of that were internally
/// consistent. This asserts the property the pair is supposed to have — that
/// something compiles the TypeScript — which neither file can assert about
/// itself.
#[test]
fn the_manifest_declares_a_check_for_each_half_of_the_repository() {
    let manifest_path = root().join("armada.yml");
    let manifest = config::Manifest::load(&manifest_path).expect("the shipped manifest");
    for name in [
        "build",
        "test",
        "format",
        "typecheck",
        "bridge_build",
        "storybook",
    ] {
        assert!(
            manifest.check(name).is_some(),
            "`{name}` is not declared; {} has {:?}",
            manifest_path.display(),
            manifest.check_names()
        );
    }
}

/// Every step that produces a diff and gates on anything gates on the Bridge
/// and on formatting too.
///
/// **`format` is here for the same reason the three Bridge Checks are.** PR
/// #199 merged nine unformatted files past both gates and both Judges, because
/// `cargo fmt --check` was not a Check — one lint over from the renderer that
/// did not compile. There is no `clippy` in this list: `armada.yml`'s `format`
/// entry says why, and it is a person's answer rather than an omission.
///
/// **The rule is stated as steps that already carry a Manifest Check**, not as
/// steps whose evidence is a diff. `prototype`'s `build` produces a diff and
/// carries no mechanical tier at all, by a decision recorded in its own header;
/// reading it as a violation would make this test an argument against that
/// decision rather than a guard on this one.
#[test]
fn a_step_that_compiles_the_rust_half_compiles_the_bridge_half() {
    const WANTED: [&str; 4] = ["format", "typecheck", "bridge_build", "storybook"];

    let mut guarded = 0;
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text).expect("a shipped definition");
        for step in def.steps() {
            let named: Vec<&str> = step
                .mechanical_checks()
                .iter()
                .filter_map(|declared| match declared {
                    config::MechanicalCheck::ManifestCheck { check, .. } => Some(check.as_str()),
                    config::MechanicalCheck::DiffNonempty => None,
                })
                .collect();
            if named.is_empty() {
                continue;
            }
            for wanted in WANTED {
                assert!(
                    named.contains(&wanted),
                    "{} step `{}` gates on {named:?} and not on `{wanted}`, so a diff \
                     that breaks the Bridge, or that is not formatted, advances past it",
                    path.display(),
                    step.id().as_str()
                );
            }
            guarded += 1;
        }
    }
    assert!(
        guarded >= 5,
        "five steps carry Manifest Checks, and {guarded} were read"
    );
}
