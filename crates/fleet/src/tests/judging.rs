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
use core_model::{
    CheckOutcome, CriterionId, EscalationTrigger, JobStatus, JudgeVerdict, StepLevelTrigger,
    StepState, StepVerdict, Timestamp, TransitionReason,
};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};

use ipc::{JobDetail, RunId};

use crate::at_step::AtStep;
use crate::gate::{apply, rule_on, Ruling};
use crate::judging::{JudgeBudget, Judging};
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, worktree_directory};
use crate::tests::detail::get;
use crate::tests::gate::{budget, diff_evidence, fresh, running_job, worktree};
use crate::tests::tmp::TempDir;

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
        scope: None,
        gaming: None,
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
    rule_on(
        at,
        &diff_evidence(),
        None,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judged_by(judge),
    )
    .await
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
    // The Drone goes and is not told: there is no retry ledger for it to retry
    // against. The Job does not go with it.
    // Kept, not ended — an escalated Job's Drone is `Alive, idle` per
    // `job-statuses.toml`, which is what a redirect resumes. See the same
    // correction in `tests::gaming`.
    assert!(!ruling.ends_the_drone());
    assert!(ruling.tell().is_none());
    let job = running_job();
    let moved = apply(
        &job,
        &ruling,
        Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z"),
    )
    .expect("a refusal moves the Job")
    .expect("a legal move");
    assert_eq!(
        moved.job.status(),
        JobStatus::Escalated,
        "a refusal is stopped-and-needs-a-person, not over"
    );
    assert!(!moved.job.status().is_terminal(), "a refusal is answerable");
    assert_eq!(
        moved.event.reason(),
        &TransitionReason::Escalation(EscalationTrigger::GateFailure),
        "the trigger says the gate stopped; the criteria say why"
    );
}

/// **The two verdicts read differently, and that is the point of the change.**
/// A Check failing means the work is broken and the Job is over. A Judge
/// refusing means the work runs and is not what was asked for, which is a
/// person's to answer.
#[tokio::test]
async fn a_failed_check_still_ends_the_job_where_a_refusal_does_not() {
    let worktree = worktree();
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
        scope: None,
        gaming: None,
    }]);
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]);
    let ruling = rule_on(
        at,
        &diff_evidence(),
        None,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judged_by(FakeJudge::with_no_objection()),
    )
    .await;

    assert!(matches!(ruling, Ruling::Failed { .. }), "{ruling:?}");
    assert!(
        ruling.judged().is_empty(),
        "the Judge is never asked past a failing Check"
    );
    let moved = apply(
        &running_job(),
        &ruling,
        Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z"),
    )
    .expect("a failure moves the Job")
    .expect("a legal move");
    assert_eq!(moved.job.status(), JobStatus::CompletedFailed);
    assert!(moved.job.status().is_terminal());
}

/// **A refusal stops the step; it does not un-run the Checks.** What the
/// mechanical tier recorded is still on the ruling, which is what the person
/// reading the escalation needs in order to see that the work builds.
#[tokio::test]
async fn a_refusal_leaves_what_the_checks_recorded_standing() {
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

    assert_eq!(
        ruling.checks().len(),
        2,
        "both declared Checks are recorded"
    );
    assert!(ruling.checks().iter().all(|check| check.outcome.passed()));
    assert_eq!(ruling.judged().len(), 1, "the record says the Judge ran");
}

/// **The end of the line for a refusal**: the loop rules, the Job escalates,
/// and the citation is on the detail view a person opens.
///
/// A terminal status had nowhere to put the three lines. This is why the
/// change was worth making — the trigger says the gate stopped, and only the
/// criterion says what is wrong with the work.
#[tokio::test]
async fn a_refusal_escalates_the_job_and_its_citation_reaches_the_detail_view() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_workflow(),
        FakeJudge::refusing(
            "the loop stops at n",
            "the loop stops at n - 1",
            "the last row is dropped",
        ),
    );
    let job = fleet
        .propose(a_proposal("widen the bound instead of fixing it"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
    fleet
        .submit_evidence(crate::tests::daemon::diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(matches!(turned.ruled, Some(Ruling::Refused { .. })));

    let escalated = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(escalated.status(), JobStatus::Escalated);
    assert!(
        !escalated.status().is_terminal(),
        "a refusal leaves something to answer"
    );
    // **The step stops, and it says why.** `job-statuses.toml` gives an
    // escalated Job the step state `stopped`; a step left `running` beneath a
    // status the inner machine freezes would be a step nothing could ever
    // write a verdict onto.
    let stopped = escalated
        .step(&core_model::StepId::new("implement"))
        .expect("the row is there");
    assert_eq!(stopped.state(), StepState::Stopped);
    assert_eq!(
        stopped.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::GateFailure).map(StepVerdict::Failed),
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");

    assert_eq!(detail.job.status.as_wire(), "escalated");
    assert_eq!(
        detail
            .job
            .reason
            .as_ref()
            .and_then(|reason| reason.named.as_deref()),
        Some("gate_failure"),
        "the Board reads the trigger, and it is not a Check's failure"
    );
    assert_eq!(detail.steps[0].state.as_wire(), "stopped");
    assert_eq!(
        detail.steps[0]
            .last_verdict
            .as_ref()
            .map(|verdict| (verdict.named.as_str(), verdict.trigger.as_deref())),
        Some(("failed", Some("gate_failure"))),
        "the step's own verdict crosses, not only the Job's reason"
    );
    let refused = &detail.steps[0].judged[0];
    assert_eq!(refused.verdict.as_wire(), "not_met");
    assert_eq!(
        refused.consequence.as_deref(),
        Some("the last row is dropped"),
        "the line a person triages on survived the escalation"
    );
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

    let ruling = rule_on(
        at,
        &diff_evidence(),
        None,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judging,
    )
    .await;

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
        scope: None,
        gaming: None,
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

    let ruling = rule_on(
        at,
        &diff_evidence(),
        None,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judging,
    )
    .await;

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

    rule_on(
        at,
        &diff_evidence(),
        None,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judging,
    )
    .await;

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
