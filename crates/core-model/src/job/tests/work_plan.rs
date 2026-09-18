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
                .map(|title| NewTask::new(title, "", &[], "").expect("a title"))
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
            shown: None,
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
                task: NewTask::new("between", "found while reading", &[], "").expect("a title"),
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
    assert_eq!(plan.tasks()[1].note(), "found while reading");
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
            task: NewTask::new("t", "", &[], "").expect("a title"),
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
    assert!(NewTask::new("   ", "detail", &[], "").is_none());
    assert!(Approach::new("").is_none());
}

/// A done task may go back to work; a dropped one stays dropped. The work comes
/// back only as a new task, and the history keeps every move made before.
#[test]
fn a_done_task_may_be_reopened_and_a_dropped_one_stays_dropped() {
    let reason = || crate::DropReason::new("covered by T1").expect("a reason");
    let plan = WorkPlan::fold(&[
        recorded(&["one", "two"]),
        updated("T1", TaskUpdate::Done),
        updated("T1", TaskUpdate::Working),
        updated("T2", TaskUpdate::Dropped(reason())),
        updated("T2", TaskUpdate::Dropped(reason())),
    ])
    .expect("reopening a done task and re-dropping a dropped one both replay")
    .expect("a plan");
    assert_eq!(
        states(&plan),
        [
            ("T1".to_string(), TaskState::Working),
            ("T2".to_string(), TaskState::Dropped)
        ]
    );
    for to in [TaskUpdate::Open, TaskUpdate::Working, TaskUpdate::Done] {
        assert_eq!(
            WorkPlan::after(Some(&plan), &updated("T2", to)),
            Err(PlanRefused::StaysDropped {
                named: TaskId::read("T2").expect("an id")
            })
        );
    }
}

fn updated_at(id: &str, to: TaskUpdate, at: &str) -> PlanEntry {
    PlanEntry {
        at: Timestamp::from_rfc3339(at),
        ..updated(id, to)
    }
}

fn windows(plan: &WorkPlan, id: &str) -> Vec<(String, Option<String>)> {
    plan.task(TaskId::read(id).expect("an id"))
        .expect("the task")
        .working_windows()
        .iter()
        .map(|w| {
            (
                w.entered.as_str().to_string(),
                w.left.as_ref().map(|at| at.as_str().to_string()),
            )
        })
        .collect()
}

fn at(minute: u32) -> String {
    format!("2026-09-13T10:{minute:02}:00.000Z")
}

/// A window opens on the move into `working` and closes on the move out, so
/// Bridge can place a turn in the task the Drone had marked when it happened.
#[test]
fn each_move_into_working_opens_a_window_and_the_move_out_closes_it() {
    let plan = WorkPlan::fold(&[
        recorded(&["one", "two", "three"]),
        updated_at("T1", TaskUpdate::Working, &at(1)),
        updated_at("T1", TaskUpdate::Working, &at(2)),
        updated_at("T1", TaskUpdate::Done, &at(3)),
        updated_at("T2", TaskUpdate::Working, &at(4)),
        updated_at("T1", TaskUpdate::Working, &at(5)),
        updated_at("T1", TaskUpdate::Open, &at(6)),
    ])
    .expect("replays")
    .expect("a plan");

    assert_eq!(
        windows(&plan, "T1"),
        [(at(1), Some(at(3))), (at(5), Some(at(6)))],
        "working twice in a row is one window, and a reopened task gets a second"
    );
    assert_eq!(
        windows(&plan, "T2"),
        [(at(4), None)],
        "a task still working has a window with no end"
    );
    assert!(
        windows(&plan, "T3").is_empty(),
        "never marked, never windowed"
    );
}

/// A drop closes the window as any move out does, and a recording starts every
/// task again with none — its tasks are new, whatever their ids.
#[test]
fn a_drop_closes_a_window_and_a_recording_forgets_them() {
    let reason = crate::DropReason::new("not needed").expect("a reason");
    let plan = WorkPlan::fold(&[
        recorded(&["one"]),
        updated_at("T1", TaskUpdate::Working, &at(1)),
        updated_at("T1", TaskUpdate::Dropped(reason), &at(2)),
    ])
    .expect("replays")
    .expect("a plan");
    assert_eq!(windows(&plan, "T1"), [(at(1), Some(at(2)))]);

    let again = WorkPlan::after(Some(&plan), &recorded(&["one again"])).expect("a recording");
    assert!(windows(&again, "T1").is_empty());
}

/// The rendering a later step's Drone and the planning step's own Judge are
/// both handed. **Every field is in it**, because this is the whole of the
/// plan the next step sees: one left out is one it re-derives.
#[test]
fn a_rendering_carries_every_field_a_task_holds() {
    let recorded = entry(
        PlanChange::Recorded {
            approach: Approach::new("Reword the stat").expect("an approach"),
            tasks: vec![
                NewTask::new(
                    "Reword the Drones stat itself",
                    "the value becomes \"1 running · 2 max\"",
                    &[
                        "packages/screens/src/overview.ts",
                        "packages/screens/src/overview.test.ts",
                    ],
                    "overview.test.ts reads \"1 running · 2 max\"",
                )
                .expect("a title"),
                NewTask::new("Cover it", "", &[], "").expect("a title"),
            ],
        },
        by_step("plan"),
    );
    let plan = WorkPlan::fold(&[recorded])
        .expect("a history that replays")
        .expect("a plan");
    assert_eq!(
        plan.rendered(),
        "Approach: Reword the stat\n\nTasks:\
         \n  T1 [open] Reword the Drones stat itself\
         \n      note: the value becomes \"1 running · 2 max\"\
         \n      files: packages/screens/src/overview.ts packages/screens/src/overview.test.ts\
         \n      expects: overview.test.ts reads \"1 running · 2 max\"\
         \n  T2 [open] Cover it",
        "a task with no fields beyond its title gets no lines under it"
    );
}

/// What the plan expected and what the work showed are both kept, and read
/// beside each other. **The work disagreeing with the plan is the useful
/// answer**, so nothing here reconciles them.
#[test]
fn the_evidence_the_work_showed_stands_beside_what_the_plan_expected() {
    let recorded = entry(
        PlanChange::Recorded {
            approach: Approach::new("Reword the stat").expect("an approach"),
            tasks: vec![NewTask::new("Reword it", "", &[], "overview.test.ts").expect("a title")],
        },
        by_step("plan"),
    );
    let done = entry(
        PlanChange::Updated {
            task: TaskId::read("T1").expect("a task id"),
            to: TaskUpdate::Done,
            shown: crate::Shown::new("left-column.test.ts:24, not the one planned"),
        },
        by_step("implement"),
    );
    let plan = WorkPlan::fold(&[recorded, done])
        .expect("replays")
        .expect("a plan");
    let task = &plan.tasks()[0];
    assert_eq!(task.expects(), "overview.test.ts");
    assert_eq!(
        task.shown().map(crate::Shown::as_str),
        Some("left-column.test.ts:24, not the one planned")
    );
    assert!(plan.rendered().contains("expects: overview.test.ts"));
    assert!(plan
        .rendered()
        .contains("shown: left-column.test.ts:24, not the one planned"));
}

/// An update carrying no evidence leaves what an earlier one recorded, so a
/// task reopened and finished again does not lose what its first pass showed.
#[test]
fn an_update_with_nothing_shown_keeps_what_was_shown_before() {
    let recorded = entry(
        PlanChange::Recorded {
            approach: Approach::new("a").expect("an approach"),
            tasks: vec![NewTask::new("one", "", &[], "").expect("a title")],
        },
        by_step("plan"),
    );
    let id = TaskId::read("T1").expect("a task id");
    let shown = entry(
        PlanChange::Updated {
            task: id,
            to: TaskUpdate::Done,
            shown: crate::Shown::new("the covering test"),
        },
        by_step("implement"),
    );
    let reopened = entry(
        PlanChange::Updated {
            task: id,
            to: TaskUpdate::Open,
            shown: None,
        },
        by_step("implement"),
    );
    let plan = WorkPlan::fold(&[recorded, shown, reopened])
        .expect("replays")
        .expect("a plan");
    assert_eq!(
        plan.tasks()[0].shown().map(crate::Shown::as_str),
        Some("the covering test")
    );
}

/// A blank path is dropped rather than stored, so a trailing comma in a tool
/// call does not become a file the next step is told to open.
#[test]
fn a_tasks_scope_keeps_no_blank_paths() {
    let task = NewTask::new("one", "", &["  ", "crates/ipc/src/lib.rs", ""], "").expect("a title");
    assert_eq!(task.scope().len(), 1);
    assert_eq!(task.scope()[0].as_str(), "crates/ipc/src/lib.rs");
}
