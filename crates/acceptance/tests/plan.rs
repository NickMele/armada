//! Plan's claim: **a running Job says how far through its plan it is.** Every
//! assertion is made against what crossed [`ipc::encode`], as `board.rs`'s are,
//! and the apparatus is [`bench::plan`]. #893, #894 and #895 are what this
//! file carries; the rest of the claim is a row, in the order a person meets
//! it:
//!
//! | Step of the claim | Carried by |
//! |---|---|
//! | Job detail's Plan region draws the plan | #896 |
//! | A person adds a fifth task, and the working Drone is told | #897 |
//! | The Board's row draws "2 of 4" | #898 |
//!
//! Which step is given which tool is `fleet::work_plan`, `pub(crate)`, and is
//! asserted in `fleet`'s own tests beside the store's append-only history.

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;

use std::sync::Arc;

use core_model::{NotAnUpdate, StepId};
use fleet::{briefing, Crossed, ThePlan};
use ipc::mcp::{NotAnArgument, PlanArgument};
use ipc::{ChangedBy, Event, JobPlanChanged};
use testkit::FakeJudge;

use bench::board::{detail, received_detail, step_facts};
use bench::plan::{
    called, gated_on_implement, gated_on_the_plan, received_event, received_row, refused, Planned,
    IMPLEMENT, PLAN,
};

const FOUR_TASKS: &str = r#"{"approach":"Stop the reader at the end, then cover the bound",
    "tasks":[{"title":"Stop read_to before end","detail":"crates/store/src/read.rs:41"},
             {"title":"Cover the last row","detail":""},
             {"title":"Check the writer's bound","detail":""},
             {"title":"Note the bound in the module","detail":""}]}"#;

const THREE_TASKS: &str = r#"{"approach":"Stop the reader at the end, then cover the bound",
    "tasks":[{"title":"Stop read_to before end","detail":"crates/store/src/read.rs:41"},
             {"title":"Cover the last row","detail":""},
             {"title":"Check the writer's bound","detail":""}]}"#;

/// #894's row: **the plan step is asked for a plan, and every brief after it
/// carries THE PLAN as Fleet holds it — with every task's id and state —
/// rather than an empty block or none at all.** Rendered through
/// `fleet::briefing::first_turn`, the same assembly a real spawn calls, so
/// what this reads is the opening turn a Drone actually gets.
#[test]
fn the_implement_brief_carries_the_plan_fleet_holds_with_every_tasks_state() {
    let mut planned = Planned::created("fix the reader's bound");
    planned.kept(called("record_plan", THREE_TASKS), PLAN, 1);
    planned.kept(
        called("update_task", r#"{"task":"T1","state":"done","reason":""}"#),
        IMPLEMENT,
        1,
    );
    let plan = planned.kept(
        called(
            "update_task",
            r#"{"task":"T2","state":"working","reason":""}"#,
        ),
        IMPLEMENT,
        1,
    );

    let step = StepId::new(IMPLEMENT);
    let follows = planned
        .job
        .workflow()
        .step(&step)
        .expect("implement is a real step")
        .follows_plan();
    let crossed = Crossed::nothing().and_the_plan(Some(ThePlan::of(&plan, follows)));
    let brief = briefing::first_turn(&planned.job, planned.job.workflow(), &step, &crossed)
        .expect("a brief assembles");
    let said = brief.as_str();

    assert!(said.contains("THE PLAN"), "{said}");
    for expected in [
        "T1 [done]",
        "T2 [working]",
        "T3 [open]",
        "update_task",
        "add_task",
    ] {
        assert!(said.contains(expected), "{expected} missing from {said}");
    }
    assert!(
        !said.contains("follows_plan"),
        "THE PLAN never says follows_plan to a Drone: {said}"
    );
}

/// #894's other half of the same row: **a step that does not follow the plan
/// is shown it and told it is not the step's to change.**
#[test]
fn the_handoff_brief_carries_the_plan_and_no_instruction_to_change_it() {
    let mut planned = Planned::created("fix the reader's bound");
    let plan = planned.kept(called("record_plan", THREE_TASKS), PLAN, 1);

    let step = StepId::new("handoff");
    let follows = planned
        .job
        .workflow()
        .step(&step)
        .expect("handoff is a real step")
        .follows_plan();
    assert!(!follows, "handoff does not follow the plan");
    let crossed = Crossed::nothing().and_the_plan(Some(ThePlan::of(&plan, follows)));
    let brief = briefing::first_turn(&planned.job, planned.job.workflow(), &step, &crossed)
        .expect("a brief assembles");
    let said = brief.as_str();

    assert!(said.contains("THE PLAN"), "{said}");
    assert!(said.contains("not yours to change"), "{said}");
    assert!(!said.contains("update_task"), "{said}");
    assert!(!said.contains("add_task"), "{said}");
}

/// The plan step's Drone records four tasks, and the plan reaches the wire
/// whole, with the row beside it counting them.
#[test]
fn a_plan_steps_drone_records_four_tasks_and_the_plan_reaches_the_wire() {
    let mut planned = Planned::created("fix the reader's bound");
    let call = called("record_plan", FOUR_TASKS);
    assert_eq!(call.tool, "record_plan");
    planned.kept(call, PLAN, 1);

    let opened = received_detail(&planned.detail());
    let plan = opened.work_plan.expect("the plan crossed");
    assert_eq!(
        plan.approach,
        "Stop the reader at the end, then cover the bound"
    );
    assert_eq!(
        plan.tasks
            .iter()
            .map(|task| (task.id.as_str(), task.state.as_wire()))
            .collect::<Vec<_>>(),
        [
            ("T1", "open"),
            ("T2", "open"),
            ("T3", "open"),
            ("T4", "open")
        ]
    );
    assert_eq!(plan.tasks[0].detail, "crates/store/src/read.rs:41");
    assert!(
        matches!(&plan.recorded_by, ChangedBy::Step { step_id, attempt: 1 } if step_id.as_str() == PLAN),
        "the plan says which run of which step recorded it"
    );
    let counted = opened.job.tasks.expect("the nested row counts the plan");
    assert_eq!(
        (counted.done, counted.working, counted.open, counted.dropped),
        (0, 0, 4, 0)
    );
}

/// The plan step's gate reads Fleet's record of the plan: it passes on one,
/// and where none was recorded it stops the step and says which it was.
#[tokio::test]
async fn the_plan_steps_gate_passes_on_a_recorded_plan_and_says_so_when_there_is_none() {
    let mut planned = Planned::created("fix the reader's bound");
    let plan = planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);

    let passed = gated_on_the_plan(&planned, Some(plan)).await;
    assert!(passed.advanced(), "four tasks recorded is a plan step done");

    let stopped = gated_on_the_plan(&planned, None).await;
    assert!(!stopped.advanced(), "no plan is not a plan of none");
    let facts = step_facts(&planned.job, &[(PLAN, &stopped)]);
    let opened = received_detail(&detail(&planned.job, None, &facts));
    let ran = &opened.steps[0].check_runs;
    assert_eq!(
        ran.iter()
            .map(|run| (run.name.as_str(), run.outcome.as_wire()))
            .collect::<Vec<_>>(),
        [("plan_recorded", "failed")]
    );
    assert_eq!(ran[0].produced.as_deref(), Some("no plan was recorded"));
}

/// Implement completes two tasks and drops one with a reason. The row counts
/// them, the detail carries the reason, and the event says the plan moved.
#[test]
fn implement_completes_two_tasks_and_drops_one_with_a_reason() {
    let mut planned = Planned::created("fix the reader's bound");
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);
    for arguments in [
        r#"{"task":"T1","state":"working","reason":""}"#,
        r#"{"task":"T1","state":"done","reason":""}"#,
        r#"{"task":"T2","state":"done","reason":""}"#,
        r#"{"task":"T3","state":"dropped","reason":"the writer's bound was never inclusive"}"#,
    ] {
        planned.kept(called("update_task", arguments), IMPLEMENT, 1);
    }

    let tasks = received_row(&planned.row())
        .tasks
        .expect("a row with a plan counts it");
    assert_eq!(
        (tasks.done, tasks.working, tasks.open, tasks.dropped),
        (2, 0, 1, 1)
    );
    assert_eq!(
        (tasks.done, tasks.done + tasks.working + tasks.open),
        (2, 3),
        "2 of 3: a dropped task is not counted against the figure"
    );

    let plan = received_detail(&planned.detail())
        .work_plan
        .expect("the plan crossed");
    assert_eq!(plan.tasks[2].state.as_wire(), "dropped");
    assert_eq!(
        plan.tasks[2].reason.as_deref(),
        Some("the writer's bound was never inclusive")
    );
    assert_eq!(plan.tasks[3].state.as_wire(), "open");

    // A done task may be reopened; a dropped one stays dropped, and says so.
    let reopened = r#"{"task":"T1","state":"working","reason":""}"#;
    assert!(planned
        .judged(called("update_task", reopened), IMPLEMENT, 1)
        .is_ok());
    let undropped = r#"{"task":"T3","state":"open","reason":""}"#;
    assert_eq!(
        planned.judged(called("update_task", undropped), IMPLEMENT, 1),
        Err(core_model::PlanRefused::StaysDropped {
            named: core_model::TaskId::read("T3").expect("an id")
        })
    );

    let event = Event::JobPlanChanged(JobPlanChanged {
        job_id: planned.job.id().into(),
        tasks,
        actor: core_model::Actor::Drone.into(),
        at: planned.row().created_at,
    });
    assert_eq!(event.kind(), "job.plan_changed");
    assert_eq!(received_event(&event), event);
}

/// A drop with no reason is refused by name, as the tool error the Drone reads,
/// and the plan it would have changed is as it was.
#[test]
fn a_drop_with_no_reason_is_refused_by_name_and_changes_nothing() {
    let mut planned = Planned::created("fix the reader's bound");
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);
    let before = received_detail(&planned.detail()).work_plan;

    let (why, sent) = refused(
        "update_task",
        r#"{"task":"T2","state":"dropped","reason":""}"#,
    );
    assert_eq!(
        why,
        NotAnArgument::Planning(PlanArgument::NotAnUpdate(
            NotAnUpdate::DroppedWithoutAReason
        ))
    );
    assert!(sent.contains("needs a `reason`"), "{sent}");
    assert!(sent.contains(r#""isError":true"#), "{sent}");
    assert_eq!(received_detail(&planned.detail()).work_plan, before);
}

/// The plan step's retry replaces the plan whole; a following step's retry
/// records nothing, so the states it left stand.
#[test]
fn a_plan_steps_retry_replaces_the_plan_and_a_following_steps_retry_keeps_its_states() {
    let mut planned = Planned::created("fix the reader's bound");
    let first_try = r#"{"approach":"A first try","tasks":[{"title":"One","detail":""}]}"#;
    planned.kept(called("record_plan", first_try), PLAN, 1);
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 2);
    let plan = received_detail(&planned.detail())
        .work_plan
        .expect("a plan");
    assert_eq!(plan.tasks.len(), 4, "the second run's plan, whole");
    assert!(matches!(
        plan.recorded_by,
        ChangedBy::Step { attempt: 2, .. }
    ));

    let done = r#"{"task":"T1","state":"done","reason":""}"#;
    planned.kept(called("update_task", done), IMPLEMENT, 1);
    let working = r#"{"task":"T2","state":"working","reason":""}"#;
    planned.kept(called("update_task", working), IMPLEMENT, 2);
    let tasks = received_row(&planned.row()).tasks.expect("counted");
    assert_eq!((tasks.done, tasks.working, tasks.open), (1, 1, 2));
}

/// #895's row: **a workflow declares both steps, and the Judge reads task
/// states.** `bug_workflow_with_a_plan` is read by `config`'s own parser from
/// `plan`, `plan_recorded` and `follows_plan`; implement names
/// `plan.evidence` in `reference_docs`, and its brief carries the plan as
/// Fleet holds it — task ids, states and a dropped one's reason — never the
/// three lines a Drone submitted about the diff.
#[tokio::test]
async fn implements_judge_reads_the_plan_fleet_holds_not_the_drones_words_about_it() {
    let mut planned = Planned::created("fix the reader's bound");
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);
    planned.kept(
        called("update_task", r#"{"task":"T1","state":"done","reason":""}"#),
        IMPLEMENT,
        1,
    );
    let plan = planned.kept(
        called(
            "update_task",
            r#"{"task":"T3","state":"dropped","reason":"the writer's bound was never inclusive"}"#,
        ),
        IMPLEMENT,
        1,
    );

    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = gated_on_implement(&planned, plan, judge.clone()).await;
    assert!(
        matches!(ruling, fleet::Ruling::Advanced { .. }),
        "tasks_match_the_diff had no objection: {ruling:?}"
    );

    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion, one call: {asked:?}");
    let brief = &asked[0];
    for expected in [
        "T1",
        "done",
        "T3",
        "dropped",
        "the writer's bound was never inclusive",
    ] {
        assert!(brief.contains(expected), "{expected} missing from {brief}");
    }
}
