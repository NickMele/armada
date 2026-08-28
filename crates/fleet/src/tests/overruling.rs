//! Overruling a verdict, and every place it must not reach.
//!
//! **Every case here starts from a real refusal.** The Judge is asked, it
//! refuses, the Job escalates and the step stops — nothing moves a Job into the
//! state these acts answer by hand. A stand-in at exactly that seam would hide
//! whether the act can be reached from where a Job actually arrives.
//!
//! The proportion is deliberate: one case advances a step and four are cases
//! where nothing does. An override is one refused step on one Job, and what
//! keeps it from becoming an approve-anything is the four.

use core_model::{EscalationTrigger, JobStatus, StepId, StepLevelTrigger, StepState, StepVerdict};
use ipc::{JobDetail, JobHistory, Movement, RunId};
use testkit::{FakeJudge, FakeWorkProduct, Gaming, Gate, Sketch};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::overruling::Overruling;
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, diff_evidence, worktree_directory};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::Adrift;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

const QUESTION: &str = "Does the fix address the cause the note names?";

fn implement() -> StepId {
    StepId::new("implement")
}

/// The Judge refusing what it was shown. The three lines are the citation a
/// person reads before deciding they disagree with it.
fn a_judge_that_refuses() -> FakeJudge {
    FakeJudge::refusing(
        "the loop stops at n",
        "the loop stops at n - 1",
        "the last row is dropped",
    )
}

/// A two-step workflow whose first step is judged and whose second is not, so
/// an override has somewhere to advance to.
///
/// The first step declares `diff_nonempty` as well as the question, which is
/// what makes the same fixture serve both the refusal and the failed Check: a
/// Drone that changed nothing fails the mechanical tier and never reaches the
/// Judge at all.
fn judged_then_summarised() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[("c1", QUESTION)],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "summarise",
            label: "Summarise",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// A Job dispatched, worked, refused by the Judge, and standing escalated with
/// its step stopped — which is where all four acts on an escalated Job start.
async fn refused(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("widen the bound instead of fixing it"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled, Some(Ruling::Refused { .. })),
        "the fixture did not reach a refusal: {:?}",
        turned.ruled
    );
    let escalated = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(escalated.status(), JobStatus::Escalated);
    assert_eq!(
        escalated.step(&implement()).map(|step| step.state()),
        Some(StepState::Stopped),
    );
    job_id
}

fn a_reason() -> Overruling {
    Overruling::saying("the criterion asks about the writer, and the Job is about the reader")
        .expect("a reason with something in it")
}

// ------------------------------------------------------ the act itself

/// **The whole of #170.** The Judge refused correct work, a person says so, and
/// the Job carries on from where it stopped — without the 118 turns that
/// produced the work being re-run.
#[tokio::test]
async fn a_refused_step_advances_when_a_person_overrules_the_judge() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_then_summarised(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home).await;

    let job = fleet
        .override_verdict(&job_id, &a_reason())
        .await
        .expect("the person overrules the verdict");

    assert_eq!(
        job.status(),
        JobStatus::Running,
        "a workflow with a step left goes back to being worked"
    );
    let reloaded = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        reloaded.step(&implement()).map(|step| step.state()),
        Some(StepState::Advanced),
        "the step a person overruled is advanced, not stopped and not re-run"
    );
    assert_eq!(
        reloaded.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
        "the cursor moved to the step that follows, which is what `restart_step` would not do"
    );
}

/// **It is an override on the record and not a pass**, which is the difference
/// between an appeal and a Judge that has quietly become decorative.
///
/// Three things say so and none of them is a second copy of the others: the
/// step's own verdict still names the trigger, the detail view says the step
/// was overridden rather than leaving a client to infer it from the pair, and
/// the Judge's citation is still there beside both.
#[tokio::test]
async fn the_record_says_the_judge_refused_and_a_person_disagreed() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_then_summarised(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home).await;
    fleet
        .override_verdict(&job_id, &a_reason())
        .await
        .expect("the person overrules the verdict");

    let advanced = fleet.load(&job_id).await.expect("the Job reads");
    let row = advanced.step(&implement()).expect("the row is there");
    assert_eq!(row.state(), StepState::Advanced);
    assert_eq!(
        row.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::GateFailure).map(StepVerdict::Failed),
        "the gate's ruling is not rewritten to `passed` by somebody disagreeing with it"
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = call(&app, "GET", &format!("/jobs/{}", job_id.as_str()), "").await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    let step = &detail.steps[0];
    assert_eq!(step.state.as_wire(), "advanced");
    assert!(
        step.overridden,
        "the wire says the step was overridden rather than leaving every surface to work it out"
    );
    assert_eq!(
        step.last_verdict
            .as_ref()
            .map(|verdict| (verdict.named.as_str(), verdict.trigger.as_deref())),
        Some(("failed", Some("gate_failure"))),
    );
    assert!(
        !step.judged.is_empty(),
        "what the Judge said stays readable beside the fact that it was overruled"
    );
}

/// **A rate can be read off `job_events` and nothing else is needed for it.**
/// One row says which step, that it went from `stopped` to `advanced`, what
/// trigger it overruled, and that a person did it.
#[tokio::test]
async fn the_log_carries_one_row_an_override_can_be_counted_from() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_then_summarised(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home).await;
    fleet
        .override_verdict(&job_id, &a_reason())
        .await
        .expect("the person overrules the verdict");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = call(
        &app,
        "GET",
        &format!("/jobs/{}/events", job_id.as_str()),
        "",
    )
    .await;
    let history: JobHistory = ipc::decode("a Job's history", &body).expect("a JobHistory");

    let overrides: Vec<_> = history
        .moves
        .iter()
        .filter_map(|row| match &row.moved {
            Movement::Step(step)
                if step.from.as_wire() == "stopped" && step.to.as_wire() == "advanced" =>
            {
                Some((step, row.actor))
            }
            _ => None,
        })
        .collect();
    assert_eq!(overrides.len(), 1, "one override, one row");
    let (step, actor) = &overrides[0];
    assert_eq!(step.step_id.as_str(), "implement");
    assert_eq!(
        step.why.as_deref(),
        Some("gate_failure"),
        "the row names what was overruled, not only that something was"
    );
    assert_eq!(
        actor.as_wire(),
        "human",
        "nothing can reconstruct afterwards that a person did this"
    );
}

// ------------------------------------------ what it must not be able to do

/// **A failed mechanical Check is not a matter of opinion.**
///
/// A Check that did not pass ends the Job at `completed_failed`, which is
/// terminal and stops no step, so there is nothing for this act to land on —
/// and it says so rather than finding somewhere to land.
#[tokio::test]
async fn a_failed_mechanical_check_cannot_be_overruled() {
    let home = TempDir::new();
    // The first step declares `diff_nonempty`, and this Drone changed nothing.
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::untouched(),
        judged_then_summarised(),
        a_judge_that_refuses(),
    );
    let job = fleet
        .propose(a_proposal("claim a fix that was never written"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("the Drone reports a diff it did not make");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled, Some(Ruling::Failed { .. })),
        "the mechanical tier is what stopped this: {:?}",
        turned.ruled
    );

    match fleet.override_verdict(&job_id, &a_reason()).await {
        Err(Adrift::NotResumable { status, .. }) => assert_eq!(
            status,
            JobStatus::CompletedFailed,
            "a Check that failed ends the Job, and no act reopens a terminal"
        ),
        other => panic!("a failed Check was overruled: {other:?}"),
    }
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job reads")
            .step(&implement())
            .map(|step| step.state()),
        Some(StepState::Running),
        "and nothing moved: the step is frozen where `completed_failed` left it"
    );
}

/// **A gaming flag is not the Judge refusing a criterion.**
///
/// `evidence_suspect` says the way the step was satisfied is not to be trusted,
/// which is a claim about the evidence rather than an opinion about the work.
/// Folding it in here would make one act answer two different findings.
#[tokio::test]
async fn a_gaming_flag_is_not_the_judges_verdict_to_overrule() {
    const GAMED: &str = "diff --git a/jest.config.js b/jest.config.js\n\
                         --- a/jest.config.js\n+++ b/jest.config.js\n\
                         @@ -1,3 +1,3 @@\n-  testMatch: ['**/*.test.ts'],\n\
                         +  testMatch: ['src/one.test.ts'],\n";
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["jest.config.js"]).showing(GAMED),
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
        }]),
        FakeJudge::saying("flag: no"),
    );
    let job = fleet
        .propose(a_proposal("narrow the suite instead of fixing it"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(matches!(turned.ruled, Some(Ruling::Suspect { .. })));

    match fleet.override_verdict(&job_id, &a_reason()).await {
        Err(Adrift::NotTheJudges { trigger, .. }) => {
            assert_eq!(trigger, EscalationTrigger::EvidenceSuspect)
        }
        other => panic!("a gaming finding was overruled: {other:?}"),
    }
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job reads")
            .step(&implement())
            .map(|step| step.state()),
        Some(StepState::Stopped),
        "and the step is still stopped, for a person to answer some other way"
    );
}

/// **An override that says nothing is how this becomes the act somebody uses to
/// quiet a gate.** Nobody is told the reason and nothing acts on it; it is the
/// only account there will ever be of why the Judge was wrong.
#[tokio::test]
async fn an_override_with_no_reason_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_then_summarised(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home).await;

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, _) = call(
        &app,
        "POST",
        &format!("/jobs/{}/override_verdict", job_id.as_str()),
        r#"{"reason":"   "}"#,
    )
    .await;
    assert_eq!(
        status.as_u16(),
        422,
        "the request is well-formed and the value in it cannot work"
    );
}

// ------------------------------------------------- and the Drone that is gone

/// **The Drone having ended does not make the verdict unappealable.**
///
/// This is where the act differs from `crate::resume`, which refuses one way or
/// the other depending on the process. Here a fresh Drone takes the *next*
/// step on the worktree the last one left — the step a person accepted is not
/// worked again by anybody.
#[tokio::test]
async fn a_job_whose_drone_has_gone_gets_a_fresh_one_at_the_next_step() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_then_summarised(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home).await;
    // The escalated Job keeps its Drone, so it is ended deliberately: the Job
    // this issue was raised on had lost its Drone to a Fleet restart, and that
    // is the case a restart-shaped act has to carry.
    fleet.kill_drone(&job_id).await.expect("the Drone ends");
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job reads")
            .assigned_drone(),
        None,
    );

    let job = fleet
        .override_verdict(&job_id, &a_reason())
        .await
        .expect("the person overrules the verdict");

    assert_eq!(job.status(), JobStatus::Running);
    assert_eq!(
        job.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
    );
    assert!(
        job.assigned_drone().is_some(),
        "a running Job with no process on it escalates as interrupted a moment later"
    );
}
