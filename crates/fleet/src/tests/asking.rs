//! A person's answer to an open Judge question, driven through a real Fleet
//! from a Job genuinely sitting at `awaiting_review`.
//!
//! **Why through a real Fleet and not `gate::apply` alone.** The three cases
//! below are the one path nothing exercised before this file: `#729`'s crash
//! report found `answer_judge`'s `Agree` arm named in five crates and driven
//! by nothing but a stub, because the two Disagree arms happen to walk edges
//! that were already legal. Only a Job that reached `awaiting_review` through
//! a genuine refusal, held there with a real open question, can show what the
//! wire actually sends `answer_judge`.

use core_model::{
    EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepVerdict,
    TransitionReason, WhenRefused,
};
use ipc::JudgeAnswer;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, diff_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";
const REVIEW: &str = "review";
const CRITERION: &str = "c1";

/// Two steps, so a Disagree answer's `Queued` is reachable without also
/// exercising `completed` — a different capability, proved in `tests::merging`.
fn a_two_step_workflow() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: IMPLEMENT,
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[(CRITERION, "Does the fix address the cause the note names?")],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: REVIEW,
            label: "Review",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// A Job genuinely at `awaiting_review`, holding a genuine open question.
///
/// `WhenRefused::AlwaysAsk` is what turns `c1`'s own `on_refusal: refuse` —
/// every `judged_on` fixture criterion declares that — into a question rather
/// than the refusal `tests::judging::ruling` already covers.
async fn asking_a_question(home: &TempDir) -> (Fixture, JobId) {
    let fleet = a_fleet_judged_by(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n"),
        a_two_step_workflow(),
        FakeJudge::refusing(
            "the loop stops at n",
            "the loop stops at n - 1",
            "the last row is dropped",
        ),
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job);
    dispatched(&fleet, &job_id).await.expect("released to run");
    fleet
        .set_when_refused(&job_id, WhenRefused::AlwaysAsk)
        .await
        .expect("a Job may ask to be asked");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Questioned { .. })),
        "{:?}",
        turned.ruled()
    );
    assert_eq!(
        fleet.load(&job_id).await.unwrap().status(),
        JobStatus::AwaitingReview
    );
    assert!(
        fleet.judge_question_of(&job_id).await.is_some(),
        "the question this suite answers has to be genuinely open"
    );
    (fleet, job_id)
}

/// **Agree fails against the code this fixes.** `awaiting_review ->
/// escalated` belongs to `interrupted` alone, so the direct move the old
/// `Agree` arm made was `fleet.illegal_move` on every press.
#[tokio::test]
async fn agreeing_escalates_the_job_through_running_and_strands_nothing() {
    let home = TempDir::new();
    let (fleet, job_id) = asking_a_question(&home).await;

    let answered = fleet
        .answer_judge(&job_id, JudgeAnswer::Agree, None)
        .await
        .expect("agreeing takes the declared route through `running`");

    assert_eq!(answered.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(&job_id).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::GateFailure))
    );
    let step = answered.step(&StepId::new(IMPLEMENT)).expect("the row");
    assert_eq!(step.state(), StepState::Stopped);
    assert_eq!(
        step.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::GateFailure).map(StepVerdict::Failed)
    );
    assert!(
        fleet.judge_question_of(&job_id).await.is_none(),
        "answered once — nothing left for the screen to keep offering"
    );
}

/// Disagreeing once advances the step and queues the Job for its next pass —
/// the edge a person approving a review gate already walks.
#[tokio::test]
async fn disagreeing_once_advances_and_queues() {
    let home = TempDir::new();
    let (fleet, job_id) = asking_a_question(&home).await;

    let answered = fleet
        .answer_judge(&job_id, JudgeAnswer::DisagreeOnce, None)
        .await
        .expect("disagreeing walks an edge that was already legal");

    assert_eq!(answered.status(), JobStatus::Queued);
    let step = answered.step(&StepId::new(IMPLEMENT)).expect("the row");
    assert_eq!(step.state(), StepState::Advanced);
    assert!(
        fleet.judge_question_of(&job_id).await.is_none(),
        "answered once — nothing left for the screen to keep offering"
    );
}

/// Disagreeing always does what disagreeing once does to this Job, and also
/// stands the criterion down — `docs/concepts/judge.md`'s `#191` closure.
#[tokio::test]
async fn disagreeing_always_advances_and_queues() {
    let home = TempDir::new();
    let (fleet, job_id) = asking_a_question(&home).await;

    let answered = fleet
        .answer_judge(&job_id, JudgeAnswer::DisagreeAlways, None)
        .await
        .expect("disagreeing walks an edge that was already legal");

    assert_eq!(answered.status(), JobStatus::Queued);
    let step = answered.step(&StepId::new(IMPLEMENT)).expect("the row");
    assert_eq!(step.state(), StepState::Advanced);
    assert!(
        fleet.judge_question_of(&job_id).await.is_none(),
        "answered once — nothing left for the screen to keep offering"
    );
}
