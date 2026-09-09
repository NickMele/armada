//! The gate that names a policy rather than an answer.
//!
//! **`manifest_rule:<key>` is the schema's fourth form**, and what separates it
//! from the other three is that the workflow file does not say what happens —
//! the repository does. `crates/config` carries the key onto the frozen step
//! without reading it, `crates/store` round-trips it unresolved, and this is
//! where it is resolved: at the gate, against the Job in hand, and nowhere
//! earlier.
//!
//! **Both keys resolve to asking a person today, and neither by accident.**
//! `review_gate` defaults to `human_always`; `auto_merge` defaults to `never`,
//! which says no machine decides that work lands. `#525` is what lets an
//! `armada.yml` say otherwise — until then the default *is* the resolution, and
//! these cases are what will fail when it stops being.

use core_model::{JobStatus, StepId, StepState};
use testkit::FakeWorkProduct;

use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_gated_on_a_manifest_rule, a_proposal, diff_evidence, worktree_directory,
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
        worktree_directory(&home, job.id());
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
    worktree_directory(&home, job.id());
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
