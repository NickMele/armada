//! Reach's claim: **Armada works on a repository I did not write the Manifest
//! for by hand.**
//!
//! The flow is the *Set Up a Project* journey — Locate, Scan, Pick, Proposal,
//! Write, Verify, Fix — and then the half no journey draws: a repository nobody
//! set up has no `.armada/workflows/`, so today it cannot dispatch at all.
//! **Almost none of it is built**, so this file asserts what the claim stands
//! on and names the rest. The apparatus is [`bench::reach`]: a Manifest and a
//! workflow definition for a repository that is not this one, held as text and
//! read through the parsers Fleet loads with.
//!
//! **Green is not the milestone; the two tables below it are.** A Reach pull
//! request that builds a step adds that step's assertion to this file, beside
//! the code, and deletes the step's row — in the same pull request. A row is
//! never moved by weakening what it would assert to fit what exists.

//! # Carried, and asserted below
//!
//! | What holds | What it does not reach |
//! |---|---|
//! | A Manifest for a repository that is not this one loads — a port, Checks in written order, the Commands a Check requires, a server, setup | That anything wrote it. Scan is #822's; Proposal and Write are #823's |
//! | A proposal saying more than the file can hold is refused, every fault in one pass | That a proposal is ever read back before it is written |
//! | A definition gating on `every_manifest_check` resolves against that repository's own Checks, and one naming Armada's by name is refused there | That any definition reaches that repository. Carrying one is #425's |
//! | A Job created against it is held to that repository's Checks, prerequisites and all | That the Job's record says where its definition came from — #425 |
//! | Running one Manifest entry in the checkout, and reading and saving the file, are operations Fleet serves | Verify and Fix. One entry is not setup and every Check once, and a saved file is not a row corrected in place |

//! # Not carried: setting it up
//!
//! **In the order a person meets them, which is the build order.** The one
//! dependency an issue writes down — #425 says #424 lands first — agrees.
//!
//! | Step | What is not carried | Carried by |
//! |---|---|---|
//! | Locate | Pointing Armada at a repository it has not seen, by path or by clone. A Fleet reads the one repository it was started in | #821 |
//! | Scan | Reading lockfiles, package scripts, CI config and workspace globs across every workspace in one pass, and writing nothing | #822 |
//! | Pick | Each workspace ticked by how strong its evidence is, and a Check name its siblings declare marked where it is missing | #822 |
//! | Proposal | Every line cites the file it came from, or says `convention` | #823 |
//! | Write | One `armada.yml` per workspace, whatever the proposal was iterated to | #823 |
//! | Verify | Setup and every Check run once on approval, on the sheet that wrote the file, writing nothing | #719 |
//! | Fix | The failing Check's command corrected in its row, and only that Check run again | #721 |

//! # Not carried: dispatching into it
//!
//! | Step | What is not carried | Carried by |
//! |---|---|---|
//! | Propose | A request naming a milestone proposes the workflow that runs one, because a definition can say which requests it is for | #424 |
//! | Dispatch | A Job runs in a repository with no `.armada/workflows/`, on a definition Armada carries or a machine adds | #425 |
//! | Override | The repository replaces any carried definition by writing one file, and its own wins | #425 |
//! | Source | A person can tell which of those sources a Job's workflow came from | #425 |

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the six.
#[allow(dead_code)]
mod bench;

use std::path::Path;

use config::{Fault, ResolveError, ResolvedCheck};
use core_model::{JobStatus, StepState};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::reach::{
    resolved_there, written, CARRYABLE, MANIFEST_AT, NAMING_ARMADAS_CHECKS, OVERREACHING, WRITTEN,
};
use bench::{states, Bench};

// ---------------------------------------------------------------------------
// The file Setup ends in
// ---------------------------------------------------------------------------

/// **The file Write is to produce is one Armada loads**, for a repository that
/// shares nothing with this one.
///
/// Every band of the journey's proposal sheet is here as the file spells it,
/// and each is read back: a line a proposal could hold and the parser could not
/// would be a Setup that ends in a refusal. What wrote it is not asserted, and
/// nothing did — that is the Scan, Proposal and Write rows.
#[test]
fn a_manifest_for_a_repository_that_is_not_this_one_loads() {
    let manifest = config::Manifest::parse(Path::new(MANIFEST_AT), WRITTEN)
        .unwrap_or_else(|why| panic!("a Manifest Setup could write is one Armada loads: {why}"));
    assert_eq!(manifest.id().as_str(), "storefront");
    assert_eq!(
        manifest.checks_as_written(),
        ["test", "lint", "e2e"],
        "the order the file writes, which is the order the gate starts them in"
    );
    assert_eq!(
        manifest
            .check("e2e")
            .expect("e2e is declared")
            .requires()
            .iter()
            .map(|prerequisite| prerequisite.name())
            .collect::<Vec<_>>(),
        ["migrate", "seed"],
        "a Check requires Commands by name, in the order they must run"
    );
    assert_eq!(
        manifest.command_names(),
        ["install", "migrate", "reset", "seed"],
        "the Commands, and not the server among them"
    );
    assert!(manifest
        .command("reset")
        .expect("reset is declared")
        .is_destructive());
    assert_eq!(manifest.server_names(), ["dev"]);
    let web = manifest.port("web").expect("the port is declared");
    assert_eq!((web.container(), web.env()), (Some(3000), Some("PORT")));
    assert_eq!(
        manifest
            .prepared_by()
            .iter()
            .map(|step| step.name())
            .collect::<Vec<_>>(),
        ["install"],
        "and what a fresh worktree runs before any step starts"
    );
}

/// **A proposal that says more than the file can hold is refused, and every
/// fault is named in one pass.**
///
/// The journey's rule is that every line traces to something a file already
/// said; the parser's is that a key nothing reads is refused. Iterating on a
/// proposal meets the second whenever the first slips, so a refusal that named
/// one fault per pass would make four slips four rounds.
#[test]
fn a_proposal_saying_more_than_the_file_holds_is_refused_whole() {
    let refused = config::Manifest::parse(Path::new(MANIFEST_AT), OVERREACHING)
        .expect_err("a file Armada would not load");
    assert_eq!(refused.path(), Path::new(MANIFEST_AT));

    let mut keys: Vec<&str> = refused
        .refusals()
        .iter()
        .map(|refusal| refusal.key.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "checks.lint.requires[0]",
            "commands.lint",
            "scripts",
            "setup.requires[0]"
        ],
        "all four, and nothing the proposal got right"
    );
    let at = |key: &str| {
        &refused
            .refusals()
            .iter()
            .find(|refusal| refusal.key == key)
            .expect("refused at that key")
            .fault
    };
    assert!(matches!(at("scripts"), Fault::Unknown { .. }));
    assert!(
        matches!(
            at("checks.lint.requires[0]"),
            Fault::NotADeclaredCommand {
                is_a_check: true,
                ..
            }
        ),
        "a Check put in front of a Check says it is one"
    );
    assert_eq!(at("commands.lint"), &Fault::DeclaredInBothRegistries);
    assert!(matches!(
        at("setup.requires[0]"),
        Fault::RequiresAServer { .. }
    ));
}

// ---------------------------------------------------------------------------
// A workflow that did not come from the repository
// ---------------------------------------------------------------------------

/// **A definition written without the repository in view resolves against
/// that repository's own Checks.** What makes carrying a definition possible.
///
/// `every_manifest_check` expands to whatever the Manifest declares, in the
/// order it declares it, with each command lifted in. And a step asking to be
/// captured resolves against a Manifest with no `evidence:` section, which is
/// the ordinary shape of a repository nobody set up for Armada.
#[test]
fn a_definition_asking_for_every_check_resolves_against_that_repositorys_own() {
    let resolved = resolved_there(CARRYABLE)
        .unwrap_or_else(|why| panic!("a carryable definition resolves there: {why}"));
    assert_eq!(resolved.manifest_path(), Path::new(MANIFEST_AT));

    let implement = &resolved.steps()[1];
    assert_eq!(
        implement
            .checks()
            .iter()
            .map(ResolvedCheck::label)
            .collect::<Vec<_>>(),
        ["test", "lint", "e2e", "diff_nonempty"],
        "the repository's Checks and not Armada's, then the step's own"
    );
    assert_eq!(implement.checks()[0].run(), Some("pnpm vitest run"));
    assert!(implement.gates_on_every_check());

    assert!(
        written().harness().is_none(),
        "the repository never said how it shows its work"
    );
    assert!(
        implement.captured(),
        "and the step that asks to be captured still resolves there"
    );
}

/// **A definition naming a Check by name is refused where that name is not
/// declared**, and the refusal says what is.
///
/// The other half of the one above, and the constraint on whatever #425
/// carries: a definition gating on Armada's `build` resolves in this
/// repository and is refused in every repository without one. `test` resolves,
/// because the storefront declares one — a shared name, not a shared meaning.
#[test]
fn a_definition_naming_armadas_checks_is_refused_there() {
    let refused =
        resolved_there(NAMING_ARMADAS_CHECKS).expect_err("a Check the repository never declared");
    let ResolveError::ChecksNotDeclared {
        manifest, unknown, ..
    } = refused
    else {
        panic!("a name that resolves to nothing, not a disagreement: {refused}");
    };
    assert_eq!(manifest, Path::new(MANIFEST_AT));
    assert_eq!(
        unknown
            .iter()
            .map(|miss| miss.check.as_str())
            .collect::<Vec<_>>(),
        ["build"],
        "only the name the repository does not have"
    );
    assert!(!unknown[0].is_a_command);
    let mut declared = unknown[0].declared.clone();
    declared.sort();
    assert_eq!(declared, ["e2e", "lint", "test"]);
}

/// **A Job created against that repository is held to that repository's
/// Checks**, prerequisites and commands included.
///
/// Read off the Job's frozen copy, which is what a person approves and what
/// Fleet reads at every step. Nothing is gated: a Manifest Check is a process,
/// and the gate is `bug_job.rs`'s claim.
#[test]
fn a_job_against_that_repository_is_held_to_its_checks() {
    let bench = Bench::judged_by(
        FakeWorkProduct::untouched(),
        resolved_there(CARRYABLE).expect("a carryable definition resolves there"),
        FakeJudge::that_fails("a Judge that should never be asked"),
    );
    let mut run = bench.created("the basket forgets its last item");
    bench.approved_and_dispatched(&mut run);
    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(
        states(&run.job),
        [
            ("plan", StepState::Running),
            ("implement", StepState::NotStarted),
            ("handoff", StepState::NotStarted)
        ]
    );

    let frozen = &run.job.workflow().steps()[1];
    let e2e = frozen
        .checks()
        .iter()
        .find(|check| check.name() == Some("e2e"))
        .expect("the repository's own e2e Check is on the Job");
    assert_eq!(e2e.run(), Some("pnpm playwright test"));
    assert_eq!(
        e2e.requires()
            .iter()
            .map(|prerequisite| (prerequisite.name(), prerequisite.run()))
            .collect::<Vec<_>>(),
        [
            ("migrate", "pnpm prisma migrate deploy"),
            ("seed", "pnpm tsx scripts/seed.ts")
        ],
        "and what it requires is frozen with it, commands and all"
    );
}

// ---------------------------------------------------------------------------
// The pieces Verify and Fix would be built from
// ---------------------------------------------------------------------------

/// **Running one Manifest entry with no Job, and reading and saving the file,
/// are operations Fleet serves.**
///
/// Verify runs setup and every Check once, and Fix corrects one row and runs
/// that Check again — neither is here. What is here is what #719 says the
/// dry-run reuses whole, and the write a corrected line would go through.
/// `api::SERVED` is the table `api`'s own tests walk against the router.
#[test]
fn running_one_entry_and_saving_the_file_are_operations_fleet_serves() {
    for act in [
        "get_checkout_run_sheet",
        "start_checkout_run",
        "get_checkout_run_output",
        "observe_checkout_run",
        "get_manifest_file",
        "save_manifest_file",
        "get_manifest_reading",
    ] {
        assert!(
            api::SERVED.iter().any(|route| route.operation == act),
            "`{act}` is a piece Verify or Fix is built from and nothing serves it"
        );
    }
}
