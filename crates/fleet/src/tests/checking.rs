//! A step's Checks, run together rather than one after another.
//!
//! Every case here uses real commands — `sleep`, `true`, `false` — because
//! every claim it makes is about wall-clock time and about processes, and a
//! fake runner would be asserting this crate's guess at both.
//!
//! The subject is what concurrency is allowed to change and what it is not. It
//! may change how long the set takes. It may not change what is reported, in
//! what order, or on whose budget — so three of the four cases below assert
//! that something stayed exactly as it was.

use std::time::{Duration, Instant};

use adapter_traits::{Footprint, Worktree};
use core_model::CheckOutcome;
use testkit::{FakeWorkProduct, Gate, Sketch};
use verification::Request;

use crate::at_step::AtStep;
use crate::gate::{rule_on, CheckBudget, Ruling};
use crate::tests::gate::{diff_evidence, judging, worktree};

/// The `(name, outcome)` of every Check the step declared, in the order the
/// ruling carries them.
fn recorded(ruling: &Ruling) -> Vec<(&str, CheckOutcome)> {
    ruling
        .checks()
        .iter()
        .map(|check| (check.name.as_str(), check.outcome))
        .collect()
}

/// Rule on a step declaring `gates`, against a worktree that touched
/// `touched`.
async fn ruled<'a>(gates: &'a [Gate<'a>], budget: Duration, touched: &[&str]) -> Ruling {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates,
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(touched);
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        Some(&Footprint::nothing()),
        &[],
        &work,
        CheckBudget::of(budget),
        &judging(),
    )
    .await
}

fn slow(name: &str) -> Gate<'_> {
    Gate::Check {
        name,
        run: "/bin/sleep 0.6",
        expect_exit_code: 0,
        when: &[],
    }
}

/// The claim the issue is about, and the only one here that is about speed.
///
/// Three Checks of six hundred milliseconds each cost 1.8s one at a time. The
/// assertion is deliberately loose — one and a bit rather than a fraction over
/// six hundred — because what would make it fail is the serial loop coming
/// back, and a threshold tight enough to catch a scheduler hiccup is a test
/// that fails on a busy machine for no reason.
#[tokio::test]
async fn three_slow_checks_cost_about_one_of_them_rather_than_the_sum() {
    let gates = [slow("build"), slow("test"), slow("storybook")];
    let began = Instant::now();
    let ruling = ruled(&gates, Duration::from_secs(20), &["src/lib.rs"]).await;
    let took = began.elapsed();

    assert!(
        ruling.advanced(),
        "three passing checks advance the step: {ruling:?}"
    );
    assert!(
        took < Duration::from_millis(1300),
        "three 600ms checks took {took:?}, which is the serial loop"
    );
}

/// **The order of the report is the step's, not the scheduler's.**
///
/// `slow` sleeps, `broken` exits 1 immediately and `quick` exits 0
/// immediately, so the order they finish in is the reverse of the order they
/// are declared in. A person reading the failure needs to find `broken` in the
/// middle, where the workflow puts it, on every run.
///
/// It is also the case for no short-circuit: `quick` was declared after the
/// failure and has a row of its own, which a loop that stopped at the first
/// failing Check could not produce.
#[tokio::test]
async fn a_check_is_reported_where_it_was_declared_and_not_where_it_finished() {
    let gates = [
        slow("slow"),
        Gate::Check {
            name: "broken",
            run: "/usr/bin/false",
            expect_exit_code: 0,
            when: &[],
        },
        Gate::Check {
            name: "quick",
            run: "/usr/bin/true",
            expect_exit_code: 0,
            when: &[],
        },
    ];

    let ruling = ruled(&gates, Duration::from_secs(20), &["src/lib.rs"]).await;

    assert_eq!(
        recorded(&ruling),
        vec![
            ("slow", CheckOutcome::Passed),
            ("broken", CheckOutcome::Failed),
            ("quick", CheckOutcome::Passed),
        ]
    );
}

/// **A Check that waits for a slot has not started spending.**
///
/// Five Checks of six hundred milliseconds against a budget of nine hundred.
/// The bound is four, so the fifth begins at about six hundred milliseconds and
/// ends at about twelve hundred — past a deadline taken over the set, and
/// comfortably inside its own. All five pass, and under one shared budget the
/// last one would read `TimedOut` for a Check that did what it was asked.
#[tokio::test]
async fn a_check_that_waited_for_a_slot_is_not_charged_for_the_wait() {
    let gates = [
        slow("one"),
        slow("two"),
        slow("three"),
        slow("four"),
        slow("five"),
    ];

    let ruling = ruled(&gates, Duration::from_millis(900), &["src/lib.rs"]).await;

    assert_eq!(
        recorded(&ruling),
        vec![
            ("one", CheckOutcome::Passed),
            ("two", CheckOutcome::Passed),
            ("three", CheckOutcome::Passed),
            ("four", CheckOutcome::Passed),
            ("five", CheckOutcome::Passed),
        ]
    );
}

/// A skip is an observation and occupies its slot, which is what `Ran::of`
/// refuses a short list for.
///
/// `elsewhere` would fail if it ran — it is `/usr/bin/false` against an
/// expected zero — so the step advancing is the proof that it did not, and its
/// row sitting between the two that did is the proof that a skip is not simply
/// left out of a concurrently-collected list.
#[tokio::test]
async fn a_skipped_check_holds_its_place_among_the_ones_that_ran() {
    let gates = [
        Gate::Check {
            name: "covered",
            run: "/usr/bin/true",
            expect_exit_code: 0,
            when: &["src/**"],
        },
        Gate::Check {
            name: "elsewhere",
            run: "/usr/bin/false",
            expect_exit_code: 0,
            when: &["docs/**"],
        },
        Gate::Check {
            name: "always",
            run: "/usr/bin/true",
            expect_exit_code: 0,
            when: &[],
        },
    ];

    let ruling = ruled(&gates, Duration::from_secs(20), &["src/lib.rs"]).await;

    assert_eq!(
        recorded(&ruling),
        vec![
            ("covered", CheckOutcome::Passed),
            ("elsewhere", CheckOutcome::Skipped),
            ("always", CheckOutcome::Passed),
        ]
    );
    assert!(
        ruling.advanced(),
        "the skipped check would have failed had it run: {ruling:?}"
    );
}

/// Rule on a step that must have written `target`, against a real directory
/// holding whatever `write` puts there.
///
/// A real directory rather than a fake: the whole of `artifact_exists` is a
/// `stat`, so a fake filesystem would be asserting this crate's guess at one.
async fn ruled_on_a_file(target: &str, write: impl FnOnce(&std::path::Path)) -> Ruling {
    let dir = crate::tests::tmp::TempDir::new();
    write(dir.path());
    let workflow = testkit::resolved(&[Sketch {
        id: "plan",
        label: "Plan the change",
        evidence_type: Some("facts_note"),
        gates: &[Gate::ArtifactExists { target }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let worktree = Worktree::at(
        dir.path().to_string_lossy().to_string(),
        "armada/01J0000000000000000000JOB0",
    );
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &crate::tests::gate::note_evidence(),
        None,
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::changed(&[".armada/artifacts/plan.md"]),
        CheckBudget::of(Duration::from_secs(5)),
        &judging(),
    )
    .await
}

/// **The step advances on the file, not on the sentence about it.**
///
/// This is the whole of #138 measured end to end: the step declares the path,
/// Fleet reads the worktree, and what it found is what decides.
#[tokio::test]
async fn a_step_that_wrote_its_file_advances_and_one_that_did_not_does_not() {
    let ruling = ruled_on_a_file(".armada/artifacts/plan.md", |root| {
        std::fs::create_dir_all(root.join(".armada/artifacts")).expect("the directory");
        std::fs::write(root.join(".armada/artifacts/plan.md"), "The cause is X.\n")
            .expect("the file");
    })
    .await;
    assert!(ruling.advanced(), "the file was written: {ruling:?}");
    assert_eq!(
        recorded(&ruling),
        [(".armada/artifacts/plan.md", CheckOutcome::Passed)]
    );

    let ruling = ruled_on_a_file(".armada/artifacts/plan.md", |_| {}).await;
    assert!(!ruling.advanced(), "nothing was written: {ruling:?}");
    assert_eq!(
        recorded(&ruling),
        [(".armada/artifacts/plan.md", CheckOutcome::Failed)]
    );
}

/// A Drone that created the file and wrote nothing into it has not delivered
/// the step's product, and the gate says which of the two happened.
#[tokio::test]
async fn an_empty_file_stops_the_step_and_the_row_says_it_is_empty() {
    let ruling = ruled_on_a_file(".armada/artifacts/plan.md", |root| {
        std::fs::create_dir_all(root.join(".armada/artifacts")).expect("the directory");
        std::fs::write(root.join(".armada/artifacts/plan.md"), "").expect("the file");
    })
    .await;

    assert!(!ruling.advanced(), "an empty file advanced: {ruling:?}");
    let row = &ruling.checks()[0];
    assert_eq!(row.outcome, CheckOutcome::Failed);
    assert_eq!(
        row.produced.as_deref(),
        Some("`.armada/artifacts/plan.md` is there and holds nothing")
    );
}

/// **The look is confined to the worktree by the parser, so nothing here has
/// to re-check it.** What this asserts is the half that is this function's:
/// the path is joined onto the Job's own worktree and not onto the process's
/// working directory, so two Jobs on one machine cannot satisfy each other.
#[tokio::test]
async fn the_file_is_looked_for_in_the_jobs_worktree_and_not_beside_the_daemon() {
    let elsewhere = crate::tests::tmp::TempDir::new();
    std::fs::create_dir_all(elsewhere.path().join(".armada/artifacts")).expect("the directory");
    std::fs::write(
        elsewhere.path().join(".armada/artifacts/plan.md"),
        "another Job's plan\n",
    )
    .expect("the file");

    let ruling = ruled_on_a_file(".armada/artifacts/plan.md", |_| {}).await;
    assert!(
        !ruling.advanced(),
        "a file in another worktree advanced this step: {ruling:?}"
    );
}
