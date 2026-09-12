//! Reconciling a Job the old `Agree` arm of `crate::asking` could strand —
//! `#733`.
//!
//! **Built directly, not through the old code path.** The fixed
//! `answer_judge` cannot produce this state any more, so the wedge is built
//! with the same primitive it always used to stop the step
//! (`Fleet::move_step_by`) and the same store call it always used to clear
//! the question, skipping only the Job-level move that used to fail.
//!
//! **A second Fleet, not the same one**, for `tests::adopting`'s own reason:
//! `reconcile` is a boot read, and the live Fleet already holds this Job's
//! Drone in a slot a restart does not carry.

use core_model::{
    Actor, EscalationTrigger, JobStatus, StepId, StepLevelTrigger, StepState, StepTarget,
    StepVerdict,
};
use testkit::FakeWorkProduct;

use crate::tests::asking::asking_a_question;
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;
use crate::transcript::log_of;

const IMPLEMENT: &str = "implement";

fn logged(home: &TempDir, handle: &str) -> String {
    std::fs::read_to_string(log_of(&home.path().to_string_lossy(), handle)).unwrap_or_default()
}

/// The bug's own shape: `awaiting_review`, current step `stopped`, no
/// question left to answer. Reconciliation finishes the move the old code
/// could not, and says so in the Job's own log.
#[tokio::test]
async fn a_wedged_review_is_moved_to_escalated_at_boot() {
    let home = TempDir::new();
    let step = StepId::new(IMPLEMENT);
    let trigger =
        StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("gate_failure is step-level");

    let (wedging, job_id) = asking_a_question(&home).await;
    let job = wedging.load(&job_id).await.unwrap();
    let handle = job.handle();
    wedging
        .move_step_by(&job, &step, StepTarget::Stopped(trigger), Actor::Human)
        .await
        .expect("the step machine admits `awaiting_human -> stopped` on its own");
    wedging
        .store()
        .lock()
        .await
        .clear_judge_question(&job_id)
        .expect("clearing what `answer_judge` always clears");
    let confirmed = wedging.load(&job_id).await.unwrap();
    assert_eq!(confirmed.status(), JobStatus::AwaitingReview);
    assert_eq!(confirmed.step(&step).unwrap().state(), StepState::Stopped);
    assert!(wedging.judge_question_of(&job_id).await.is_none());
    drop(wedging);

    let second = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let reconciled = second.reconcile().await.expect("the boot read");
    assert_eq!(reconciled.mended, vec![job_id.clone()]);

    let mended = second.load(&job_id).await.unwrap();
    assert_eq!(mended.status(), JobStatus::Escalated);
    let mended_step = mended.step(&step).unwrap();
    assert_eq!(mended_step.state(), StepState::Stopped);
    assert_eq!(
        mended_step.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::GateFailure).map(StepVerdict::Failed)
    );
    assert!(
        logged(&home, &handle).contains("Fleet finished the move on boot"),
        "a person reading the record later has to see it was Fleet and not them"
    );
}

/// **The state this must not catch.** A Job genuinely holding an open
/// question sits `awaiting_review` too, but its current step is
/// `awaiting_human`, never `stopped` — the one field the condition is
/// narrowed on. A boot read must leave it exactly where it was.
#[tokio::test]
async fn a_job_genuinely_awaiting_review_is_left_alone() {
    let home = TempDir::new();
    let step = StepId::new(IMPLEMENT);

    let (asking, job_id) = asking_a_question(&home).await;
    let held = asking.load(&job_id).await.unwrap();
    assert_eq!(held.step(&step).unwrap().state(), StepState::AwaitingHuman);
    drop(asking);

    let second = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let reconciled = second.reconcile().await.expect("the boot read");
    assert!(
        reconciled.mended.is_empty(),
        "a step waiting on a person is not a step this repairs: {:?}",
        reconciled.mended
    );

    let untouched = second.load(&job_id).await.unwrap();
    assert_eq!(untouched.status(), JobStatus::AwaitingReview);
    assert_eq!(
        untouched.step(&step).unwrap().state(),
        StepState::AwaitingHuman
    );
    assert!(second.judge_question_of(&job_id).await.is_some());
}
