//! What the Judge is shown of the file a step was asked to write.
//!
//! **A separate file from `judging` because the subject is different**, not
//! because that one grew: these cases are about a step whose product is a
//! document on disk rather than a diff, and every one of them turns on Fleet
//! opening a path the *frozen workflow* named. `judging` is about the tier —
//! a veto, a no-objection, a call that failed.
//!
//! The two that must not be confused are here together on purpose. A missing
//! file is the mechanical tier's answer and costs nothing; a file too big to
//! put in a call is an undecided ruling that reaches a person. Neither is a
//! refusal, and a Judge asked to notice either would be a model call spent on
//! a `stat`.

use std::sync::Arc;

use adapter_traits::{Footprint, Worktree};
use config::ResolvedWorkflow;
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};
use verification::Request;

use crate::at_step::AtStep;
use crate::gate::{rule_on, Ruling};
use crate::tests::gate::{budget, judged_by_shared, note_evidence};
use crate::tests::tmp::TempDir;

/// A `facts_note` step gated on the file it was asked to write, and on one
/// question about it.
fn delivering_workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "plan",
        label: "Plan the change",
        evidence_type: Some("facts_note"),
        gates: &[Gate::ArtifactExists {
            target: ".armada/artifacts/plan.md",
        }],
        judged_on: &[("c1", "Does this plan name a specific root cause?")],
        scope: None,
        gaming: None,
    }])
}

/// Rule on that step against a real directory, with `contents` at the declared
/// path where `write` is asked for.
async fn ruled_on_a_deliverable(judge: Arc<FakeJudge>, contents: Option<&str>) -> Ruling {
    let dir = TempDir::new();
    if let Some(contents) = contents {
        std::fs::create_dir_all(dir.path().join(".armada/artifacts")).expect("the directory");
        std::fs::write(dir.path().join(".armada/artifacts/plan.md"), contents).expect("the file");
    }
    let worktree = Worktree::at(
        dir.path().to_string_lossy().to_string(),
        "armada/01J0000000000000000000JOB0",
    );
    let workflow = delivering_workflow();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    rule_on(
        at,
        Request::of(testkit::asked_for()),
        &note_evidence(),
        None,
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::changed(&[]),
        budget(),
        &judged_by_shared(judge),
    )
    .await
}

/// **The whole redirection, end to end.** Fleet opens the path the frozen
/// workflow named and the bytes are in the call — no diff, no git, and nothing
/// the Drone chose. The submission said one thing and the file says another,
/// so a brief carrying only the summary would be visibly the wrong material.
#[tokio::test]
async fn the_judge_is_shown_the_file_the_step_was_asked_to_write() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = ruled_on_a_deliverable(
        Arc::clone(&judge),
        Some("The cause is the `-0.0` comparison in spend.rs:88.\n"),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion, one call");
    assert!(
        asked[0].contains("The cause is the `-0.0` comparison in spend.rs:88."),
        "the file's contents are in the call: {}",
        asked[0]
    );
    assert!(
        asked[0].contains(".armada/artifacts/plan.md"),
        "and the call says where they were read from: {}",
        asked[0]
    );
}

/// **A missing file never reaches the Judge**, because the mechanical tier
/// stops the step first. Asserted so that the read added at the gate cannot
/// quietly become the thing that catches it — a Judge asked to notice an absent
/// deliverable is a model call spent on a `stat`.
#[tokio::test]
async fn a_step_that_wrote_nothing_is_stopped_before_a_call_is_made() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = ruled_on_a_deliverable(Arc::clone(&judge), None).await;

    assert!(!ruling.advanced(), "{ruling:?}");
    assert!(
        judge.asked().is_empty(),
        "the mechanical tier answered, so nothing was spent: {:?}",
        judge.asked()
    );
}

/// **Too big is a call that could not be made, not a refusal.** A Drone that
/// wrote five megabytes produced something no criterion was written for, and
/// answering `not_met` would be a verdict about the size. The step stops and a
/// person reads it, which is what every other unmakeable call does.
#[tokio::test]
async fn a_deliverable_too_big_for_a_call_decides_neither_way() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let huge = "x".repeat(verification::A_DELIVERABLE + 1);
    let ruling = ruled_on_a_deliverable(Arc::clone(&judge), Some(&huge)).await;

    assert!(judge.asked().is_empty(), "no call was made");
    let Ruling::CouldNotDecide {
        artifact,
        cause,
        checks,
        ..
    } = &ruling
    else {
        panic!("expected an undecided ruling, got {ruling:?}");
    };
    assert_eq!(*artifact, "the step's deliverable");
    assert!(
        cause.to_string().contains(".armada/artifacts/plan.md"),
        "{cause}"
    );
    // The mechanical tier still held and its result is real: the file is there,
    // and being there is all `artifact_exists` ever asserted.
    assert!(checks.iter().all(|check| check.outcome.passed()));
}

/// **A file that is not text is undecided, not refused.** `artifact_exists`
/// asserts a file is there and has bytes in it, which a PNG satisfies; what
/// cannot happen is a criterion answered against material nobody could read.
/// The failure has to be visible as a call that was not made, because a step
/// that quietly dropped its deliverable and asked the Judge about the summary
/// instead is the substitution this whole capability removes.
#[tokio::test]
async fn a_deliverable_that_is_not_text_decides_neither_way() {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.path().join(".armada/artifacts")).expect("the directory");
    std::fs::write(
        dir.path().join(".armada/artifacts/plan.md"),
        [0x89u8, 0x50, 0x4e, 0x47, 0xff, 0xfe],
    )
    .expect("the file");
    let worktree = Worktree::at(
        dir.path().to_string_lossy().to_string(),
        "armada/01J0000000000000000000JOB0",
    );
    let workflow = delivering_workflow();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &note_evidence(),
        None,
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::changed(&[]),
        budget(),
        &judged_by_shared(Arc::clone(&judge)),
    )
    .await;

    assert!(judge.asked().is_empty(), "no call was made");
    let Ruling::CouldNotDecide { artifact, .. } = &ruling else {
        panic!("expected an undecided ruling, got {ruling:?}");
    };
    assert_eq!(*artifact, "the step's deliverable");
}
