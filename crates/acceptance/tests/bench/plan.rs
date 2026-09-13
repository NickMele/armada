//! Plan's apparatus: a Bug Job whose frozen workflow records a plan and then
//! follows it, a Drone's tool call read the way `/mcp` reads it, the history a
//! store would append, and the gate run on the plan step. It asserts nothing.
//!
//! **The workflow is built frozen**, because `config` cannot parse `plan`,
//! `plan_recorded` or `follows_plan` until #895. **Every call is kept as
//! `fleet::work_plan::permitted` would allow it** — that predicate is
//! `pub(crate)`, so `fleet`'s own tests assert it.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model, Vcs, Worktree, WorktreeSpec};
use core_model::{
    AdvanceGate, Attempt, EvidenceType, Facts, FrozenWorkflow, Job, JobId, JobNumber, ManifestId,
    ModelName, NewJob, PlanAuthor, PlanEntry, ResolvedCheck, ResolvedStep, StepId, StepSeed,
    TaskCounts, Timestamp, Title, TopLevelOrigin, Ulid, Urgency, WhenRefused, WorkPlan, WorkflowId,
};
use fleet::{
    rule_on, Asked, AtStep, CheckBudget, JudgeBudget, Judging, Keeping, Marking, Policies, Ruling,
};
use ipc::mcp::{answer, read, Answered, Incoming, NotAnArgument, NotRecorded, PlanCall};
use ipc::{Event, JobDetail, JobSummary};
use testkit::{FakeJudge, FakeVcs, FakeWorkProduct};
use verification::{Claimed, Lifted, NotClaimed, Request, ShownBy, Submission};

use super::board::detail;
use super::{criteria, REPO_ROOT};

pub const PLAN: &str = "plan";
pub const IMPLEMENT: &str = "implement";

/// Bug's shape once #895 switches it: a plan step gated on `plan_recorded`, an
/// implement step that follows the plan, and a handoff that does neither.
pub fn bug_workflow_with_a_plan() -> FrozenWorkflow {
    let plan_recorded = ResolvedCheck::PlanRecorded {
        min_tasks: NonZeroU32::MIN,
    };
    FrozenWorkflow::frozen(
        WorkflowId::carried(Ulid::carried("01BUGWITHAPLAN")),
        "bug".to_string(),
        1,
        vec![
            step(
                PLAN,
                "Plan the change",
                Some(EvidenceType::Plan),
                vec![plan_recorded],
            ),
            step(
                IMPLEMENT,
                "Implement",
                Some(EvidenceType::Diff),
                vec![ResolvedCheck::DiffNonempty],
            )
            .following_plan(true),
            step("handoff", "Hand off", None, Vec::new()).delivering(true),
        ],
    )
}

fn step(
    id: &str,
    label: &str,
    made: Option<EvidenceType>,
    checks: Vec<ResolvedCheck>,
) -> ResolvedStep {
    ResolvedStep::frozen(
        StepId::new(id),
        label.to_string(),
        made,
        checks,
        AdvanceGate::Auto,
        Vec::new(),
        None,
        0,
        None,
    )
}

fn at(second: usize) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-13T10:00:{second:02}.000Z"))
}

/// A Job, its worktree, and the history of its plan — which `store` holds for
/// a real Job, and a list holds here.
pub struct Planned {
    pub job: Job,
    pub worktree: Worktree,
    history: Vec<PlanEntry>,
}

impl Planned {
    pub fn created(title: &str) -> Planned {
        let workflow = bug_workflow_with_a_plan();
        let id = JobId::carried(Ulid::carried(format!("01PLAN{:020}", 1)));
        let spec = WorktreeSpec::for_job(REPO_ROOT, id.as_str()).expect("a legal spec");
        let worktree = FakeVcs::new().create_worktree(&spec).expect("a worktree");
        let steps = workflow
            .steps()
            .iter()
            .enumerate()
            .map(|(ordinal, step)| StepSeed {
                step_id: step.id().clone(),
                ordinal: ordinal as u32,
            })
            .collect();
        let new = NewJob {
            id,
            title: Title::new(title).expect("a title"),
            workflow,
            owner_manifest_id: ManifestId::carried(Ulid::carried("01FIXTUREMANIFEST")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("a-model").expect("a model name"),
            acceptance_criteria: criteria(),
            steps,
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            number: JobNumber::carried(1),
            proposal_id: None,
            facts: Facts::new("the store's cursor reads one row past the end"),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        };
        Planned {
            job: Job::create_top_level(new, TopLevelOrigin::Manual, at(0)),
            worktree,
            history: Vec::new(),
        }
    }

    /// Keep one call as `store` appends it, and answer with the plan it leaves.
    pub fn kept(&mut self, call: PlanCall, step: &str, attempt: u32) -> WorkPlan {
        self.history.push(PlanEntry {
            change: call.change,
            by: PlanAuthor::Step {
                step_id: StepId::new(step),
                attempt: Attempt::stored(attempt).expect("one-based"),
            },
            at: at(self.history.len() + 1),
        });
        self.plan().expect("a plan")
    }

    pub fn plan(&self) -> Option<WorkPlan> {
        WorkPlan::fold(&self.history).expect("the history replays")
    }

    /// `get_job`'s answer, filled the way `fleet::serving` fills the plan in.
    pub fn detail(&self) -> JobDetail {
        let mut opened = detail(&self.job, None, &[]);
        let plan = self.plan();
        opened.job.tasks = plan.as_ref().map(|plan| plan.counts().into());
        opened.work_plan = plan.as_ref().map(ipc::WorkPlan::from);
        opened
    }

    /// The Board's row, with the counts `fleet::summarising` fills in.
    pub fn row(&self) -> JobSummary {
        let mut row = JobSummary::from(&self.job);
        row.tasks = self.plan().map(|plan| plan.counts().into());
        row
    }
}

fn call_body(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

/// A Drone's call, read as `/mcp` reads it. Panics where it would be refused,
/// which is [`refused`]'s case.
pub fn called(tool: &str, arguments: &str) -> PlanCall {
    match read(call_body(tool, arguments).as_bytes()) {
        Incoming::Plan { call, .. } => call,
        other => panic!("`{tool}` did not read as a plan call: {other:?}"),
    }
}

/// A call that did not read as a change, and the tool error the Drone is sent —
/// answered the way `api::mcp` answers it.
pub fn refused(tool: &str, arguments: &str) -> (NotAnArgument, String) {
    let Incoming::NotASubmission { id, why } = read(call_body(tool, arguments).as_bytes()) else {
        panic!("`{tool}` was taken");
    };
    let because = why.to_string();
    let sent = answer(Answered::Refused {
        id,
        why: NotRecorded { because },
    })
    .expect("plain data");
    (why, sent)
}

/// The plan step's gate, handed the counts `fleet::settling` reads off the store.
pub async fn gated_on_the_plan(planned: &Planned, plan: Option<TaskCounts>) -> Ruling {
    let at = AtStep::named(
        planned.job.workflow(),
        &StepId::new(PLAN),
        &planned.worktree,
    )
    .expect("the plan step");
    let submitted = Submission::submitted(
        EvidenceType::Plan,
        Claimed("The fix is planned as four tasks."),
        ShownBy("the plan recorded with record_plan"),
        NotClaimed(""),
    )
    .expect("a well-formed submission");
    let judging = Judging {
        client: Arc::new(FakeJudge::that_fails("a Judge that should never be asked")),
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: Marking::detached(),
        asked: Asked::nowhere(),
    };
    rule_on(
        at,
        Request::of(&planned.job),
        &submitted,
        None,
        &Lifted::of(&planned.job),
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::untouched(),
        CheckBudget::of(Duration::from_secs(5)),
        &judging,
        &Keeping::of(REPO_ROOT, &planned.job.handle()),
        Policies::unstated(),
        &fleet::Announcing::nowhere(),
        &BTreeMap::new(),
        &[],
        WhenRefused::default(),
        &[],
        plan,
    )
    .await
}

/// The row as a Board receives it.
pub fn received_row(row: &JobSummary) -> JobSummary {
    let body = ipc::encode(row).expect("a row that serialises");
    ipc::decode("a Board row", body.as_bytes()).expect("a row that reads back")
}

/// The event as a connected client receives it.
pub fn received_event(event: &Event) -> Event {
    let body = ipc::encode(event).expect("an event that serialises");
    ipc::decode("an event", body.as_bytes()).expect("an event that reads back")
}
