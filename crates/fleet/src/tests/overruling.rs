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

use std::sync::Arc;

use adapter_traits::{BroughtUpToDate, Standing};
use core_model::{EscalationTrigger, JobStatus, StepId, StepLevelTrigger, StepState, StepVerdict};
use ipc::{JobDetail, JobHistory, Movement, RunId};
use testkit::{Delivered, Delivering, FakeJudge, FakeVcs, FakeWorkProduct, Gaming, Gate, Sketch};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::overruling::Overruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_judged_by, a_proposal, diff_evidence, fittings, one, worktree_directory,
};
use crate::tests::http::call;
use crate::tests::restarting::{on_it, until_spoken};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
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
    dispatched(&fleet, &job_id).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Refused { .. })),
        "the fixture did not reach a refusal: {:?}",
        turned.ruled()
    );
    let escalated = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(escalated.status(), JobStatus::Escalated);
    assert_eq!(
        escalated.step(&implement()).map(|step| step.state()),
        Some(StepState::Stopped),
    );
    job_id
}

/// A test config narrowed to one file. `check_config_edited` reads this as the
/// gate being weakened; the same diff is what a person makes when narrowing the
/// suite *is* the step, which is why the flag is a guess and not a finding.
const GAMED: &str = "diff --git a/jest.config.js b/jest.config.js\n\
                     --- a/jest.config.js\n+++ b/jest.config.js\n\
                     @@ -1,3 +1,3 @@\n-  testMatch: ['**/*.test.ts'],\n\
                     +  testMatch: ['src/one.test.ts'],\n";

/// [`judged_then_summarised`]'s shape with the gaming check in place of the
/// criterion, so an overruled flag has a step to advance to and the Job does
/// not reach the commit that ending a workflow does. The two fixtures differ in
/// the one thing under test.
fn gaming_then_summarised() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
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

/// [`refused`]'s counterpart on the other trigger: dispatched, worked, flagged
/// by the gaming check, standing escalated with its step stopped.
async fn flagged(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("narrow the suite instead of fixing it"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job_id);
    dispatched(&fleet, &job_id).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Suspect { .. })),
        "the fixture did not reach a gaming flag: {:?}",
        turned.ruled()
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

/// **An override is a step boundary, and it had `approve_review`'s hole.**
/// Neither of the two acts a person takes to advance a step ran the catch-up
/// that every mechanical advance runs, so the step being advanced to started
/// from a branch that was two commits behind. #150 names the approval path;
/// this is the same defect on the other one.
#[tokio::test]
async fn an_overruled_step_catches_the_branch_up_like_any_other_boundary() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(judged_then_summarised());
    fittings.judge = Arc::new(a_judge_that_refuses());
    fittings.vcs = FakeVcs::new().delivering(Delivering {
        standing: Standing::Behind { commits: 2 },
        rebase: Some(BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 2,
        }),
        ..Delivering::default()
    });
    let fleet = Fleet::assembled(fittings);
    let job_id = refused(&fleet, &home).await;
    // One, and it is the spawn's: every spawn catches its branch up (#180), and
    // a fake scripted `Behind` is behind on that call too. What matters here is
    // that nothing rebased *between* the spawn and the person's decision — an
    // escalated Job is standing still under whoever is reading it.
    let before = fleet.vcs().delivered().len();

    fleet
        .override_verdict(&job_id, &a_reason())
        .await
        .expect("the person overrules the verdict");

    assert_eq!(
        fleet.vcs().delivered().split_off(before),
        vec![Delivered::BroughtUpToDate {
            branch: format!("armada/{}", job_id.as_str()),
            base: String::from("main"),
        }],
        "the step an override advanced to starts where an auto-advanced one would"
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

/// **A gaming flag is a machine's reading of a diff, and a person can be right
/// where it is wrong.**
///
/// `evidence_suspect` was refused here until the owner settled it: anything a
/// machine decides, he can overrule. The check that fires this one is
/// `check_config_edited`, which cannot tell a test config narrowed to hide a
/// failure from a test config edited because the edit was the work — and that
/// is exactly the call the flag is a machine's guess at.
///
/// **What is asserted is the record, not the advance.** A gaming flag overruled
/// has to read as legibly afterwards as a refusal overruled: `advanced` beside
/// `failed(evidence_suspect)`, `overridden` on the wire, and the patterns that
/// were flagged still there to be read.
#[tokio::test]
async fn a_gaming_flag_is_overruled_and_the_step_advances_still_carrying_it() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["jest.config.js"]).showing(GAMED),
        gaming_then_summarised(),
        FakeJudge::saying("flag: no"),
    );
    let job_id = flagged(&fleet, &home).await;

    fleet
        .override_verdict(&job_id, &a_reason())
        .await
        .expect("the person overrules the gaming flag");

    let advanced = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(advanced.status(), JobStatus::Running);
    let row = advanced.step(&implement()).expect("the row is there");
    assert_eq!(row.state(), StepState::Advanced);
    assert_eq!(
        row.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::EvidenceSuspect).map(StepVerdict::Failed),
        "the flag is not rewritten to `passed` by somebody disagreeing with it"
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = call(&app, "GET", &format!("/jobs/{}", job_id.as_str()), "").await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    let step = &detail.steps[0];
    assert!(
        step.overridden,
        "the same field carries both, so no surface has to learn a second rule"
    );
    assert_eq!(
        step.last_verdict
            .as_ref()
            .map(|verdict| (verdict.named.as_str(), verdict.trigger.as_deref())),
        Some(("failed", Some("evidence_suspect"))),
        "the trigger is what tells an overruled flag from an overruled refusal"
    );
    assert!(
        !step.flagged.is_empty(),
        "which pattern fired stays readable beside the fact that it was overruled — \
         `flagged` is to evidence_suspect what `judged` is to gate_failure"
    );
}

// ------------------------------------------ what it must not be able to do

/// **A failed mechanical Check is not a matter of opinion.**
///
/// **And this is the case #208 had to leave alone.** The Job used to be
/// terminal here, so the act was refused by terminality and by nothing else; it
/// now holds at `awaiting_repair`, over a step that is `stopped` carrying
/// `failed(gate_failure)` — the very shape an override lands on beneath
/// `escalated`. What refuses it is the status, and behind that `Stuck` reads
/// `checks_passed` out of the store rather than reading the trigger. A person
/// still cannot wave a red suite through; what changed is who is asked to fix
/// it.
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
    dispatched(&fleet, &job_id).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports a diff it did not make");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Failed { .. })),
        "the mechanical tier is what stopped this: {:?}",
        turned.ruled()
    );

    match fleet.override_verdict(&job_id, &a_reason()).await {
        Err(Adrift::NotResumable { status, .. }) => assert_eq!(
            status,
            JobStatus::AwaitingRepair,
            "a Job held for repair is not a Job with a verdict to disagree with"
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
        Some(StepState::Stopped),
        "and the refusal moved nothing: the step is frozen where the failure left \
         it, stopped and carrying the verdict that stopped it — #179"
    );
}

/// **A machine that could not decide left nothing to disagree with.**
///
/// `gate_undecided` is the gate saying it could not read the artifact. The
/// owner's rule reaches what a machine decided, and here it decided nothing —
/// overruling would advance work no tier ever weighed, which is the one thing
/// `evidence_suspect` moving must not be read as licence for.
#[tokio::test]
async fn a_gate_that_could_not_decide_has_no_verdict_to_overrule() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::refusing("a worktree that would not read"),
        testkit::resolved(&[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[("c1", QUESTION)],
            scope: None,
            gaming: None,
        }]),
        a_judge_that_refuses(),
    );
    let job = fleet
        .propose(a_proposal("fix the reader"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    dispatched(&fleet, &job_id).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(Ruling::CouldNotDecide { .. })),
        "the fixture did not reach an undecided gate: {:?}",
        turned.ruled()
    );

    match fleet.override_verdict(&job_id, &a_reason()).await {
        Err(Adrift::NotTheJudges { trigger, .. }) => {
            assert_eq!(trigger, EscalationTrigger::GateUndecided)
        }
        other => panic!("a gate that never ruled was overruled: {other:?}"),
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

/// **The first live path on which a Drone starts a step it did not begin the
/// Job on**, and the whole of what `#139` is for. `#140` makes every step
/// boundary this path; today it is reached only here, so it is asserted here.
///
/// It is read off the transcript rather than off the assembled block, for the
/// reason `crate::tests::restarting` reads one: what is in doubt is not that
/// `briefing` renders the block — `crate::tests::crossing` asserts that — but
/// that the record reached it and it reached the far end of the pipe.
#[tokio::test]
async fn a_fresh_drone_at_the_next_step_is_told_what_the_overruled_one_produced() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(judged_then_summarised());
    fittings.judge = Arc::new(a_judge_that_refuses());
    // Echoes its opening brief and exits, which is how the brief becomes
    // something a test can read at all.
    fittings.harness = testkit::FakeHarness::that_echoes_its_first_turn();
    let fleet = Fleet::assembled(fittings);

    let job_id = refused(&fleet, &home).await;
    fleet.kill_drone(&job_id).await.expect("the Drone ends");
    fleet
        .override_verdict(&job_id, &a_reason())
        .await
        .expect("the person overrules the verdict");

    let said = until_spoken(&home, &on_it(&fleet, &job_id).await).await;
    assert!(
        said.contains("What part 1 produced"),
        "the Drone on part 2 was told nothing about part 1: {said}"
    );
    assert!(
        said.contains("The reader stops one line later."),
        "and it was not told it from the record — this is `claimed`, verbatim: {said}"
    );
    assert!(
        said.contains("was read by a person and accepted"),
        "a person took that part, and a check did not: {said}"
    );
}
