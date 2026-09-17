//! A Job's plan, as Bridge is served it: whole on `JobDetail::work_plan`,
//! counted on `JobSummary::tasks`, and pointed at by `job.plan_changed`.
//!
//! **`work_plan` and not `plan`**, because [`DeclaredPlan`](crate::DeclaredPlan)
//! is already on this wire and means where a step said its work would be.

use serde::{Deserialize, Serialize};

use crate::enums::{Actor, TaskState};
use crate::ids::{Instant, JobId, StepId};

/// An approach and its tasks, in plan order. What a Drone recorded and every
/// change since, with the history folded away.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPlan {
    pub approach: String,
    /// Who recorded the plan as it stands. A later whole recording replaces it.
    pub recorded_by: ChangedBy,
    pub recorded_at: Instant,
    pub tasks: Vec<PlanTask>,
}

/// Who made a change: a run of a step, or a person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum ChangedBy {
    Step { step_id: StepId, attempt: u32 },
    Person,
}

/// One line of the plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTask {
    /// `T1`, `T2`, … — stable for the life of the plan, never renumbered.
    pub id: String,
    pub title: String,
    /// Left out where the task has none, rather than sent empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// A Drone's claim, or a person's. **It gates nothing.**
    pub state: TaskState,
    /// Present on a dropped task and on nothing else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Each stretch the task was marked `working`, oldest first — what Bridge
    /// places a turn in a task by. **Left out where empty**, which is every
    /// task no change ever marked working. Since 14.5.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub working_windows: Vec<WorkingWindow>,
}

/// From the change that marked a task `working` to the change that moved it
/// out. `left` is absent while it is still marked working.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingWindow {
    pub entered: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<Instant>,
}

/// How many tasks stand where. `done` over `done + working + open` is the
/// figure a person reads; a dropped task is not counted against it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCounts {
    pub done: u32,
    pub working: u32,
    pub open: u32,
    pub dropped: u32,
}

/// A Job's plan changed. **The counts ride along and the plan does not**: a
/// Board redraws its row from these, and an open Job reads `get_job` for the
/// rest — the stream is one bounded channel every Job shares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobPlanChanged {
    pub job_id: JobId,
    pub tasks: TaskCounts,
    /// A Drone's tool call, or a person's act.
    pub actor: Actor,
    pub at: Instant,
}

impl From<core_model::TaskCounts> for TaskCounts {
    fn from(counts: core_model::TaskCounts) -> TaskCounts {
        TaskCounts {
            done: counts.done,
            working: counts.working,
            open: counts.open,
            dropped: counts.dropped,
        }
    }
}

/// A person adds a task to a Job's plan. `#897`.
///
/// **`detail` and `after` may both be `""`.** A task with nothing beyond its
/// title is legal, and `""` for `after` is the end of the list — the same
/// spelling the `add_task` MCP tool takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddTask {
    pub title: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub after: String,
}

/// A person drops a task from a Job's plan, with a reason. `#897`.
///
/// **`reason` is never empty.** A blank one is refused at the Fleet boundary
/// for `Redirection`'s reason: a decoded request is well-formed, and a reason
/// with nothing in it is a value that cannot work — a 422 and not a 400.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropTask {
    pub task: String,
    pub reason: String,
}

impl From<&core_model::PlanAuthor> for ChangedBy {
    fn from(author: &core_model::PlanAuthor) -> ChangedBy {
        match author {
            core_model::PlanAuthor::Step { step_id, attempt } => ChangedBy::Step {
                step_id: step_id.into(),
                attempt: attempt.number(),
            },
            core_model::PlanAuthor::Person => ChangedBy::Person,
        }
    }
}

impl From<&core_model::WorkPlan> for WorkPlan {
    fn from(plan: &core_model::WorkPlan) -> WorkPlan {
        WorkPlan {
            approach: plan.approach().to_string(),
            recorded_by: plan.recorded_by().into(),
            recorded_at: plan.recorded_at().into(),
            tasks: plan
                .tasks()
                .iter()
                .map(|task| PlanTask {
                    id: task.id().to_string(),
                    title: task.title().to_string(),
                    detail: task.detail().to_string(),
                    state: task.state().into(),
                    reason: task.reason().map(str::to_string),
                    working_windows: task
                        .working_windows()
                        .iter()
                        .map(|window| WorkingWindow {
                            entered: (&window.entered).into(),
                            left: window.left.as_ref().map(Instant::from),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
