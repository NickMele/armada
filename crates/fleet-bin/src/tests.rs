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
