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

/// **One shipped step may create Jobs, and this names which.**
///
/// This assertion read `no_shipped_workflow_grants_the_dispatch_tool_yet` until
/// `epic.json` was written, and the sentence under it said what was missing: the
/// grant, the tool and the loop all existed and no definition used any of them.
/// One does now, so the claim inverts rather than retiring — a set of exactly
/// one, spelled out, is what makes a *second* step acquiring the ability to
/// create Jobs a failing test rather than a Drone with an extra tool.
///
/// **Why `epic.dispatch` alone may.** It is the one step in the repository whose
/// product is other Jobs. Every other shipped step produces a diff, a note or a
/// document that a person or a Judge reads, and a wrong one costs a refusal; a
/// wrong dispatch costs Drones that run and spend. What makes it safe to grant
/// there and nowhere else is that the step before it is `human_always`: the plan
/// is read and approved, and the approval is what advances into this step. See
/// the file's own header.
///
/// The pair is asserted rather than the flag, because "the epic workflow grants
/// it" and "the epic workflow's dispatching step grants it" are different
/// claims, and the second is the one the design makes.
#[test]
fn epic_s_dispatch_step_is_the_only_shipped_step_that_may_create_jobs() {
    let mut granted: Vec<(String, String)> = Vec::new();
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for step in def.steps() {
            if step.may_dispatch_jobs() {
                let file = path
                    .file_name()
                    .expect("a shipped definition is a file")
                    .to_string_lossy()
                    .to_string();
                granted.push((file, step.id().as_str().to_string()));
            }
        }
    }
    assert_eq!(
        granted,
        vec![("epic.json".to_string(), "dispatch".to_string())],
        "exactly one shipped step creates Jobs, and it is the epic's dispatching step",
    );
}

/// **The grant is on the step after the one a person answers.** The placement is
/// the whole of `#215`'s gate decision and it is forced rather than chosen: an
/// `advance_gate` is read after a step's Drone has submitted, so `human_always`
/// on the dispatching step itself would be a person approving Jobs that already
/// exist and are already spending.
///
/// Asserted off the file rather than trusted to its header, because the two keys
/// are one intent written on two steps and nothing else in the workspace pairs
/// them.
#[test]
fn the_step_before_the_epic_s_dispatch_is_the_one_a_person_answers() {
    let path = root().join(".armada/workflows/epic.json");
    let text = std::fs::read_to_string(&path).expect("a readable definition");
    let def = config::WorkflowDef::parse(&path, &text, &roster())
        .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    let steps = def.steps();
    let at = steps
        .iter()
        .position(|step| step.may_dispatch_jobs())
        .expect("the epic dispatches somewhere");
    let before = at
        .checked_sub(1)
        .and_then(|earlier| steps.get(earlier))
        .expect("the dispatching step is not the first");
    assert_eq!(
        before.advance_gate(),
        config::AdvanceGate::HumanAlways,
        "the step before the dispatch is `{}`, and a person has to answer it",
        before.id().as_str(),
    );
    assert_eq!(
        steps[at].advance_gate(),
        config::AdvanceGate::Auto,
        "and the dispatching step itself asks nobody, because the answer was given already",
    );
}

/// The key is real on any definition, not only on the one that ships with it.
/// **Parsed rather than asserted against `epic.json`**, which is the file the
/// two tests above read: this one is about the parser taking the key wherever it
/// is written, and reading it off the same file would make three assertions of
/// one file's contents and none of the language.
#[test]
fn a_definition_may_grant_the_dispatch_tool() {
    let text = "version: 1\nworkflow_id: grants\nname: grants\nstructure: linear\n\
                steps:\n  - id: split\n    label: \"Split\"\n    \
                evidence_type: facts_note\n    may_dispatch_jobs: true\n    \
                advance_gate: auto\n";
    let def = config::WorkflowDef::parse(Path::new("grants.yml"), text, &roster())
        .expect("a definition may say a step creates Jobs");
    assert!(def.steps()[0].may_dispatch_jobs());
}

/// A value that is not a boolean is refused rather than read as `false`. A step
/// written to create Jobs that silently cannot is a Job that goes quiet, which
/// is the hardest failure here to see.
#[test]
fn a_dispatch_grant_that_is_not_a_boolean_is_refused() {
    let text = "version: 1\nworkflow_id: grants\nname: grants\nstructure: linear\n\
                steps:\n  - id: split\n    label: \"Split\"\n    \
                evidence_type: facts_note\n    may_dispatch_jobs: dispatches\n    \
                advance_gate: auto\n";
    assert!(config::WorkflowDef::parse(Path::new("grants.yml"), text, &roster()).is_err());
}
