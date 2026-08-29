//! Asking the gate again after it could not decide.
//!
//! **Every case starts from a real undecided gate.** The Job is proposed,
//! approved, worked and submitted; the gate is asked, cannot derive the
//! artifact it needs, and escalates on `gate_undecided`. Nothing puts a Job
//! into that state by hand, because a stand-in at that seam would hide whether
//! the act can be reached from where a Job actually arrives.
//!
//! The two outcomes are the claim and they are one fixture apart: the cause
//! lifts and the step advances, or it does not and the gate says so again. What
//! separates them here is a single call on the fake, which is the whole of what
//! `#156` said Fleet cannot tell apart on its own.

use core_model::{Attempt, EscalationTrigger, JobStatus, StepId, StepLevelTrigger};
use core_model::{StepState, StepVerdict};
use ipc::RunId;
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, diff_evidence, worktree_directory};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::transcript::log_of;
use crate::Adrift;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

const QUESTION: &str = "Does the fix address the cause the note names?";

fn implement() -> StepId {
    StepId::new("implement")
}

/// A two-step workflow whose first step asks the Judge and gates on nothing
/// mechanical, so the artifact the gate fails to derive is **the step's
/// patch** — the read that happens on the way to a model call.
///
/// Gating on `diff_nonempty` as well would prove less, not more: a fake that
/// refuses every read also refuses the baseline `dispatch` takes when the step
/// starts, so the second reading would fail a Check for want of a baseline
/// rather than answer the question that failed to be asked.
fn judged_then_summarised() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
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

/// The same shape with a Check that can fail, for the case that reaches a
/// verdict on the second reading rather than an advance.
fn gated_then_summarised() -> config::ResolvedWorkflow {
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

/// A Job dispatched, worked, submitted, and standing escalated because the gate
/// could not read what it needed. **Where this act starts, and the only place.**
async fn undecided(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("fix the reader"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate ran");
    assert!(
        matches!(turned.ruled, Some(Ruling::CouldNotDecide { .. })),
        "the fixture did not reach an undecided gate: {:?}",
        turned.ruled
    );
    let escalated = fleet.load(&job_id).await.expect("the Job reads");
    assert_eq!(escalated.status(), JobStatus::Escalated);
    assert_eq!(
        escalated.step(&implement()).map(|step| step.state()),
        Some(StepState::Stopped),
    );
    assert_eq!(
        escalated
            .step(&implement())
            .and_then(|step| step.last_verdict()),
        StepLevelTrigger::of(EscalationTrigger::GateUndecided).map(StepVerdict::Failed),
    );
    job_id
}

fn logged(home: &TempDir, job: &core_model::JobId) -> String {
    std::fs::read_to_string(log_of(&home.path().to_string_lossy(), job)).unwrap_or_default()
}

// ------------------------------------------------------ the act itself

/// **The whole of #175.** The gate could not read the patch, a person asks it
/// again, the read succeeds and the step advances — on the Drone's own work,
/// which was never in question and was never redone.
#[tokio::test]
async fn a_gate_that_could_not_decide_is_re_run_and_the_step_advances() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]).but_refusing("a worktree that would not read"),
        judged_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job_id = undecided(&fleet, &home).await;

    // The cause was transient, which is the common case and the one nothing in
    // Fleet can establish for itself.
    fleet.work().reads_now();

    let job = fleet
        .rerun_gate(&job_id)
        .await
        .expect("the gate is asked again");

    assert_eq!(
        job.status(),
        JobStatus::Running,
        "a workflow with a step left goes back to being worked"
    );
    let reloaded = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        reloaded.step(&implement()).map(|step| step.state()),
        Some(StepState::Advanced),
        "the step advanced on a ruling, which is what makes this not an override"
    );
    assert_eq!(
        reloaded
            .step(&implement())
            .and_then(|step| step.last_verdict()),
        Some(StepVerdict::Passed),
        "and the verdict says the gate passed it — an override would have left `failed` there"
    );
    assert_eq!(
        reloaded.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
        "the cursor moved to the step that follows"
    );
}

/// **Where the cause is permanent it fails again and says so.** #153's blind
/// Judge was this case: no re-run would have fixed it, and the honest outcome
/// is the same escalation with the same trigger rather than an advance
/// somebody's press bought.
#[tokio::test]
async fn a_gate_that_still_cannot_decide_says_so_again_and_moves_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::refusing("a Judge that cannot be handed a patch"),
        judged_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job_id = undecided(&fleet, &home).await;

    let job = fleet
        .rerun_gate(&job_id)
        .await
        .expect("the gate is asked again and answers with what it found");

    assert_eq!(
        job.status(),
        JobStatus::Escalated,
        "the Job is where it was, which is the honest answer rather than an advance"
    );
    let row = job.step(&implement()).expect("the row is there");
    assert_eq!(row.state(), StepState::Stopped);
    assert_eq!(
        row.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::GateUndecided).map(StepVerdict::Failed),
        "the same trigger, because the same thing is still true"
    );

    let written = logged(&home, &job_id);
    assert!(
        written.contains("a person asked the gate again on the evidence already submitted"),
        "a press that changed nothing is exactly the press a person needs to see happened: \
         {written}"
    );
    assert!(
        written.contains("\"came_to\":\"undecided\""),
        "and what it came to is a value a query groups on, not a sentence: {written}"
    );
    assert_eq!(
        written
            .lines()
            .filter(|line| line.contains("the gate could not read what it needed to rule"))
            .count(),
        2,
        "one line per reading: the gate could not read it twice, and both are on the record"
    );
}

/// **A re-run is not a run of the step, and the retry budget must not think it
/// was.** `#63` hands a failed Check back to the Drone inside `retry_limit`,
/// which counts entries into `running` — so a re-run that entered one would
/// take a hand-back off a Drone that had not spent it.
///
/// The permanent case is where this bites hardest: a person may press until
/// they give up, and the step's run count must be the same afterwards as
/// before.
#[tokio::test]
async fn re_running_the_gate_spends_none_of_the_step_retry_budget() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::refusing("a Judge that cannot be handed a patch"),
        judged_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job_id = undecided(&fleet, &home).await;
    let before = fleet
        .store()
        .lock()
        .await
        .step_attempt(&job_id, &implement())
        .expect("the step's run count reads");
    assert_eq!(before, Attempt::FIRST, "the Drone worked the step once");

    fleet.rerun_gate(&job_id).await.expect("asked again");
    fleet.rerun_gate(&job_id).await.expect("and again");
    fleet.rerun_gate(&job_id).await.expect("and again");

    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .step_attempt(&job_id, &implement())
            .expect("the step's run count reads"),
        before,
        "three presses, no Drone work, and the run count is where the Drone left it"
    );
}

/// **The second reading is a reading and not a pass**, so it may find what the
/// first never reached. A Check the gate never got to run is run now, it fails,
/// and the Job ends on that verdict — which is the gate deciding, which is what
/// was asked for.
#[tokio::test]
async fn a_re_run_that_reaches_a_failing_check_rules_on_it() {
    let home = TempDir::new();
    // Refusing at dispatch leaves the step with no baseline, so `diff_nonempty`
    // reads as nothing known to have moved — an unread baseline must not
    // advance a step, and here it is what the first reading never got to.
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]).but_refusing("a worktree that would not read"),
        gated_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job_id = undecided(&fleet, &home).await;
    fleet.work().reads_now();

    let job = fleet
        .rerun_gate(&job_id)
        .await
        .expect("the gate is asked again");

    assert_eq!(
        job.status(),
        JobStatus::CompletedFailed,
        "a Check that ran and failed is a verdict, and this act carries it out"
    );
    let written = logged(&home, &job_id);
    assert!(
        written.contains("\"came_to\":\"failed\""),
        "the log says what the second reading came to, and it was not `undecided`: {written}"
    );
}

// ------------------------------------------ what it must not be able to do

/// **This act and the override partition the triggers.** `overrulable` admits
/// `gate_failure` because a machine ruled; this refuses it for the same reason,
/// turned around — running the gate again would ask a question that was
/// answered and draw the same answer.
#[tokio::test]
async fn a_step_the_judge_refused_is_not_re_run() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_then_summarised(),
        FakeJudge::refusing(
            "the loop stops at n",
            "the loop stops at n - 1",
            "the last row is dropped",
        ),
    );
    let job = fleet
        .propose(a_proposal("widen the bound instead of fixing it"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
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

    match fleet.rerun_gate(&job_id).await {
        Err(Adrift::NotUndecided { trigger, .. }) => assert_eq!(
            trigger,
            EscalationTrigger::GateFailure,
            "a verdict is disagreed with, not asked for again"
        ),
        other => panic!("a gate that ruled was asked again: {other:?}"),
    }
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job reads")
            .step(&implement())
            .map(|step| step.state()),
        Some(StepState::Stopped),
        "and nothing moved"
    );
}

/// **The baseline the first reading used lives in the slot, and a re-run
/// without it would answer a different question.** A Fleet restarted since the
/// escalation holds no slot, so the honest answer is the act that does apply
/// rather than a reading that looks like the first one and is not.
#[tokio::test]
async fn a_job_fleet_is_no_longer_standing_at_is_refused_and_told_what_applies() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::refusing("a worktree that would not read"),
        judged_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job_id = undecided(&fleet, &home).await;
    // What a Fleet restart leaves: the record says escalated on a stopped step,
    // and nothing is holding the session or the baseline.
    fleet.kill_drone(&job_id).await.expect("the Drone is gone");
    fleet.work().reads_now();

    match fleet.rerun_gate(&job_id).await {
        Err(Adrift::NotStandingThere { job }) => assert_eq!(job, job_id),
        other => panic!("a reading was made against a baseline that is gone: {other:?}"),
    }
}

// ------------------------------------------------------------ the wire

/// The route is served, it takes no body, and it answers the Job.
///
/// **No body is the assertion.** An override carries a required reason and is
/// 422 without one; nothing is being disagreed with here, so there is no
/// sentence to withhold and an empty request is well-formed.
#[tokio::test]
async fn the_route_takes_no_body_and_answers_the_job() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]).but_refusing("a worktree that would not read"),
        judged_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job_id = undecided(&fleet, &home).await;
    fleet.work().reads_now();

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(
        &app,
        "POST",
        &format!("/jobs/{}/rerun_gate", job_id.as_str()),
        "",
    )
    .await;

    assert_eq!(status, 200);
    let job: ipc::JobSummary = ipc::decode("a Job", &body).expect("a JobSummary");
    assert_eq!(job.id.as_str(), job_id.as_str());
    assert_eq!(job.status.as_wire(), "running");
}

/// A Job that never stopped reaches no act on a stopped step, and 409 is what
/// the seam says about that.
#[tokio::test]
async fn a_job_that_is_not_escalated_is_refused_over_the_wire() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged_then_summarised(),
        FakeJudge::with_no_objection(),
    );
    let job = fleet
        .propose(a_proposal("fix the reader"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, _) = call(
        &app,
        "POST",
        &format!("/jobs/{job_id}/rerun_gate", job_id = job_id.as_str()),
        "",
    )
    .await;

    assert_eq!(status, 409);
}
