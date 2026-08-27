//! What advances a step, what ends a Job, and what a Drone cannot reach.
//!
//! # These run real Checks
//!
//! A gated step's Check is a real command in a real directory, because the
//! failures the milestone step names — a hanging Check, a command that is not
//! installed — are the operating system's behaviour and a fake would assert
//! this crate's guess at it. The diff is faked, because git's opinion about a
//! diff is tested where git is.

use std::time::Duration;

use std::sync::Arc;

use adapter_traits::{Environment, Model, Worktree};
use config::{EvidenceType, ResolvedWorkflow};
use core_model::{
    AcceptanceCriterion, Actor, CriterionId, CriterionSource, Facts, Job, JobId, JobStatus,
    ManifestId, ModelName, NewJob, StepId, StepSeed, Subject, Target, Timestamp, Title,
    TopLevelOrigin, Ulid, Urgency,
};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};
use verification::{
    CheckFailed, Claimed, NeverRan, NotASubmission, NotClaimed, ShownBy, Submission,
};

use crate::at_step::AtStep;
use crate::evidence::{Call, EvidenceInbox, EvidenceTool};
use crate::gate::{apply, rule_on, CheckBudget, Ruling};
use crate::judging::{JudgeBudget, Judging};

const NOW: &str = "2026-08-26T09:00:00.000Z";

fn at(instant: &str) -> Timestamp {
    Timestamp::from_rfc3339(instant)
}

fn job_id() -> JobId {
    JobId::carried(Ulid::carried("01J0000000000000000000JOB0"))
}

/// A Job at `running`, reached by walking edges. There is no constructor that
/// takes a status, which is the point of the machine.
pub(super) fn running_job() -> Job {
    let created = Job::create_top_level(
        NewJob {
            id: job_id(),
            title: Title::new("make the suite pass").expect("a title"),
            workflow: workflow("/usr/bin/true").frozen().clone(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01J0000000000000000000MAN0")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("the-configured-model").expect("a model name"),
            acceptance_criteria: vec![AcceptanceCriterion {
                criterion_id: CriterionId::new("c1"),
                text: "the suite passes".into(),
                source: CriterionSource::Check,
            }],
            steps: vec![
                StepSeed {
                    step_id: StepId::new("implement"),
                    ordinal: 0,
                },
                StepSeed {
                    step_id: StepId::new("summarise"),
                    ordinal: 1,
                },
            ],
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None::<Subject>,
            redispatched_from: None,
            facts: Facts::empty(),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        at(NOW),
    );
    let queued = created
        .transition(Target::Queued, Actor::Human, at(NOW))
        .expect("approved")
        .job;
    queued
        .transition(Target::Running, Actor::Fleet, at(NOW))
        .expect("dispatched")
        .job
}

/// Two steps: one gated on a Check that passes and a non-empty diff, one gated
/// on nothing at all.
pub(super) fn workflow(run: &str) -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[
                Gate::Check {
                    name: "suite",
                    run,
                    expect_exit_code: 0,
                },
                Gate::DiffNonempty,
            ],
            judged_on: &[],
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

pub(super) fn worktree() -> Worktree {
    Worktree::at("/", "armada/01J0000000000000000000JOB0")
}

pub(super) fn diff_evidence() -> Submission {
    Submission::submitted(
        EvidenceType::Diff,
        Claimed("The loop is a fold."),
        ShownBy("`cargo test -p vcs` exit 0, 34 passing"),
        NotClaimed(""),
    )
    .expect("a legal submission")
}

pub(super) fn note_evidence() -> Submission {
    Submission::submitted(
        EvidenceType::FactsNote,
        Claimed("The path is derived from the repo name."),
        ShownBy("`worktree.rs:40`"),
        NotClaimed(""),
    )
    .expect("a legal submission")
}

fn diff_call<'a>() -> Call<'a> {
    Call {
        evidence_type: EvidenceType::Diff,
        claimed: Claimed("The loop is a fold."),
        shown_by: ShownBy("`cargo test -p vcs` exit 0, 34 passing"),
        not_claimed: NotClaimed(""),
    }
}

pub(super) fn budget() -> CheckBudget {
    CheckBudget::of(Duration::from_secs(20))
}

/// A Judge that fails every call it is given.
///
/// **The default for every case in this file**, because every step in them
/// declares no criterion. A gate that asked anyway would produce
/// `CouldNotDecide` rather than quietly advancing, so the cold-by-default rule
/// is a case that would break rather than a comment.
pub(super) fn judging() -> Judging {
    judged_by(FakeJudge::that_fails("a Judge that should never be asked"))
}

pub(super) fn judged_by(client: FakeJudge) -> Judging {
    Judging {
        client: Arc::new(client),
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
    }
}

// ------------------------------------------------------------------ the tool

#[test]
fn the_tool_returns_recorded_and_nothing_else() {
    let inbox = EvidenceInbox::new();
    let tool = EvidenceTool::for_job(job_id(), &inbox);
    let receipt = tool.submit(diff_call(), at(NOW)).expect("a legal call");
    assert_eq!(receipt.word(), "recorded");
}

/// The call cannot block on `cargo test`, so it must return before anything is
/// decided. What that looks like from outside: the submission is waiting and no
/// ruling exists.
#[test]
fn submitting_decides_nothing_and_leaves_the_evidence_waiting() {
    let inbox = EvidenceInbox::new();
    let tool = EvidenceTool::for_job(job_id(), &inbox);
    tool.submit(diff_call(), at(NOW)).expect("a legal call");

    assert_eq!(inbox.waiting(), 1);
    let landed = inbox.take().expect("the submission");
    assert_eq!(landed.job, job_id());
    assert_eq!(landed.submission.claimed(), "The loop is a fold.");
    assert_eq!(inbox.waiting(), 0);
}

/// "Tests pass" is not an artifact and no string at all is less than one. The
/// tool refuses it, and refusing means nothing reaches the inbox — a Drone that
/// evidenced nothing has not queued a gate run.
#[test]
fn a_call_naming_no_artifact_records_nothing() {
    let inbox = EvidenceInbox::new();
    let tool = EvidenceTool::for_job(job_id(), &inbox);
    let refused = tool.submit(
        Call {
            shown_by: ShownBy(""),
            ..diff_call()
        },
        at(NOW),
    );
    assert_eq!(refused, Err(NotASubmission::ShownByEmpty));
    assert_eq!(inbox.waiting(), 0);
}

#[test]
fn a_malformed_call_records_nothing() {
    let inbox = EvidenceInbox::new();
    let tool = EvidenceTool::for_job(job_id(), &inbox);
    let refused = tool.submit(
        Call {
            shown_by: ShownBy("   "),
            ..diff_call()
        },
        at(NOW),
    );
    assert!(refused.is_err());
    assert_eq!(inbox.waiting(), 0);
}

// ------------------------------------------------------------------ the gate

#[tokio::test]
async fn evidence_and_every_check_passing_advances_the_step() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert!(ruling.advanced(), "the ruling was {ruling:?}");
    assert!(matches!(ruling, Ruling::Advanced { .. }));
    let told = ruling.tell().expect("a turn").text().to_string();
    assert!(told.contains("Implement is verified"), "told: {told}");
    assert!(told.contains("Summarise"), "told: {told}");
    assert!(!ruling.ends_the_drone());
}

/// The assertion the milestone step exists for.
#[tokio::test]
async fn evidence_with_every_check_failing_advances_nothing() {
    let workflow = workflow("/usr/bin/false");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::untouched();

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert!(!ruling.advanced());
    let Ruling::Failed { failures, .. } = &ruling else {
        panic!("the ruling was {ruling:?}");
    };
    assert_eq!(
        failures,
        &[
            CheckFailed::WrongExitCode {
                check: "suite".to_string(),
                expected: 0,
                actual: 1,
            },
            CheckFailed::DiffEmpty,
        ]
    );
}

#[tokio::test]
async fn a_step_with_no_checks_advances_on_evidence_alone() {
    let workflow = workflow("/usr/bin/false");
    let worktree = worktree();
    let at_step = AtStep::named(workflow.frozen(), &StepId::new("summarise"), &worktree)
        .expect("the second step");
    // Nothing is asked of the worktree, and nothing is run in it.
    let work = FakeWorkProduct::untouched();

    let ruling = rule_on(
        at_step,
        &note_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert!(ruling.advanced());
    assert!(matches!(ruling, Ruling::Finished { .. }));
    assert!(
        work.asked().is_empty(),
        "the diff was read for a step that declares none"
    );
}

#[tokio::test]
async fn a_hanging_check_fails_rather_than_hanging() {
    let workflow = workflow("/bin/sleep 3600");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let started = std::time::Instant::now();
    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        CheckBudget::of(Duration::from_millis(300)),
        &judging(),
    )
    .await;
    let took = started.elapsed();

    assert!(!ruling.advanced());
    assert!(
        took < Duration::from_secs(10),
        "the gate took {took:?}, so the budget did not end the Check"
    );
    let Ruling::Failed { failures, .. } = &ruling else {
        panic!("the ruling was {ruling:?}");
    };
    assert_eq!(
        failures,
        &[CheckFailed::TimedOut {
            check: "suite".to_string(),
            after: Duration::from_millis(300),
        }]
    );
}

#[tokio::test]
async fn a_check_whose_command_does_not_exist_fails_rather_than_passing() {
    let workflow = workflow("armada-no-such-program");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert!(!ruling.advanced());
    let Ruling::Failed { failures, .. } = &ruling else {
        panic!("the ruling was {ruling:?}");
    };
    assert_eq!(
        failures,
        &[CheckFailed::NeverRan {
            check: "suite".to_string(),
            why: NeverRan::NoSuchCommand {
                program: "armada-no-such-program".to_string()
            },
        }]
    );
}

#[tokio::test]
async fn the_check_output_comes_back_for_a_person_to_read() {
    let workflow = workflow("/bin/sh -c 'echo the suite is unhappy; exit 2'");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    let Ruling::Failed { output, .. } = &ruling else {
        panic!("the ruling was {ruling:?}");
    };
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].check, "suite");
    assert_eq!(output[0].output.stdout.trim(), "the suite is unhappy");
}

#[tokio::test]
async fn evidence_of_the_wrong_kind_runs_no_checks_and_moves_nothing() {
    let workflow = workflow("/usr/bin/false");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::untouched();

    let ruling = rule_on(
        at_step,
        &note_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert!(matches!(ruling, Ruling::NotWhatTheStepAsked(_)));
    assert!(
        work.asked().is_empty(),
        "a check ran for evidence the step did not ask for"
    );
    assert!(apply(&running_job(), &ruling, at(NOW)).is_none());
}

/// A machine that cannot answer must not answer. An unreadable work product is
/// not an empty diff, and it is not a failed step either.
#[tokio::test]
async fn a_diff_that_cannot_be_read_decides_nothing() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::refusing("a repository that would not open");

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert!(!ruling.advanced());
    assert!(matches!(ruling, Ruling::CouldNotDecide { .. }));
    assert!(apply(&running_job(), &ruling, at(NOW)).is_none());
}

#[tokio::test]
async fn the_diff_fleet_reads_is_of_the_job_s_own_worktree() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert_eq!(work.asked(), [worktree.path().to_string()]);
}

// ------------------------------------------------------- what the Job then does

#[tokio::test]
async fn a_failed_check_ends_the_job_and_fleet_is_the_actor() {
    let workflow = workflow("/usr/bin/false");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;
    let moved = apply(&running_job(), &ruling, at(NOW))
        .expect("the Job moves")
        .expect("a legal move");

    assert_eq!(moved.job.status(), JobStatus::CompletedFailed);
    assert_eq!(moved.event.actor(), Actor::Fleet);
    assert!(ruling.ends_the_drone());
    assert!(
        ruling.tell().is_none(),
        "a terminated Drone was told something"
    );
}

#[tokio::test]
async fn an_advancing_step_does_not_move_the_job() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;
    assert!(apply(&running_job(), &ruling, at(NOW)).is_none());
}

#[tokio::test]
async fn the_last_step_advancing_completes_the_job() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::named(workflow.frozen(), &StepId::new("summarise"), &worktree)
        .expect("the last step");
    let work = FakeWorkProduct::untouched();

    let ruling = rule_on(
        at_step,
        &note_evidence(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;
    let moved = apply(&running_job(), &ruling, at(NOW))
        .expect("the Job moves")
        .expect("a legal move");

    assert_eq!(moved.job.status(), JobStatus::CompletedSuccess);
    assert_eq!(moved.event.actor(), Actor::Fleet);
    assert!(ruling.ends_the_drone());
    let told = ruling.tell().expect("a turn").text().to_string();
    assert!(told.contains("last part"), "told: {told}");
}

/// Killing and failing are different states and read differently. `killed`
/// carries no verdict; `completed_failed` is one.
#[test]
fn killing_a_job_reaches_killed_rather_than_failed() {
    let killed = running_job()
        .transition(Target::Killed, Actor::Human, at(NOW))
        .expect("a legal move");
    assert_eq!(killed.job.status(), JobStatus::Killed);
    assert_ne!(killed.job.status(), JobStatus::CompletedFailed);
    assert_eq!(killed.event.actor(), Actor::Human);
}

// ------------------------------------------------- what the gate cannot reach

/// A step cannot be gated against a step id the frozen workflow does not
/// declare, so there is no way to point the gate at a step the Job is not on.
#[test]
fn the_gate_cannot_be_pointed_at_a_step_the_workflow_does_not_declare() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    assert!(AtStep::named(workflow.frozen(), &StepId::new("invented"), &worktree).is_none());
}
