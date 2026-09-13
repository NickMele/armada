//! A plan is its history replayed: a recording replaces, an addition takes the
//! next id, an update moves one task, and nothing else resets a state.

use core::num::NonZeroU32;

use crate::{
    Approach, Attempt, NewTask, NotAnUpdate, PlanAuthor, PlanChange, PlanEntry, PlanRefused,
    StepId, TaskId, TaskState, TaskUpdate, Timestamp, WorkPlan,
};

fn by_step(step: &str) -> PlanAuthor {
    PlanAuthor::Step {
        step_id: StepId::new(step),
        attempt: Attempt::FIRST,
    }
}

fn entry(change: PlanChange, by: PlanAuthor) -> PlanEntry {
    PlanEntry {
        change,
        by,
        at: Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z"),
    }
}

fn recorded(titles: &[&str]) -> PlanEntry {
    entry(
        PlanChange::Recorded {
            approach: Approach::new("Fix the reader's bound, then cover it").expect("an approach"),
            tasks: titles
                .iter()
                .map(|title| NewTask::new(title, "").expect("a title"))
                .collect(),
        },
        by_step("plan"),
    )
}

fn updated(id: &str, to: TaskUpdate) -> PlanEntry {
    entry(
        PlanChange::Updated {
            task: TaskId::read(id).expect("a task id"),
            to,
        },
        by_step("implement"),
    )
}

fn states(plan: &WorkPlan) -> Vec<(String, TaskState)> {
    plan.tasks()
        .iter()
        .map(|task| (task.id().to_string(), task.state()))
        .collect()
}

#[test]
fn a_recording_mints_ids_in_order_and_every_task_starts_open() {
    let plan = WorkPlan::fold(&[recorded(&["one", "two", "three"])])
        .expect("a history that replays")
        .expect("a plan");
    assert_eq!(
        states(&plan),
        [
            ("T1".to_string(), TaskState::Open),
            ("T2".to_string(), TaskState::Open),
            ("T3".to_string(), TaskState::Open),
        ]
    );
    assert_eq!(plan.recorded_by(), &by_step("plan"));
    assert_eq!(plan.approach(), "Fix the reader's bound, then cover it");
}

#[test]
fn nothing_recorded_is_no_plan_rather_than_an_empty_one() {
    assert_eq!(WorkPlan::fold(&[]), Ok(None));
}

/// The recording step's retry replaces the plan whole, states included.
#[test]
fn a_second_recording_replaces_the_plan_whole() {
    let plan = WorkPlan::fold(&[
        recorded(&["one", "two"]),
        updated("T1", TaskUpdate::Done),
        recorded(&["another"]),
    ])
    .expect("a history that replays")
    .expect("a plan");
    assert_eq!(states(&plan), [("T1".to_string(), TaskState::Open)]);
    assert_eq!(plan.tasks()[0].title(), "another");
}

/// A following step's retry records nothing, so the states it left stand.
#[test]
fn updates_stand_until_a_recording_replaces_them() {
    let plan = WorkPlan::fold(&[
        recorded(&["one", "two", "three", "four"]),
        updated("T1", TaskUpdate::Done),
        updated("T2", TaskUpdate::Working),
        updated("T2", TaskUpdate::Done),
        updated(
            "T4",
            TaskUpdate::Dropped(crate::DropReason::new("the writer never had it").expect("r")),
        ),
    ])
    .expect("a history that replays")
    .expect("a plan");
    let counts = plan.counts();
    assert_eq!((counts.done, counts.open, counts.dropped), (2, 1, 1));
    assert_eq!(
        counts.not_dropped(),
        3,
        "done over not dropped reads 2 of 3"
    );
    assert_eq!(
        plan.task(TaskId::read("T4").expect("id"))
            .and_then(|task| task.reason()),
        Some("the writer never had it")
    );
}

#[test]
fn an_added_task_takes_the_next_id_and_the_place_it_was_put() {
    let plan = WorkPlan::fold(&[
        recorded(&["one", "two"]),
        entry(
            PlanChange::Added {
                task: NewTask::new("between", "found while reading").expect("a title"),
                after: TaskId::read("T1"),
            },
            PlanAuthor::Person,
        ),
    ])
    .expect("a history that replays")
    .expect("a plan");
    assert_eq!(
        plan.tasks()
            .iter()
            .map(|task| task.id().to_string())
            .collect::<Vec<_>>(),
        ["T1", "T3", "T2"],
        "an id is never reused and never renumbered"
    );
    assert_eq!(plan.tasks()[1].detail(), "found while reading");
}

#[test]
fn a_change_the_plan_cannot_take_is_refused_by_name() {
    let no_plan = WorkPlan::after(None, &updated("T1", TaskUpdate::Done));
    assert_eq!(no_plan, Err(PlanRefused::NoPlan));

    let plan = WorkPlan::fold(&[recorded(&["one"])])
        .expect("replays")
        .expect("a plan");
    assert_eq!(
        WorkPlan::after(Some(&plan), &updated("T9", TaskUpdate::Done)),
        Err(PlanRefused::NoSuchTask {
            named: TaskId::numbered(NonZeroU32::new(9).expect("nine"))
        })
    );
    let misplaced = entry(
        PlanChange::Added {
            task: NewTask::new("t", "").expect("a title"),
            after: TaskId::read("T7"),
        },
        by_step("implement"),
    );
    assert!(matches!(
        WorkPlan::after(Some(&plan), &misplaced),
        Err(PlanRefused::NoSuchPlace { .. })
    ));
}

#[test]
fn dropped_needs_a_reason_and_no_other_state_keeps_one() {
    assert_eq!(
        TaskUpdate::read("dropped", "  "),
        Err(NotAnUpdate::DroppedWithoutAReason)
    );
    assert_eq!(
        TaskUpdate::read("done", "because"),
        Err(NotAnUpdate::ReasonWithoutADrop {
            state: TaskState::Done
        })
    );
    assert!(matches!(
        TaskUpdate::read("finished", ""),
        Err(NotAnUpdate::NoSuchState { .. })
    ));
    assert_eq!(
        TaskUpdate::read("dropped", "covered by T2")
            .expect("a drop")
            .reason(),
        Some("covered by T2")
    );
    assert_eq!(TaskUpdate::read("working", ""), Ok(TaskUpdate::Working));
}

#[test]
fn a_task_id_reads_only_its_own_spelling() {
    assert_eq!(TaskId::read("T12").map(TaskId::number), Some(12));
    for spelling in ["T0", "t1", "T01", "1", "T", "T1a", ""] {
        assert_eq!(TaskId::read(spelling), None, "{spelling:?}");
    }
    assert!(NewTask::new("   ", "detail").is_none());
    assert!(Approach::new("").is_none());
}
