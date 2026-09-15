//! Evidence a Drone submitted survives a restart before the gate ever ran
//! on it. #796.

use core_model::{JobStatus, StepId, StepState};
use testkit::FakeWorkProduct;

use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, diff_evidence, note_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// The Definition of Done, run directly: submit, restart before the gate
/// runs, and the second Fleet rules on the same submission at boot.
///
/// `summarise` has no Checks — `two_steps()`'s own doc calls that the common
/// shape rather than the edge one — so what a restart recovers is not
/// entangled with `diff_nonempty`'s separate need for a baseline no Drone
/// survives to hand back; `implement`, gated on one, carries this Job there
/// first over an ordinary turn.
#[tokio::test]
async fn evidence_submitted_before_a_restart_is_ruled_on_after_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("implement's gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "{:?}",
        turned.ruled()
    );

    // The receipt returns; the gate never runs. No `fleet.turn()` follows —
    // this is the crash the durable row exists for.
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    drop(fleet);

    let second = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let reconciled = second.reconcile().await.expect("the boot read");
    assert_eq!(
        reconciled.recovered_evidence,
        vec![job_id.clone()],
        "the row the restart found was ruled on"
    );

    let carried = second.load(&job_id).await.unwrap();
    assert_eq!(
        carried
            .step(&StepId::new("summarise"))
            .expect("the step")
            .state(),
        StepState::Advanced,
        "no Checks stood between the submission and a ruling"
    );
    assert_eq!(
        carried.status(),
        JobStatus::CompletedSuccess,
        "the last step delivers, and the ruling finished the Job"
    );
    assert!(
        second
            .store()
            .lock()
            .await
            .pending_evidence()
            .expect("the table reads")
            .is_empty(),
        "the row is cleared once the ruling is applied"
    );
}
