//! The material a person reviews, and the three answers they may give.
//!
//! **The claim is that a decision on the work is a transition like any other.**
//! Every case below reads the Job back out of the store afterwards rather than
//! trusting what the call answered, because what is being asserted is that the
//! log moved — not that a method returned.
//!
//! # `awaiting_review` is reached the way a Job reaches it
//!
//! Every case below drives a real dispatch through a workflow whose step is
//! gated `human_always`: the Drone submits, the mechanical tier holds, and the
//! gate rules `HeldForReview`. **Nothing here moves a Job to the gate by hand.**
//! An earlier draft did, because `AdvanceGate` had no variant to reach it with,
//! and a stand-in at exactly that seam would have hidden the fact that no
//! dispatch could arrive at the surface these tests cover.

use std::sync::Arc;

use axum::http::StatusCode;
use core_model::{JobId, JobStatus, StepState};
use ipc::{JobDiff, JobEvidence, RunId};
use testkit::{FakeJudge, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::daemon::{
    a_fleet_gated_on_a_person, a_fleet_judged_by, a_proposal, diff_evidence, note_evidence,
    two_steps_gated_on_a_person, worktree_directory,
};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::Adrift;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// A Fleet whose first step is a person's to answer. Every case that needs a
/// Job at the gate builds one of these.
fn a_fleet_reviewing_the_first_step(home: &TempDir, work: FakeWorkProduct) -> Fixture {
    a_fleet_gated_on_a_person(home, work, "implement", FakeVcs::new())
}

/// A Job dispatched, worked, and standing at a human gate — reached through the
/// gate rather than around it.
///
/// The Drone submits its diff, the step's `diff_nonempty` holds, and the gate
/// answers [`Ruling::HeldForReview`] because the step is gated `human_always`.
/// The step is still `running` while the Job stands there, which is what lets
/// `approve_review` advance it from the gate.
async fn at_the_gate(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, job.id());
    let job = fleet.approve(job.id()).await.expect("it dispatches");
    assert_eq!(job.status(), JobStatus::Running);

    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate runs");
    assert!(
        matches!(turned.ruled, Some(Ruling::HeldForReview { .. })),
        "the gate did not hold the Job for a person: {:?}",
        turned.ruled
    );
    let held = fleet.load(job.id()).await.expect("the Job is there");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
    assert_eq!(
        held.step(&core_model::StepId::new("implement".to_string()))
            .map(|step| step.state()),
        Some(StepState::Running),
        "the step is what the person is standing at, not something already moved"
    );
    job.id().clone()
}

/// Approving advances the step and puts the Job back on the machine, with the
/// person recorded as the one who did it.
#[tokio::test]
async fn approving_advances_the_step_and_the_job_goes_on() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;

    let job = fleet
        .approve_review(&job_id)
        .await
        .expect("the work is taken");

    assert_eq!(
        job.status(),
        JobStatus::Running,
        "a workflow with a step left goes back to being worked"
    );
    let reloaded = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        reloaded
            .step(&core_model::StepId::new("implement".to_string()))
            .map(|step| step.state()),
        Some(StepState::Advanced),
        "the step a person passed is advanced, not still running"
    );
    assert_eq!(
        reloaded.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
        "the cursor moved to the step that follows"
    );
}

/// A gate on the workflow's last step. **Approving there lands the work**: the
/// commit is made before the Job is recorded complete, for the reason
/// `landing` gives — a `completed_success` whose branch is uncommitted is
/// correct, verified and unmergeable.
#[tokio::test]
async fn approving_the_last_step_commits_the_work_and_ends_the_job() {
    let home = TempDir::new();
    // The gate is on `summarise` here, so the first step advances on its own
    // and the Job walks to the last one the way a Job does.
    let fleet = a_fleet_gated_on_a_person(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "summarise",
        FakeVcs::new(),
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, job.id());
    let job = fleet.approve(job.id()).await.expect("it dispatches");

    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate runs");
    assert!(
        matches!(turned.ruled, Some(Ruling::Advanced { .. })),
        "an auto step still advances on its own: {:?}",
        turned.ruled
    );
    fleet
        .submit_evidence(note_evidence())
        .await
        .expect("the Drone reports its summary");
    let turned = fleet.turn().await.expect("the gate runs again");
    assert!(
        matches!(turned.ruled, Some(Ruling::HeldForReview { .. })),
        "the last step is a person's: {:?}",
        turned.ruled
    );
    assert!(
        fleet.vcs().committed().is_empty(),
        "nothing lands while a person is still looking at it"
    );

    let done = fleet
        .approve_review(job.id())
        .await
        .expect("the work is taken");

    assert_eq!(done.status(), JobStatus::CompletedSuccess);
    assert_eq!(
        fleet.vcs().committed().len(),
        1,
        "the branch a reviewer accepted is a branch a merge can take"
    );
}

/// The other two answers, and the refusal that guards all three.
#[tokio::test]
async fn rejecting_ends_the_job_and_only_from_the_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    // A Job that never reached a gate. **The refusal is the point**: the
    // machine has `awaiting_approval -> rejected`, so nothing but this check
    // stops a review act from being the dispatch gate's denial under a second
    // name.
    let waiting = fleet
        .propose(a_proposal("something nobody approved"))
        .await
        .expect("a Job at the approval gate");
    let refused = fleet.reject(waiting.id()).await;
    assert!(
        matches!(refused, Err(Adrift::NotUnderReview { .. })),
        "a Job at the approval gate is not under review: {refused:?}"
    );
    assert_eq!(
        fleet
            .load(waiting.id())
            .await
            .expect("still there")
            .status(),
        JobStatus::AwaitingApproval,
        "and the refused act moved nothing"
    );

    let job_id = at_the_gate(&fleet, &home).await;
    let rejected = fleet.reject(&job_id).await.expect("a verdict on the work");
    assert_eq!(rejected.status(), JobStatus::Rejected);
    assert!(
        rejected.status().is_terminal(),
        "the hard stop is a hard stop"
    );
}

/// **A human gate is not a way past the machine.** The gate is read after every
/// tier has run, so a step whose Check failed ends the Job rather than putting
/// broken work in front of a person to approve.
#[tokio::test]
async fn work_that_fails_a_check_never_reaches_the_person() {
    let home = TempDir::new();
    // The human-gated step declares `diff_nonempty`, and this Drone changed
    // nothing.
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::untouched());
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.expect("it dispatches");

    fleet
        .submit_evidence(diff_evidence())
        .await
        .expect("the Drone reports a diff it did not make");
    let turned = fleet.turn().await.expect("the gate runs");

    assert!(
        matches!(turned.ruled, Some(Ruling::Failed { .. })),
        "the mechanical tier still ends the Job: {:?}",
        turned.ruled
    );
    assert_eq!(
        fleet.load(job.id()).await.expect("the Job").status(),
        JobStatus::CompletedFailed,
        "and it ends rather than waiting for someone to approve it"
    );
}

/// The Judge under a human gate, both ways — and this is what the parser's
/// gate-and-judge agreement rule is deliberately silent about.
///
/// **A refusal stops the step short of the person**, which is what makes a
/// Judge here more than decoration: it filters what a person is asked to read.
/// A criterion it did not refuse is carried onto the held ruling instead, as
/// part of the material they open.
#[tokio::test]
async fn a_judge_under_a_human_gate_filters_what_reaches_the_person() {
    const QUESTION: &str = "Does the fix address the cause the note names?";

    for (judge, expected) in [
        (
            FakeJudge::refusing("the cause fixed", "the symptom hidden", "it comes back"),
            JobStatus::Escalated,
        ),
        (FakeJudge::with_no_objection(), JobStatus::AwaitingReview),
    ] {
        let home = TempDir::new();
        let fleet = a_fleet_judged_by(
            &home,
            FakeWorkProduct::changed(&["src/log.rs"]),
            two_steps_gated_on_a_person("implement", Some(QUESTION)),
            judge,
        );
        let job = fleet
            .propose(a_proposal("fix the off-by-one"))
            .await
            .expect("a Job at the approval gate");
        worktree_directory(&home, job.id());
        fleet.approve(job.id()).await.expect("it dispatches");

        fleet
            .submit_evidence(diff_evidence())
            .await
            .expect("the Drone reports its diff");
        let turned = fleet.turn().await.expect("the gate runs");

        assert_eq!(
            fleet.load(job.id()).await.expect("the Job").status(),
            expected,
            "the ruling was {:?}",
            turned.ruled
        );
        let ruled = turned.ruled.expect("the gate answered");
        if expected == JobStatus::AwaitingReview {
            assert!(matches!(ruled, Ruling::HeldForReview { .. }));
            assert_eq!(
                ruled.judged().len(),
                1,
                "what the Judge answered is carried to the person, not thrown away"
            );
            assert!(ruled.tell().is_none(), "the Drone is told nothing yet");
            assert!(!ruled.ends_the_drone(), "and it waits rather than ending");
        }
    }
}

/// Changes go back to the Drone that is standing there, and nowhere else.
#[tokio::test]
async fn requesting_changes_needs_a_drone_to_tell() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;
    let said = crate::resume::Redirection::saying("the second case is untested")
        .expect("a note with something in it");

    let job = fleet
        .request_changes(&job_id, &said)
        .await
        .expect("the Drone is there to be told");
    assert_eq!(job.status(), JobStatus::Running);
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job is there")
            .step(&core_model::StepId::new("implement".to_string()))
            .map(|step| step.state()),
        Some(StepState::Running),
        "the step did not advance — the work is being done again, not accepted"
    );
}

/// The Drone gone, and the note with nowhere to go. **Refused at the gate**
/// rather than half-answered: a Job put back to `running` with no process on it
/// escalates as interrupted a moment later, having lost what the person wrote.
#[tokio::test]
async fn changes_asked_for_with_no_drone_leave_the_job_at_the_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;
    let said = crate::resume::Redirection::saying("the second case is untested")
        .expect("a note with something in it");
    {
        let mut working = fleet.slot().lock().await;
        fleet.end_the_drone(&mut working).await;
    }

    let refused = fleet.request_changes(&job_id, &said).await;
    assert!(
        matches!(refused, Err(Adrift::NoDroneToTell { .. })),
        "there is nobody to tell: {refused:?}"
    );
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("the Job is there")
            .status(),
        JobStatus::AwaitingReview,
        "and the Job is still at the gate, not running with a lost note"
    );
}

/// The two reads, over the router that ships. **Evidence carries what was
/// submitted and the diff carries the bytes**, and neither is on `get_job`.
#[tokio::test]
async fn the_work_and_the_claims_are_two_reads() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reviewing_the_first_step(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]).showing("--- a/src/log.rs\n+++ b/src/log.rs\n"),
    ));
    let events = fleet.events();
    // The row the read finds is the gate's own: reaching the gate is what wrote
    // it, and nothing here plants one.
    let job_id = at_the_gate(&fleet, &home).await;
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));

    let (status, body) = call(
        &app,
        "GET",
        &format!("/jobs/{}/evidence", job_id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let evidence: JobEvidence = ipc::decode("a Job's evidence", &body).expect("a JobEvidence");
    assert_eq!(evidence.job_id.as_str(), job_id.as_str());
    let claim = evidence.steps.first().expect("the step that submitted");
    assert_eq!(claim.step_id.as_str(), "implement");
    assert_eq!(claim.evidence_type.as_wire(), "diff");
    assert_eq!(claim.claimed, "The reader stops one line later.");
    assert!(
        claim.not_claimed.is_some(),
        "this submission drew a boundary, so it crosses"
    );

    let (status, body) = call(&app, "GET", &format!("/jobs/{}/diff", job_id.as_str()), "").await;
    assert_eq!(status, StatusCode::OK);
    let diff: JobDiff = ipc::decode("a Job's diff", &body).expect("a JobDiff");
    let work = diff.work.expect("the worktree is there and was read");
    assert_eq!(
        work.files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/log.rs"],
        "the file list, beside the bytes rather than instead of them"
    );
    assert_eq!(
        work.patch.as_deref(),
        Some("--- a/src/log.rs\n+++ b/src/log.rs\n"),
        "the expensive half, on the route that asks for it"
    );

    // The same Job through `get_job`, which is read on every open. **Nothing of
    // either read is on it**, which is the whole reason both have routes.
    let (status, body) = call(&app, "GET", &format!("/jobs/{}", job_id.as_str()), "").await;
    assert_eq!(status, StatusCode::OK);
    let detail = String::from_utf8(body).expect("a body that is text");
    assert!(
        !detail.contains("+++ b/src/log.rs"),
        "the patch is not folded into the summary read: {detail}"
    );

    // A Job with no worktree answers `work` absent rather than an empty
    // reading. A Drone that changed nothing is a different answer.
    let ungated = fleet
        .propose(a_proposal("never dispatched"))
        .await
        .expect("a Job at the approval gate");
    let (status, body) = call(
        &app,
        "GET",
        &format!("/jobs/{}/diff", ungated.id().as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let diff: JobDiff = ipc::decode("a Job's diff", &body).expect("a JobDiff");
    assert!(
        diff.work.is_none(),
        "there was no worktree to read, which is not the same as reading nothing"
    );
}

/// A Job with no id behind it is a 404 on both reads, and never an empty
/// answer — the refusal `get_job_events` makes, for the same reason.
#[tokio::test]
async fn an_id_that_names_nothing_is_refused_on_both_reads() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    for path in ["evidence", "diff"] {
        let (status, _) = call(&app, "GET", &format!("/jobs/01NOTAJOB/{path}"), "").await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{path} answers 404 rather than an empty {path}"
        );
    }
    for path in ["approve_review", "request_changes", "reject"] {
        let (status, _) = call(
            &app,
            "POST",
            &format!("/jobs/01NOTAJOB/{path}"),
            r#"{"note": "anything"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path} answers 404");
    }
}
