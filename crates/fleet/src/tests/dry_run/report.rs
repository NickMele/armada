//! What comes back: a row per Check the step declares, saying what each one did
//! and where to read the rest of it.
//!
//! The three between them hold the report to the frozen step and to what the
//! gate would do, which are the two places it could be written from the wrong
//! one — a Check list `dry_run` knows for itself rather than one the step
//! declares, and a row for a Check the gate would never have run.

use std::sync::Arc;

use config::ResolvedWorkflow;
use testkit::{Gate, Sketch};

use crate::tests::dry_run::{a_fleet_checking, ask, one_step, router, started, Held};
use crate::tests::tmp::TempDir;

/// **The claim of the whole issue, over the wire a Drone actually uses.** A
/// tool call arrives as JSON-RPC on the router that ships, Fleet runs the
/// step's Checks in the Drone's worktree, and what comes back names each one,
/// says what it did, and says where to read more.
#[tokio::test]
async fn a_drone_asking_for_the_checks_is_told_what_each_one_did() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/false"),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    let job = started(&fleet, &home).await;
    let handle = fleet.load(&job).await.expect("the Job").handle();

    let said = ask(&app).await;
    assert!(!said.is_error, "{}", said.text);
    assert!(
        said.text.contains("suite") && said.text.contains("FAILED"),
        "the Check that did not pass is named and so is what it did: {}",
        said.text
    );
    assert!(
        said.text.contains("it exited 1"),
        "the failure's own sentence, not a bare word: {}",
        said.text
    );
    assert!(
        said.text.contains("diff_nonempty"),
        "the built-in is a row like any other: {}",
        said.text
    );
    assert!(
        said.text.contains("implement.1.dry.0.log"),
        "and where to read the rest of it: {}",
        said.text
    );
    assert!(
        said.text.contains("not a verdict"),
        "said in the answer and not only in the briefing: {}",
        said.text
    );

    // The path it named is a file that is there, holding what the Check
    // printed — a pointer to nothing would be worse than no pointer.
    let log = crate::check_output::checks_dir(&home.path().to_string_lossy(), &handle)
        .join("implement.1.dry.0.log");
    assert!(
        log.is_file(),
        "{} was named and does not exist",
        log.display()
    );
    assert!(
        std::fs::read_to_string(&log)
            .expect("the log reads")
            .contains("--- stdout ---"),
        "both streams, each behind its marker"
    );
    // **And not where the gate writes.** A dry run writes no row, so a file at
    // the gate's own path would be output the record does not point at. Both
    // carry the attempt, so this holds per run rather than once per step —
    // #63 made a step workable twice and the path is the whole key.
    assert!(
        !crate::check_output::checks_dir(&home.path().to_string_lossy(), &handle)
            .join("implement.1.0.log")
            .exists(),
        "a dry run wrote over the gate's log"
    );
}

/// **Adding a Check to a step is the whole of adding it.** #200 named three
/// more Manifest Checks on every step that produces a diff, and the worry was
/// that `run_checks` reached the two it already knew — a Drone that cannot see
/// what failed cannot fix it, and the allowlist denies it `pnpm` as it denies
/// it `cargo`.
///
/// Nothing in `dry_run` names a Check. It walks whatever the frozen step
/// declares, so a step declaring five is answered with five rows, and the
/// failing one among them is named whichever position it sits in.
#[tokio::test]
async fn every_check_the_step_declares_gets_a_row_however_many_there_are() {
    let home = TempDir::new();
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "build",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::Check {
                name: "typecheck",
                run: "/usr/bin/false",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::Check {
                name: "bridge_build",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::Check {
                name: "storybook",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let fleet = Arc::new(a_fleet_checking(
        &home,
        workflow,
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = ask(&app).await;
    assert!(!said.is_error, "{}", said.text);
    for named in ["build", "typecheck", "bridge_build", "storybook"] {
        assert!(
            said.text.contains(named),
            "`{named}` was declared and is not in the report: {}",
            said.text
        );
    }
    assert!(
        said.text.contains("typecheck") && said.text.contains("it exited 1"),
        "the one that failed is the one reported failing: {}",
        said.text
    );
}

/// One step whose named Check covers `packages/**` — paths the fixture Fleet's
/// worktree, which holds `src/parse.rs`, does not touch.
fn one_scoped_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "storybook",
                run: "/usr/bin/false",
                expect_exit_code: 0,
                when: &["packages/**"],
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// **The rehearsal has to agree with the gate.** The report's own closing
/// sentence promises the Drone the same Checks, run by Fleet — so a dry run
/// that spent a Check the gate will skip would be telling a Drone its work
/// failed something no gate is going to ask.
#[tokio::test]
async fn a_dry_run_skips_the_same_check_the_gate_would() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_scoped_step(),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    let _job = started(&fleet, &home).await;

    let said = ask(&app).await;
    assert!(!said.is_error, "{}", said.text);
    // The command exits 1. A row that is not FAILED is a Check that never ran.
    assert!(
        said.text.contains("storybook") && said.text.contains("SKIPPED"),
        "the Check is named and said to have been skipped: {}",
        said.text
    );
    assert!(
        !said.text.contains("FAILED"),
        "nothing failed, so nothing may say so: {}",
        said.text
    );
    // And the closing line does not report a pass nobody earned.
    assert!(
        said.text.contains("cover paths this step did not touch"),
        "the summary says what was not run: {}",
        said.text
    );
    assert!(
        !said.text.contains("2 of 2 passed"),
        "one of the two was never run: {}",
        said.text
    );
}
