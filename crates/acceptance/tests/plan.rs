//! Plan's claim: **a running Job says how far through its plan it is.** Every
//! assertion is made against what crossed [`ipc::encode`], as `board.rs`'s are,
//! and the apparatus is [`bench::plan`]. Every step of the claim is carried:
//! #893 to #895 and #897 here, and #896 and #898 as the values their screens draw.
//! #1006 widens it: a step may keep its own product and record the plan
//! beside it, and neither a plan step nor a following one is ever told to plan
//! a task for the checks that already run on their own.
//!
//! | Not proved here | Why not, and what proves it |
//! |---|---|
//! | That job detail's Plan region draws the plan | Nothing here renders. `JobDetail.work_plan` is asserted below; `planOf` is exercised only by the `PlanPartwayDone` and `PlanWithADroppedTask` stories |
//! | That the Board's row draws "2 of 4" | The same. `JobSummary.tasks` is asserted below, the dropped task left out of the figure; `taskFigureOf` and `taskBarSegmentsOf` are tested in `board.test.ts` |
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
    bug_workflow_with_a_plan_beside_a_product, called, gated_on_implement, gated_on_the_plan,
    received_event, received_row, refused, revert_shaped_workflow, Planned, IMPLEMENT, PLAN,
    REVERT_SHAPED,
};

const FOUR_TASKS: &str = r#"{"approach":"Stop the reader at the end, then cover the bound",
    "tasks":[{"title":"Stop read_to before end","note":"","scope":["crates/store/src/read.rs"],"expects":"a test for the last row"},
             {"title":"Cover the last row","note":"","scope":[],"expects":""},
             {"title":"Check the writer's bound","note":"","scope":[],"expects":""},
             {"title":"Note the bound in the module","note":"","scope":[],"expects":""}]}"#;

const THREE_TASKS: &str = r#"{"approach":"Stop the reader at the end, then cover the bound",
    "tasks":[{"title":"Stop read_to before end","note":"","scope":["crates/store/src/read.rs"],"expects":"a test for the last row"},
             {"title":"Cover the last row","note":"","scope":[],"expects":""},
             {"title":"Check the writer's bound","note":"","scope":[],"expects":""}]}"#;

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
        called(
            "update_task",
            r#"{"task":"T1","state":"done","reason":"","shown":""}"#,
        ),
        IMPLEMENT,
        1,
    );
    let plan = planned.kept(
        called(
            "update_task",
            r#"{"task":"T2","state":"working","reason":"","shown":""}"#,
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

/// #894's first `## In` item: **a plan step's own brief asks it to record the
/// plan with `record_plan`**, rather than depending on a Drone noticing the
/// tool's description by chance. Rendered through `fleet::briefing::first_turn`,
/// the same assembly a real spawn calls.
#[test]
fn the_plan_steps_brief_asks_it_to_record_the_plan() {
    let planned = Planned::created("fix the reader's bound");

    let step = StepId::new(PLAN);
    let brief = briefing::first_turn(
        &planned.job,
        planned.job.workflow(),
        &step,
        &Crossed::nothing(),
    )
    .expect("a brief assembles");
    let said = brief.as_str();

    assert!(said.contains("WHAT THIS PART DELIVERS"), "{said}");
    assert!(said.contains("record_plan"), "{said}");
    assert!(
        !said.contains("Write this part's finding to a file"),
        "the plan step no longer asks for a file: {said}"
    );
    assert!(
        !said.contains("THE PLAN\n"),
        "the recording step is not shown a plan it has not recorded: {said}"
    );
    assert!(
        said.contains(
            "The checks each part must pass run on their own when that \
             part is submitted, so the plan carries no task for running them."
        ),
        "the plan step is told not to plan a task for the checks that already run on their own: {said}"
    );
}

/// **`#1006`'s widening: a step keeps its own product and records the plan
/// beside it.** `read`'s brief asks for both — `WHAT THIS PART DELIVERS`
/// still names `read.md`, and `RECORDING THE PLAN` is a second block beside
/// it, carrying the same "no task for the checks" sentence as a pure plan
/// step's. `implement`'s brief still shows THE PLAN, so `follows_plan` on a
/// later step reads a plan recorded by a step whose own product was not
/// `plan`.
#[test]
fn a_step_that_keeps_its_own_product_may_also_record_the_plan_beside_it() {
    let planned = Planned::created_with(
        "fix the reader's bound",
        bug_workflow_with_a_plan_beside_a_product(),
    );

    let step = StepId::new("read");
    let brief = briefing::first_turn(
        &planned.job,
        planned.job.workflow(),
        &step,
        &Crossed::nothing(),
    )
    .expect("a brief assembles");
    let said = brief.as_str();

    assert!(said.contains("WHAT THIS PART DELIVERS"), "{said}");
    assert!(said.contains(".armada/artifacts/read.md"), "{said}");
    assert!(said.contains("RECORDING THE PLAN"), "{said}");
    assert!(said.contains("record_plan"), "{said}");
    assert!(
        said.contains(
            "The checks each part must pass run on their own when that \
             part is submitted, so the plan carries no task for running them."
        ),
        "{said}"
    );
}

/// **The owner's follow-up, 13 Sep 2026: a step that records the plan and
/// also follows it must still see THE PLAN on a retry.** `revert`'s shape —
/// one step, `records_plan: true` and `follows_plan: true` together. Attempt
/// 1 records the plan and marks a task done; attempt 2's opening brief must
/// carry THE PLAN with that task's state, not a bare record instruction —
/// without it a retried Drone holding `update_task` cannot see what it is
/// updating. A step that only records, with no `follows_plan`, still gets no
/// THE PLAN block; `the_plan_steps_brief_asks_it_to_record_the_plan` above
/// pins that half.
#[test]
fn a_step_that_records_and_follows_the_plan_sees_it_on_a_retry() {
    let mut planned = Planned::created_with("undo the change", revert_shaped_workflow());
    planned.kept(called("record_plan", THREE_TASKS), REVERT_SHAPED, 1);
    let plan = planned.kept(
        called(
            "update_task",
            r#"{"task":"T1","state":"done","reason":"","shown":""}"#,
        ),
        REVERT_SHAPED,
        1,
    );

    let step = StepId::new(REVERT_SHAPED);
    let follows = planned
        .job
        .workflow()
        .step(&step)
        .expect("revert_shaped is a real step")
        .follows_plan();
    assert!(follows, "the fixture step follows the plan it records");
    let crossed = Crossed::nothing().and_the_plan(Some(ThePlan::of(&plan, follows)));
    let brief = briefing::first_turn(&planned.job, planned.job.workflow(), &step, &crossed)
        .expect("attempt 2's brief assembles");
    let said = brief.as_str();

    assert!(said.contains("THE PLAN"), "{said}");
    assert!(said.contains("T1 [done]"), "{said}");
    assert!(said.contains("update_task"), "{said}");
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
    assert_eq!(plan.tasks[0].scope, ["crates/store/src/read.rs"]);
    assert_eq!(plan.tasks[0].expects, "a test for the last row");
    assert!(plan.tasks[0].note.is_empty());
    assert!(
        plan.tasks[1].scope.is_empty(),
        "a task naming no paths sends none, rather than an empty entry"
    );
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
        r#"{"task":"T1","state":"working","reason":"","shown":""}"#,
        r#"{"task":"T1","state":"done","reason":"","shown":""}"#,
        r#"{"task":"T2","state":"done","reason":"","shown":""}"#,
        r#"{"task":"T3","state":"dropped","reason":"the writer's bound was never inclusive","shown":""}"#,
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
    let reopened = r#"{"task":"T1","state":"working","reason":"","shown":""}"#;
    assert!(planned
        .judged(called("update_task", reopened), IMPLEMENT, 1)
        .is_ok());
    let undropped = r#"{"task":"T3","state":"open","reason":"","shown":""}"#;
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
        r#"{"task":"T2","state":"dropped","reason":"","shown":""}"#,
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
    let first_try =
        r#"{"approach":"A first try","tasks":[{"title":"One","note":"","scope":[],"expects":""}]}"#;
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

    let done = r#"{"task":"T1","state":"done","reason":"","shown":""}"#;
    planned.kept(called("update_task", done), IMPLEMENT, 1);
    let working = r#"{"task":"T2","state":"working","reason":"","shown":""}"#;
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
        called(
            "update_task",
            r#"{"task":"T1","state":"done","reason":"","shown":""}"#,
        ),
        IMPLEMENT,
        1,
    );
    let plan = planned.kept(
        called(
            "update_task",
            r#"{"task":"T3","state":"dropped","reason":"the writer's bound was never inclusive","shown":""}"#,
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
    // **One call per criterion, and the step declares two since `#1432`.** The
    // rule this pins is the ratio, not the number: a broad question produces
    // agreeable prose, so each criterion is asked on its own.
    assert_eq!(asked.len(), 2, "one call per criterion: {asked:?}");
    let brief = asked
        .iter()
        .find(|brief| brief.contains("Does the diff do every task set to done"))
        .expect("tasks_match_the_diff was asked");
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

/// #897's row: **a person adds a fifth task, and the working Drone is
/// told.** The add is kept under `PlanAuthor::Person`, which is the whole of
/// what tells a person's change from a Drone's on the record — a plan takes
/// either the same way. What would reach a working Drone is asserted on the
/// exact bytes crossing [`ipc::encode`]: `fleet::session::Turn::plan_changed`
/// is the one constructor that can build the turn, from `fleet::PlanChanged`,
/// which is `docs/contracts/agent-prompt.md`'s drafted wording.
#[test]
fn a_person_adds_a_fifth_task_and_the_working_drone_is_told() {
    let mut planned = Planned::created("fix the reader's bound");
    let plan = planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);
    assert_eq!(plan.tasks().len(), 4, "four tasks recorded");

    let task = core_model::NewTask::new("Add a regression test for the bound", "", &[], "")
        .expect("a title makes a task");
    let entry = core_model::PlanEntry {
        change: core_model::PlanChange::Added {
            task: task.clone(),
            after: None,
        },
        by: core_model::PlanAuthor::Person,
        at: core_model::Timestamp::from_rfc3339("2026-09-13T10:00:05.000Z"),
    };
    let after = core_model::WorkPlan::after(Some(&plan), &entry).expect("a person may add");
    assert_eq!(after.tasks().len(), 5, "the fifth task");
    let fifth = after.tasks().last().expect("the task just added");
    assert_eq!(fifth.id().to_string(), "T5");
    assert_eq!(fifth.title(), "Add a regression test for the bound");
    assert!(
        matches!(after.recorded_by(), core_model::PlanAuthor::Step { .. }),
        "an add is not a recording, so the plan's last whole recording is still the plan step's"
    );

    let note = fleet::PlanChanged::added(fifth.id(), &task);
    let turn = fleet::session::Turn::plan_changed(&note);
    let wire = ipc::encode(&turn).expect("a turn that serialises");
    for expected in [
        "T5",
        "Add a regression test for the bound",
        "not a question",
    ] {
        assert!(wire.contains(expected), "{expected} missing from {wire}");
    }

    let event = Event::JobPlanChanged(JobPlanChanged {
        job_id: planned.job.id().into(),
        tasks: after.counts().into(),
        actor: core_model::Actor::Human.into(),
        at: (&entry.at).into(),
    });
    assert_eq!(event.kind(), "job.plan_changed");
    assert_eq!(received_event(&event), event);
}

/// The other half of #897: **a person drops a task with a reason, and it
/// shows struck through with the reason.** A drop is `WorkPlan::after`'s
/// `Updated` change under `PlanAuthor::Person`, exactly as a Drone's own
/// `update_task` is under `PlanAuthor::Step` — one plan, two authors.
#[test]
fn a_person_drops_a_task_with_a_reason_and_it_shows_struck_through() {
    let mut planned = Planned::created("fix the reader's bound");
    let plan = planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);

    let third = plan.tasks()[2].id();
    let entry = core_model::PlanEntry {
        change: core_model::PlanChange::Updated {
            task: third,
            to: core_model::TaskUpdate::Dropped(
                core_model::DropReason::new("the writer's bound was never inclusive")
                    .expect("a reason"),
            ),
            shown: None,
        },
        by: core_model::PlanAuthor::Person,
        at: core_model::Timestamp::from_rfc3339("2026-09-13T10:00:06.000Z"),
    };
    let after = core_model::WorkPlan::after(Some(&plan), &entry).expect("a person may drop");
    let dropped = after.task(third).expect("the task is still on the list");
    assert_eq!(dropped.state().as_wire(), "dropped");
    assert_eq!(
        dropped.reason(),
        Some("the writer's bound was never inclusive")
    );

    let note = fleet::PlanChanged::dropped(
        third,
        dropped.title(),
        "the writer's bound was never inclusive",
    );
    let turn = fleet::session::Turn::plan_changed(&note);
    let wire = ipc::encode(&turn).expect("a turn that serialises");
    for expected in [
        third.to_string().as_str(),
        "the writer's bound was never inclusive",
        "settled",
    ] {
        assert!(wire.contains(expected), "{expected} missing from {wire}");
    }
}

/// **The account a Drone gives of its own evidence reaches a Judge.** `#1432`
/// gave a task `expects` and `shown`; nothing asked about them, so a task
/// could prove something other than what it set out to prove and only a person
/// reading both lines would know. The question is the Judge's now, and this is
/// the proof it arrives with what it needs to answer.
///
/// **Both criteria are asked, and separately.** `tasks_match_the_diff` reads
/// the diff, which is ground truth; this one reads the Drone's prose about it.
/// One call per criterion is Judge's own rule — a broad question produces
/// agreeable prose.
#[tokio::test]
async fn the_evidence_a_task_claims_reaches_the_judge_with_what_it_expected() {
    let mut planned = Planned::created("fix the reader's bound");
    planned.kept(called("record_plan", FOUR_TASKS), PLAN, 1);
    let plan = planned.kept(
        called(
            "update_task",
            r#"{"task":"T1","state":"done","reason":"","shown":"left-column.test.ts covers it, not the fixture the plan named"}"#,
        ),
        IMPLEMENT,
        1,
    );

    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = gated_on_implement(&planned, plan, judge.clone()).await;
    assert!(
        matches!(ruling, fleet::Ruling::Advanced { .. }),
        "{ruling:?}"
    );

    let asked = judge.asked();
    assert_eq!(asked.len(), 2, "one call per criterion: {asked:?}");
    let evidence = asked
        .iter()
        .find(|brief| brief.contains("accounts for itself") || brief.contains("`shown`"))
        .expect("the evidence criterion was asked");
    for expected in [
        // What the plan asked of the task, written before the work.
        "a test for the last row",
        // What the work says proved it, written after — and different.
        "left-column.test.ts covers it, not the fixture the plan named",
    ] {
        assert!(
            evidence.contains(expected),
            "the Judge cannot answer without `{expected}`: {evidence}"
        );
    }
}
