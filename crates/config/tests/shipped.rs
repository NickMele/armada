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

use config::Roster;

/// The repository root, from this crate's manifest directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What this machine can run a Drone as, read from the adapter rather than
/// written out here.
///
/// **A list typed into this file would defeat the test it is part of.** A
/// shipped step naming a model the adapter does not offer would keep parsing
/// against the local copy and fail at spawn, where the Job already has a
/// worktree — which is the whole failure `config::Roster` exists to move
/// earlier. So the roster the parse is checked against is the roster the
/// running daemon resolves, and `adapters` is a dev-dependency for this one
/// call.
fn roster() -> Roster {
    Roster::of(adapters::HeadlessAgent::models())
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
        if let Err(why) = config::WorkflowDef::parse(&path, &text, &roster()) {
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
        let def = config::WorkflowDef::parse(&path, &text, &roster())
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
///
/// **An `artifact_exists` check does not put a step in this list either**, and
/// that is why the filter drops it rather than counting it as `named`. It
/// compiles nothing, so a written step that declares one would otherwise be
/// asked to gate on `typecheck`.
#[test]
fn a_step_that_compiles_the_rust_half_compiles_the_bridge_half() {
    const WANTED: [&str; 4] = ["format", "typecheck", "bridge_build", "storybook"];

    let mut guarded = 0;
    for (path, text) in shipped() {
        let def =
            config::WorkflowDef::parse(&path, &text, &roster()).expect("a shipped definition");
        for step in def.steps() {
            let named: Vec<&str> = step
                .mechanical_checks()
                .iter()
                .filter_map(|declared| match declared {
                    config::MechanicalCheck::ManifestCheck { check, .. } => Some(check.as_str()),
                    config::MechanicalCheck::DiffNonempty
                    | config::MechanicalCheck::ArtifactExists { .. } => None,
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

/// **A step whose evidence a later step reads writes a file that is checked.**
///
/// The record a step hands the next one used to be three prose strings, two of
/// which `verification::submission` says outright that nothing routes on. On a
/// step that writes files that is survivable, because the branch holds the work
/// and Fleet reads the real diff. On a step that writes nothing it is the whole
/// product, and it inverts `docs/concepts/drone.md`'s founding rule that a
/// Drone cannot be trusted to manage its own state.
///
/// **`reference_docs` is the machine-readable form of "a later step depends on
/// this one".** A step naming `plan.evidence` is a step that will be shown what
/// `plan` produced, so `plan` has to have produced something. Steps no later
/// step names are not covered: `handoff` is the last thing that happens and
/// nobody reads it, and an artifact requirement with no reader is a file
/// nothing opens.
#[test]
fn a_step_another_step_reads_from_produces_a_file() {
    let mut checked = 0;
    for (path, text) in shipped() {
        let def =
            config::WorkflowDef::parse(&path, &text, &roster()).expect("a shipped definition");
        let writes: Vec<&config::Step> = def
            .steps()
            .iter()
            .filter(|step| {
                step.mechanical_checks()
                    .iter()
                    .any(|check| matches!(check, config::MechanicalCheck::ArtifactExists { .. }))
            })
            .collect();
        for reader in def.steps() {
            let Some(scope) = reader.evidence_scope() else {
                continue;
            };
            for named in scope.reference_docs() {
                assert!(
                    writes.iter().any(|step| step.id() == named.step()),
                    "{}: step `{}` reads `{}`, and `{}` declares no `artifact_exists` — \
                     so what it hands on is a sentence its own Drone typed",
                    path.display(),
                    reader.id().as_str(),
                    named.as_wire(),
                    named.step().as_str()
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 4,
        "four steps read an earlier step's evidence, and {checked} were read"
    );
}

/// An `artifact_exists` target names one file, inside the worktree.
///
/// **v1 measured the cost of the alternative.** Its `design` workflow named
/// `docs/design/*.md` and its gate probed the target as a literal path, so the
/// step could never pass whatever the Drone wrote and the Job burned its token
/// ceiling retrying. The parser refuses a pattern now; this asserts the shipped
/// files do not carry one, so the refusal is never what somebody meets.
#[test]
fn every_shipped_artifact_target_is_one_path_in_the_worktree() {
    let mut seen = 0;
    for (path, text) in shipped() {
        let def =
            config::WorkflowDef::parse(&path, &text, &roster()).expect("a shipped definition");
        for step in def.steps() {
            for check in step.mechanical_checks() {
                let config::MechanicalCheck::ArtifactExists { target } = check else {
                    continue;
                };
                let at = format!("{} step `{}`", path.display(), step.id().as_str());
                assert!(
                    !target.contains('*') && !target.contains('?'),
                    "{at}: `{target}` globs"
                );
                assert!(!target.starts_with('/'), "{at}: `{target}` is absolute");
                assert!(
                    !target.split('/').any(|segment| segment == ".."),
                    "{at}: `{target}` climbs out of the worktree"
                );
                seen += 1;
            }
        }
    }
    assert!(
        seen >= 6,
        "six steps declare an artifact, and {seen} were read"
    );
}

/// **A shipped step names a model only where it wants a different one from the
/// Job's**, and the step that produces a diff is never the one that wants
/// less.
///
/// Two claims, and the second is why this is a test rather than a comment.
///
/// The first is that no step restates the fallback. A step naming no model is
/// run as the Job was proposed, so a step naming the Job's default says nothing
/// the absence did not — and it says it in a second place, which is a second
/// place to keep in sync when the default moves. The default is
/// `adapters::HeadlessAgent::default_model()`, read from the adapter here for
/// [`roster`]'s reason.
///
/// The second is that the dial only ever moves *down*. Owner's decision, 31 Aug
/// 2026: cheap where a step reports, the Job's model where a step decides. A
/// step producing a diff is deciding — `docs/concepts/judge.md` calls model "a
/// deliberate per-step dial" and #141 says outright that the dial exists so a
/// mechanical step stops paying for judgement it does not use, not so a step
/// that needs judgement gets less of it. Nothing in the parser can tell those
/// apart; this can, because `evidence_type: diff` is what a step producing one
/// declares.
#[test]
fn a_shipped_step_names_a_model_only_to_ask_for_less() {
    let default = adapters::HeadlessAgent::default_model();
    let mut named: Vec<String> = Vec::new();
    for (path, text) in shipped() {
        let def =
            config::WorkflowDef::parse(&path, &text, &roster()).expect("a shipped definition");
        for step in def.steps() {
            let Some(model) = step.model() else {
                continue;
            };
            let at = format!("{} step `{}`", path.display(), step.id().as_str());
            assert_ne!(
                model.as_str(),
                default,
                "{at} names `{default}`, which is what it would be run as \
                 anyway — an absent key says the same thing in one place \
                 instead of two"
            );
            assert_ne!(
                step.evidence_type(),
                Some(config::EvidenceType::Diff),
                "{at} produces a diff and names `{}`. The dial is for a step \
                 that reports, not for one that decides",
                model.as_str()
            );
            named.push(format!("{}.{}", def.id().as_str(), step.id().as_str()));
        }
    }
    named.sort();
    assert_eq!(
        named,
        vec![
            "bug.handoff",
            "code_review.deliver",
            "feature.handoff",
            "prototype.write_up",
            "refactor.handoff",
            "revert.handoff",
        ],
        "the six steps whose work is reporting. Every other shipped step \
         decides something or produces a diff, and is run as the Job was \
         proposed — a step joining or leaving this list is a spend decision \
         and belongs in the file's own header, not in a diff nobody reads"
    );
}
