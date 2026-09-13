//! The plan tools read as the change they ask for, and a call that cannot be
//! one is refused by name where the Drone can still fix it.

use core_model::{PlanChange, TaskState};

use crate::mcp::{
    answer, read, Answered, Incoming, NotAnArgument, NotRecorded, PlanArgument, PlanCall,
};

fn called(tool: &str, arguments: &str) -> Incoming {
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    );
    read(body.as_bytes())
}

fn change(incoming: Incoming) -> PlanChange {
    match incoming {
        Incoming::Plan {
            call: PlanCall { change, .. },
            ..
        } => change,
        other => panic!("a plan call, not {other:?}"),
    }
}

fn refusal(incoming: Incoming) -> NotAnArgument {
    match incoming {
        Incoming::NotASubmission { why, .. } => why,
        other => panic!("a refusal, not {other:?}"),
    }
}

#[test]
fn a_recorded_plan_reads_as_its_approach_and_tasks_in_order() {
    let PlanChange::Recorded { approach, tasks } = change(called(
        "record_plan",
        r#"{"approach":"Bound the reader","tasks":[
            {"title":"Stop at the end","detail":"read.rs:41"},
            {"title":"Cover it","detail":""}]}"#,
    )) else {
        panic!("a recording");
    };
    assert_eq!(approach.as_str(), "Bound the reader");
    assert_eq!(
        tasks.iter().map(|task| task.title()).collect::<Vec<_>>(),
        ["Stop at the end", "Cover it"]
    );
    assert_eq!(tasks[0].detail(), "read.rs:41");
}

#[test]
fn an_update_names_a_task_and_a_state() {
    let PlanChange::Updated { task, to } = change(called(
        "update_task",
        r#"{"task":"T3","state":"done","reason":""}"#,
    )) else {
        panic!("an update");
    };
    assert_eq!((task.number(), to.state()), (3, TaskState::Done));

    let PlanChange::Added { after, .. } = change(called(
        "add_task",
        r#"{"title":"Also this","detail":"","after":""}"#,
    )) else {
        panic!("an addition");
    };
    assert_eq!(after, None, "an empty `after` is the end of the list");
}

/// **The refusal a Drone reads**, through the answer it is sent as.
#[test]
fn a_drop_with_no_reason_is_refused_by_name_as_a_tool_error() {
    let why = refusal(called(
        "update_task",
        r#"{"task":"T2","state":"dropped","reason":"   "}"#,
    ));
    assert_eq!(
        why,
        NotAnArgument::Planning(PlanArgument::NotAnUpdate(
            core_model::NotAnUpdate::DroppedWithoutAReason
        ))
    );
    let sent = answer(Answered::Refused {
        id: crate::mcp::CallId::of(serde_json::json!(1)),
        why: NotRecorded {
            because: why.to_string(),
        },
    })
    .expect("plain data");
    assert!(sent.contains("needs a `reason`"), "{sent}");
    assert!(sent.contains(r#""isError":true"#), "{sent}");
}

#[test]
fn a_call_that_cannot_be_a_change_says_which_field_is_wrong() {
    for (tool, arguments, expected) in [
        (
            "update_task",
            r#"{"task":"three","state":"done","reason":""}"#,
            "not a task id",
        ),
        (
            "update_task",
            r#"{"task":"T1","state":"finished","reason":""}"#,
            "`state` is `finished`",
        ),
        (
            "update_task",
            r#"{"task":"T1","state":"done","reason":"it was easy"}"#,
            "kept only for `dropped`",
        ),
        (
            "record_plan",
            r#"{"approach":"a","tasks":["one"]}"#,
            "not a list of tasks",
        ),
        (
            "record_plan",
            r#"{"approach":"  ","tasks":[]}"#,
            "`approach` is empty",
        ),
        (
            "add_task",
            r#"{"title":"x","detail":"","after":"","step_id":"plan"}"#,
            "Fleet knows which Job",
        ),
    ] {
        let why = refusal(called(tool, arguments)).to_string();
        assert!(why.contains(expected), "{tool} {arguments}: {why}");
    }
}

#[test]
fn every_drone_is_shown_the_three_plan_tools() {
    let listed = answer(Answered::Tools {
        id: crate::mcp::CallId::of(serde_json::json!(1)),
    })
    .expect("plain data");
    for tool in ["record_plan", "add_task", "update_task"] {
        assert!(listed.contains(&format!(r#""name":"{tool}""#)), "{tool}");
    }
}
