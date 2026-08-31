//! What this repository's own setup says, checked against what it has to say.
//!
//! **No daemon is started here.** `Setup::at` is a read and a resolve over two
//! files, which is exactly the part of starting Fleet that can be wrong on
//! disk — everything after it needs a port, a store and a process. Whether the
//! five operations answer is asserted in `fleet`'s own suite, over the router,
//! with no socket.
//!
//! # These are the tests that stop the real files rotting
//!
//! `armada.yml` and every definition in `.armada/workflows/` are read by
//! nothing else in the workspace. Without a test over them, a Check renamed in
//! one and not the other is a daemon that refuses to start, discovered by
//! whoever next tried to start it.

use config::{Fault, LoadError, ResolvedCheck, Roster, WorkflowDef};

use crate::setup::{Setup, SetupRefused, MANIFEST, WORKFLOWS};
use crate::tests::{repository, TempDir};

/// What this machine can run a Drone as, resolved the way `serve` resolves it.
///
/// **Read through [`crate::model_choices`] rather than written out**, so these
/// tests check the shipped definitions against the roster the daemon would
/// actually use — a list typed here would go on passing after the adapter's
/// changed.
fn roster() -> Roster {
    Roster::of(crate::model_choices(None).models)
}

/// A repository with an `armada.yml` that declares no Checks, so a workflow
/// gated on nothing resolves against it without also having to write a Check.
fn a_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\n");
    dir
}

/// A minimal, legal, one-step definition — gated on nothing, so it resolves
/// against a Manifest that declares no Checks.
fn a_workflow(id: &str) -> String {
    format!(
        "version: 1\nworkflow_id: {id}\nname: {id}\nstructure: linear\nsteps:\n  - id: only\n    \
         label: \"Only step\"\n    advance_gate: auto\n"
    )
}

/// Bug, this repository's one workflow the tests below name by hand. The
/// other six live beside it and are not this file's business.
fn bug(setup: &Setup) -> &config::ResolvedWorkflow {
    setup
        .workflows()
        .get(&core_model::WorkflowId::carried(core_model::Ulid::carried(
            "bug",
        )))
        .expect("bug.json declares workflow_id `bug`")
}

/// **The whole claim of this step, over the real files.** Fleet is pointed at a
/// repository and the repository's setup is enough to build a workflow that can
/// be dispatched.
#[test]
fn this_repositorys_own_setup_loads_and_resolves() {
    let setup = match Setup::at(&repository(), &roster()) {
        Ok(setup) => setup,
        Err(refused) => panic!("{} and {WORKFLOWS} must load:\n{refused}", MANIFEST),
    };

    // Sorted, because `check_names` walks a `BTreeMap` — so `bridge_build`
    // leads and the reading order of the file is not the reading order here.
    assert_eq!(
        setup.manifest().check_names(),
        vec![
            "bridge_build".to_string(),
            "build".to_string(),
            "format".to_string(),
            "storybook".to_string(),
            "test".to_string(),
            "typecheck".to_string(),
        ],
        "the six Checks this workspace is built and tested with — two for the \
         Rust half and three for the Bridge, which is #200: every Check used to \
         compile Rust, so a Job that changed only `apps/` was verified entirely \
         on the code it had not touched. `format` is the sixth and is the same \
         defect one lint over: PR #199 also merged nine unformatted files, \
         because `cargo fmt --check` was not a Check. There is no `clippy` — \
         `[clippy-as-a-check]` in `docs/OPEN.md` says why"
    );
    assert_eq!(bug(&setup).name(), "bug");
    let steps: Vec<&str> = bug(&setup)
        .steps()
        .iter()
        .map(|step| step.id().as_str())
        .collect();
    assert_eq!(
        steps,
        vec!["plan", "implement", "handoff"],
        "three steps, which is M1's reduced form of the designed Bug workflow \
         — `implement` carries the test Check itself rather than handing off \
         to a separate `verify` step"
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
    let setup = Setup::at(&repository(), &roster()).expect("a setup that loads");
    let resolved: Vec<(&str, &str)> = bug(&setup)
        .steps()
        .iter()
        .flat_map(|step| step.checks())
        .filter_map(|check| match check {
            ResolvedCheck::ManifestCheck { name, run, .. } => Some((name.as_str(), run.as_str())),
            ResolvedCheck::DiffNonempty | ResolvedCheck::ArtifactExists { .. } => None,
        })
        .collect();

    // **Declaration order, and it is the order they run in.** `WorkflowDef` has
    // no field for sequencing — see `config`'s own test saying so — so this list
    // is `bug.json`'s `implement` step read top to bottom.
    assert_eq!(
        resolved,
        vec![
            ("build", "cargo build --workspace --locked"),
            ("test", "cargo nextest run --workspace --exclude acceptance"),
            ("format", "cargo fmt --all --check"),
            ("typecheck", "pnpm -C apps/desktop typecheck"),
            ("bridge_build", "pnpm -C apps/desktop build"),
            ("storybook", "pnpm -C packages/components build-storybook"),
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
    let setup = Setup::at(&repository(), &roster()).expect("a setup that loads");
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
    let refused =
        WorkflowDef::load(&designed, &roster()).expect_err("a loop, and M1 carries linear");
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
/// M1's reduced form has three steps because M1 has no Judge to answer
/// `auto_if_judge_passes` and no verdict to route on, and `implement` carries
/// the test Check itself rather than handing off to a step of its own. The
/// designed one has seven and loops. A change that made the two the same
/// length would mean one of them had been quietly rewritten into the other.
#[test]
fn the_designed_definition_and_m1s_reduced_form_are_not_the_same_workflow() {
    let setup = Setup::at(&repository(), &roster()).expect("a setup that loads");
    assert_eq!(bug(&setup).steps().len(), 3);

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

/// **The whole point of this step.** A repository may declare more than one
/// workflow, and every one of them loads and is held by its own id.
#[test]
fn two_or_more_workflow_definitions_load_and_are_held_by_their_own_ids() {
    let dir = a_repository();
    dir.write(".armada/workflows/alpha.yml", &a_workflow("alpha"));
    dir.write(".armada/workflows/beta.yml", &a_workflow("beta"));

    let setup = Setup::at(dir.path(), &roster()).expect("two definitions with distinct ids load");
    let mut ids: Vec<&str> = setup.workflows().keys().map(|id| id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, vec!["alpha", "beta"]);
}

/// Two files naming the same `workflow_id` is refused, and the refusal names
/// both paths — a person reading it must not have to search the directory to
/// find the second.
#[test]
fn a_duplicate_workflow_id_across_two_files_is_refused_naming_both() {
    let dir = a_repository();
    dir.write(".armada/workflows/first.yml", &a_workflow("shared"));
    dir.write(".armada/workflows/second.yml", &a_workflow("shared"));

    let refused = Setup::at(dir.path(), &roster()).expect_err("two files agree on one id");
    assert!(matches!(refused, SetupRefused::DuplicateWorkflowId { .. }));
    let said = refused.to_string();
    assert!(said.contains("first.yml"), "{said}");
    assert!(said.contains("second.yml"), "{said}");
    assert!(said.contains("shared"), "{said}");
}

/// An empty `.armada/workflows/` is still refused — a repository is not set up
/// until at least one workflow is there to dispatch.
#[test]
fn zero_workflow_files_is_still_refused() {
    let dir = a_repository();
    std::fs::create_dir_all(dir.path().join(WORKFLOWS)).expect("the empty directory");

    let refused = Setup::at(dir.path(), &roster()).expect_err("no definition is in the directory");
    assert!(matches!(refused, SetupRefused::NoWorkflow { .. }));
}
