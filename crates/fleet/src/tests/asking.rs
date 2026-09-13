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

use std::sync::Arc;
use std::time::Duration;

use api::{Next, Subscription};
use core_model::{
    EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepVerdict,
    TransitionReason, WhenRefused,
};
use ipc::{Event, JudgeAnswer};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_judged_by, a_proposal, diff_evidence, fittings, one, worktree_directory, Ticking,
};
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
pub(super) async fn asking_a_question(home: &TempDir) -> (Fixture, JobId, ipc::Instant) {
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
    let asked_at = fleet
        .judge_question_of(&job_id)
        .await
        .expect("the question this suite answers has to be genuinely open")
        .asked_at;
    (fleet, job_id, asked_at)
}

/// Find the one event a Bridge dock reacts to: this Job's move to
/// `awaiting_review`. `#935`'s race is between this announcement and the
/// question `get_job` answers with, so a case proving the race is closed
/// reads exactly this — the same signal a client has, and nothing more.
async fn moved_to_awaiting_review(watching: &mut Subscription, job_id: &JobId) -> ipc::Instant {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::JobStateChanged(changed) = delivered.event {
                        if changed.job_id == ipc::JobId::from(job_id)
                            && changed.to.domain() == JobStatus::AwaitingReview
                        {
                            return changed.at;
                        }
                    }
                }
                Some(Next::Missed(dropped)) => {
                    panic!("the stream dropped {dropped} events in so short a run")
                }
                None => panic!("the stream closed before the Job moved"),
            }
        }
    })
    .await
    .expect("the Job reaches awaiting_review")
}

/// **`#935`'s race, closed — proved by the clock rather than by timing.**
/// `Ticking` hands out a strictly later reading on every call `self.now()`
/// makes, in the order Fleet makes them, on one thread — so the two readings
/// this case compares are not a snapshot taken after the fact, they are two
/// specific calls inside the one `act_on` invocation this turn makes, ordered
/// exactly as that invocation made them. A stream re-drained after `turn`
/// returns cannot tell the two call orders apart — both writes are long done
/// by then regardless of which came first — which is why this compares the
/// two stamps `Ticking` gave each write, not the state the store settles into.
#[tokio::test]
async fn the_question_is_stamped_before_the_move_to_awaiting_review_is() {
    let home = TempDir::new();
    let mut fittings = fittings(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n"),
    );
    fittings.starting().workflows = one(a_two_step_workflow());
    fittings.judge = Arc::new(FakeJudge::refusing(
        "the loop stops at n",
        "the loop stops at n - 1",
        "the last row is dropped",
    ));
    fittings.clock = Arc::new(Ticking::from_nine());
    let fleet: Fixture = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("released to run");
    fleet
        .set_when_refused(&job_id, WhenRefused::AlwaysAsk)
        .await
        .expect("a Job may ask to be asked");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the tool took it");

    // Subscribed before the ruling runs — the same order `Broadcaster`'s own
    // doc requires of a resync, so nothing published in this turn can be
    // missed.
    let mut watching = fleet.events().subscribe();
    let turned = fleet.turn().await.expect("the gate rules");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Questioned { .. })),
        "{:?}",
        turned.ruled()
    );

    let announced_at = moved_to_awaiting_review(&mut watching, &job_id).await;
    let asked_at = fleet
        .judge_question_of(&job_id)
        .await
        .expect("the question is on record at all")
        .asked_at;
    assert!(
        asked_at < announced_at,
        "the question was stamped {asked_at:?}, the announcement {announced_at:?} — \
         the write has to precede the move it is about"
    );
}

/// **Agree fails against the code this fixes.** `awaiting_review ->
/// escalated` belongs to `interrupted` alone, so the direct move the old
/// `Agree` arm made was `fleet.illegal_move` on every press.
#[tokio::test]
async fn agreeing_escalates_the_job_through_running_and_strands_nothing() {
    let home = TempDir::new();
    let (fleet, job_id, asked_at) = asking_a_question(&home).await;

    let answered = fleet
        .answer_judge(&job_id, JudgeAnswer::Agree, Some(asked_at), None)
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
    let (fleet, job_id, asked_at) = asking_a_question(&home).await;

    let answered = fleet
        .answer_judge(&job_id, JudgeAnswer::DisagreeOnce, Some(asked_at), None)
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
    let (fleet, job_id, asked_at) = asking_a_question(&home).await;

    let answered = fleet
        .answer_judge(&job_id, JudgeAnswer::DisagreeAlways, Some(asked_at), None)
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

/// An answer naming a question this Job is not holding open is refused
/// rather than applied to whatever is open now — the same shape as
/// `crate::questioning::NotAnswered::Superseded`.
#[tokio::test]
async fn answering_a_stale_question_is_refused() {
    let home = TempDir::new();
    let (fleet, job_id, asked_at) = asking_a_question(&home).await;
    let stale = ipc::Instant::carried(format!("{}-stale", asked_at.as_str()));

    let refused = fleet
        .answer_judge(&job_id, JudgeAnswer::Agree, Some(stale), None)
        .await
        .expect_err("a mismatched asked_at is refused, not applied");

    assert!(
        matches!(refused, Adrift::NotAnswerable { .. }),
        "{refused:?}"
    );
    assert!(
        fleet.judge_question_of(&job_id).await.is_some(),
        "a refused answer leaves the real question open"
    );
}

/// A peer built before 13.35 sends none, and this trusts whatever is open —
/// the whole of what `answer_judge` did before `asked_at` existed.
#[tokio::test]
async fn answering_with_no_asked_at_trusts_whatever_is_open() {
    let home = TempDir::new();
    let (fleet, job_id, _asked_at) = asking_a_question(&home).await;

    let answered = fleet
        .answer_judge(&job_id, JudgeAnswer::Agree, None, None)
        .await
        .expect("an absent asked_at answers the question genuinely open");

    assert_eq!(answered.status(), JobStatus::Escalated);
}
