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
         label: \"Only step\"\n    delivers: true\n    advance_gate: auto\n"
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
            "bridge_test".to_string(),
            "build".to_string(),
            "format".to_string(),
            "storybook".to_string(),
            "test".to_string(),
            "typecheck".to_string(),
        ],
        "the seven Checks this workspace is built and tested with — two for the \
         Rust half and four for the Bridge, which is #200: every Check used to \
         compile Rust, so a Job that changed only `apps/` was verified entirely \
         on the code it had not touched. `format` is the seventh and is the same \
         defect one lint over: PR #199 also merged nine unformatted files, \
         because `cargo fmt --check` was not a Check. `bridge_test` is the \
         newest and closes the other half of #200's gap: the three Bridge \
         Checks before it were all compilers, so nothing ran a line of \
         TypeScript. There is no `clippy` — `[clippy-as-a-check]` in \
         `docs/OPEN.md` says why"
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
    //
    // **Which file declares the sequence is moving.** While a step names its
    // Checks, the step is where the order is written and this list is it. A step
    // that says `every_manifest_check` instead takes `armada.yml`'s order —
    // `checks_as_written`, asserted by the test below against the real
    // Manifest — and the two differ: this file puts `format` third and
    // `armada.yml` declares it last. So when these seven names come out of the
    // shipped definitions, the fix here is `armada.yml`'s order, not a set
    // comparison; the property both orders exist to keep is that a failure
    // surfaces as early as it can, and a set gives that up.
    assert_eq!(
        resolved,
        vec![
            ("build", "cargo build --workspace --locked"),
            ("test", "cargo nextest run --workspace --exclude acceptance"),
            ("format", "cargo fmt --all --check"),
            // **One name, not a chain.** A `run` gets no shell, so the `&&`
            // this used to hold was passed to `tsc` as an argument and the
            // Check could never pass. The chain lives in a `package.json`
            // script, where a shell exists and the packages stay named.
            ("typecheck", "pnpm typecheck"),
            ("bridge_build", "pnpm -C apps/desktop build"),
            ("storybook", "pnpm -C packages/components build-storybook"),
            // The one Check that runs TypeScript rather than compiling it. A
            // script again, and for the same reason: it chains two runners —
            // the screens' pure modules in node, then every story in a browser.
            ("bridge_test", "pnpm bridge-test"),
        ]
    );
}

/// **`every_manifest_check` expands in the order `armada.yml` writes, and that
/// order is the order the gate runs.**
///
/// The seven names the shipped steps spell out are moving out of the workflow
/// files, and until they did, the workflow file was the only place this
/// repository sequenced its gate — the assertion above is that sequence read
/// top to bottom. Expanded alphabetically the same seven come back as
/// `bridge_build, bridge_test, build, …`: the same set, and the two slowest
/// Checks leading.
///
/// `armada.yml` argues the sequence in its own words on `bridge_test` — *"the
/// order is the order they answer in — 6s, 40s, then the stories — so a failure
/// surfaces as early as it can"* — with the figures beside it: `build` 37.6s,
/// `test` 41s, `bridge_test` 6s then 40s. A Drone that broke the compile should
/// not wait on a browser to be told.
///
/// **Asserted against the real Manifest and a definition written here**, rather
/// than by editing a shipped file: the claim is about the expansion, and it has
/// to hold before the seven files switch over as well as after.
#[test]
fn gating_on_every_check_runs_them_in_the_order_armada_yml_writes_them() {
    let setup = Setup::at(&repository(), &roster()).expect("a setup that loads");
    let text = "version: 1\nworkflow_id: sweeping\nname: sweeping\nstructure: linear\nsteps:\n  \
                - id: implement\n    label: Implement\n    evidence_type: diff\n    \
                delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      \
                - { type: every_manifest_check }\n      - { type: diff_nonempty }\n";
    let def = WorkflowDef::parse(std::path::Path::new("sweeping.yml"), text, &roster())
        .expect("a step may gate on every declared Check");
    let resolved = config::ResolvedWorkflow::resolve(&def, setup.manifest())
        .expect("every declared Check resolves against the file that declared it");

    let names: Vec<&str> = resolved.steps()[0]
        .checks()
        .iter()
        .filter_map(ResolvedCheck::name)
        .collect();
    assert_eq!(
        names,
        vec![
            "build",
            "test",
            "typecheck",
            "bridge_build",
            "storybook",
            "bridge_test",
            "format",
        ],
        "the order `armada.yml` declares them in, which is the order they answer \
         in — not `check_names`' alphabetical, which leads with the two slowest"
    );
    // The same seven, and no eighth: the expansion is the registry and the
    // registry is what `check_names` lists.
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, setup.manifest().check_names());
    // And the declaration survives beside what it came to, which is what a
    // repository declaring no Checks would be left with.
    assert!(resolved.steps()[0].gates_on_every_check());
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
/// and says so correctly.
///
/// **The structure is no longer what refuses it.** Until #263 that was this
/// test's subject — `loop` was `NotYetCarried`, one of two values the milestone had
/// not built — and both are carried now, so the claim is asked of the next
/// deferral instead: `test_run` is a check type the schema sanctions and M1 has
/// no per-step test invocation for.
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
    let refused = WorkflowDef::load(&designed, &roster())
        .expect_err("seven steps, and M1 reads a slice of what they declare");
    let LoadError::Refused { refusals, .. } = &refused else {
        panic!("a document that parsed and was refused, not {refused}");
    };
    assert!(
        !refusals.iter().any(|refusal| refusal.key == "structure"),
        "`loop` is carried, and the file declares it correctly: {refusals:?}"
    );
    let deferred = refusals
        .iter()
        .find(|refusal| refusal.key == "steps[0].mechanical_checks[0].type")
        .unwrap_or_else(|| panic!("`test_run` is refused: {refusals:?}"));
    assert!(
        matches!(
            &deferred.fault,
            Fault::NotYetCarried { value, .. } if value == "test_run"
        ),
        "deferred, not wrong: {:?}",
        deferred.fault
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

/// **The whole daemon-start path, over a workflow that names no Check.** The
/// test above resolves one definition against the real Manifest; this walks
/// `Setup::at` — read the Manifest, read the definitions beside it, resolve
/// each — which is what a repository whose workflows have been switched over
/// actually goes through.
///
/// The Manifest here declares its Checks in an order the alphabet does not
/// agree with, so a resolution that quietly sorted them would come back
/// `apple, build, zebra` and fail here rather than in a Job.
#[test]
fn a_repository_whose_workflow_names_no_check_starts_and_keeps_its_order() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  zebra:\n    run: run zebra\n  \
         build:\n    run: run build\n  apple:\n    run: run apple\n",
    );
    dir.write(
        ".armada/workflows/sweeping.yml",
        "version: 1\nworkflow_id: sweeping\nname: sweeping\nstructure: linear\nsteps:\n  \
         - id: only\n    label: \"Only step\"\n    evidence_type: diff\n    delivers: true\n    \
         advance_gate: auto\n    mechanical_checks:\n      - { type: every_manifest_check }\n",
    );

    let setup = Setup::at(dir.path(), &roster()).expect("a repository that gates on all of them");
    let workflow = setup
        .workflows()
        .values()
        .next()
        .expect("the one definition");
    let ran: Vec<(&str, &str)> = workflow.steps()[0]
        .checks()
        .iter()
        .filter_map(|check| Some((check.name()?, check.run()?)))
        .collect();
    assert_eq!(
        ran,
        vec![
            ("zebra", "run zebra"),
            ("build", "run build"),
            ("apple", "run apple"),
        ],
        "the Manifest's order, with each Check's own command lifted in"
    );
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
