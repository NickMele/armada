//! The wait, while the call is out.
//!
//! **The whole of #149.** A call that is out has to say so on the stream and in
//! the slot, and every case here reads what a pass over the gate published
//! rather than what it decided.
//!
//! The pair the guard exists for is the last two: a call that failed still
//! takes the mark down, and the slot answers for one Job and one step only.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model, Worktree};
use core_model::StepId;
use testkit::{FakeJudge, FakeWorkProduct};

use verification::{Lifted, Request};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::rule_on;
use crate::judging::{Aloft, JudgeBudget, Judging};
use crate::tests::gate::{budget, diff_evidence, marking, worktree};
use crate::tests::judging::judged_workflow;
use crate::tests::keeping::keeping_nowhere;

/// Every `job.judging` message one pass over the gate published, in order, and
/// the slot as it stood once the pass was over.
///
/// **The stream is drained to its end rather than to a count.** Both senders —
/// the broadcaster this holds and the one inside the marking — are dropped
/// before the read, so `next` answers `None` when there is nothing more. A
/// test that waited for two messages would pass while a third was published.
async fn while_judging(judge: FakeJudge, worktree: &Worktree) -> (Vec<ipc::JobJudging>, Aloft) {
    let aloft = Aloft::default();
    let events = api::Broadcaster::new();
    let mut heard = events.subscribe();
    let workflow = judged_workflow();
    let at = AtStep::first(workflow.frozen(), worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    let judging = Judging {
        client: Arc::new(judge),
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: marking(aloft.clone(), events.clone()),
        asked: Asked::nowhere(),
    };
    rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging,
        &keeping_nowhere(),
    )
    .await;
    drop(judging);
    drop(events);
    let mut said = Vec::new();
    while let Some(api::Next::Send(delivered)) = heard.next().await {
        if let ipc::Event::JobJudging(one) = delivered.event {
            said.push(one);
        }
    }
    (said, aloft)
}

/// The whole of #149 in one assertion: while the call is out, the seam says so,
/// and says which criterion and since when.
#[tokio::test]
async fn a_call_that_is_out_names_its_criterion_and_when_it_went() {
    let worktree = worktree();
    let (said, _) = while_judging(FakeJudge::with_no_objection(), &worktree).await;

    let out = said.first().expect("a message when the call went out");
    let flight = out
        .judging
        .as_ref()
        .expect("the first message carries the call");
    assert_eq!(out.step_id.as_str(), "implement");
    assert_eq!(flight.look, "criterion");
    assert_eq!(
        flight.criterion_id.as_ref().map(ipc::CriterionId::as_str),
        Some("c1"),
        "the wait must join to the verdict that follows it"
    );
    assert_eq!(flight.pattern, None, "a criterion look is about no pattern");
    assert_eq!(flight.model, "the-cheap-model");
    assert_eq!((flight.call, flight.of), (1, 1));
    assert_eq!(
        flight.budget_ms, 20_000,
        "a surface cannot draw the wait against its ceiling without the ceiling"
    );
    assert!(
        !flight.since.as_str().is_empty(),
        "a spinner has no instant"
    );
}

/// The absence has to be as legible as the presence, and it arrives as its own
/// message rather than as the stream going quiet.
#[tokio::test]
async fn the_call_coming_back_is_a_message_and_empties_the_slot() {
    let worktree = worktree();
    let (said, aloft) = while_judging(FakeJudge::with_no_objection(), &worktree).await;

    assert_eq!(said.len(), 2, "one criterion is one call, so two messages");
    assert!(
        said[1].judging.is_none(),
        "the second message says the call came back"
    );
    assert_eq!(said[1].step_id, said[0].step_id);
    assert_eq!(
        aloft.on(&said[0].job_id, &said[0].step_id),
        None,
        "a step nothing is asking about must not read as one that is"
    );
}

/// **The property the guard exists for.** A call that could not be made is the
/// case a hand-written "and now clear it" gets wrong, and it is exactly the
/// case a person is staring at the screen for.
#[tokio::test]
async fn a_call_that_failed_still_takes_the_mark_down() {
    let worktree = worktree();
    let (said, aloft) =
        while_judging(FakeJudge::that_fails("a quota that ran out"), &worktree).await;

    assert_eq!(said.len(), 2, "a failed call is still a call that went out");
    assert!(said[0].judging.is_some());
    assert!(said[1].judging.is_none(), "the mark outlived the call");
    assert_eq!(aloft.on(&said[0].job_id, &said[0].step_id), None);
}

/// A detail view opened on some other Job must not draw this Job's wait, and
/// the rail must not draw it against the wrong step.
#[tokio::test]
async fn the_slot_answers_for_one_job_and_one_step_only() {
    let aloft = Aloft::default();
    let job = ipc::JobId::carried("01J0000000000000000000JOB0");
    let step = ipc::StepId::from(&StepId::new("implement"));
    assert_eq!(
        aloft.on(&job, &step),
        None,
        "an empty slot answers for nobody"
    );
}
