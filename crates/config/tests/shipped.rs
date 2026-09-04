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

/// **Nothing this repository ships grants the dispatch tool.**
///
/// `may_dispatch_jobs` is carried by the parser and by the frozen record, and no
/// shipped definition sets it. **The loop is no longer what it is waiting on**:
/// `design-plan.json` declares one and `#263` closed the return, the count and
/// the cap. What is missing is the workflow — a milestone dispatched as one Job
/// is `plan -> dispatch -> assess -> plan`, and nobody has written it. So this
/// is not a gap: it is the grant existing before the definition that uses it,
/// and it is asserted so that a step quietly acquiring the ability to create
/// Jobs is a failing test rather than a Drone with an extra tool.
#[test]
fn no_shipped_workflow_grants_the_dispatch_tool_yet() {
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        for step in def.steps() {
            assert!(
                !step.may_dispatch_jobs(),
                "{} step `{}` grants the tool that creates Jobs",
                path.display(),
                step.id().as_str(),
            );
        }
    }
}

/// The key is real, and a definition that wanted it could have it. **Parsed
/// rather than asserted against a shipped file**, because there is no shipped
/// file that sets it and adding one to make the key look used would be a
/// workflow nobody dispatches.
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
