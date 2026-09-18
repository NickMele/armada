//! Plan's apparatus: a Bug Job whose frozen workflow records a plan and then
//! follows it, a Drone's tool call read the way `/mcp` reads it, the history a
//! store would append, and the gate run on the plan step. It asserts nothing.
//!
//! **Through `config`'s own parser**, as #895 makes possible: `plan`,
//! `plan_recorded` and `follows_plan` are read off YAML the way a real
//! `.armada/workflows/` file would be, and `ResolvedWorkflow::resolve` is
//! what checks it against a Manifest — a workflow built by hand would prove
//! nothing about either parser. **Every call is kept as
//! `fleet::work_plan::permitted` would allow it** — that predicate is
//! `pub(crate)`, so `fleet`'s own tests assert it.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model, Vcs, Worktree, WorktreeSpec};
use core_model::{
    Attempt, DeclaredPaths, EvidenceType, Facts, FrozenWorkflow, Job, JobId, JobNumber, ManifestId,
    ModelName, NewJob, PlanAuthor, PlanEntry, PlanRefused, RepoPath, StepEvidence, StepId,
    StepSeed, Timestamp, Title, TopLevelOrigin, Ulid, Urgency, WhenRefused, WorkPlan,
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

/// The shape #894 puts on Bug's own `.armada/workflows/bug.json`: a plan
/// step gated on `plan_recorded`, an implement step that follows the plan
/// and is judged against it, and a handoff that does neither.
///
/// `implement` names `plan.evidence` in `reference_docs`, which is what
/// [`gated_on_implement`] exercises: the plan step's own product reaching a
/// later step's Judge brief with task states, not the Drone's words about it.
pub fn bug_workflow_with_a_plan() -> FrozenWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-bug-with-a-plan.yml"),
        r#"
version: 1
workflow_id: bug-with-a-plan
name: bug
structure: linear
steps:
  - id: plan
    label: "Plan the change"
    evidence: {submitted: {type: plan}}
    mechanical_checks:
      - { type: plan_recorded, min_tasks: 1 }
    delivers: false
    advance_gate: auto
  - id: implement
    label: "Implement"
    follows_plan: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
    evidence_scope:
      context_source: drone_declared
      reference_docs:
        - plan.evidence
    judge_checks:
      - criteria:
          - criterion_id: tasks_match_the_diff
            question: "Does the diff do every task set to done, and does each dropped task's reason hold?"
            on_refusal: refuse
          - criterion_id: the_evidence_accounts_for_itself
            question: "For each task the plan records as done, does what it recorded under `shown` either demonstrate what that task recorded under `expects`, or say why the work proved it another way?"
            on_refusal: refuse
    delivers: false
    advance_gate: auto_if_judge_passes
  - id: handoff
    label: "Hand off"
    delivers: true
    advance_gate: auto
"#,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
        .frozen()
        .clone()
}

/// **`#1006`'s shape: a step keeps its own product and records the plan
/// beside it.** `read` delivers `read.md` — `document`, not `plan` — and
/// declares `records_plan: true` rather than `submitted.type: "plan"`; a
/// following step still names `read.evidence` in `reference_docs`, which is
/// what proves the plan sits beside the step's own product on the record
/// rather than replacing it.
pub fn bug_workflow_with_a_plan_beside_a_product() -> FrozenWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-bug-plan-beside-a-product.yml"),
        r#"
version: 1
workflow_id: bug-with-a-plan-beside-a-product
name: bug
structure: linear
steps:
  - id: read
    label: "Read the code"
    records_plan: true
    evidence: {submitted: {type: document}}
    mechanical_checks:
      - { type: artifact_exists, target: ".armada/artifacts/read.md" }
      - { type: plan_recorded, min_tasks: 1 }
    delivers: false
    advance_gate: auto
  - id: implement
    label: "Implement"
    follows_plan: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
    evidence_scope:
      context_source: drone_declared
      reference_docs:
        - read.evidence
    delivers: false
    advance_gate: auto
"#,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
        .frozen()
        .clone()
}

/// **`revert`'s shape: one step that records the plan and follows it.**
/// Nothing here folds — one step, `records_plan: true` and `follows_plan:
/// true` together, which `#1006` made legal. There is no second step to put
/// either half on.
pub const REVERT_SHAPED: &str = "revert_shaped";

pub fn revert_shaped_workflow() -> FrozenWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-revert-shaped.yml"),
        r#"
version: 1
workflow_id: revert-shaped
name: revert-shaped
structure: linear
steps:
  - id: revert_shaped
    label: "Undo the change"
    records_plan: true
    follows_plan: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
      - { type: plan_recorded }
    delivers: true
    advance_gate: auto
"#,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    config::ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
        .frozen()
        .clone()
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
        Planned::created_with(title, bug_workflow_with_a_plan())
    }

    /// The same Job, on a workflow of the caller's choosing — `#1006`'s
    /// beside-a-product fixture is the one caller so far that is not
    /// [`bug_workflow_with_a_plan`].
    pub fn created_with(title: &str, workflow: FrozenWorkflow) -> Planned {
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

    /// What the plan as it stands says to one more call, **without keeping it** —
    /// `store` judges a change inside the write, and a refused one writes nothing.
    pub fn judged(
        &self,
        call: PlanCall,
        step: &str,
        attempt: u32,
    ) -> Result<WorkPlan, PlanRefused> {
        let entry = PlanEntry {
            change: call.change,
            by: PlanAuthor::Step {
                step_id: StepId::new(step),
                attempt: Attempt::stored(attempt).expect("one-based"),
            },
            at: at(self.history.len() + 1),
        };
        WorkPlan::after(self.plan().as_ref(), &entry)
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

/// A Judge that should never be asked, for a step whose gate carries no
/// `judge_checks` — every case but [`gated_on_implement`].
fn no_judge() -> Judging {
    Judging {
        client: Arc::new(FakeJudge::that_fails("a Judge that should never be asked")),
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        second_opinion_model: Model::named("the-second-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: Marking::detached(),
        asked: Asked::nowhere(),
    }
}

/// The plan step's gate, handed the plan `fleet::settling` would read off the
/// store.
pub async fn gated_on_the_plan(planned: &Planned, plan: Option<WorkPlan>) -> Ruling {
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
    rule_on(
        at,
        Request::of(&planned.job),
        &submitted,
        None,
        &Lifted::of(&planned.job),
        fleet::Began::At(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::untouched(),
        CheckBudget::of(Duration::from_secs(5)),
        &fleet::Room::ignoring_the_machine(fleet::ChecksAtOnce::of(4)),
        &no_judge(),
        &Keeping::of(REPO_ROOT, &planned.job.handle()),
        Policies::unstated(),
        &fleet::Announcing::nowhere(),
        &BTreeMap::new(),
        &[],
        WhenRefused::default(),
        &[],
        plan.as_ref(),
        None,
    )
    .await
}

/// Implement's gate, handed the recorded evidence a later step's
/// `reference_docs` reaches through `AtStep::baseline` — the plan step's row
/// replaced by the plan as it stands, exactly as `fleet::work_plan::with_the_plan`
/// replaces it before a real gate runs. `judge` is what answers
/// `tasks_match_the_diff`; its own `.asked()` is what a test reads the brief
/// back from.
pub async fn gated_on_implement(
    planned: &Planned,
    plan: WorkPlan,
    judge: Arc<FakeJudge>,
) -> Ruling {
    let at = AtStep::named(
        planned.job.workflow(),
        &StepId::new(IMPLEMENT),
        &planned.worktree,
    )
    .expect("the implement step");
    let submitted = Submission::submitted(
        EvidenceType::Diff,
        Claimed("Two tasks are done and one is dropped."),
        ShownBy("the diff"),
        NotClaimed(""),
    )
    .expect("a well-formed submission");
    let recorded = vec![(
        StepId::new(PLAN),
        StepEvidence {
            evidence_type: EvidenceType::Plan,
            claimed: plan.rendered(),
            shown_by: String::from("Fleet's own record of the plan"),
            not_claimed: String::new(),
        },
    )];
    let judging = Judging {
        client: judge,
        ..no_judge()
    };
    let declared = DeclaredPaths::of(vec![RepoPath::new("src/log.rs")]);
    rule_on(
        at,
        Request::of(&planned.job),
        &submitted,
        Some(&declared),
        &Lifted::of(&planned.job),
        fleet::Began::At(&Footprint::nothing()),
        &recorded,
        &FakeWorkProduct::changed(&["src/log.rs"]),
        CheckBudget::of(Duration::from_secs(5)),
        &fleet::Room::ignoring_the_machine(fleet::ChecksAtOnce::of(4)),
        &judging,
        &Keeping::of(REPO_ROOT, &planned.job.handle()),
        Policies::unstated(),
        &fleet::Announcing::nowhere(),
        &BTreeMap::new(),
        &[],
        WhenRefused::default(),
        &[],
        None,
        None,
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
