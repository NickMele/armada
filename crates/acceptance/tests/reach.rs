//! Reach's claim: **Armada works on a repository I did not write the Manifest
//! for by hand.**
//!
//! The flow is the *Set Up a Project* journey — Locate, Scan, Pick, Proposal,
//! Write, Verify, Fix — and then the half no journey draws: a repository nobody
//! set up has no `.armada/workflows/`, and dispatches on what Armada carries.
//! **Almost none of it is built**, so this file asserts what the claim stands
//! on and names the rest. The apparatus is [`bench::reach`]: a repository that
//! is not this one — its files before anybody set it up, a Manifest and a
//! workflow definition — held as text and read through what Fleet reads with.
//!
//! **Green is not the milestone; the two tables below it are.** A Reach pull
//! request that builds a step adds that step's assertion to this file, beside
//! the code, and deletes the step's row — in the same pull request. A row is
//! never moved by weakening what it would assert to fit what exists.

//! **Over 500 lines, and still one file.** A milestone is one test, so every
//! step's assertion lands here; splitting it would move the count, not the
//! claim — `docs/practices/acceptance-tests.md`.

//! # Carried, and asserted below
//!
//! | What holds | What it does not reach |
//! |---|---|
//! | Scan reads every workspace of a repository nobody set up, in one pass — workspace globs, lockfiles, package scripts, compose services, the ports a file declares — each finding naming a file the repository has, what it did not read said beside it, and nothing written, because the tree it is handed has no write | That a checkout on disk reads the same, and that Fleet serves it: both touch a repository, and are `fleet`'s and `api`'s own tests. CI configuration is reported unread and never read, since naming whose it is belongs to `adapters` |
//! | Each workspace carries how strong its evidence is, and a name every strong sibling declares is marked where one lacks it — the root never a sibling | That a picker ticks by it or draws the grid — #824. The mark is over the batch ticked by default; a batch a person re-ticks is the screen's to recompute |
//! | A Manifest for a repository that is not this one loads — a port, Checks in written order, the Commands a Check requires, a server, setup | That anything wrote it. Scan writes nothing; Proposal and Write are #823's |
//! | A proposal saying more than the file can hold is refused, every fault in one pass | That a proposal is ever read back before it is written |
//! | A definition gating on `every_manifest_check` resolves against that repository's own Checks, and one naming Armada's by name is refused there | That a definition written for another repository names only what this one declares. The carried set does; `config`'s `tests/carried.rs` holds it to three other shapes |
//! | A Job created against it is held to that repository's Checks, prerequisites and all | That any of them runs. A Manifest Check is a process, and the gate is `bug_job.rs`'s claim |
//! | **Dispatch.** With no workflows of its own, all eight Armada carries resolve there, and a Job on the carried `bug` is created and dispatched | That a Fleet started there serves them. `Setup::at` reads directories, and `armada`'s own tests read them |
//! | **Override.** One file from Kit replaces a carried definition by id, and the repository's replaces Kit's, whatever order they arrive in | That `~/.armada/workflows/` is where Kit's are read from. That is a directory, `armada`'s tests again |
//! | **Source.** Each workflow says which of the three places it came from, in words a person reads, and a Job created on the carried `bug` freezes `armada` onto its own record | That the record reads it back out of `store`, which has no in-memory constructor — `store`'s own round-trip test does. That a person sees it on a Job: nothing on the wire carries it, which is Bridge's half |
//! | Running one Manifest entry in the checkout, and reading and saving the file, are operations Fleet serves | Verify and Fix. One entry is not setup and every Check once, and a saved file is not a row corrected in place |
//! | A request naming a milestone is offered `epic` with what it is for, in a requester's words, beside its steps — and a definition saying nothing is offered as before | That a model reading it proposes `epic`. Choosing is a model's, and this file calls none |

//! # Not carried: setting it up
//!
//! **In the order a person meets them, which is the build order.** The one
//! dependency an issue writes down — #425 says #424 lands first — agrees.
//!
//! | Step | What is not carried | Carried by |
//! |---|---|---|
//! | Locate | Pointing Armada at a repository it has not seen, by path or by clone. A Fleet reads the one repository it was started in | #821 |
//! | Proposal | Every line cites the file it came from, or says `convention` | #823 |
//! | Write | One `armada.yml` per workspace, whatever the proposal was iterated to | #823 |
//! | Verify | Setup and every Check run once on approval, on the sheet that wrote the file, writing nothing | #719 |
//! | Fix | The failing Check's command corrected in its row, and only that Check run again | #721 |

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the six.
#[allow(dead_code)]
mod bench;

use std::collections::BTreeMap;
use std::path::Path;

use config::{Fault, ResolveError, ResolvedCheck, ResolvedWorkflow, WorkflowSource};
use core_model::{JobStatus, StepState, WorkflowId};
use fleet::scanning::scan;
use fleet::{Brief, Proposal};
use ipc::{
    EvidenceStrength, MissingName, RepositoryScan, ScannedWorkspace, ToolFile, WorkspaceGlob,
};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::reach::{
    carried_there, catalogued, one_step, resolved_there, written, Held, A_MILESTONE, CARRYABLE,
    CHECKOUT, EPIC, EPIC_AT, MANIFEST_AT, NAMING_ARMADAS_CHECKS, OVERREACHING, UNSET_UP, WRITTEN,
};
use bench::{states, Bench};

// ---------------------------------------------------------------------------
// Scan and Pick
// ---------------------------------------------------------------------------

/// A scan as a picker receives it: through `ipc::encode` and back.
fn received(repository: &Held) -> RepositoryScan {
    let sent = ipc::encode(&scan(CHECKOUT, repository)).expect("a scan that serialises");
    ipc::decode("a repository scan", sent.as_bytes()).expect("and reads back")
}

fn workspace<'a>(scan: &'a RepositoryScan, dir: &str) -> &'a ScannedWorkspace {
    scan.workspaces
        .iter()
        .find(|one| one.dir == dir)
        .unwrap_or_else(|| panic!("{dir} is a workspace"))
}

/// **Scan reads every workspace of a repository nobody set up, in one pass,
/// and every finding names a file that repository has.** The Scan step, #822.
///
/// Handed to Scan as a `Tree`, which has no write on it — so *writes nothing*
/// is the type, not an assertion. What it did not read is said beside what it
/// did, and a tool it does not follow is never clean.
#[test]
fn scan_reads_every_workspace_and_cites_the_file_each_finding_came_from() {
    let repository = Held::of(UNSET_UP);
    let scan = received(&repository);
    let dirs: Vec<&str> = scan.workspaces.iter().map(|one| one.dir.as_str()).collect();
    assert_eq!(
        dirs,
        [
            ".",
            "apps/admin",
            "apps/shop",
            "packages/tokens",
            "packages/ui",
            "services/mailer"
        ],
        "every workspace, the one no pattern names included"
    );

    for one in &scan.workspaces {
        let cited = one
            .manifests
            .iter()
            .chain(&one.lockfiles)
            .map(|found| &found.file)
            .chain(one.runnables.iter().map(|found| &found.file))
            .chain(one.tools.iter().map(|found| &found.file))
            .chain(one.services.iter().map(|found| &found.file))
            .chain(one.ports.iter().map(|found| &found.file))
            .chain(one.not_read.iter().map(|found| &found.file));
        for file in cited {
            assert!(
                repository.has(file),
                "{} cites {file}, which is not there",
                one.dir
            );
        }
    }

    let shop = workspace(&scan, "apps/shop");
    assert_eq!(
        shop.declared_by,
        [WorkspaceGlob {
            file: "pnpm-workspace.yaml".to_string(),
            entry: "apps/*".to_string()
        }]
    );
    let e2e = shop
        .runnables
        .iter()
        .find(|one| one.name == "e2e")
        .expect("e2e");
    assert_eq!(
        (e2e.file.as_str(), e2e.key.as_str(), e2e.run.as_str()),
        ("apps/shop/package.json", "scripts.e2e", "playwright test")
    );
    assert_eq!(
        workspace(&scan, ".").lockfiles,
        [ToolFile {
            file: "pnpm-lock.yaml".to_string(),
            tool: "pnpm".to_string()
        }]
    );
    assert!(
        shop.lockfiles.is_empty(),
        "a shared lockfile is the root's, not copied"
    );

    let ports: Vec<(&str, &str, u16)> = scan
        .workspaces
        .iter()
        .flat_map(|one| &one.ports)
        .map(|port| (port.file.as_str(), port.key.as_str(), port.container))
        .collect();
    assert_eq!(
        ports,
        [
            ("compose.yaml", "services.db.ports[0]", 5432),
            ("apps/shop/package.json", "scripts.dev", 3000)
        ],
        "only where a file declares one"
    );

    let mailer = workspace(&scan, "services/mailer");
    assert_eq!(mailer.evidence, EvidenceStrength::NotFollowed);
    assert_eq!(mailer.not_read[0].file, "services/mailer/go.mod");
    let unread: Vec<&str> = scan.not_read.iter().map(|one| one.file.as_str()).collect();
    assert_eq!(unread, [".ci/pipeline.yml"], "said, rather than skipped");
}

/// **Each workspace carries how strong its evidence is, and a name every strong
/// sibling declares is marked where one lacks it.** The Pick step, #822.
///
/// Narrowly: `e2e` is the shop's alone and marks nothing, the root's `lint`
/// makes it no sibling, and a thin or unread workspace is neither marked nor
/// allowed to erase a mark.
#[test]
fn pick_ticks_by_evidence_and_marks_a_name_every_strong_sibling_declares() {
    let scan = received(&Held::of(UNSET_UP));
    let strengths: Vec<(&str, EvidenceStrength)> = scan
        .workspaces
        .iter()
        .map(|one| (one.dir.as_str(), one.evidence))
        .collect();
    assert_eq!(
        strengths,
        [
            (".", EvidenceStrength::Strong),
            ("apps/admin", EvidenceStrength::Strong),
            ("apps/shop", EvidenceStrength::Strong),
            ("packages/tokens", EvidenceStrength::Thin),
            ("packages/ui", EvidenceStrength::Strong),
            ("services/mailer", EvidenceStrength::NotFollowed)
        ]
    );

    for one in &scan.workspaces {
        let expected = match one.dir.as_str() {
            "packages/ui" => vec![MissingName {
                name: "lint".to_string(),
                declared_in: vec!["apps/admin".to_string(), "apps/shop".to_string()],
            }],
            _ => Vec::new(),
        };
        assert_eq!(one.missing, expected, "on {}", one.dir);
    }
}

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

/// The workflow a catalogue resolved to for `id`, against the storefront.
fn held_there(catalogue: &config::ResolvedCatalogue, id: &str) -> ResolvedWorkflow {
    catalogue
        .workflows()
        .values()
        .find(|workflow| workflow.id().as_str() == id)
        .unwrap_or_else(|| panic!("`{id}` is held"))
        .clone()
}

/// **A repository with no workflows of its own dispatches on what Armada
/// carries.** Dispatch.
///
/// All eight resolve against the storefront, and a Job on the carried `bug` is
/// created and dispatched, gated on the storefront's own Checks. Nothing here
/// read a directory: that the carried set is what `Setup::at` hands a Fleet is
/// `armada`'s own test.
#[test]
fn a_repository_with_no_workflows_of_its_own_dispatches_on_what_armada_carries() {
    let carried = catalogued(&[], &[]);
    assert_eq!(carried.workflows().len(), 8, "all eight resolve there");
    assert!(carried.left_out().is_empty(), "and none is left out");
    let bug = held_there(&carried, "bug");
    assert_eq!(bug.source(), WorkflowSource::Armada);
    assert_eq!(
        bug.steps()[1]
            .checks()
            .iter()
            .map(ResolvedCheck::label)
            .collect::<Vec<_>>(),
        ["test", "lint", "e2e", "diff_nonempty"],
        "the storefront's Checks, on a definition nobody there wrote"
    );

    let bench = Bench::judged_by(
        FakeWorkProduct::untouched(),
        bug,
        FakeJudge::that_fails("a Judge that should never be asked"),
    );
    let mut run = bench.created("the basket forgets its last item");
    bench.approved_and_dispatched(&mut run);
    assert_eq!(run.job.status(), JobStatus::Running);
    assert_eq!(states(&run.job)[0], ("plan", StepState::Running));
    assert_eq!(
        run.job.workflow().source(),
        WorkflowSource::Armada,
        "and the Job's own record says where its workflow came from"
    );
}

/// **One file replaces a carried definition by id, and the repository's own
/// wins.** Override, and Source in the words a person reads.
///
/// Kit writes `bug` and `revert`; the storefront writes `bug`. Its three-step
/// `bug` asks no Judge anything, which is how it is told from Armada's.
#[test]
fn one_file_replaces_a_carried_definition_and_the_repositorys_own_wins() {
    let kit = [
        ("bug.yml", one_step("bug")),
        ("revert.yml", one_step("revert")),
    ];
    let own = [("bug.json", CARRYABLE.to_string())];
    let catalogue = catalogued(&kit, &own);

    let bug = held_there(&catalogue, "bug");
    assert_eq!(bug.source(), WorkflowSource::Repository);
    assert!(bug
        .steps()
        .iter()
        .all(|step| step.judge_checks().is_empty()));
    assert_eq!(
        held_there(&catalogue, "revert").steps().len(),
        1,
        "Kit's, over Armada's"
    );

    let said: Vec<String> = ["bug", "revert", "feature"]
        .map(|id| format!("{id} {}", held_there(&catalogue, id).source()))
        .into();
    assert_eq!(
        said,
        [
            "bug from the repository",
            "revert from Kit",
            "feature carried by Armada"
        ]
    );
}

// ---------------------------------------------------------------------------
// Proposing a Job there
// ---------------------------------------------------------------------------

/// **A request naming a milestone is offered the workflow that runs one, in
/// words the request could match.** The Propose step, and #424.
///
/// What the proposer reads is asserted on the brief, without a model: `epic` as
/// the file ships, carried to the storefront beside a `bug` that declares
/// nothing. The line saying what `epic` is for sits under its name and names a
/// milestone; `wave`, the steps' word, is not in it; the steps are still there;
/// and the definition that declares nothing is offered exactly as it was
/// before the key existed. An answer choosing `epic` then reads back as `epic`.
///
/// **What a model does with it is not asserted**, and the header says so.
#[test]
fn a_request_naming_a_milestone_is_offered_the_workflow_that_runs_one() {
    let held: BTreeMap<WorkflowId, ResolvedWorkflow> = [
        carried_there(EPIC_AT, EPIC),
        resolved_there(CARRYABLE).expect("a carryable definition resolves there"),
    ]
    .into_iter()
    .map(|workflow| (workflow.id().clone(), workflow))
    .collect();
    let brief = Brief::about(A_MILESTONE, &held);
    let question = brief.question();

    let offered = "  epic — epic\n    for: ";
    let at = question
        .find(offered)
        .expect("epic is offered with what it is for, on the line under its name");
    let what_for = question[at + offered.len()..]
        .lines()
        .next()
        .expect("the line has words on it");
    assert!(
        what_for.contains("milestone"),
        "in the word the request used: {what_for}"
    );
    assert!(
        !what_for.to_lowercase().contains("wave"),
        "and not in the steps' word, which is the defect restated: {what_for}"
    );
    assert!(
        question[at..].contains("\n    Plan the wave -> Dispatch the wave -> Roll up the wave\n"),
        "the steps stay beside it — they answer a different question"
    );
    assert!(
        question.contains("  bug — bug\n    Plan the change -> Implement -> Summarise\n"),
        "a definition that says nothing is offered exactly as before"
    );

    let Ok(Proposal::Resolved(jobs)) =
        brief.read("workflow: epic\ntitle: Finish the Board milestone", &held)
    else {
        panic!("an answer choosing the offered workflow is a proposal");
    };
    assert_eq!(jobs[0].workflow_id.as_str(), "epic");
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
