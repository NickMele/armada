//! What this repository's own setup says, checked against what it has to say.
//!
//! **No daemon is started here.** `Setup::at` is a read and a resolve over two
//! files, which is exactly the part of starting Fleet that can be wrong on
//! disk — everything after it needs a port, a store and a process. Whether the
//! five operations answer is asserted in `fleet`'s own suite, over the router,
//! with no socket.
//!
//! # These are the tests that stop the two files rotting
//!
//! `armada.yml` and `.armada/workflows/bug.json` are read by nothing else in
//! the workspace. Without a test over them, a Check renamed in one and not the
//! other is a daemon that refuses to start, discovered by whoever next tried to
//! start it.

use std::path::{Path, PathBuf};

use config::{Fault, LoadError, ResolvedCheck, WorkflowDef};

use crate::setup::{Setup, MANIFEST, WORKFLOWS};

/// The workspace root, from the crate that is being compiled.
///
/// Derived rather than searched for: `CARGO_MANIFEST_DIR` is set by cargo and
/// is exact, where a walk upward for an `armada.yml` would find whichever one
/// came first and pass on a machine that had one somewhere else.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/fleet-bin sits two directories below the workspace root")
        .to_path_buf()
}

/// **The whole claim of this step, over the real files.** Fleet is pointed at a
/// repository and the repository's setup is enough to build a workflow that can
/// be dispatched.
#[test]
fn this_repositorys_own_setup_loads_and_resolves() {
    let setup = match Setup::at(&repository()) {
        Ok(setup) => setup,
        Err(refused) => panic!("{} and {WORKFLOWS} must load:\n{refused}", MANIFEST),
    };

    assert_eq!(
        setup.manifest().check_names(),
        vec!["build".to_string(), "test".to_string()],
        "the two Checks this workspace is built and tested with"
    );
    assert_eq!(setup.workflow().name(), "bug");
    let steps: Vec<&str> = setup
        .workflow()
        .steps()
        .iter()
        .map(|step| step.id().as_str())
        .collect();
    assert_eq!(
        steps,
        vec!["plan", "implement", "verify", "handoff"],
        "four steps, which is M1's reduced form of the designed Bug workflow"
    );
}

/// **The Checks the workflow names are the Checks the Manifest declares**, and
/// the command each resolved to is the command that will run.
///
/// Resolution having succeeded above already proves the names matched; this
/// asserts what they matched *to*, because a Check renamed in one file and left
/// in the other resolves to a command nobody meant.
#[test]
fn each_named_check_resolved_to_the_command_the_manifest_holds() {
    let setup = Setup::at(&repository()).expect("a setup that loads");
    let resolved: Vec<(&str, &str)> = setup
        .workflow()
        .steps()
        .iter()
        .flat_map(|step| step.checks())
        .filter_map(|check| match check {
            ResolvedCheck::ManifestCheck { name, run, .. } => Some((name.as_str(), run.as_str())),
            ResolvedCheck::DiffNonempty => None,
        })
        .collect();

    assert_eq!(
        resolved,
        vec![
            ("build", "cargo build --workspace --locked"),
            ("test", "cargo nextest run --workspace --exclude acceptance"),
        ]
    );
}

/// **`--exclude acceptance` is load-bearing and is asserted as such.**
///
/// The acceptance test is required to fail for the whole of M1, so a plain
/// `--workspace` test command would report red for the one reason that is
/// deliberate. Dropping the exclusion would leave a Manifest that still parses
/// and a `verify` step that can never pass.
#[test]
fn the_test_check_excludes_the_crate_that_must_not_compile() {
    let setup = Setup::at(&repository()).expect("a setup that loads");
    let test = setup.manifest().check("test").expect("a `test` Check");
    assert!(
        test.run().contains("--exclude acceptance"),
        "the test Check must not run the acceptance crate: {}",
        test.run()
    );
}

/// **The designed Bug workflow is refused, and the reason is that M1 is small
/// rather than that the file is wrong.**
///
/// `crates/core-model/domain/workflow-samples/bug.json` is the authority on
/// what Bug becomes: seven steps, a Judge on every gate, and a `review` step
/// that routes `request_changes` back to `fix`. It declares `structure: "loop"`
/// and says so correctly. M1 carries one of the two structures, so the refusal
/// is `OutsideM1` — a value the schema sanctions that this milestone has not
/// built, whose fix is a later milestone rather than an edit.
///
/// That distinction is why the two `bug.json` files in this repository are not
/// duplicates and must not be reconciled, and it is asserted rather than left
/// to a comment: a refusal that turned into `NotInTheSchema` would mean
/// somebody had changed the designed definition to make it load here.
#[test]
fn the_designed_bug_workflow_is_refused_for_a_reason_a_later_milestone_removes() {
    let designed = repository()
        .join("crates/core-model/domain/workflow-samples")
        .join("bug.json");
    let refused = WorkflowDef::load(&designed).expect_err("a loop, and M1 carries linear");
    let LoadError::Refused { refusals, .. } = &refused else {
        panic!("a document that parsed and was refused, not {refused}");
    };
    let structure = refusals
        .iter()
        .find(|refusal| refusal.key == "structure")
        .unwrap_or_else(|| panic!("`structure` is refused: {refusals:?}"));
    assert!(
        matches!(
            &structure.fault,
            Fault::OutsideM1 { value, .. } if value == "loop"
        ),
        "deferred, not wrong: {:?}",
        structure.fault
    );
}

/// The two definitions are different files with different scope, and nothing
/// reconciles them by accident.
///
/// M1's reduced form has four steps because M1 has no Judge to answer
/// `auto_if_judge_passes` and no verdict to route on. The designed one has
/// seven and loops. A change that made the two the same length would mean one of
/// them had been quietly rewritten into the other.
#[test]
fn the_designed_definition_and_m1s_reduced_form_are_not_the_same_workflow() {
    let setup = Setup::at(&repository()).expect("a setup that loads");
    assert_eq!(setup.workflow().steps().len(), 4);

    let designed = repository()
        .join("crates/core-model/domain/workflow-samples")
        .join("bug.json");
    let text = std::fs::read_to_string(&designed).expect("the designed definition is readable");
    // Counted rather than parsed, because the definition does not load at M1 —
    // which is the fact the test above is about.
    let steps = text.matches("\"id\":").count();
    assert_eq!(
        steps, 7,
        "repro, root_cause, fix, regression_verify, review, merge, close"
    );
}

// ------------------------------------------- which binary a Drone is started as
//
// **No process is started here either.** Whether a name resolves is a question
// about a filesystem and a `PATH`, and the whole point of asking it in this
// crate is that it is answered before the bind rather than at the first Drone.
//
// Nothing below writes the agent CLI's name. It is a vendor's and `adapters`
// owns it; these compare against `HeadlessAgent::on_path()`, which is the same
// value `agent_binary` reaches for — so the settings default can change without
// a test here having to be edited to agree with it.

use adapters::HeadlessAgent;

use crate::agent::agent_binary;

/// **The unset case is the ordinary case.** A machine with the agent CLI
/// installed the ordinary way starts a Fleet with no environment variable set,
/// and the harness it gets is the settings default.
#[test]
fn a_fleet_with_no_named_agent_binary_gets_the_settings_default() {
    let harness = agent_binary(None, "/usr/bin:/bin").expect("unset is not a refusal");
    assert_eq!(harness, HeadlessAgent::on_path());
}

/// And the default is not probed. `PATH` is empty here, which is a `PATH` the
/// CLI cannot be on — the answer to "is it installed" is Doctor's, and making
/// it a start-up precondition would refuse a Fleet whose Drone would have found
/// it anyway.
#[test]
fn the_default_is_not_probed_against_the_path() {
    assert!(agent_binary(None, "").is_ok());
}

/// A binary somebody named that is not there is refused, **and the refusal
/// carries the name** — the one thing the reader has to go and fix.
#[test]
fn a_named_binary_that_is_not_there_is_refused_by_name() {
    let refused = agent_binary(Some("no-such-agent-binary".to_string()), "/usr/bin:/bin")
        .expect_err("a name with nothing behind it is a refusal");
    let said = refused.to_string();
    assert!(
        said.contains("no-such-agent-binary"),
        "the refusal names what was named: {said}"
    );
    assert!(
        said.contains("ARMADA_AGENT_BINARY"),
        "and where the name came from: {said}"
    );
}

/// A path, rather than a bare name, is answered by that one place.
#[test]
fn a_named_path_with_nothing_at_it_is_refused_whatever_the_path_holds() {
    let refused = agent_binary(Some("/nowhere/at/all/agent".to_string()), "/usr/bin:/bin")
        .expect_err("a path to nothing is a refusal");
    assert!(refused.to_string().contains("/nowhere/at/all/agent"));
}

/// The override that resolves. Two spellings of one binary this machine
/// certainly has — a bare name found on the `PATH`, and the same file named
/// outright.
#[test]
fn a_named_binary_that_is_there_is_the_one_fleet_uses() {
    assert_eq!(
        agent_binary(Some("sh".to_string()), "/nowhere:/bin").expect("`sh` is on `/bin`"),
        HeadlessAgent::at("sh")
    );
    assert_eq!(
        agent_binary(Some("/bin/sh".to_string()), "").expect("`/bin/sh` is a file that runs"),
        HeadlessAgent::at("/bin/sh")
    );
}

/// A directory is not runnable, and neither is a file with no execute bit. Both
/// are names that would fail at spawn, which is the failure this probe exists to
/// move to the terminal the operator is standing at.
#[test]
fn something_that_is_not_runnable_is_not_an_agent() {
    assert!(agent_binary(Some("/bin".to_string()), "").is_err());
    assert!(agent_binary(Some("/etc/hosts".to_string()), "").is_err());
}
