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

/// **Which shipped workflows send their work out, read off the files.**
///
/// Four of the eight produce a diff somebody merges and four produce something
/// somebody reads. Until a workflow could say so, all eight pushed a branch and
/// opened a pull request for whatever was in the worktree — and the only thing
/// standing between a design document and a pull request was this repository's
/// `.gitignore` line over `.armada/`.
///
/// Asserted off the files rather than trusted to their headers, because this is
/// the one property of the set that a single edit to one file can break for
/// everybody: a workflow that starts delivering opens pull requests for
/// documents, and one that stops leaves a Job's diff on a branch nobody sees.
#[test]
fn four_shipped_workflows_send_their_work_out_and_four_do_not() {
    let mut delivering: Vec<(String, String)> = Vec::new();
    let mut silent: Vec<String> = Vec::new();
    for (path, text) in shipped() {
        let def = config::WorkflowDef::parse(&path, &text, &roster())
            .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
        let sends: Vec<&config::Step> = def.steps().iter().filter(|s| s.delivers()).collect();
        match sends.as_slice() {
            [] => silent.push(def.id().as_str().to_string()),
            [step] => delivering.push((
                def.id().as_str().to_string(),
                step.id().as_str().to_string(),
            )),
            _ => panic!("{} sends its work out more than once", path.display()),
        }
    }
    delivering.sort();
    silent.sort();
    assert_eq!(
        delivering,
        vec![
            (String::from("bug"), String::from("handoff")),
            (String::from("feature"), String::from("handoff")),
            (String::from("refactor"), String::from("handoff")),
            (String::from("revert"), String::from("handoff")),
        ],
        "the four that produce a diff deliver, each on its last step",
    );
    assert_eq!(
        silent,
        vec!["code_review", "design_plan", "epic", "prototype"],
        "and the four that produce something read rather than merged deliver nothing",
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
                evidence: {submitted: {type: facts_note}}\n    may_dispatch_jobs: true\n    \
                delivers: false\n    advance_gate: auto\n";
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
                evidence: {submitted: {type: facts_note}}\n    may_dispatch_jobs: dispatches\n    \
                delivers: false\n    advance_gate: auto\n";
    assert!(config::WorkflowDef::parse(Path::new("grants.yml"), text, &roster()).is_err());
}

/// **This repository's own `armada.yml` loads**, which nothing asked until now.
///
/// The header above has claimed since `#200` that these definitions resolve
/// against the shipped Manifest, and no test here read that file — so a typo in
/// it was found by starting a daemon, and by nothing before that. `#414` gave it
/// a `drone:` section and made the gap worth closing rather than only worth
/// naming.
///
/// **The value is asserted and not only the parse.** A `quiet_after_seconds`
/// silently dropped by a parser that stopped reading the key would leave this
/// file loading exactly as well as before, and every Drone here back on a
/// threshold shorter than one of its own commands.
#[test]
fn this_repositorys_own_manifest_loads_and_states_its_patience() {
    let path = root().join("armada.yml");
    let manifest = config::Manifest::load(&path)
        .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    assert_eq!(
        manifest.quiet_after_seconds(),
        Some(300),
        "the repository's own patience is what its `drone:` section writes"
    );
    // Nothing here has a reason to want more or fewer nudges than Fleet's, and
    // the two halves fall back separately — so an absent one is the assertion.
    assert_eq!(manifest.poke_limit(), None);
}

/// **The workflow that runs milestones says so, in the words of somebody
/// asking for one.** #424: a request to finish a milestone was declined with
/// `epic` on the list, because nothing the proposer was shown about it was a
/// word the request used.
///
/// Asserted off the file, and against the steps' vocabulary as well as for the
/// requester's: a line that restates `Plan the wave -> Dispatch the wave` is
/// the defect at greater length. `code_review` is asserted beside it since
/// `#1379`, for the same reason and against its own steps' words; a definition
/// that declares nothing is still legal, and the other six do not yet.
#[test]
fn the_epic_says_it_is_for_a_milestone_in_a_requesters_words() {
    let what_for = says_what_it_is_for("epic.json");
    assert!(what_for.contains("milestone"), "{what_for}");
    for steps_word in ["wave", "roll up", "dispatch"] {
        assert!(
            !what_for.to_lowercase().contains(steps_word),
            "`{steps_word}` is the steps' word, not a requester's: {what_for}"
        );
    }
}

/// **A pull request arrives as a link and nothing else, so `code_review` has
/// to say it is the one for a link to one.** `#1379`.
///
/// A Studio dispatches a Link naming a pull request as its address, and Fleet
/// resolves no text behind one — `adapters::IssueLookup` reads an issue link
/// and no other shape — so the proposer is choosing off a URL and eight step
/// lists. `Read the diff -> Assess -> Deliver the review` is this workflow's
/// own vocabulary and says nothing about whether a request is this kind of
/// work, which is exactly what `#424` measured about `epic` above.
#[test]
fn code_review_says_it_is_for_a_pull_request_in_a_requesters_words() {
    let what_for = says_what_it_is_for("code-review.json");
    assert!(what_for.contains("pull request"), "{what_for}");
    for steps_word in ["diff", "assess", "deliver"] {
        assert!(
            !what_for.to_lowercase().contains(steps_word),
            "`{steps_word}` is the steps' word, not a requester's: {what_for}"
        );
    }
}

/// The `for_requests` line one shipped definition declares, off the file.
fn says_what_it_is_for(file: &str) -> String {
    let path = root().join(".armada/workflows").join(file);
    let text = std::fs::read_to_string(&path).expect("a readable definition");
    let def = config::WorkflowDef::parse(&path, &text, &roster())
        .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
    def.for_requests()
        .unwrap_or_else(|| panic!("{file} says what requests it is for"))
        .to_string()
}

/// **What the proposer is handed for each of the three addresses a Studio
/// dispatches**, off the shipped catalogue rather than a fixture — `#1379`.
///
/// It measures everything up to the model call and nothing past it: which
/// workflow is chosen is a model's answer, and this file spawns none. What it
/// holds is the half that was wrong before — that the catalogue offers a line
/// a person asking about a pull request or a milestone would have used, and
/// that the other six offer nothing to be matched on by accident.
#[test]
fn a_forge_link_is_offered_a_line_written_in_the_words_somebody_asking_would_use() {
    let declared: Vec<(String, Option<String>)> = shipped()
        .into_iter()
        .map(|(path, text)| {
            let def = config::WorkflowDef::parse(&path, &text, &roster())
                .unwrap_or_else(|why| panic!("{} is refused:\n{why}", path.display()));
            (
                def.id().as_str().to_string(),
                def.for_requests().map(str::to_string),
            )
        })
        .collect();
    let saying: Vec<&str> = declared
        .iter()
        .filter(|(_, line)| line.is_some())
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        saying,
        ["code_review", "epic"],
        "exactly the two a bare forge link has to reach say what they are for"
    );
}
