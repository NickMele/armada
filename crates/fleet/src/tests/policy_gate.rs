//! The gate that names a policy rather than an answer.
//!
//! **`manifest_rule:<key>` is the schema's fourth form**, and what separates it
//! from the other three is that the workflow file does not say what happens —
//! the repository does. `crates/config` carries the key onto the frozen step
//! without reading it, `crates/store` round-trips it unresolved, and this is
//! where it is resolved: at the gate, against the Job in hand, and nowhere
//! earlier.
//!
//! **Both keys default to asking a person, and neither by accident.**
//! `review_gate` defaults to `human_always`; `auto_merge` defaults to `never`,
//! which says no machine decides that work lands. `#525` is what lets an
//! `armada.yml` say otherwise, and the cases below the divider are what it
//! says — including the one that refuses to be said, where a policy asks a step
//! for a judgment the step never declared.

use core_model::{JobStatus, StepId, StepState};
use testkit::FakeWorkProduct;

use crate::gate::Ruling;
use crate::policy::HeldBecause;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_gated_on_a_manifest_rule, a_fleet_gated_on_a_manifest_rule_saying, a_proposal,
    diff_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// A step gated on either policy reaches the same place a `human_always` step
/// reaches, and reaches it the same way — through a real dispatch, a real
/// submission and the real gate, because a stand-in at exactly this seam would
/// hide whether the fourth form can arrive at the surface at all.
#[tokio::test]
async fn a_step_gated_on_a_manifest_policy_holds_for_a_person() {
    for key in ["review_gate", "auto_merge"] {
        let home = TempDir::new();
        let fleet = a_fleet_gated_on_a_manifest_rule(
            &home,
            FakeWorkProduct::changed(&["src/log.rs"]),
            "implement",
            key,
        );

        let job = fleet
            .propose(a_proposal("fix the off-by-one"))
            .await
            .expect("a Job at the approval gate");
        worktree_directory(&home, &job);
        dispatched(&fleet, job.id()).await.expect("it dispatches");
        submitted_by_the_one(&fleet, diff_evidence())
            .await
            .expect("the Drone reports its diff");

        let turned = fleet.turn().await.expect("the gate runs");
        assert!(
            matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
            "manifest_rule:{key} did not hold the Job for a person: {:?}",
            turned.ruled()
        );
        let held = fleet.load(job.id()).await.expect("the Job is there");
        assert_eq!(held.status(), JobStatus::AwaitingReview);
        assert_eq!(
            held.step(&StepId::new("implement".to_string()))
                .map(|step| step.state()),
            Some(StepState::AwaitingHuman)
        );
    }
}

/// **And the frozen step still says which policy it was.** A gate that had been
/// resolved on the way to the record would read as a workflow that declared
/// `human_always`, which is a claim about the file rather than about the
/// repository — and it would go on making it after the repository changed its
/// mind, which is what the policy being `Live` forbids.
#[tokio::test]
async fn the_record_keeps_the_policy_rather_than_the_answer() {
    let home = TempDir::new();
    let fleet = a_fleet_gated_on_a_manifest_rule(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "implement",
        "review_gate",
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, &job);
    let dispatched = dispatched(&fleet, job.id()).await.expect("it dispatches");
    assert_eq!(
        dispatched
            .workflow()
            .step(&StepId::new("implement".to_string()))
            .expect("the gated step")
            .advance_gate()
            .as_wire(),
        "manifest_rule:review_gate"
    );
}

// ------------------------------------------------- what the repository decides

/// **A repository can now say otherwise, and the step advances.**
/// `review_gate: auto_if_judge_passes` on a step that asks the Judge something
/// reaches the same place `advance_gate: auto_if_judge_passes` reaches, by
/// resolution rather than by declaration — which is the whole of what the
/// fourth gate form buys.
#[tokio::test]
async fn a_repository_that_says_auto_if_judge_passes_advances_a_judged_step() {
    let home = TempDir::new();
    let fleet = a_fleet_gated_on_a_manifest_rule_saying(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "implement",
        "review_gate",
        "review_gate: auto_if_judge_passes\n",
        Some("does it read?"),
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.expect("it dispatches");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");

    let turned = fleet.turn().await.expect("the gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the policy did not advance a judged step: {:?}",
        turned.ruled()
    );
    let moved = fleet.load(job.id()).await.expect("the Job is there");
    assert_eq!(moved.status(), JobStatus::Running);
}

/// **The hazard, and the rule.** `review_gate` may resolve to
/// `auto_if_judge_passes` on a step that asks the Judge nothing — legal, and
/// the shape the designed Code Review declares — and such a step advancing
/// would advance on the mechanical tier alone while reading as judged. That is
/// the disagreement `config::workflow::step` refuses when the gate is a plain
/// word and cannot refuse when it is a policy, because parse time knows the key
/// and not what it resolved to.
///
/// **It holds for a person rather than failing.** The step passed every tier it
/// declared; nothing about the work is wrong, and a Drone handed this back
/// could not fix it from a worktree. Holding is the cautious value of the
/// policy's own two, which is the principle the resolution already runs on.
#[tokio::test]
async fn a_judgeless_step_holds_for_a_person_however_the_policy_resolved() {
    let home = TempDir::new();
    let fleet = a_fleet_gated_on_a_manifest_rule_saying(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "implement",
        "review_gate",
        "review_gate: auto_if_judge_passes\n",
        None,
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.expect("it dispatches");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");

    let turned = fleet.turn().await.expect("the gate runs");
    let Some(Ruling::HeldForReview { held, .. }) = turned.ruled() else {
        panic!(
            "a judgeless step advanced on a policy it cannot satisfy: {:?}",
            turned.ruled()
        );
    };
    // **The reason is on the ruling and not inferred**, because the repository
    // asked for automation and did not get it — and the sentence naming why is
    // what tells somebody there is a file to fix rather than a Job to wait on.
    assert_eq!(*held, HeldBecause::TheStepAsksNoJudge);
    assert!(held
        .worth_saying()
        .is_some_and(|said| said.contains("review_gate")));
    let stopped = fleet.load(job.id()).await.expect("the Job is there");
    assert_eq!(stopped.status(), JobStatus::AwaitingReview);
}

/// **`auto_merge` holds whatever it resolved to**, and that is the policy being
/// honoured rather than ignored: it decides who may take the work off the gate,
/// not whether the gate is there. A step that advanced here would advance past
/// the merge without one — the sentence the record already got wrong once, in
/// the other direction, and `crate::merging` is what presses instead.
#[tokio::test]
async fn auto_merge_always_still_holds_the_step_and_does_not_advance_it() {
    let home = TempDir::new();
    let fleet = a_fleet_gated_on_a_manifest_rule_saying(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "implement",
        "auto_merge",
        "auto_merge: always\n",
        None,
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.expect("it dispatches");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");

    let turned = fleet.turn().await.expect("the gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "auto_merge: always advanced the step instead of holding it: {:?}",
        turned.ruled()
    );
    assert_eq!(
        fleet.load(job.id()).await.expect("the Job").status(),
        JobStatus::AwaitingReview
    );
}
