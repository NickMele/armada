//! Which of a step's Checks run at all, decided from what the step changed.
//!
//! **The subject is `when`, and the decision is taken before any spawn.** A
//! Check declares the paths it covers, `core_model::covers` owns the dialect,
//! and this is the one place that proves Fleet reads a step's changed files,
//! asks that question of every Check the step declares, and records a row for
//! the ones it decided not to run. Every command here would fail if it ran,
//! which is how a case tells a Check that was skipped from a Check that
//! passed: a skip that actually spawned produces `Ruling::Failed`.
//!
//! **Not `scope`, and the two are the pair most easily confused.** That file is
//! the *evidence* scope — the footprint a step left against the paths it was
//! told it could touch, which tags a step for a Judge look and fails nothing.
//! This one is a Check's own `when`, which decides whether a command runs. One
//! is about what the Drone was allowed to write; the other is about what is
//! worth verifying.
//!
//! **Not `gate` either, which is why it left.** That file rules on evidence
//! against Checks that run; nothing here is about the ruling. It also carries
//! the last case of the cold-by-default half — a step whose Checks declare no
//! `when` never reads a file list for one — because that assertion is about the
//! read that did not happen, and the read is this file's subject.

use adapter_traits::{Change, Footprint};
use config::ResolvedWorkflow;
use core_model::CheckOutcome;
use testkit::{FakeWorkProduct, Gate, Sketch};
use verification::{CheckFailed, Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, Ruling};
use crate::tests::gate::{budget, diff_evidence, judging, workflow, worktree};
use crate::tests::keeping::keeping_nowhere;

/// A one-step workflow whose only Check declares which paths it covers.
///
/// **`run` is a command that would fail if it ran**, which is how a test can
/// tell a Check that was skipped from a Check that passed: a skip that actually
/// spawned would produce `Ruling::Failed` rather than an advance.
fn scoped_workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "storybook",
            run: "/usr/bin/false",
            expect_exit_code: 0,
            when: &["packages/**"],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// The gate over [`scoped_workflow`], against a worktree holding `files`.
async fn ruling_over(work: &FakeWorkProduct) -> Ruling {
    let workflow = scoped_workflow();
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await
}

#[tokio::test]
async fn a_check_whose_paths_the_step_did_not_touch_is_not_run() {
    // The Check's command exits 101. A step that advances is a step that never
    // spawned it — the skip is decided before the spawn, which is the whole
    // saving.
    let work = FakeWorkProduct::changed(&["crates/fleet/src/gate.rs"]);
    let ruling = ruling_over(&work).await;

    assert!(ruling.advanced(), "the ruling was {ruling:?}");
    // One step, so the advance is the last one.
    let Ruling::Finished { checks, output, .. } = &ruling else {
        panic!("the ruling was {ruling:?}");
    };
    assert_eq!(checks.len(), 1, "a skip is a row like any other");
    assert_eq!(checks[0].name, "storybook");
    assert_eq!(checks[0].outcome, CheckOutcome::Skipped);
    assert_eq!(
        checks[0].produced.as_deref(),
        Some("no changed file is under packages/**")
    );
    assert!(
        output.is_empty(),
        "a Check that did not run printed nothing"
    );
}

#[tokio::test]
async fn a_check_whose_paths_the_step_touched_runs_and_can_fail() {
    // The mirror. The same Check, the same command, one changed path — and the
    // step fails, which is what proves the skip above was the `when` and not
    // the gate quietly declining to run anything.
    let work = FakeWorkProduct::changed(&["packages/components/src/Badge.tsx"]);
    let ruling = ruling_over(&work).await;

    assert!(!ruling.advanced(), "the ruling was {ruling:?}");
    let Ruling::Failed { failures, .. } = &ruling else {
        panic!("the ruling was {ruling:?}");
    };
    assert_eq!(
        failures,
        &[CheckFailed::WrongExitCode {
            check: "storybook".to_string(),
            expected: 0,
            actual: 1,
        }]
    );
}

#[tokio::test]
async fn a_step_that_skipped_every_check_advances_and_does_not_read_as_passing_them() {
    // **The distinction the issue is about.** The step advanced; the record
    // says nothing was verified. A gate that recorded a pass here would be
    // reporting a verification it never did, and a gate that recorded no rows
    // at all would be indistinguishable from a step whose Checks never ran.
    let work = FakeWorkProduct::changed(&["docs/OPEN.md"]);
    let ruling = ruling_over(&work).await;

    assert!(ruling.advanced());
    let checks = ruling.checks();
    assert!(
        !checks.iter().all(|check| check.outcome.passed()),
        "no Check passed"
    );
    assert!(
        checks.iter().all(|check| check.outcome.advances()),
        "and none of them stopped the step"
    );

    // **And the Drone is not told otherwise.** "It passed every check the step
    // declared" is the sentence a step whose checks actually ran gets, and
    // saying it here would be telling a Drone a check passed that nobody ran.
    let told = ruling.tell().expect("a turn").text().to_string();
    assert!(
        !told.contains("passed every check"),
        "nothing passed: {told}"
    );
    assert!(
        told.contains("No check the step declares covers what you changed"),
        "told: {told}"
    );
}

#[tokio::test]
async fn a_deleted_file_is_a_change_to_the_directory_it_was_in() {
    // A file removed from `packages/` is a change to `packages/`. The kind of
    // change is not read anywhere in the skip decision, which is what makes
    // this fall out rather than be a case somebody wrote.
    let work = FakeWorkProduct::changing(&[("packages/components/src/Gone.tsx", Change::Deleted)]);
    assert!(!ruling_over(&work).await.advanced(), "the Check ran");
}

#[tokio::test]
async fn a_rename_out_of_a_covered_directory_still_runs_the_check() {
    // **Established from the adapter, not assumed.** `crates/adapters` runs no
    // rename detection, so git reports a rename as two deltas — the old path
    // deleted and the new one added — and `changed_files` carries both. Either
    // side is enough on its own, which is why moving a file *out* of
    // `packages/` still runs the Check that covers `packages/`.
    let work = FakeWorkProduct::changing(&[
        ("packages/components/src/Badge.tsx", Change::Deleted),
        ("apps/desktop/src/Badge.tsx", Change::Added),
    ]);
    assert!(!ruling_over(&work).await.advanced(), "the Check ran");
}

#[tokio::test]
async fn a_step_whose_checks_declare_no_paths_never_reads_the_diff_for_one() {
    // The cold-by-default half. `workflow` declares no `when` anywhere, so the
    // gate asks for a file list only where `diff_nonempty` or an evidence scope
    // wants one — and neither does here. What it reads is the footprint, which
    // is a different question and a different call.
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "the ruling was {ruling:?}");
    assert!(
        work.listed().is_empty(),
        "no Check declares `when`, so no file list was read: {:?}",
        work.listed()
    );
}
