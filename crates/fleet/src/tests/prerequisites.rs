//! `checks.<name>.requires`: the Commands Fleet runs before a Check.
//!
//! **Real commands and a real directory**, for `checking`'s reason and one
//! more of its own: every claim here is about an effect one process left in a
//! worktree for the next one to find, and a fake runner would be asserting this
//! crate's guess at that.
//!
//! The effect is a file. `/usr/bin/touch` writes it, `/bin/test -f` reads it,
//! and `/bin/mkdir` is how "did this run twice?" is asked without a shell —
//! the second `mkdir` of one name fails, so a Check that passes is a
//! prerequisite that ran once.
//!
//! # The incident
//!
//! Job `01M1M6YP7G0012GVQ2PCB6VF2E` spent all three `implement` attempts on
//! `format` — `cargo fmt --all --check` — failing at a different unformatted
//! line each time, while `commands.fmt: cargo fmt --all` sat in the same
//! `armada.yml`. Nothing could make the fix run. The first test below is that
//! Job, in miniature.

use std::time::Duration;

use adapter_traits::{Footprint, Worktree};
use core_model::CheckOutcome;
use testkit::{FakeWorkProduct, Gate, Sketch};
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, CheckBudget, Ruling};
use crate::tests::gate::{diff_evidence, judging};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;

/// The `(name, outcome, produced)` of every Check the step declared, in order.
fn recorded(ruling: &Ruling) -> Vec<(String, CheckOutcome, Option<String>)> {
    ruling
        .checks()
        .iter()
        .map(|check| (check.name.clone(), check.outcome, check.produced.clone()))
        .collect()
}

/// Rule on one step whose Checks may require Commands, in a real directory.
///
/// The worktree is the temp directory itself, so a prerequisite's effect is
/// somewhere the Checks that follow it can see — which is the whole subject.
async fn ruled(
    home: &TempDir,
    gates: &[Gate<'_>],
    commands: &[(&str, &str)],
    requires: &[(&str, &[&str])],
    touched: &[&str],
) -> Ruling {
    let workflow = testkit::requiring(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates,
            judged_on: &[],
            scope: None,
            gaming: None,
        }],
        commands,
        requires,
    );
    let worktree = Worktree::at(
        home.path().to_string_lossy().to_string(),
        "armada/01J0000000000000000000JOB0",
    );
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(touched);
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        CheckBudget::of(Duration::from_secs(20)),
        &judging(),
        &keeping_nowhere(),
    )
    .await
}

fn check<'a>(name: &'a str, run: &'a str) -> Gate<'a> {
    Gate::Check {
        name,
        run,
        expect_exit_code: 0,
        when: &[],
    }
}

#[tokio::test]
async fn a_check_whose_fix_is_declared_passes_where_it_used_to_fail() {
    // #387, in miniature. `formatted` stands in for a formatted worktree:
    // without the prerequisite the Check fails, and no number of reattempts
    // changes that, because a Drone was never given the fix to run.
    let home = TempDir::new();
    let ruling = ruled(
        &home,
        &[check("format", "/bin/test -f formatted")],
        &[("fmt", "/usr/bin/touch formatted")],
        &[("format", &["fmt"])],
        &["src/lib.rs"],
    )
    .await;

    assert_eq!(
        recorded(&ruling),
        [("format".to_string(), CheckOutcome::Passed, None)]
    );
    assert!(home.path().join("formatted").is_file());
}

#[tokio::test]
async fn two_checks_naming_one_command_run_it_once() {
    // The skip `docs/concepts/manifest.md` specifies, asked without a shell:
    // `mkdir` of a name that exists fails, so a second run would block both
    // Checks. Both passing is the assertion that it ran exactly once.
    let home = TempDir::new();
    let ruling = ruled(
        &home,
        &[
            check("one", "/bin/test -d once"),
            check("two", "/bin/test -d once"),
        ],
        &[("migrate", "/bin/mkdir once")],
        &[("one", &["migrate"]), ("two", &["migrate"])],
        &["src/lib.rs"],
    )
    .await;

    assert_eq!(
        recorded(&ruling)
            .iter()
            .map(|(name, outcome, _)| (name.clone(), *outcome))
            .collect::<Vec<_>>(),
        [
            ("one".to_string(), CheckOutcome::Passed),
            ("two".to_string(), CheckOutcome::Passed),
        ]
    );
}

#[tokio::test]
async fn a_failed_prerequisite_is_reported_as_the_prerequisite_and_not_the_check() {
    // The attribution rule. A Check told it failed, when what failed was the
    // command meant to fix it, sends a Drone to the wrong file — which is
    // exactly what a broken `migrate` reading as a broken test suite would do.
    let home = TempDir::new();
    let ruling = ruled(
        &home,
        &[check("test", "/usr/bin/true")],
        &[("migrate", "/usr/bin/false")],
        &[("test", &["migrate"])],
        &["src/lib.rs"],
    )
    .await;

    let recorded = recorded(&ruling);
    assert_eq!(recorded[0].1, CheckOutcome::NeverRan);
    let produced = recorded[0].2.clone().expect("a reason");
    assert!(
        produced.contains("migrate") && produced.contains("/usr/bin/false"),
        "the reason names neither the Command nor its line: {produced}"
    );
}

#[tokio::test]
async fn a_check_that_requires_nothing_still_runs_when_another_check_is_blocked() {
    // A broken `migrate` is not a reason to stop asking `lint`. The step fails
    // either way; what is at stake is whether the person reading it gets one
    // result or two.
    let home = TempDir::new();
    let ruling = ruled(
        &home,
        &[
            check("needs_it", "/usr/bin/true"),
            check("lint", "/usr/bin/true"),
        ],
        &[("migrate", "/usr/bin/false")],
        &[("needs_it", &["migrate"])],
        &["src/lib.rs"],
    )
    .await;

    assert_eq!(
        recorded(&ruling)
            .iter()
            .map(|(name, outcome, _)| (name.clone(), *outcome))
            .collect::<Vec<_>>(),
        [
            ("needs_it".to_string(), CheckOutcome::NeverRan),
            ("lint".to_string(), CheckOutcome::Passed),
        ]
    );
}

#[tokio::test]
async fn a_skipped_check_does_not_pay_for_its_prerequisites() {
    // A Check the step's changes do not cover runs nothing, so the minutes its
    // `migrate` would cost are minutes spent for an answer nobody asked for.
    let home = TempDir::new();
    let ruling = ruled(
        &home,
        &[Gate::Check {
            name: "e2e",
            run: "/usr/bin/true",
            expect_exit_code: 0,
            when: &["packages/**"],
        }],
        &[("migrate", "/bin/mkdir migrated")],
        &[("e2e", &["migrate"])],
        &["crates/fleet/src/lib.rs"],
    )
    .await;

    assert_eq!(recorded(&ruling)[0].1, CheckOutcome::Skipped);
    assert!(
        !home.path().join("migrated").exists(),
        "the prerequisite of a skipped Check ran anyway"
    );
}

#[tokio::test]
async fn prerequisites_run_in_the_order_the_manifest_names_them() {
    // A sequence, not a set: the second `mkdir` needs the parent the first one
    // made. **The names are deliberately against the alphabet** — `seed` is
    // declared before `migrate` — so a list that got sorted or collected into
    // a `BTreeSet` on its way here fails rather than passing by luck.
    let home = TempDir::new();
    let ruling = ruled(
        &home,
        &[check("e2e", "/bin/test -d schema/rows")],
        &[
            ("seed", "/bin/mkdir schema"),
            ("migrate", "/bin/mkdir schema/rows"),
        ],
        &[("e2e", &["seed", "migrate"])],
        &["src/lib.rs"],
    )
    .await;

    assert_eq!(recorded(&ruling)[0].1, CheckOutcome::Passed);
}
