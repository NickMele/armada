//! A Job's plan as Bridge receives it: whole on the detail, counted on the row,
//! pointed at by `job.plan_changed` — and absent, never null, where there is none.

use core_model::{
    Approach, Attempt, DropReason, NewTask, PlanAuthor, PlanChange, PlanEntry, StepId, TaskId,
    TaskUpdate, Timestamp,
};

use crate::tests::job;
use crate::{decode, encode, ChangedBy, Event, JobPlanChanged, JobSummary, WorkPlan};

fn a_plan() -> core_model::WorkPlan {
    let at = Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z");
    let by = PlanAuthor::Step {
        step_id: StepId::new("plan"),
        attempt: Attempt::FIRST,
    };
    let entries = [
        PlanEntry {
            change: PlanChange::Recorded {
                approach: Approach::new("Bound the reader").expect("an approach"),
                tasks: vec![
                    NewTask::new("Stop at the end", "read.rs:41").expect("a title"),
                    NewTask::new("Cover the bound", "").expect("a title"),
                ],
            },
            by: by.clone(),
            at: at.clone(),
        },
        PlanEntry {
            change: PlanChange::Updated {
                task: TaskId::read("T2").expect("an id"),
                to: TaskUpdate::Dropped(DropReason::new("T1's test covers it").expect("a reason")),
            },
            by: PlanAuthor::Person,
            at,
        },
    ];
    core_model::WorkPlan::fold(&entries)
        .expect("replays")
        .expect("a plan")
}

#[test]
fn a_plan_crosses_whole_and_reads_back_as_itself() {
    let sent = WorkPlan::from(&a_plan());
    let body = encode(&sent).expect("plain data");
    let received: WorkPlan = decode("a plan", body.as_bytes()).expect("reads back");

    assert_eq!(received, sent);
    assert_eq!(
        received.recorded_by,
        ChangedBy::Step {
            step_id: crate::StepId::from(&StepId::new("plan")),
            attempt: 1
        }
    );
    assert!(body.contains(r#""by":"step""#), "{body}");
    assert_eq!(received.tasks[0].id, "T1");
    assert_eq!(received.tasks[1].state.as_wire(), "dropped");
    assert_eq!(
        received.tasks[1].reason.as_deref(),
        Some("T1's test covers it")
    );
    assert!(
        !body.contains(r#""detail":"""#) && !body.contains("null"),
        "an empty detail and an absent reason are left out: {body}"
    );
}

#[test]
fn a_task_marked_working_carries_its_windows_and_one_never_marked_carries_none() {
    let mut history = vec![PlanEntry {
        change: PlanChange::Recorded {
            approach: Approach::new("Bound the reader").expect("an approach"),
            tasks: vec![
                NewTask::new("Stop at the end", "").expect("a title"),
                NewTask::new("Cover the bound", "").expect("a title"),
            ],
        },
        by: PlanAuthor::Person,
        at: Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z"),
    }];
    for (to, at) in [
        (TaskUpdate::Working, "2026-09-13T10:01:00.000Z"),
        (TaskUpdate::Done, "2026-09-13T10:02:00.000Z"),
        (TaskUpdate::Working, "2026-09-13T10:03:00.000Z"),
    ] {
        history.push(PlanEntry {
            change: PlanChange::Updated {
                task: TaskId::read("T1").expect("an id"),
                to,
            },
            by: PlanAuthor::Person,
            at: Timestamp::from_rfc3339(at),
        });
    }
    let plan = core_model::WorkPlan::fold(&history)
        .expect("replays")
        .expect("a plan");
    let body = encode(&WorkPlan::from(&plan)).expect("plain data");
    let received: WorkPlan = decode("a plan", body.as_bytes()).expect("reads back");

    let windows = &received.tasks[0].working_windows;
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].entered.as_str(), "2026-09-13T10:01:00.000Z");
    assert_eq!(
        windows[0].left.as_ref().map(|at| at.as_str()),
        Some("2026-09-13T10:02:00.000Z")
    );
    assert_eq!(windows[1].left, None, "still working, so no end is sent");
    assert!(received.tasks[1].working_windows.is_empty());
    assert_eq!(
        body.matches("working_windows").count(),
        1,
        "left out on the task never marked: {body}"
    );
}

#[test]
fn a_row_with_no_plan_carries_no_task_field() {
    let summary = JobSummary::from(&job());
    let body = encode(&summary).expect("plain data");
    assert!(!body.contains("tasks"), "{body}");

    let mut counted = summary;
    counted.tasks = Some(a_plan().counts().into());
    let body = encode(&counted).expect("plain data");
    let received: JobSummary = decode("a row", body.as_bytes()).expect("reads back");
    let tasks = received.tasks.expect("the counts crossed");
    assert_eq!((tasks.open, tasks.dropped), (1, 1));
}

#[test]
fn a_plan_change_travels_under_the_name_the_inventory_declares() {
    let event = Event::JobPlanChanged(JobPlanChanged {
        job_id: (job().id()).into(),
        tasks: a_plan().counts().into(),
        actor: core_model::Actor::Drone.into(),
        at: (&Timestamp::from_rfc3339("2026-09-13T10:01:00.000Z")).into(),
    });
    assert_eq!(event.kind(), "job.plan_changed");
    let body = encode(&event).expect("plain data");
    let received: Event = decode("an event", body.as_bytes()).expect("reads back");
    assert_eq!(received, event);
}
