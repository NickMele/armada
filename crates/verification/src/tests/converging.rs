//! What the mid-step look is told, and what it may answer.
//!
//! The cases are the same two properties `judge` holds, on a call that gates
//! nothing: the brief carries the work product and no account of the Drone, and
//! an answer that establishes nothing is an error rather than a finding.

use adapter_traits::Patch;
use config::ResolvedWorkflow;
use core_model::{CheckOutcome, DeclaredPaths, RepoPath};
use testkit::{Gate, Sketch};

use crate::{Convergence, ConvergenceBrief, NotConverging, Unreadable, MID_STEP_CONVERGENCE};

fn workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "fix",
        label: "Fix",
        evidence_type: Some("diff"),
        gates: &[Gate::DiffNonempty],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn brief(off_plan: &[&str]) -> ConvergenceBrief {
    let workflow = workflow();
    ConvergenceBrief::about(
        &workflow.steps()[0],
        &Patch::of(String::from("+    let n = n - 1;\n")),
        Some(&DeclaredPaths::of(vec![RepoPath::new("src/log.rs")])),
        &off_plan
            .iter()
            .map(|path| RepoPath::new(*path))
            .collect::<Vec<RepoPath>>(),
    )
}

/// **Rule 2, on a call that is not a gate.** There is no parameter for the
/// submission or the transcript, so what the Drone said about its own progress
/// cannot reach the thing deciding whether it is making any.
#[test]
fn the_brief_carries_the_work_product_and_no_account_of_the_drone() {
    let asked = brief(&[]).question().to_string();
    assert!(asked.contains("let n = n - 1;"), "{asked}");
    assert!(asked.contains("src/log.rs"), "the declared plan is context");
    assert!(!asked.contains("turn"), "{asked}");
}

/// Drift is put as an observation with a question attached, not as a charge.
/// The step is not failed by it and the brief must not read as though it were.
#[test]
fn work_outside_the_plan_is_named_and_asked_about() {
    let asked = brief(&["src/parse.rs"]).question().to_string();
    assert!(asked.contains("src/parse.rs"), "{asked}");
    assert!(asked.contains("justified_drift"), "{asked}");
}

#[test]
fn the_three_states_read_back_as_themselves() {
    assert_eq!(
        brief(&[]).read("state: converging"),
        Ok(Convergence::Converging)
    );
    assert_eq!(
        brief(&[]).read("state: justified_drift"),
        Ok(Convergence::JustifiedDrift)
    );
}

/// A finding that names no observable is nothing the Drone could act on, and
/// the directive it would produce would be a scolding.
#[test]
fn a_finding_that_cites_nothing_is_unreadable_rather_than_thrashing() {
    assert_eq!(
        brief(&[]).read("state: thrashing"),
        Err(Unreadable::FindingCitesNothing)
    );
    assert_eq!(
        brief(&[]).read("It looks like it is going in circles."),
        Err(Unreadable::NoState)
    );
}

/// The row a person reads on the step that stopped, so the escalation says what
/// about the step rather than only that it stopped.
#[test]
fn a_finding_is_written_down_under_its_own_name() {
    let Ok(Convergence::Thrashing(why)) = brief(&[]).read(
        "state: thrashing\nexpected: the counter reaches zero\n\
         produced: the counter is still four\nconsequence: the loop never ends",
    ) else {
        panic!("a cited finding");
    };
    assert_eq!(
        why,
        NotConverging::cited(
            "the counter reaches zero",
            "the counter is still four",
            "the loop never ends"
        )
    );
    let row = why.recorded();
    assert_eq!(row.name, MID_STEP_CONVERGENCE);
    assert_eq!(row.outcome, CheckOutcome::Failed);
    assert_eq!(row.produced.as_deref(), Some("the counter is still four"));
}
