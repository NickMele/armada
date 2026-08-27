//! The Judge, end to end through Fleet's own runner.
//!
//! # These start a real child
//!
//! The fake renders a shell rather than a model, and everything else is real:
//! Fleet's spawn, Fleet's stdin write, its budget, and `verification`'s answer
//! parser. What is faked is the one thing a suite must never call.
//!
//! The three cases the milestone turns on are here — a veto stops a step whose
//! Check passed, a no-objection lets it advance, and a failed call is neither —
//! and so is the one that says the tier is cold.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Model, Worktree};
use config::ResolvedWorkflow;
use core_model::{CheckOutcome, CriterionId, JobStatus, JudgeVerdict, Timestamp};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};

use crate::gate::{apply, rule_on, AtStep, Ruling};
use crate::judging::{JudgeBudget, Judging};
use crate::tests::gate::{budget, diff_evidence, running_job, worktree};

const THE_QUESTION: &str = "Does the fix address the cause the note names?";

/// One step, gated on a Check that passes and on one narrow question.
fn judged_workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "suite",
                run: "/usr/bin/true",
                expect_exit_code: 0,
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[("c1", THE_QUESTION)],
    }])
}

fn judged_by(client: FakeJudge) -> Judging {
    Judging {
        client: Arc::new(client),
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
    }
}

async fn ruled(judge: FakeJudge, worktree: &Worktree) -> Ruling {
    let workflow = judged_workflow();
    let at = AtStep::first(workflow.frozen(), worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    rule_on(at, &diff_evidence(), &work, budget(), &judged_by(judge)).await
}

// ------------------------------------------------------------- the three

#[tokio::test]
async fn a_veto_stops_a_step_whose_check_passed() {
    let worktree = worktree();
    let ruling = ruled(
        FakeJudge::refusing(
            "the loop stops at n",
            "the loop stops at n - 1",
            "the last row is dropped",
        ),
        &worktree,
    )
    .await;

    assert!(!ruling.advanced(), "a refused step advanced");
    let Ruling::Refused {
        refusals, checks, ..
    } = &ruling
    else {
        panic!("expected a refusal, got {ruling:?}");
    };
    // The mechanical tier held. That is what makes this the Judge's doing.
    assert!(checks.iter().all(|check| check.outcome.passed()));
    assert_eq!(refusals.criteria(), vec![&CriterionId::new("c1")]);
    assert_eq!(
        refusals.cited()[0].consequence.as_deref(),
        Some("the last row is dropped")
    );
    // It ends the Job the same way a failed Check does, and the Drone is not
    // told: there is no retry ledger for it to retry against.
    assert!(ruling.ends_the_drone());
    assert!(ruling.tell().is_none());
    let job = running_job();
    let moved = apply(
        &job,
        &ruling,
        Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z"),
    )
    .expect("a refusal moves the Job")
    .expect("a legal move");
    assert_eq!(moved.job.status(), JobStatus::CompletedFailed);
}

#[tokio::test]
async fn a_no_objection_lets_the_step_advance() {
    let worktree = worktree();
    let ruling = ruled(FakeJudge::with_no_objection(), &worktree).await;

    assert!(ruling.advanced(), "{ruling:?}");
    // The record says the Judge cleared it rather than that it never ran, and
    // those are different facts about the same green step.
    assert_eq!(ruling.judged().len(), 1);
    assert_eq!(ruling.judged()[0].verdict, JudgeVerdict::Met);
    assert_eq!(ruling.judged()[0].criterion_id, CriterionId::new("c1"));
}

#[tokio::test]
async fn a_judge_call_that_fails_is_neither_a_refusal_nor_a_pass() {
    let worktree = worktree();
    let ruling = ruled(FakeJudge::that_fails("a quota that ran out"), &worktree).await;

    assert!(!ruling.advanced());
    assert!(
        matches!(ruling, Ruling::CouldNotDecide { artifact, .. } if artifact == "the Judge's answer"),
        "{ruling:?}"
    );
    assert!(
        !ruling.ends_the_drone(),
        "a failed verification ended the Job"
    );
    // The Checks it did run are still on the ruling. They happened.
    assert_eq!(ruling.checks().len(), 2);
    assert!(ruling.judged().is_empty());
}

#[tokio::test]
async fn an_answer_that_is_not_a_verdict_is_neither_either() {
    let worktree = worktree();
    let ruling = ruled(FakeJudge::saying("Looks fine to me."), &worktree).await;

    assert!(!ruling.advanced());
    assert!(
        matches!(ruling, Ruling::CouldNotDecide { .. }),
        "{ruling:?}"
    );
}

// ----------------------------------------------------------- cold by default

#[tokio::test]
async fn a_step_that_declares_no_criterion_never_asks() {
    let workflow = crate::tests::gate::workflow("/usr/bin/true");
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]);
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = Judging {
        client: Arc::clone(&judge) as Arc<_>,
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
    };

    let ruling = rule_on(at, &diff_evidence(), &work, budget(), &judging).await;

    assert!(ruling.advanced());
    assert!(
        judge.asked().is_empty(),
        "the Judge was asked about a step that declares nothing"
    );
    assert!(ruling.judged().is_empty());
}

#[tokio::test]
async fn a_failing_check_never_reaches_the_judge() {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "suite",
            run: "/usr/bin/false",
            expect_exit_code: 0,
        }],
        judged_on: &[("c1", THE_QUESTION)],
    }]);
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]);
    let judge = Arc::new(FakeJudge::refusing("a", "b", "c"));
    let judging = Judging {
        client: Arc::clone(&judge) as Arc<_>,
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
    };

    let ruling = rule_on(at, &diff_evidence(), &work, budget(), &judging).await;

    let Ruling::Failed { checks, .. } = &ruling else {
        panic!("expected a check failure, got {ruling:?}");
    };
    assert_eq!(checks[0].outcome, CheckOutcome::Failed);
    assert!(
        judge.asked().is_empty(),
        "money was spent judging work a Check had already refused"
    );
}

// ------------------------------------------------------- what it was told

#[tokio::test]
async fn the_call_carries_the_patch_and_the_facts_and_nothing_the_drone_wrote() {
    let workflow = judged_workflow();
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = Judging {
        client: Arc::clone(&judge) as Arc<_>,
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
    };

    rule_on(at, &diff_evidence(), &work, budget(), &judging).await;

    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion is one call");
    let question = &asked[0];
    assert!(question.contains("let n = n - 1"), "{question}");
    assert!(question.contains(THE_QUESTION), "{question}");
    // The submission's own words. Constitutional rule 2: a verifier that reads
    // the defendant's testimony is not independent.
    assert!(!question.contains("The loop is a fold."), "{question}");
    assert!(!question.contains("34 passing"), "{question}");
}
