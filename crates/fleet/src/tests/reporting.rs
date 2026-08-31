//! Filing what a person knows went wrong, over the router that ships.
//!
//! **Every case starts from a real refusal**, for `overruling`'s reason: a Job
//! moved into the state by hand would hide whether a report can be filed from
//! where a wrong verdict actually leaves one. The Judge is asked, it refuses on
//! a criterion, the Job escalates, and the person disagrees — which is the case
//! this whole operation was written for, twice over, in one week.
//!
//! The five claims here are the five that decide whether this is worth having:
//! the record carries what the Job left behind, a report with no sentence is
//! not a report, a report outlives the Job it is about, nothing credential-
//! shaped and no home directory reaches the file, and **the count reads the
//! claim rather than the sentence** — which is what makes it survive a reason
//! that says `probe`.
//!
//! And three about the scope, which the first pass of this got wrong in both
//! directions: a step with no criterion is a scope rather than a refusal, a
//! criterion with no step is still a refusal, and each refusal names the field
//! it is about rather than sharing one sentence written for a different act.

use std::sync::Arc;

use axum::http::StatusCode;
use core_model::{JobStatus, StepId, StepState};
use ipc::{Report, ReportList, RunId};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::daemon::{
    a_fleet_judged_by, a_proposal, diff_evidence, fittings, worktree_directory,
};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

const QUESTION: &str = "Does the fix address the cause the note names?";

/// The Judge refusing what it was shown, in the three lines a person reads
/// before deciding they disagree with it. The record has to carry all three.
fn a_judge_that_refuses() -> FakeJudge {
    FakeJudge::refusing(
        "the scope note's own words",
        "a sentence that appears in no scope note",
        "correct work is refused",
    )
}

fn judged() -> config::ResolvedWorkflow {
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

/// A Job dispatched, worked, and refused by the Judge — standing escalated with
/// its step stopped, which is where a person meets a wrong verdict.
async fn refused(fleet: &Fixture, home: &TempDir, brief: &str) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal(brief))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
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
        escalated
            .step(&StepId::new("implement"))
            .map(|step| step.state()),
        Some(StepState::Stopped)
    );
    job_id
}

/// What the button sends: the claim, the sentence, and the criterion whose
/// verdict is disputed.
fn a_filing(said: &str) -> String {
    format!(
        r#"{{"claim":"wrongly_refused","said":{said},"step_id":"implement","criterion_id":"c1"}}"#,
        said = quoted(said)
    )
}

/// A filing scoped to a step and to no criterion — what a person can send about
/// a step the gate judged nothing on.
fn a_filing_about_a_step(said: &str) -> String {
    format!(
        r#"{{"claim":"wrongly_refused","said":{said},"step_id":"implement"}}"#,
        said = quoted(said)
    )
}

/// A filing that names a criterion and no step. The half that is still refused.
fn a_filing_about_an_orphan_criterion(said: &str) -> String {
    format!(
        r#"{{"claim":"wrongly_refused","said":{said},"criterion_id":"c1"}}"#,
        said = quoted(said)
    )
}

/// A JSON string, without reaching for a serialiser this crate is not allowed
/// to have: reading and writing JSON is `store`'s and `ipc`'s, and a fixture
/// that needed one would be a test earning the crate a dependency.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// **The act, whole.** The Judge refused correct work, a person says so in
/// their own words, and what is filed carries the Job's own record around the
/// sentence — every one of which was already written down before anybody
/// pressed anything.
#[tokio::test]
async fn a_filed_report_carries_the_persons_sentence_and_the_jobs_own_record() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home, "fix the off-by-one").await;
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(
        &app,
        "POST",
        &format!("/jobs/{}/report", job_id.as_str()),
        &a_filing("the quoted sentence is in no scope note and in no submission"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "a report now exists");
    let filed: Report = ipc::decode("a report", &body).expect("a Report");
    assert_eq!(
        filed.said, "the quoted sentence is in no scope note and in no submission",
        "the finding is the person's, word for word"
    );
    assert_eq!(filed.claim, ipc::Claim::WronglyRefused);
    assert_eq!(filed.origin, ipc::ReportOrigin::Human);
    assert_eq!(filed.job_id.as_str(), job_id.as_str());
    assert_eq!(
        filed.step_id.as_ref().map(ipc::StepId::as_str),
        Some("implement")
    );
    assert_eq!(
        filed.criterion_id.as_ref().map(ipc::CriterionId::as_str),
        Some("c1")
    );
    // The evidence, collected rather than captured: the citation the Judge
    // gave, the step's own stop, what the Drone claimed, and the brief.
    for expected in [
        "a sentence that appears in no scope note",
        "correct work is refused",
        "gate_failure",
        "The reader stops one line later.",
        "fix the off-by-one",
        "c1",
    ] {
        assert!(
            filed.record.contains(expected),
            "the record does not carry {expected:?}, and it says:\n{}",
            filed.record
        );
    }
}

/// **The record was already there.** A filing with the bundle and no sentence
/// adds nothing, so it is refused — and refused before anything is written, so
/// a second press is not filing over a half-record.
#[tokio::test]
async fn a_report_with_no_sentence_is_refused_and_files_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home, "fix the off-by-one").await;
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(
        &app,
        "POST",
        &format!("/jobs/{}/report", job_id.as_str()),
        &a_filing("   "),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "a blank sentence is the request being unworkable, not the bytes"
    );
    let (status, body) = call(&app, "GET", "/reports", "").await;
    assert_eq!(status, StatusCode::OK);
    let listed: ReportList = ipc::decode("the reports", &body).expect("a ReportList");
    assert!(
        listed.reports.is_empty(),
        "a refused filing left a report behind: {:?}",
        listed.reports
    );
}

/// **The refusal names the field.** A blank sentence and an orphan criterion
/// were one 422 with one message, and the message was `override_verdict`'s —
/// so a filing refused for its scope read as a filing with no reason, and the
/// reason was in the request all along.
#[tokio::test]
async fn each_cause_of_a_refused_filing_says_which_one_it_is() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home, "fix the off-by-one").await;
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let route = format!("/jobs/{}/report", job_id.as_str());

    let (status, body) = call(&app, "POST", &route, &a_filing("   ")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let blank: ipc::WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert!(
        blank.message.contains("sentence"),
        "the blank sentence is not named: {}",
        blank.message
    );

    let (status, body) = call(
        &app,
        "POST",
        &route,
        &a_filing_about_an_orphan_criterion(
            "the judge marked this met and the diff does not do what the criterion asks",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let orphan: ipc::WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert!(
        orphan.message.contains("c1") && orphan.message.contains("step"),
        "the orphan criterion is not named: {}",
        orphan.message
    );
    assert!(
        !orphan.message.contains("reason"),
        "the scope refusal is still describing an override's missing reason: {}",
        orphan.message
    );
    assert_ne!(
        blank.message, orphan.message,
        "two causes are still one sentence"
    );
}

/// **A step with no criterion is a scope, not a half of one.**
///
/// The case that produced this: a step escalates on `gate_undecided`, the
/// Judge's answer could not be read at all, so `judged` is empty and there is no
/// criterion to name. Refusing the step on its own would push the report onto
/// the whole Job and throw away the only scope the person had.
#[tokio::test]
async fn a_report_can_name_a_step_the_gate_judged_no_criterion_on() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home, "fix the off-by-one").await;
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(
        &app,
        "POST",
        &format!("/jobs/{}/report", job_id.as_str()),
        &a_filing_about_a_step("the gate could not read the judge's answer and no criterion was ever marked, so this is about the step"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "a report now exists");
    let filed: Report = ipc::decode("a report", &body).expect("a Report");
    assert_eq!(
        filed.step_id.as_ref().map(ipc::StepId::as_str),
        Some("implement"),
        "the one piece of scope that existed was thrown away"
    );
    assert_eq!(
        filed.criterion_id, None,
        "a criterion was invented for a step that had none"
    );

    // And it reads back the same way, rather than the row being refused as a
    // half-set scope on the way out.
    let (_, body) = call(&app, "GET", "/reports", "").await;
    let listed: ReportList = ipc::decode("the reports", &body).expect("a ReportList");
    assert_eq!(listed.reports.len(), 1);
    assert_eq!(
        listed.reports[0].step_id.as_ref().map(ipc::StepId::as_str),
        Some("implement")
    );
    assert_eq!(listed.reports[0].criterion_id, None);
}

/// **The claim is what counts, and the sentence is what a person reads.**
///
/// The first override in this repository carries the reason `probe`, sent to
/// find out whether the route was served, and `job_events` is append-only so it
/// says that for ever. A report with the same reason is filed the same way, and
/// the calibration count is unchanged by it — because nothing counting reads
/// the prose.
#[tokio::test]
async fn a_useless_sentence_still_counts_because_the_count_reads_the_claim() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged(),
        a_judge_that_refuses(),
    );
    let job_id = refused(&fleet, &home, "fix the off-by-one").await;
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(
        &app,
        "POST",
        &format!("/jobs/{}/report", job_id.as_str()),
        &a_filing("probe"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, body) = call(&app, "GET", "/reports", "").await;
    let listed: ReportList = ipc::decode("the reports", &body).expect("a ReportList");
    assert_eq!(listed.calibration.reports_filed, 1);
    assert_eq!(
        listed.calibration.refusals_disputed, 1,
        "the count is over the closed set, so a reason that says nothing still counts"
    );
    assert_eq!(listed.calibration.passes_disputed, 0);
    assert_eq!(
        listed.calibration.refusals_recorded, 1,
        "the one criterion the judge answered not_met"
    );
    assert_eq!(
        listed.reports[0].said, "probe",
        "and the sentence is served exactly as it was written, weak and visible"
    );
}

/// **It outlives `armada clean`.** The Job is forgotten with every row beneath
/// it, and the report is still whole — which is the case a report is most
/// needed in, since a Job cleaned up is a Job whose history is gone.
#[tokio::test]
async fn a_report_survives_the_job_being_forgotten() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged(),
        a_judge_that_refuses(),
    ));
    let job_id = refused(&fleet, &home, "fix the off-by-one").await;
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    let (status, _) = call(
        &app,
        "POST",
        &format!("/jobs/{}/report", job_id.as_str()),
        &a_filing("the judge refused work that was right"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    fleet
        .store()
        .lock()
        .await
        .forget_job(&job_id)
        .expect("the job is forgotten");

    let (status, _) = call(&app, "GET", &format!("/jobs/{}", job_id.as_str()), "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the job really is gone");
    let (_, body) = call(&app, "GET", "/reports", "").await;
    let listed: ReportList = ipc::decode("the reports", &body).expect("a ReportList");
    assert_eq!(listed.reports.len(), 1);
    assert!(
        listed.reports[0]
            .record
            .contains("a sentence that appears in no scope note"),
        "the record went with the job, and it should not have"
    );
    assert_eq!(
        listed.reports[0].job_id.as_str(),
        job_id.as_str(),
        "the id it had is kept, dangling on purpose"
    );
}

/// **Redacted on the way in.** A credential-shaped value in the brief and a
/// home directory in a path are both in the Job's record; neither reaches the
/// file, and the words around them do.
///
/// The fixture's home is the temporary directory the whole Fleet is assembled
/// under, so the worktree paths in the record carry it — this does not have to
/// plant one to be a real case.
#[tokio::test]
async fn no_credential_and_no_home_directory_reaches_the_filed_record() {
    let home = TempDir::new();
    let root = home.path().to_string_lossy().to_string();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        judged(),
        a_judge_that_refuses(),
    );
    let brief = format!(
        "run it with AGENT_API_TOKEN=not-a-real-one against {root}/worktrees, \
         and the reader is off by one"
    );
    let job_id = refused(&fleet, &home, &brief).await;
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(
        &app,
        "POST",
        &format!("/jobs/{}/report", job_id.as_str()),
        &a_filing(&format!("it was told to read {root}/secrets first")),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let filed: Report = ipc::decode("a report", &body).expect("a Report");
    assert!(
        !filed.record.contains("not-a-real-one"),
        "a credential reached the file: {}",
        filed.record
    );
    assert!(
        !filed.record.contains(&root),
        "the home directory reached the file: {}",
        filed.record
    );
    assert!(
        !filed.said.contains(&root),
        "the home directory reached the person's own sentence: {}",
        filed.said
    );
    // Redacted *from*, never *with*: that the variable was set is often the
    // diagnostic, and the sentence around it is the report.
    assert!(
        filed.record.contains("AGENT_API_TOKEN=[redacted]"),
        "the fact that the variable was set was thrown away too: {}",
        filed.record
    );
    assert!(
        filed.record.contains("~/worktrees"),
        "the path stopped being readable: {}",
        filed.record
    );
    assert!(filed.said.contains("~/secrets"));
}

/// A report about a Job Fleet does not hold is a 404, not a report about
/// nothing. The one thing filed on a Job has to be about that Job.
#[tokio::test]
async fn a_report_about_a_job_that_does_not_exist_is_refused() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, _) = call(
        &app,
        "POST",
        "/jobs/01NOSUCHJOB/report",
        &a_filing("this job never existed"),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}
