//! A Job's plan: an approach, and the tasks the steps after it keep current.
//!
//! **Not the declared scope.** `DeclaredPaths` and `store::DeclaredPlan` say
//! where a step's work will be; this says what the work is. The types are
//! `WorkPlan` and `PlanTask` so neither reads as the other.
//!
//! **The history is the record and the list is derived.** Every change is a
//! [`PlanEntry`], appended and never edited, and [`WorkPlan::after`] is the one
//! place a change is applied — when it is appended and when it is replayed —
//! so a refused call and a row that will not replay are the same rule.
//!
//! **A task's state is a Drone's claim.** Nothing here gates a submission on
//! it; the Judge weighs it beside the diff.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write as _;
use core::num::NonZeroU32;

use crate::envelope::Timestamp;
use crate::job::attempt::Attempt;
use crate::job::ids::{RepoPath, StepId};

/// A task's stable name within one plan: `T1`, `T2`, … in the order minted.
///
/// A number rather than a string, so a spelling that is not one cannot be held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(NonZeroU32);

impl TaskId {
    pub fn numbered(number: NonZeroU32) -> TaskId {
        TaskId(number)
    }

    /// `T` and a positive number with no leading zero, or `None`. Case matters:
    /// the id a Drone was handed is the one it names.
    pub fn read(spelling: &str) -> Option<TaskId> {
        let digits = spelling.strip_prefix('T')?;
        if digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse::<NonZeroU32>().ok().map(TaskId)
    }

    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "T{}", self.0)
    }
}

/// Where one task stands, as the last change to it said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Open,
    Working,
    Done,
    Dropped,
}

impl TaskState {
    pub const ALL: &'static [TaskState] = &[
        TaskState::Open,
        TaskState::Working,
        TaskState::Done,
        TaskState::Dropped,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            TaskState::Open => "open",
            TaskState::Working => "working",
            TaskState::Done => "done",
            TaskState::Dropped => "dropped",
        }
    }

    pub fn from_wire(value: &str) -> Option<TaskState> {
        TaskState::ALL
            .iter()
            .copied()
            .find(|s| s.as_wire() == value)
    }
}

/// Why a task was dropped. **Never blank**, so a drop with no reason cannot be
/// constructed, let alone recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropReason(String);

impl DropReason {
    pub fn new(text: &str) -> Option<DropReason> {
        let text = text.trim();
        (!text.is_empty()).then(|| DropReason(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The state a change moves a task to. `Dropped` carries its reason, which is
/// what makes "dropped needs a reason" a type rather than a check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskUpdate {
    Open,
    Working,
    Done,
    Dropped(DropReason),
}

/// Why a state and a reason did not read as an update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnUpdate {
    NoSuchState {
        named: String,
    },
    DroppedWithoutAReason,
    /// A reason given with a state that keeps none. Refused rather than
    /// dropped, because a reason nothing keeps is one the caller believes was.
    ReasonWithoutADrop {
        state: TaskState,
    },
}

impl TaskUpdate {
    /// A state as spelled, and the reason beside it; empty is no reason.
    pub fn read(state: &str, reason: &str) -> Result<TaskUpdate, NotAnUpdate> {
        let named = TaskState::from_wire(state).ok_or_else(|| NotAnUpdate::NoSuchState {
            named: String::from(state),
        })?;
        let given = DropReason::new(reason);
        match (named, given) {
            (TaskState::Dropped, Some(reason)) => Ok(TaskUpdate::Dropped(reason)),
            (TaskState::Dropped, None) => Err(NotAnUpdate::DroppedWithoutAReason),
            (state, Some(_)) => Err(NotAnUpdate::ReasonWithoutADrop { state }),
            (TaskState::Open, None) => Ok(TaskUpdate::Open),
            (TaskState::Working, None) => Ok(TaskUpdate::Working),
            (TaskState::Done, None) => Ok(TaskUpdate::Done),
        }
    }

    pub fn state(&self) -> TaskState {
        match self {
            TaskUpdate::Open => TaskState::Open,
            TaskUpdate::Working => TaskState::Working,
            TaskUpdate::Done => TaskState::Done,
            TaskUpdate::Dropped(_) => TaskState::Dropped,
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            TaskUpdate::Dropped(reason) => Some(reason.as_str()),
            _ => None,
        }
    }
}

/// A task as it is asked for.
///
/// **Four fields, and the note is the only free text.** `scope` and `expects`
/// hold what a planning step used to bury in prose, where the step after it
/// could not act on either: the paths and the artifact that should prove the
/// task. The note is what neither of those can hold — the exact new wording, a
/// gotcha found while planning — and it is one line, deliberately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewTask {
    title: String,
    note: String,
    scope: Vec<RepoPath>,
    expects: String,
}

impl NewTask {
    /// `None` where the title is blank. Every other field may be empty: a task
    /// whose paths are not yet known is a task, and refusing one would push
    /// the planner back into writing them into the note.
    pub fn new(title: &str, note: &str, scope: &[&str], expects: &str) -> Option<NewTask> {
        let title = title.trim();
        (!title.is_empty()).then(|| NewTask {
            title: String::from(title),
            note: String::from(note.trim()),
            scope: scope
                .iter()
                .map(|path| path.trim())
                .filter(|path| !path.is_empty())
                .map(RepoPath::new)
                .collect(),
            expects: String::from(expects.trim()),
        })
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// What the other three fields cannot hold. May be empty.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// The repository-relative paths this task touches, in the order given.
    pub fn scope(&self) -> &[RepoPath] {
        &self.scope
    }

    /// What the planner says should prove the task. May be empty.
    pub fn expects(&self) -> &str {
        &self.expects
    }
}

/// What the work itself says proved a task. **Never blank**, for
/// [`DropReason`]'s reason: an empty one is a caller believing it said
/// something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown(String);

impl Shown {
    pub fn new(text: &str) -> Option<Shown> {
        let text = text.trim();
        (!text.is_empty()).then(|| Shown(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The paragraph a plan opens with. **Never blank**, for [`DropReason`]'s reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Approach(String);

impl Approach {
    pub fn new(text: &str) -> Option<Approach> {
        let text = text.trim();
        (!text.is_empty()).then(|| Approach(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Who made a change: a run of a step, or a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanAuthor {
    Step { step_id: StepId, attempt: Attempt },
    Person,
}

/// One change to a Job's plan, as it is appended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanChange {
    /// A whole plan. **Replaces whatever was there**, which is how a retry of
    /// the recording step starts again rather than appending to its last try.
    Recorded {
        approach: Approach,
        tasks: Vec<NewTask>,
    },
    /// One task, after the one named or at the end.
    Added {
        task: NewTask,
        after: Option<TaskId>,
    },
    /// **`shown` is the other end of `expects`.** The planner wrote what ought
    /// to prove the task; this is what did, written by whoever worked it.
    /// `None` leaves whatever an earlier update recorded, so marking a task
    /// `open` and `done` again does not erase it.
    Updated {
        task: TaskId,
        to: TaskUpdate,
        shown: Option<Shown>,
    },
}

/// One row of a plan's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEntry {
    pub change: PlanChange,
    pub by: PlanAuthor,
    pub at: Timestamp,
}

/// A change the plan as it stands cannot take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanRefused {
    /// A task added or updated on a Job no plan was recorded for.
    NoPlan,
    NoSuchTask {
        named: TaskId,
    },
    /// `after` names a task the plan does not hold.
    NoSuchPlace {
        named: TaskId,
    },
    /// A move out of `dropped`. A dropped task stays dropped; the work comes
    /// back only as a new task, which is a person's add.
    StaysDropped {
        named: TaskId,
    },
}

impl fmt::Display for PlanRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanRefused::NoPlan => out.write_str("no plan has been recorded for this Job"),
            PlanRefused::NoSuchTask { named } => write!(out, "the plan holds no task {named}"),
            PlanRefused::NoSuchPlace { named } => {
                write!(out, "the plan holds no task {named} to add after")
            }
            PlanRefused::StaysDropped { named } => {
                write!(
                    out,
                    "task {named} was dropped, and a dropped task stays dropped"
                )
            }
        }
    }
}

/// A stretch a task was marked `working`, read off the history and never guessed
/// from the work: a Drone that never marks a task leaves none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingWindow {
    pub entered: Timestamp,
    /// `None` while the task is still marked working.
    pub left: Option<Timestamp>,
}

/// One line of the plan, as the history leaves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTask {
    id: TaskId,
    task: NewTask,
    state: TaskUpdate,
    shown: Option<Shown>,
    windows: Vec<WorkingWindow>,
}

impl PlanTask {
    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn title(&self) -> &str {
        self.task.title()
    }

    /// What the other fields cannot hold. Empty where the task has none.
    pub fn note(&self) -> &str {
        self.task.note()
    }

    /// The paths the planner said this task touches.
    pub fn scope(&self) -> &[RepoPath] {
        self.task.scope()
    }

    /// What the planner said should prove it. Empty where none was named.
    pub fn expects(&self) -> &str {
        self.task.expects()
    }

    /// What the work said proved it, where an update carried one.
    pub fn shown(&self) -> Option<&Shown> {
        self.shown.as_ref()
    }

    pub fn state(&self) -> TaskState {
        self.state.state()
    }

    /// Present on a dropped task and on nothing else.
    pub fn reason(&self) -> Option<&str> {
        self.state.reason()
    }

    /// Every stretch it was marked working, oldest first.
    pub fn working_windows(&self) -> &[WorkingWindow] {
        &self.windows
    }
}

/// How many tasks stand where. `done` over [`not_dropped`](Self::not_dropped)
/// is the figure a person reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskCounts {
    pub done: u32,
    pub working: u32,
    pub open: u32,
    pub dropped: u32,
}

impl TaskCounts {
    pub fn not_dropped(&self) -> u32 {
        self.done + self.working + self.open
    }
}

/// A Job's plan as its history leaves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkPlan {
    approach: Approach,
    recorded_by: PlanAuthor,
    recorded_at: Timestamp,
    tasks: Vec<PlanTask>,
}

impl WorkPlan {
    /// The plan a whole history leaves, or `None` where nothing was recorded.
    /// A history holding a change its plan could not take is refused, because
    /// replaying past it would draw a list nobody wrote.
    pub fn fold(history: &[PlanEntry]) -> Result<Option<WorkPlan>, PlanRefused> {
        let mut plan = None;
        for entry in history {
            plan = Some(WorkPlan::after(plan.as_ref(), entry)?);
        }
        Ok(plan)
    }

    /// The plan once one more change is applied to `current`.
    pub fn after(current: Option<&WorkPlan>, entry: &PlanEntry) -> Result<WorkPlan, PlanRefused> {
        if let PlanChange::Recorded { approach, tasks } = &entry.change {
            return Ok(WorkPlan {
                approach: approach.clone(),
                recorded_by: entry.by.clone(),
                recorded_at: entry.at.clone(),
                tasks: tasks
                    .iter()
                    .zip(1u32..)
                    .map(|(task, n)| PlanTask {
                        id: TaskId(NonZeroU32::MIN.saturating_add(n - 1)),
                        task: task.clone(),
                        state: TaskUpdate::Open,
                        shown: None,
                        windows: Vec::new(),
                    })
                    .collect(),
            });
        }
        let mut plan = current.cloned().ok_or(PlanRefused::NoPlan)?;
        match &entry.change {
            PlanChange::Recorded { .. } => unreachable!("answered above"),
            PlanChange::Added { task, after } => {
                let at = match after {
                    None => plan.tasks.len(),
                    Some(named) => plan
                        .position(*named)
                        .map(|at| at + 1)
                        .ok_or(PlanRefused::NoSuchPlace { named: *named })?,
                };
                let next = plan.tasks.iter().map(|t| t.id.number()).max().unwrap_or(0);
                plan.tasks.insert(
                    at,
                    PlanTask {
                        id: TaskId(NonZeroU32::MIN.saturating_add(next)),
                        task: task.clone(),
                        state: TaskUpdate::Open,
                        shown: None,
                        windows: Vec::new(),
                    },
                );
            }
            PlanChange::Updated { task, to, shown } => {
                let at = plan
                    .position(*task)
                    .ok_or(PlanRefused::NoSuchTask { named: *task })?;
                // A new reason on a dropped task is not a move out of it.
                if plan.tasks[at].state() == TaskState::Dropped && to.state() != TaskState::Dropped
                {
                    return Err(PlanRefused::StaysDropped { named: *task });
                }
                let task = &mut plan.tasks[at];
                let was = task.state();
                // Opened by the move into `working`, closed by the move out.
                match (was == TaskState::Working, to.state() == TaskState::Working) {
                    (false, true) => task.windows.push(WorkingWindow {
                        entered: entry.at.clone(),
                        left: None,
                    }),
                    (true, false) => {
                        if let Some(open) = task.windows.last_mut() {
                            open.left = Some(entry.at.clone());
                        }
                    }
                    _ => {}
                }
                task.state = to.clone();
                // Kept rather than replaced, so reopening a task and finishing
                // it again does not lose what the first pass showed.
                if shown.is_some() {
                    task.shown = shown.clone();
                }
            }
        }
        Ok(plan)
    }

    fn position(&self, id: TaskId) -> Option<usize> {
        self.tasks.iter().position(|task| task.id == id)
    }

    pub fn approach(&self) -> &str {
        self.approach.as_str()
    }

    /// Who recorded the plan as it stands — the last whole recording.
    pub fn recorded_by(&self) -> &PlanAuthor {
        &self.recorded_by
    }

    pub fn recorded_at(&self) -> &Timestamp {
        &self.recorded_at
    }

    /// In plan order: recorded order, with each added task where it was put.
    pub fn tasks(&self) -> &[PlanTask] {
        &self.tasks
    }

    pub fn task(&self, id: TaskId) -> Option<&PlanTask> {
        self.tasks.iter().find(|task| task.id == id)
    }

    pub fn counts(&self) -> TaskCounts {
        let mut counts = TaskCounts::default();
        for task in &self.tasks {
            match task.state() {
                TaskState::Open => counts.open += 1,
                TaskState::Working => counts.working += 1,
                TaskState::Done => counts.done += 1,
                TaskState::Dropped => counts.dropped += 1,
            }
        }
        counts
    }

    /// The plan as it stands, in words: the approach, then each task with its
    /// id, title, state, detail and — for a dropped one — the reason.
    ///
    /// **Fleet's own reading of its own record, and never the Drone's.**
    /// `record_plan`, `add_task` and `update_task` are the only three calls a
    /// plan can be built from, so what this renders is what Fleet kept, not an
    /// account of a turn. `#895` is where a step's Judge is handed this — its
    /// own, where its product is a plan, and a later step's through
    /// `reference_docs`.
    ///
    /// **Every field a task carries is rendered.** The step after a planning
    /// one reads this and nothing else of the plan, so a field left out of it
    /// is a field the next Drone re-derives — which is what a title-only
    /// rendering cost, before `scope` and `expects` were fields at all.
    pub fn rendered(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "Approach: {}", self.approach.as_str());
        let _ = writeln!(out);
        let _ = write!(out, "Tasks:");
        for task in &self.tasks {
            let _ = write!(
                out,
                "\n  {} [{}] {}",
                task.id(),
                task.state().as_wire(),
                task.title()
            );
            if let Some(reason) = task.reason() {
                let _ = write!(out, " — dropped: {reason}");
            }
            // Hung under the title and labelled, so a task carrying three of
            // them cannot read as three tasks.
            if !task.note().is_empty() {
                let _ = write!(out, "\n      note: {}", task.note());
            }
            if !task.scope().is_empty() {
                let _ = write!(out, "\n      files:");
                for path in task.scope() {
                    let _ = write!(out, " {}", path.as_str());
                }
            }
            if !task.expects().is_empty() {
                let _ = write!(out, "\n      expects: {}", task.expects());
            }
            if let Some(shown) = task.shown() {
                let _ = write!(out, "\n      shown: {}", shown.as_str());
            }
        }
        out
    }
}
