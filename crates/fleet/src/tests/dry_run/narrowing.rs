//! The second of the two runs a Drone can ask for: the Checks, against what it
//! has changed.
//!
//! # The Checks are real commands, and what they print is the assertion
//!
//! `/bin/echo` exits zero and writes its arguments, so a Check whose narrowed
//! command is `/bin/echo` proves two things at once in its own log: that the
//! narrowed command ran rather than the whole one, and what it was narrowed to.
//! The whole command in every case below is `/usr/bin/false`, which cannot pass
//! — so a passing row is a row that ran the other command and nothing else.
//!
//! # What is being held apart
//!
//! A narrowed run is feedback and never a verdict, and the ways that could stop
//! being true are all here: a Check with no narrowing quietly narrowing anyway,
//! a Check narrowing to nothing and reading as a pass, a whole run picking up a
//! narrowing nobody asked for, and a report that does not say which of the two
//! it is.

use std::sync::Arc;

use config::ResolvedWorkflow;
use testkit::{Gate, Narrows, Sketch};

use crate::tests::dry_run::{a_fleet_over, asking, router, started, submit, Held};
use crate::tests::tmp::TempDir;

/// One step gated on two named Checks and a diff. `suite` declares a narrowing
/// and `whole` does not, which is the pair every case here needs: the answer
/// has to differ between them under one call.
fn two_checks(narrows: &[Narrows<'_>]) -> ResolvedWorkflow {
    testkit::narrowing(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[
                Gate::Check {
                    name: "suite",
                    run: "/usr/bin/false",
                    expect_exit_code: 0,
                    when: &[],
                },
                Gate::Check {
                    name: "whole",
                    run: "/bin/echo",
                    expect_exit_code: 0,
                    when: &[],
                },
                Gate::DiffNonempty,
            ],
            judged_on: &[],
            scope: None,
            gaming: None,
        }],
        narrows,
    )
}

/// `suite`, narrowed to the changed paths themselves.
fn by_file() -> Narrows<'static> {
    Narrows {
        check: "suite",
        run: "/bin/echo",
        each: "{}",
        from: &[],
        under: None,
        except: &[],
    }
}

/// `suite`, narrowed to the directory under `crates` that changed — which the
/// fixture's `src/parse.rs` is not under.
fn by_package() -> Narrows<'static> {
    Narrows {
        check: "suite",
        run: "/bin/echo",
        each: "-p {}",
        from: &[],
        under: Some("crates"),
        except: &[],
    }
}

/// **The claim of the issue.** A Drone asks about its own change and the Check
/// runs a different, smaller command — the one the Manifest declared — against
/// the paths Fleet read out of the worktree.
#[tokio::test]
async fn a_narrowed_run_runs_the_command_the_manifest_declared_for_it() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[by_file()]),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
    ));
    let app = router(&fleet);
    let job = started(&fleet, &home).await;

    let said = asking(&app, true).await;
    assert!(!said.is_error, "{}", said.text);
    // The whole command is `/usr/bin/false` and cannot pass, so a passing row
    // is a row that ran the narrowed one.
    assert!(
        said.text.contains("suite") && !said.text.contains("FAILED"),
        "the narrowed command ran instead of the Check's own: {}",
        said.text
    );
    // And the row says what it was narrowed to, because a Drone reading a pass
    // has to be able to see how much of the tree it covers.
    assert!(
        said.text.contains("/bin/echo src/parse.rs"),
        "the narrowed command is on the row: {}",
        said.text
    );
    let log = crate::check_output::checks_dir(&home.path().to_string_lossy(), &job)
        .join("implement.1.dry.0.log");
    assert!(
        std::fs::read_to_string(&log)
            .expect("the log reads")
            .contains("src/parse.rs"),
        "the path reached the command as an argument"
    );
}

/// **The default, and the whole of what must not move.** A Check the Manifest
/// gave no narrower way to run is run exactly as it always was, in the same
/// call that narrowed the one beside it.
#[tokio::test]
async fn a_check_that_declares_no_narrowing_runs_whole_in_a_narrowed_run() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[by_file()]),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = asking(&app, true).await;
    assert!(said.text.contains("whole"), "{}", said.text);
    // One narrowed command on the report and not two: `whole` ran its own.
    assert_eq!(
        said.text.matches("/bin/echo ").count(),
        1,
        "the Check with no narrowing was narrowed anyway: {}",
        said.text
    );
}

/// **A Check that narrows to nothing is not passed.** A change touching nothing
/// under `crates` gives `-p` nothing to name, and a Drone told `suite` passed
/// would have been told something false about its own work.
#[tokio::test]
async fn a_check_that_narrows_to_nothing_is_skipped_rather_than_passed() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[by_package()]),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = asking(&app, true).await;
    assert!(
        said.text.contains("suite") && said.text.contains("SKIPPED"),
        "the Check is named and said to have been skipped: {}",
        said.text
    );
    assert!(
        !said.text.contains("FAILED"),
        "nothing failed, so nothing may say so: {}",
        said.text
    );
    assert!(
        !said.text.contains("3 of 3 passed"),
        "one of the three was never run: {}",
        said.text
    );
}

/// **The whole run is unmoved by a narrowing existing.** The same step, the
/// same Manifest, `only_what_changed` false — and `suite` runs `/usr/bin/false`
/// and fails, which is what the gate will do.
#[tokio::test]
async fn a_whole_run_ignores_the_narrowing_the_manifest_declares() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[by_file()]),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = asking(&app, false).await;
    assert!(
        said.text.contains("suite") && said.text.contains("FAILED"),
        "the Check's own command ran: {}",
        said.text
    );
    assert!(
        !said.text.contains("src/parse.rs"),
        "nothing was narrowed, so no row may name a path: {}",
        said.text
    );
}

/// **The report says which of the two runs it is, and says the smaller thing.**
/// A Drone that read a narrowed pass as a whole one would submit on it, so the
/// closing sentence is not the whole run's.
#[tokio::test]
async fn a_narrowed_report_says_a_pass_is_the_smaller_claim() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[by_file()]),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let narrowed = asking(&app, true).await;
    assert!(
        narrowed.text.contains("narrowed to what you changed"),
        "the report names the question that was asked: {}",
        narrowed.text
    );
    assert!(
        narrowed.text.contains("not that the repository does"),
        "and what the answer does not cover: {}",
        narrowed.text
    );
    assert!(
        narrowed.text.contains("not a verdict"),
        "without dropping what every report says: {}",
        narrowed.text
    );
}

/// **A Drone that has changed nothing is told so, and pays nothing for it.** A
/// run narrowed to an empty list would either measure nothing or, worse,
/// measure everything under a command that read as narrow.
#[tokio::test]
async fn a_narrowed_run_of_an_unchanged_worktree_is_refused_before_anything_runs() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[by_file()]),
        Arc::new(Held::started()),
        3,
        &[],
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = asking(&app, true).await;
    assert!(said.is_error, "{}", said.text);
    assert!(
        said.text.contains("holds no change"),
        "the refusal says what is missing: {}",
        said.text
    );
    // And the whole run is still there to ask for, which the refusal names.
    assert!(
        said.text.contains("only_what_changed"),
        "and what to do instead: {}",
        said.text
    );
    // Nothing was spent: the whole run still answers on the next call.
    let whole = asking(&app, false).await;
    assert!(!whole.is_error, "{}", whole.text);
}

/// **The sharpest form of "a narrowed run is not a verdict".** Every row of the
/// narrowed report passed, the Drone submitted on the strength of it, and the
/// gate ran the whole of the same Check and failed the step — because the gate
/// reads the whole of every Check whatever the Drone asked for mid-work.
///
/// Nothing changed in the worktree between the two. The difference is entirely
/// the command each run made, which is what makes this the case a narrowed pass
/// could quietly become a gate pass through.
#[tokio::test]
async fn a_narrowed_pass_does_not_reach_the_gate() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[by_file()]),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
    ));
    let app = router(&fleet);
    let job = started(&fleet, &home).await;

    let said = asking(&app, true).await;
    assert!(
        said.text.contains("PASSED") && !said.text.contains("FAILED"),
        "every check passed in the narrowed run: {}",
        said.text
    );

    submit(&app).await;
    fleet.turn().await.expect("the gate runs");
    assert_eq!(
        fleet.load(&job).await.expect("the Job").status(),
        core_model::JobStatus::AwaitingRepair,
        "the gate ran `/usr/bin/false` and reached its own verdict"
    );
}

/// **The one case where a narrowed run would be misleading rather than merely
/// smaller.** The whole `test` run in this repository excludes `acceptance`
/// because a milestone's claim is read on its own, and the narrowed command
/// cannot carry `--exclude` without `--workspace`. So the exclusion is restated
/// as `except`, and a change that touches the excluded crate and one other is
/// run against the other alone.
#[tokio::test]
async fn an_excluded_value_stays_out_of_a_narrowed_run() {
    let home = TempDir::new();
    let excluding = Narrows {
        check: "suite",
        run: "/bin/echo",
        each: "{}",
        from: &[],
        under: Some("crates"),
        except: &["acceptance"],
    };
    let fleet = Arc::new(a_fleet_over(
        &home,
        two_checks(&[excluding]),
        Arc::new(Held::started()),
        3,
        &["crates/acceptance/tests/board.rs", "crates/ipc/src/lib.rs"],
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    let said = asking(&app, true).await;
    assert!(!said.is_error, "{}", said.text);
    assert!(
        said.text.contains("/bin/echo ipc"),
        "the crate the whole run measures is what ran: {}",
        said.text
    );
    assert!(
        !said.text.contains("acceptance"),
        "a bar the gate does not apply was put in front of the Drone: {}",
        said.text
    );
}
