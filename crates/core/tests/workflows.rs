//! The four starters Armada ships, held against the parser that reads them.
//!
//! `templates/guild/workflows/` is what `armada guild init` copies into
//! `~/.armada/guild/workflows/`, so a starter that does not parse is a machine
//! on which no Job can be spawned at all. The files are the specification and
//! the parser follows them — the same relationship `tests/fixtures/` has with
//! the `armada.yml` contract, and for the same reason: a shipped document that
//! only its author ever validated is a document that stops being true.

use armada_core::fleet::workflow::{self, EndsAt, Predicate, Runner};
use std::path::{Path, PathBuf};

fn templates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates/guild/workflows")
}

fn read(name: &str) -> workflow::Workflow {
    let path = templates().join(format!("{name}.yml"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is shipped and readable ({e})", path.display()));
    workflow::parse(&text, &format!("workflows/{name}.yml"))
        .unwrap_or_else(|e| panic!("{} does not parse: {e}", path.display()))
}

#[test]
fn every_shipped_starter_parses_and_is_named_after_its_file() {
    for name in workflow::STARTERS {
        let parsed = read(name);
        assert_eq!(parsed.name, name, "{name}.yml names a different workflow");
        assert!(!parsed.steps.is_empty());
        assert!(
            !parsed.description.is_empty(),
            "{name}.yml has no description, and `fleet spawn` shows one"
        );
    }
}

/// **Design and plan always end at you** (PLAN.md §14.4). Design has no
/// automated pass condition — no command can tell you an approach is right — and
/// a plan is the thing you approve before anything builds. Only `feature` and
/// `bug` may close autonomously, and only because `check` gives them something
/// objective to close against.
#[test]
fn only_the_two_workflows_with_an_objective_gate_can_close_on_their_own() {
    assert_eq!(read("design").ends_at, EndsAt::Human);
    assert_eq!(read("plan").ends_at, EndsAt::Human);
    assert_eq!(read("feature").ends_at, EndsAt::Branch);
    assert_eq!(read("bug").ends_at, EndsAt::Branch);
}

/// **The two predicates that make the set trustworthy.** Without
/// `human_approves`, a Drone builds the wrong thing efficiently; without
/// `failing_test_exists`, a Drone "fixes" a bug it never reproduced and closes
/// green.
#[test]
fn feature_stops_for_approval_and_bug_reproduces_before_it_fixes() {
    let feature = read("feature");
    assert!(
        feature
            .steps
            .iter()
            .any(|step| step.verify.must == Predicate::HumanApproves),
        "feature would build without asking"
    );

    let bug = read("bug");
    assert_eq!(
        bug.first_step().verify.must,
        Predicate::FailingTestExists,
        "bug's first step is the reproduction, or the fix has no evidence"
    );
    assert!(
        bug.first_step().verify.test.is_some(),
        "the predicate names no test, so it holds vacuously"
    );
}

/// `feature`'s first step is a **sub-Job**, so the plan you approved is a
/// durable artifact you can point at later rather than a paragraph buried in a
/// longer Job's transcript (PLAN.md §14.6).
#[test]
fn features_first_step_runs_plan_as_a_sub_job() {
    let feature = read("feature");
    assert_eq!(
        feature.first_step().runner(),
        Runner::SubJob("plan".to_string())
    );
    assert_eq!(feature.first_step().verify.must, Predicate::SubjobPassed);
}

/// **`review` is Fleet's to satisfy, not the Drone's.** Fleet spawns a second
/// Job with the diff and the original task, in its own context; the Drone under
/// review never calls `fleet.spawn`, because a worker able to spawn workers is a
/// fork bomb with a budget.
#[test]
fn the_review_step_names_no_runner_because_fleet_is_the_runner() {
    for name in ["feature", "bug"] {
        let workflow = read(name);
        let review = workflow
            .steps
            .iter()
            .find(|step| step.id == "review")
            .unwrap_or_else(|| panic!("{name} has no review step"));
        assert_eq!(review.runner(), Runner::Fleet);
        assert_eq!(review.verify.must, Predicate::ReviewClean);
    }
}

/// **Ceilings are per workflow, and exhaustion is never a silent stop**
/// (PLAN.md §14.6). Design and plan end at you regardless, so theirs is a
/// runaway guard; feature and bug can close autonomously, so theirs bounds real
/// spend.
#[test]
fn every_starter_carries_a_ceiling_that_ends_at_a_person() {
    for name in workflow::STARTERS {
        let budget = read(name).budget;
        assert!(budget.attempts > 0, "{name} has no attempt ceiling");
        assert!(budget.cost_usd > 0.0, "{name} has no cost ceiling");
        assert!(budget.wall_clock_ms > 0, "{name} has no wall clock");
        assert_eq!(
            budget.on_exhausted,
            workflow::OnExhausted::NeedsHuman,
            "{name} would stop silently"
        );
    }
}
