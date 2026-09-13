//! Which step may change the plan, and what a Drone on any other step is told.
//!
//! **The permitted path stops at the predicate.** The store half of a kept
//! change is `store`'s own test. A dispatched Drone standing on a real,
//! parsed plan step is [`a_dispatched_drone_may_record_the_plan_its_step_asks_for`]
//! — blocked until `config` could read `plan` and `follows_plan`, `#895`.

use adapter_traits::Grant;
use core_model::{
    AdvanceGate, Approach, EvidenceType, JobId, NewTask, PlanAuthor, PlanChange, PlanEntry,
    ResolvedStep, StepId, TaskId, TaskUpdate, Timestamp, Ulid, WorkPlan,
};
use testkit::FakeWorkProduct;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet, a_proposal, a_proposal_for, fittings, one, plan_and_implement, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::work_plan::{permitted, plan_grants, receipt_word, NotPlanned};

fn a_step(id: &str, evidence: Option<EvidenceType>, follows: bool) -> ResolvedStep {
    ResolvedStep::frozen(
        StepId::new(id),
        id.to_string(),
        evidence,
        Vec::new(),
        AdvanceGate::Auto,
        Vec::new(),
        None,
        0,
        None,
    )
    .following_plan(follows)
}

fn a_recording() -> PlanChange {
    PlanChange::Recorded {
        approach: Approach::new("Bound the reader").expect("an approach"),
        tasks: vec![NewTask::new("Stop at the end", "").expect("a title")],
    }
}

fn an_update() -> PlanChange {
    PlanChange::Updated {
        task: TaskId::read("T1").expect("an id"),
        to: TaskUpdate::Done,
    }
}

fn an_addition() -> PlanChange {
    PlanChange::Added {
        task: NewTask::new("Cover it", "").expect("a title"),
        after: None,
    }
}

#[test]
fn only_the_step_whose_product_is_a_plan_may_record_it_or_is_granted_to() {
    let plan = a_step("plan", Some(EvidenceType::Plan), false);
    assert!(permitted(&plan, &a_recording()).is_ok());
    assert_eq!(plan_grants(&plan), [Grant::RecordThePlan]);
    assert!(matches!(
        permitted(&plan, &an_update()),
        Err(NotPlanned::NotItsToFollow { .. })
    ));

    let implement = a_step("implement", Some(EvidenceType::Diff), true);
    assert!(matches!(
        permitted(&implement, &a_recording()),
        Err(NotPlanned::NotItsToRecord { .. })
    ));
}

#[test]
fn only_a_step_that_follows_the_plan_may_add_or_update_a_task_or_is_granted_to() {
    let implement = a_step("implement", Some(EvidenceType::Diff), true);
    assert!(permitted(&implement, &an_update()).is_ok());
    assert!(permitted(&implement, &an_addition()).is_ok());
    assert_eq!(plan_grants(&implement), [Grant::WorkThePlan]);

    let handoff = a_step("handoff", None, false);
    assert!(plan_grants(&handoff).is_empty());
    for change in [a_recording(), an_update(), an_addition()] {
        assert!(permitted(&handoff, &change).is_err(), "{change:?}");
    }
}

#[test]
fn an_added_task_is_answered_with_the_id_it_was_given() {
    let at = Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z");
    let by = PlanAuthor::Person;
    let plan = WorkPlan::fold(&[
        PlanEntry {
            change: a_recording(),
            by: by.clone(),
            at: at.clone(),
        },
        PlanEntry {
            change: an_addition(),
            by,
            at,
        },
    ])
    .expect("replays")
    .expect("a plan");
    assert_eq!(receipt_word(&an_addition(), &plan), "T2");
    assert_eq!(receipt_word(&a_recording(), &plan), "recorded");
}

/// The shipped workflow's first step neither records nor follows a plan, so
/// every change is refused in words and nothing is written or published.
#[tokio::test]
async fn a_drone_on_a_step_with_no_part_in_the_plan_is_told_so_and_nothing_is_kept() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("keep a plan it was not given"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("released to run");

    let refused = fleet
        .change_plan(&job_id, &a_recording())
        .await
        .expect_err("not this step's to record");
    assert!(
        refused
            .to_string()
            .contains("does not record the Job's plan"),
        "{refused}"
    );
    let refused = fleet
        .change_plan(&job_id, &an_update())
        .await
        .expect_err("not this step's to follow");
    assert!(matches!(refused, NotPlanned::NotItsToFollow { .. }));
    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .work_plan(&job_id)
            .expect("reads"),
        None
    );
}

#[tokio::test]
async fn a_call_with_nothing_working_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::untouched());
    let nobody = JobId::carried(Ulid::carried("01NOTHINGWORKING"));
    assert!(matches!(
        fleet.change_plan(&nobody, &a_recording()).await,
        Err(NotPlanned::NothingIsWorking)
    ));
}

/// A real workflow, read by `config` from `plan`, `plan_recorded` and
/// `follows_plan`, resolved and frozen onto a Job. A Drone dispatched onto
/// its plan step may record; `plan_recorded` gates the advance on a plan
/// with at least one task; and the following step's Drone may add and
/// update — none of which this suite could stand up before `#895`.
#[tokio::test]
async fn a_dispatched_drone_may_record_the_plan_its_step_asks_for() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(plan_and_implement());
    let fleet = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal_for("fix the reader's bound", "fixture-plan"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    let running = dispatched(&fleet, &job_id).await.expect("it dispatches");
    assert_eq!(
        running.current_step_id().map(StepId::as_str),
        Some("plan"),
        "the parsed workflow's first step"
    );

    let loaded = fleet.load(&job_id).await.expect("reads");
    let plan = loaded
        .workflow()
        .step(&StepId::new("plan"))
        .expect("the plan step resolved");
    assert!(plan.records_plan());
    assert_eq!(plan_grants(plan), [Grant::RecordThePlan]);
    let implement = loaded
        .workflow()
        .step(&StepId::new("implement"))
        .expect("the implement step resolved");
    assert!(implement.follows_plan());
    assert_eq!(plan_grants(implement), [Grant::WorkThePlan]);

    fleet
        .change_plan(&job_id, &a_recording())
        .await
        .expect("the plan step's own product is a plan");

    submitted_by_the_one(
        &fleet,
        Call {
            evidence_type: EvidenceType::Plan,
            claimed: Claimed("The fix is planned as one task."),
            shown_by: ShownBy("the plan recorded with record_plan"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("the plan step asked for a plan");
    let turned = fleet.turn().await.expect("the gate runs");
    assert!(
        matches!(turned.ruled(), Some(crate::gate::Ruling::Advanced { .. })),
        "one recorded task clears plan_recorded's default min_tasks: {:?}",
        turned.ruled()
    );
    assert_eq!(
        fleet
            .load(&job_id)
            .await
            .expect("reads")
            .current_step_id()
            .map(StepId::as_str),
        Some("implement")
    );

    let plan = fleet
        .change_plan(&job_id, &an_update())
        .await
        .expect("implement follows the plan");
    assert_eq!(plan.tasks()[0].state(), core_model::TaskState::Done);
}
