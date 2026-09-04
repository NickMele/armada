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
//!
//! The last two are #153's: a step whose work product is a note is judged
//! against the note, and a later step is judged against what an earlier one
//! established. Both go through Fleet's real runner, because both were built
//! at the type level and the thing that shipped broken was the wiring.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model, Worktree};
use config::ResolvedWorkflow;
use core_model::{
    CheckOutcome, CriterionId, DeclaredPaths, EscalationTrigger, JobStatus, JudgeVerdict, RepoPath,
    StepId, StepLevelTrigger, StepState, StepVerdict, Timestamp, TransitionReason,
};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Scoped, Sketch};

use ipc::{JobDetail, RunId};
use verification::{Lifted, Request};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::{apply, rule_on, Ruling};
use crate::judging::{Aloft, JudgeBudget, Judging, Marking};
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, worktree_directory};
use crate::tests::detail::get;
use crate::tests::gate::{
    budget, diff_evidence, judged_by_shared, marking, note_evidence, running_job, worktree,
};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

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
                when: &[],
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
        marking: Marking::detached(),
        asked: Asked::nowhere(),
    }
}

async fn ruled(judge: FakeJudge, worktree: &Worktree) -> Ruling {
    let workflow = judged_workflow();
    let at = AtStep::first(workflow.frozen(), worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
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
        &judged_by(judge),
        &keeping_nowhere(),
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
    // The Drone goes and is not told. The retry budget exists and does not
    // reach here: it answers a check that ran and said no, and resubmitting
    // against the same instructions would produce the same work a Judge just
    // refused. The Job does not go with the Drone.
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
/// A Check failing means the work is unfinished, so the Job waits at
/// `awaiting_repair` for somebody to say what is missing. A Judge refusing
/// means the work runs and is not what was asked for, which is a different
/// question and lands at `escalated`. Neither is terminal, and #208 is why the
/// first stopped being — what the two still do not share is the trigger, the
/// acts offered, or whether a person may overrule it.
#[tokio::test]
async fn a_failed_check_holds_the_job_somewhere_else_than_a_refusal() {
    let worktree = worktree();
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "suite",
            run: "/usr/bin/false",
            expect_exit_code: 0,
            when: &[],
        }],
        judged_on: &[("c1", THE_QUESTION)],
        scope: None,
        gaming: None,
    }]);
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]);
    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judged_by(FakeJudge::with_no_objection()),
        &keeping_nowhere(),
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
    assert_eq!(moved.job.status(), JobStatus::AwaitingRepair);
    assert!(
        !moved.job.status().is_terminal(),
        "unfinished work is not a failed Job"
    );
    assert_ne!(
        moved.job.status(),
        JobStatus::Escalated,
        "and it is not a verdict waiting to be overruled either"
    );
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
    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(matches!(turned.ruled(), Some(Ruling::Refused { .. })));

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
        marking: Marking::detached(),
        asked: Asked::nowhere(),
    };

    let ruling = rule_on(
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
            when: &[],
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
        marking: Marking::detached(),
        asked: Asked::nowhere(),
    };

    let ruling = rule_on(
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
        marking: Marking::detached(),
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

// ---------------------------------------------- a work product that is a note

/// Feature's opening pair, reduced: a step that writes a scope note and is
/// judged on it, then a step whose diff is measured against that note.
fn writing_workflow() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "scope",
            label: "Scope the change",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[(
                "names_what_it_will_touch",
                "Does this scope note name the specific files it will touch?",
            )],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[(
                "implements_the_scope",
                "Does this diff implement the scope note?",
            )],
            scope: Some(Scoped {
                diff_check: false,
                at_step_start: false,
                exclude: &[],
                references: &["scope.evidence"],
            }),
            gaming: None,
        },
    ])
}

/// **The defect this issue names, end to end.** A `facts_note` step has no
/// diff, and the Judge used to be handed an empty one and refuse — every time,
/// on the first gated step of four workflows. The note is what the step
/// produced, so the note is what the call carries.
#[tokio::test]
async fn a_step_whose_work_product_is_a_note_is_judged_against_the_note() {
    let workflow = writing_workflow();
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    // Nothing on disk. That is the whole shape of the failure: the step is
    // finished and the worktree has not moved.
    let work = FakeWorkProduct::untouched();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));

    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &note_evidence(),
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

    assert!(
        ruling.advanced(),
        "a note the Judge did not refuse advances the step: {ruling:?}"
    );
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion is one call");
    let question = &asked[0];
    assert!(
        question.contains("The path is derived from the repo name."),
        "the note is the work product and did not reach the call: {question}"
    );
    assert!(question.contains("worktree.rs:40"), "{question}");
    assert!(
        !question.contains("The change, as a diff"),
        "there is no diff on this step: {question}"
    );
    assert_eq!(ruling.judged().len(), 1, "the record says the Judge ran");
}

/// The other half of the same rule, through the runner. A step whose product is
/// the change gets the diff, and its submission — prose about work the diff
/// already shows — reaches nothing.
#[tokio::test]
async fn a_later_step_is_measured_against_what_an_earlier_one_established() {
    let workflow = writing_workflow();
    let worktree = worktree();
    let at = AtStep::named(workflow.frozen(), &StepId::new("implement"), &worktree)
        .expect("the second step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));
    let recorded = vec![(StepId::new("scope"), note_evidence().recorded())];
    // The step's scope asks the Drone where its work will be, so it declares.
    // That is unrelated to the yardstick and is what the block's other key is
    // for — `context_paths` and `reference_docs` stay separate on purpose.
    let declared = DeclaredPaths::of(vec![RepoPath::new("src")]);

    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        Some(&declared),
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &recorded,
        &work,
        budget(),
        &judging,
        &keeping_nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    let question = &judge.asked()[0];
    assert!(
        question.contains("`scope` established: The path is derived from the repo name."),
        "the yardstick the step names did not reach the call: {question}"
    );
    assert!(
        question.contains("is not itself under judgment"),
        "the yardstick has to be told apart from the target: {question}"
    );
    assert!(question.contains("let n = n - 1"), "{question}");
    // Rule 2, unchanged: this step's product is the change, so this step's own
    // words are a claim about it and stay out.
    assert!(!question.contains("The loop is a fold."), "{question}");
}

/// **A guaranteed refusal is now a call that was never made.** A step whose
/// product is the change, with nothing changed, used to reach the Judge with an
/// empty patch. It decides neither way instead, which is what a person needs to
/// see.
#[tokio::test]
async fn a_step_with_nothing_to_show_costs_no_call_and_draws_no_verdict() {
    let workflow = writing_workflow();
    let worktree = worktree();
    let at = AtStep::named(workflow.frozen(), &StepId::new("implement"), &worktree)
        .expect("the second step");
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));
    let declared = DeclaredPaths::of(vec![RepoPath::new("src")]);
    // The step declares `diff_nonempty`, so the mechanical tier stops it first
    // and the Judge is never reached — which is the cheaper of the two guards
    // and the one that fires here.
    let ruling = rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        Some(&declared),
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::untouched(),
        budget(),
        &judging,
        &keeping_nowhere(),
    )
    .await;

    assert!(!ruling.advanced(), "{ruling:?}");
    assert!(
        judge.asked().is_empty(),
        "a call was bought against an empty page"
    );
}

/// **The request reaches the model, not just the brief.** `verification` proves
/// `Brief::about` renders it; this proves the whole path — a Job row, `rule_on`,
/// Fleet's own process runner — puts it in front of the thing that answers.
///
/// The criterion is Feature's designed `scope` wording, restored on #169. It is
/// unanswerable unless the request is in the same question, which is why it was
/// narrowed the first time the workflows were ported.
#[tokio::test]
async fn a_criterion_asking_what_was_requested_reaches_a_call_that_carries_it() {
    let workflow = testkit::resolved(&[Sketch {
        id: "scope",
        label: "Scope the change",
        evidence_type: Some("facts_note"),
        gates: &[],
        judged_on: &[(
            "scope",
            "Does this scope note address what was actually requested, without \
             expanding beyond it?",
        )],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    // A real record, not the shared fixture: the three parts of this request
    // are what the assertions below look for, one each.
    let asked_for = testkit::asking(
        "The log reader drops the last line",
        "`read_all` stops one row short with no trailing newline.",
        &["the last line is returned"],
    );
    let judge = Arc::new(FakeJudge::with_no_objection());
    let judging = judged_by_shared(Arc::clone(&judge));

    let ruling = rule_on(
        at,
        Request::of(&asked_for),
        &note_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::untouched(),
        budget(),
        &judging,
        &keeping_nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    let question = &judge.asked()[0];
    for part in [
        "The log reader drops the last line",
        "stops one row short",
        "the last line is returned",
    ] {
        assert!(
            question.contains(part),
            "the Judge was asked about the request and not shown it: {question}"
        );
    }
}

// ------------------------------------------------------ a call while it is out

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
