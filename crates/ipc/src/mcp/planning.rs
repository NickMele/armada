//! The three plan tools: `record_plan` for the step whose product is a plan,
//! and `add_task` and `update_task` for a step that follows it. Their own
//! module for [`ask`](mod@super::ask)'s reason.
//!
//! **A call reads as `core_model::PlanChange`**, the change the store appends,
//! so there is no second shape of a change between the tool and the record.
//! Whether this step may make it is the daemon's answer, not this module's.
//!
//! **No field names a Job, a step or who is changing the plan.** The step is
//! the connection's, and the author of a tool call is always a Drone.

use std::fmt;

use core_model::{Approach, NewTask, NotAnUpdate, PlanChange, TaskId, TaskUpdate};
use serde_json::{json, Map, Value};

use super::tools::{closed, filled, text, NotAnArgument};

pub const RECORD_PLAN_TOOL: &str = "record_plan";
pub const ADD_TASK_TOOL: &str = "add_task";
pub const UPDATE_TASK_TOOL: &str = "update_task";

pub const RECORD_PLAN_FIELDS: &[&str] = &["approach", "tasks"];
/// The fields of one entry of `record_plan`'s `tasks`.
pub const TASK_FIELDS: &[&str] = &["title", "detail"];
pub const ADD_TASK_FIELDS: &[&str] = &["title", "detail", "after"];
pub const UPDATE_TASK_FIELDS: &[&str] = &["task", "state", "reason"];

/// One call of a plan tool, read as the change it asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCall {
    /// Which of the three was called. Fleet gives each to a different step.
    pub tool: &'static str,
    pub change: PlanChange,
}

/// Why a plan tool's arguments did not read as a change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanArgument {
    NotATaskList,
    NotATaskId { field: &'static str, named: String },
    NotAnUpdate(NotAnUpdate),
}

impl fmt::Display for PlanArgument {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanArgument::NotATaskList => out.write_str(
                "`tasks` is not a list of tasks. Each entry is an object with `title`, \
                 one line saying what the task is, and `detail`, which may be \"\"",
            ),
            PlanArgument::NotATaskId { field, named } => write!(
                out,
                "`{field}` is `{named}`, which is not a task id. Task ids are `T1`, \
                 `T2` and so on, as the plan numbered them"
            ),
            PlanArgument::NotAnUpdate(NotAnUpdate::NoSuchState { named }) => write!(
                out,
                "`state` is `{named}`. It is one of `open`, `working`, `done` or `dropped`"
            ),
            PlanArgument::NotAnUpdate(NotAnUpdate::DroppedWithoutAReason) => out.write_str(
                "`dropped` needs a `reason`: say in a sentence why the task will not be \
                 done, and call again",
            ),
            PlanArgument::NotAnUpdate(NotAnUpdate::ReasonWithoutADrop { state }) => write!(
                out,
                "`reason` is kept only for `dropped`, and this sets the task to `{}`. \
                 Send \"\" as the reason and call again",
                state.as_wire()
            ),
        }
    }
}

/// A call of one of the three, or `None` where `tool` is not one of them.
pub(super) fn read(
    tool: &'static str,
    arguments: &Map<String, Value>,
) -> Option<Result<PlanCall, NotAnArgument>> {
    let change = match tool {
        RECORD_PLAN_TOOL => recorded(arguments),
        ADD_TASK_TOOL => added(arguments),
        UPDATE_TASK_TOOL => updated(arguments),
        _ => return None,
    };
    Some(change.map(|change| PlanCall { tool, change }))
}

fn not_a_list() -> NotAnArgument {
    NotAnArgument::Planning(PlanArgument::NotATaskList)
}

fn recorded(arguments: &Map<String, Value>) -> Result<PlanChange, NotAnArgument> {
    closed(arguments, RECORD_PLAN_TOOL, RECORD_PLAN_FIELDS)?;
    let approach = Approach::new(&filled(arguments, "approach")?)
        .ok_or(NotAnArgument::Blank { field: "approach" })?;
    let listed = arguments
        .get("tasks")
        .ok_or(NotAnArgument::Missing { field: "tasks" })?
        .as_array()
        .ok_or_else(not_a_list)?;
    let mut tasks = Vec::with_capacity(listed.len());
    for entry in listed {
        let entry = entry.as_object().ok_or_else(not_a_list)?;
        closed(entry, RECORD_PLAN_TOOL, TASK_FIELDS)?;
        tasks.push(task(entry)?);
    }
    Ok(PlanChange::Recorded { approach, tasks })
}

fn task(arguments: &Map<String, Value>) -> Result<NewTask, NotAnArgument> {
    NewTask::new(&filled(arguments, "title")?, &text(arguments, "detail")?)
        .ok_or(NotAnArgument::Blank { field: "title" })
}

/// `after` is required and may be empty, which is the end of the list.
fn added(arguments: &Map<String, Value>) -> Result<PlanChange, NotAnArgument> {
    closed(arguments, ADD_TASK_TOOL, ADD_TASK_FIELDS)?;
    let task = task(arguments)?;
    let after = match text(arguments, "after")?.trim() {
        "" => None,
        named => Some(task_id("after", named)?),
    };
    Ok(PlanChange::Added { task, after })
}

fn updated(arguments: &Map<String, Value>) -> Result<PlanChange, NotAnArgument> {
    closed(arguments, UPDATE_TASK_TOOL, UPDATE_TASK_FIELDS)?;
    let task = task_id("task", &text(arguments, "task")?)?;
    let to = TaskUpdate::read(&text(arguments, "state")?, &text(arguments, "reason")?)
        .map_err(|why| NotAnArgument::Planning(PlanArgument::NotAnUpdate(why)))?;
    Ok(PlanChange::Updated { task, to })
}

fn task_id(field: &'static str, named: &str) -> Result<TaskId, NotAnArgument> {
    TaskId::read(named.trim()).ok_or_else(|| {
        NotAnArgument::Planning(PlanArgument::NotATaskId {
            field,
            named: named.to_string(),
        })
    })
}

/// **The description says it is not the report**, because a Drone that read a
/// recorded plan as a finished step would stop before submitting.
pub(super) fn record_plan_tool() -> Value {
    json!({
        "name": RECORD_PLAN_TOOL,
        "description":
            "Record the plan for this Job: the approach in a paragraph, and the \
             tasks it breaks into in the order they will be done. Fleet keeps it, \
             and the parts after this one work from it and keep each task's state \
             current. Tasks are named T1, T2 and so on in the order you give them. \
             Calling it again replaces the whole plan, so correct a plan by \
             recording it again. It does not finish this part — submit_evidence \
             still does.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "approach": {
                    "type": "string",
                    "description": "How the work will be done, in a paragraph a person can read.",
                },
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "One line saying what the task is.",
                            },
                            "detail": {
                                "type": "string",
                                "description": "Where or how, if a line is not enough. \"\" if not.",
                            },
                        },
                        "required": ["title", "detail"],
                        "additionalProperties": false,
                    },
                    "description": "The tasks, in the order they will be done.",
                },
            },
            "required": ["approach", "tasks"],
            "additionalProperties": false,
        },
    })
}

pub(super) fn add_task_tool() -> Value {
    json!({
        "name": ADD_TASK_TOOL,
        "description":
            "Add a task to the plan you are working from, when the work turns out \
             to need one the plan does not name. It is added open, after the task \
             you name or at the end, and the call answers with its id.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "One line saying what the task is." },
                "detail": {
                    "type": "string",
                    "description": "Where or how, if a line is not enough. \"\" if not.",
                },
                "after": {
                    "type": "string",
                    "description": "The id of the task it comes after, such as T2. \"\" for the end.",
                },
            },
            "required": ["title", "detail", "after"],
            "additionalProperties": false,
        },
    })
}

/// **The description says a state decides nothing**: the diff is still what is
/// checked, and a Drone that believed otherwise would mark its way to a pass.
pub(super) fn update_task_tool() -> Value {
    json!({
        "name": UPDATE_TASK_TOOL,
        "description":
            "Say where one task of the plan stands: working when you start it, done \
             when it is done, open to put it back, or dropped with a reason when it \
             will not be done. A task's state is your account of the work. It \
             passes nothing and advances nothing; the work itself is what is \
             checked.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "The task's id, such as T3." },
                "state": {
                    "type": "string",
                    "enum": ["open", "working", "done", "dropped"],
                },
                "reason": {
                    "type": "string",
                    "description": "Why the task is dropped. Required for dropped, \"\" otherwise.",
                },
            },
            "required": ["task", "state", "reason"],
            "additionalProperties": false,
        },
    })
}
