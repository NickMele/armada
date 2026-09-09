//! What the mid-step look is told, and what it may answer.
//!
//! The cases are the same two properties `judge` holds, on a call that gates
//! nothing: the brief carries the work product and no account of the Drone, and
//! an answer that establishes nothing is an error rather than a finding.

use adapter_traits::Patch;
use config::ResolvedWorkflow;
use core_model::{DeclaredPaths, RepoPath, Timestamp};
use testkit::{Gate, Sketch};

use crate::{Convergence, ConvergenceBrief, NotConverging, Unreadable, A_DELIVERABLE};

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

/// The other shape a step has: its product is a file, its `artifact_exists`
/// check names it, and it changes nothing git tracks.
fn written() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "scope",
        label: "Scope the change",
        evidence_type: Some("facts_note"),
        gates: &[Gate::ArtifactExists { target: ARTIFACT }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// Where every shipped workflow sends a written step's deliverable.
const ARTIFACT: &str = ".armada/artifacts/scope.md";

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
        None,
    )
}

/// The look at a written step, with whatever Fleet read from its deliverable
/// and the empty diff such a step leaves behind.
fn about_the_file(held: Option<&str>) -> String {
    let workflow = written();
    ConvergenceBrief::about(
        &workflow.steps()[0],
        &Patch::of(String::new()),
        Some(&DeclaredPaths::of(vec![RepoPath::new(ARTIFACT)])),
        &[],
        held,
    )
    .question()
    .to_string()
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

/// **A finding outlives the instant it was taken, so anything quoting it says
/// when.** A step stopped for a missing report read an undated snapshot back as
/// its cause — the look had run two minutes earlier and the observable it named
/// as unmoved had moved nineteen seconds before that.
#[test]
fn a_finding_is_quoted_with_the_instant_the_look_was_taken() {
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
    let quoted = why.as_of(&Timestamp::from_rfc3339("2026-09-02T19:17:22.658Z"));
    assert!(quoted.contains("2026-09-02T19:17:22.658Z"), "{quoted}");
    assert!(quoted.contains("the counter reaches zero"), "{quoted}");
    assert!(quoted.contains("the counter is still four"), "{quoted}");
    // The two fields the row it replaces carried. This change is about the
    // stamp and not about what is disclosed, so the selection stays as it was.
    assert!(!quoted.contains("the loop never ends"), "{quoted}");
}

/// **The defect the deliverable closes.** A `facts_note` step produces no diff
/// — its product is the file its `artifact_exists` check names, and `.armada/`
/// is gitignored — so a look shown the diff alone saw an empty change and could
/// answer nothing but `thrashing`. One such directive read *"Produced empty
/// diff with no scoping artifacts"* against a step that had written its plan.
#[test]
fn a_written_steps_deliverable_is_what_the_look_is_shown() {
    let asked = about_the_file(Some(
        "## Boundaries\n\nsrc/log.rs, and nothing under src/net.\n",
    ));
    assert!(
        asked.contains("src/log.rs, and nothing under src/net."),
        "{asked}"
    );
    assert!(asked.contains(ARTIFACT), "the product is named: {asked}");
    assert!(
        asked.contains("an empty diff here is not itself the observable"),
        "the empty diff is explained rather than left bare: {asked}"
    );
}

/// "It declares a deliverable and it is still empty" and "it declares none" are
/// opposite findings. The first is an observable a finding can cite; read as
/// the second it disappears, which is how a step with nothing written reached a
/// Judge as a step that was asked for nothing.
#[test]
fn a_deliverable_still_empty_is_named_as_empty_rather_than_absent() {
    for held in [None, Some(""), Some("   \n")] {
        let asked = about_the_file(held);
        assert!(
            asked.contains(&format!(
                "is `{ARTIFACT}`, and nothing has been written to it yet"
            )),
            "{asked}"
        );
    }
}

/// The same false `thrashing` from the other direction: a document too big for
/// a call, called empty, would be a finding against a step that has written a
/// great deal.
#[test]
fn a_deliverable_too_big_for_a_call_is_named_rather_than_called_empty() {
    let asked = about_the_file(Some(&"x".repeat(A_DELIVERABLE + 1)));
    assert!(asked.contains("is not shown here"), "{asked}");
    assert!(asked.contains("It is not empty."), "{asked}");
    assert!(
        !asked.contains(&"x".repeat(64)),
        "the bytes reached the call"
    );
}

/// **A step that declares no deliverable is asked what it was always asked**,
/// byte for byte. The whole question is the assertion rather than a fragment of
/// it, because "unchanged" is a claim about the text and not about the lines
/// the change happened to touch.
#[test]
fn a_step_whose_product_is_the_diff_is_asked_what_it_always_was() {
    assert_eq!(brief(&[]).question(), A_DIFF_STEP_IS_ASKED);
}

/// The question a step whose product is the diff has always been asked, whole.
const A_DIFF_STEP_IS_ASKED: &str = "You are looking at a change somebody else is part-way through. Answer only the question at the end.

Step: Fix

Where the step said its work would be:
  src/log.rs

What has been produced so far, as a diff:

+    let n = n - 1;


The question: is this converging on the step, is the work outside the plan justified by the step, or is it thrashing?

Answer with nothing but the lines below.

If what has been produced is moving towards the step:

    state: converging

If it has moved outside the declared plan and the move serves the step:

    state: justified_drift

If it is not converging:

    state: thrashing
    expected: <what would be seen by now if the work were on track>
    produced: <the observable that has not moved>
    consequence: <what that difference does to whoever consumes it>

Each of the three is one line and names something in the diff above. A finding that could be written about any other change is not a finding.";
