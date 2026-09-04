//! The gaming check through Fleet's own gate: what it stops, and how.
//!
//! Every case here starts from evidence whose Checks all pass — that is the
//! condition the tier exists for. What differs is the diff, and what the Job
//! does about it.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model};
use config::ResolvedWorkflow;
use core_model::{
    EscalationTrigger, EvidenceRef, EvidenceType, GamingPattern, JobStatus, StepEvidence, StepId,
    StepLevelTrigger, StepVerdict, Timestamp, TransitionReason,
};

use ipc::{JobDetail, RunId};
use testkit::{FakeJudge, FakeWorkProduct, Gaming, Sketch};
use verification::{Lifted, Request};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::{apply, rule_on, Ruling};
use crate::judging::{JudgeBudget, Judging, Marking};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, worktree_directory};
use crate::tests::detail::get;
use crate::tests::gate::{budget, diff_evidence, running_job, worktree};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// The diff a Drone submits when it narrows the gate instead of fixing the
/// bug: the `run:` string is untouched and the config it resolves through is
/// not.
const GAMED: &str = "diff --git a/jest.config.js b/jest.config.js\n\
                     --- a/jest.config.js\n\
                     +++ b/jest.config.js\n\
                     -  testPathIgnorePatterns: [\"/node_modules/\"],\n\
                     +  testPathIgnorePatterns: [\"/node_modules/\", \"/tests/edge-cases/\"],\n";

const HONEST: &str = "diff --git a/src/limiter.ts b/src/limiter.ts\n\
                      -    if (this.tokens < 0) {\n\
                      +    if (this.tokens <= 0) {\n";

/// Two steps: one that records a baseline, one that is checked against it. The
/// second declares only patterns the diff answers, so nothing here reaches a
/// model.
fn workflow(flag_if: &[&str], baseline: Option<&str>) -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "scope",
            label: "Scope",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "regression_verify",
            label: "Verify the suite",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: Some(Gaming { baseline, flag_if }),
        },
    ])
}

fn judging() -> Judging {
    Judging {
        // Every case below declares only diff-answered patterns, so a judge
        // that refuses everything proves the model is never reached: a call
        // made here would come back flagged for the wrong pattern.
        client: Arc::new(FakeJudge::saying("flag: yes\ncited: everything")),
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: Marking::detached(),
        asked: Asked::nowhere(),
    }
}

fn recorded() -> Vec<(StepId, StepEvidence)> {
    vec![(
        StepId::new("scope"),
        StepEvidence {
            evidence_type: EvidenceType::FactsNote,
            claimed: "the rollover tests hold the window boundary".to_string(),
            shown_by: "docs/notes/rollover.md".to_string(),
            not_claimed: String::new(),
        },
    )]
}

async fn ruled(patch: &str, flag_if: &[&str], recorded: &[(StepId, StepEvidence)]) -> Ruling {
    let workflow = workflow(flag_if, Some("scope.evidence"));
    let worktree = worktree();
    let at = AtStep::named(
        workflow.frozen(),
        &StepId::new("regression_verify"),
        &worktree,
    )
    .expect("a step of the workflow");
    let work = FakeWorkProduct::changed(&["jest.config.js"]).showing(patch);
    rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        recorded,
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await
}

/// **The whole claim.** Every Check passed, nothing was refused, and the Job
/// stops anyway — as `evidence_suspect`, which says the evidence is not to be
/// trusted rather than that the work failed.
#[tokio::test]
async fn evidence_that_narrows_its_own_gate_escalates_as_suspect_rather_than_failing() {
    let ruling = ruled(GAMED, &["check_config_edited"], &recorded()).await;

    let Ruling::Suspect { ref flagged, .. } = ruling else {
        panic!("the gaming check let it through: {ruling:?}");
    };
    assert_eq!(flagged.patterns(), [GamingPattern::CheckConfigEdited]);
    assert!(
        flagged.cited()[0].cited.contains("jest.config.js"),
        "a flag names what a person is being asked to look at"
    );
    assert!(!ruling.advanced());
    // **The Drone is kept, not ended.** `job-statuses.toml` says an escalated
    // Job's Drone is `Alive, idle. Gone only on interrupted`, and that is what
    // a redirect acts on — the process is still there holding its session, so
    // an instruction is a turn injected rather than a respawn. This asserted
    // the opposite until redirect was built, because ending it was what the
    // reap path made easy.
    assert!(!ruling.ends_the_drone());

    let moved = apply(
        &running_job(),
        &ruling,
        Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z"),
    )
    .expect("a suspect ruling moves the Job")
    .expect("a legal move");
    assert_eq!(moved.job.status(), JobStatus::Escalated);
    assert!(
        !moved.job.status().is_terminal(),
        "a person still has somewhere to take it"
    );
    // **Not `gate_failure`.** Resubmission under the same instructions would
    // reproduce the same gaming, so the retry flow is the wrong destination.
    assert_eq!(
        moved.event.reason(),
        &TransitionReason::Escalation(EscalationTrigger::EvidenceSuspect)
    );
}

#[test]
fn the_trigger_this_routes_to_is_about_a_step_and_can_say_which_one() {
    assert_eq!(
        EscalationTrigger::EvidenceSuspect.level(),
        core_model::TriggerLevel::Step,
        "evidence is submitted per step, so suspect evidence is suspect on one"
    );
}

/// The step advances, and the record shows the check ran and found nothing.
#[tokio::test]
async fn a_clean_submission_passes_a_gaming_check_that_is_watching_for_it() {
    let ruling = ruled(
        HONEST,
        &["check_config_edited", "test_deleted"],
        &recorded(),
    )
    .await;
    assert!(ruling.advanced(), "{ruling:?}");
    assert!(ruling.flagged().is_none());
}

/// **The no-baseline case.** A step whose `baseline_ref` names a step that has
/// recorded nothing still runs, and still catches what the diff answers.
#[tokio::test]
async fn a_gaming_check_with_no_baseline_still_runs() {
    let ruling = ruled(GAMED, &["check_config_edited"], &[]).await;
    assert!(matches!(ruling, Ruling::Suspect { .. }), "{ruling:?}");
}

/// A step that declares no gaming check spends nothing, which is the whole
/// cold-by-default rule applied to the second look.
#[tokio::test]
async fn a_step_that_asks_nothing_about_gaming_is_never_looked_at() {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["jest.config.js"]).showing(GAMED);
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
        &judging(),
        &keeping_nowhere(),
    )
    .await;
    assert!(ruling.advanced(), "{ruling:?}");
}

/// A baseline is an **earlier** step's evidence. There is no way through this
/// type to reach a later step's, which is the shape rather than a check.
#[test]
fn a_baseline_naming_a_later_step_or_this_one_resolves_to_nothing() {
    let workflow = workflow(&["check_config_edited"], Some("scope.evidence"));
    let worktree = worktree();
    let ahead = vec![(StepId::new("regression_verify"), recorded().remove(0).1)];
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");

    let forward = EvidenceRef::parse("regression_verify.evidence").expect("a reference");
    assert!(at.baseline(&forward, &ahead).is_none(), "a step ahead");

    let itself = EvidenceRef::parse("scope.evidence").expect("a reference");
    assert!(
        at.baseline(&itself, &recorded()).is_none(),
        "a step comparing against itself compares against nothing"
    );

    // From the second step, the first one resolves.
    let second = AtStep::named(
        workflow.frozen(),
        &StepId::new("regression_verify"),
        &worktree,
    )
    .expect("a step of the workflow");
    let held = recorded();
    let (named, evidence) = second.baseline(&itself, &held).expect("an earlier step");
    assert_eq!(named.as_str(), "scope");
    assert!(evidence.claimed.contains("rollover tests"));
}

/// A flagged step still carries what the Judge said about its criteria. A step
/// whose Judge cleared it and a step whose Judge never ran are different facts,
/// and the gaming flag must not erase the first.
#[tokio::test]
async fn a_flagged_step_keeps_what_the_judge_said_about_its_criteria() {
    let workflow = testkit::resolved(&[Sketch {
        id: "fix",
        label: "Fix",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[("c1", "Does the fix address the cause the note names?")],
        scope: None,
        gaming: Some(Gaming {
            baseline: None,
            flag_if: &["check_config_edited"],
        }),
    }]);
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["jest.config.js"]).showing(GAMED);
    let judging = Judging {
        client: Arc::new(FakeJudge::answering(&[("Does the fix", "verdict: met")])),
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

    assert!(matches!(ruling, Ruling::Suspect { .. }), "{ruling:?}");
    assert_eq!(ruling.judged().len(), 1, "the criterion was answered");
    assert!(!ruling.judged()[0].verdict.refuses(), "and not refused");
}

/// **The end of the line for a gaming finding.** The Job escalates as
/// `evidence_suspect`, the step stops carrying that verdict, and the patterns
/// that tripped reach the detail view a person opens.
///
/// Without the last of those, a person sees that evidence was suspect and not
/// what about it — which is the whole content of the finding, and the same
/// defect an uncited refusal would be.
#[tokio::test]
async fn a_gaming_finding_names_its_patterns_on_the_detail_view() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["jest.config.js"]).showing(GAMED),
        one_step_watching_for_gaming(),
        // Never reached: `check_config_edited` is answered by the diff.
        FakeJudge::saying("flag: no"),
    );
    let job = fleet
        .propose(a_proposal("narrow the suite instead of fixing it"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    dispatched(&fleet, &job_id).await.expect("released to run");
    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(matches!(turned.ruled(), Some(Ruling::Suspect { .. })));

    let escalated = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(escalated.status(), JobStatus::Escalated);
    let stopped = escalated
        .step(&StepId::new("implement"))
        .expect("the row is there");
    assert_eq!(stopped.state(), core_model::StepState::Stopped);
    assert_eq!(
        stopped.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::EvidenceSuspect).map(StepVerdict::Failed),
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");

    assert_eq!(detail.steps[0].state.as_wire(), "stopped");
    assert_eq!(
        detail.steps[0].flagged.len(),
        1,
        "the finding says which pattern, not only that there was one"
    );
    assert_eq!(detail.steps[0].flagged[0].pattern, "check_config_edited");
    assert!(
        detail.steps[0].flagged[0].cited.contains("jest.config.js"),
        "an uncited flag is unactionable: {:?}",
        detail.steps[0].flagged[0]
    );
}

/// One step, gated on nothing a command runs, watching for the one pattern the
/// diff answers. Nothing here reaches a model.
fn one_step_watching_for_gaming() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: Some(Gaming {
            baseline: None,
            flag_if: &["check_config_edited"],
        }),
    }])
}

/// A step nothing was flagged on carries an empty list, the way an unjudged
/// step does about its criteria.
#[tokio::test]
async fn a_step_nothing_was_flagged_on_carries_an_empty_list() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/limiter.ts"]).showing(HONEST),
        one_step_watching_for_gaming(),
        FakeJudge::saying("flag: no"),
    );
    let job = fleet
        .propose(a_proposal("fix it honestly"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    dispatched(&fleet, &job_id).await.expect("released to run");
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (_, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    let text = String::from_utf8(body.clone()).expect("the body is text");
    assert!(text.contains("\"flagged\":[]"), "{text}");
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    assert!(detail.steps[0].flagged.is_empty());
}
