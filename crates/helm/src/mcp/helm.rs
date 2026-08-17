//! **The orchestrator's toolbelt**: the Fleet verbs and the Manifest verbs.
//!
//! Its job is decompose, delegate, aggregate, report (PLAN.md §15.1), and every
//! tool here is one of those four reaching a verb that already exists. Nothing
//! in this file decides anything — a handler parses its arguments, calls
//! [`crate::verbs`], and hands the envelope to [`super::answer`]
//! (`ARCHITECTURE.md` §1.3). If a tool reimplemented a verb the two would
//! disagree the first time either changed, which is the whole reason that rule
//! exists.
//!
//! **This type is not the Drone's, and that is the enforcement.** [`TOOLS`] and
//! [`super::drone::TOOLS`] are two lists on two types with two routers; there is
//! no shared list and no filter, so `fleet.spawn` cannot leak onto a worker's
//! belt by an omission (`docs/commands/helm/mcp.md`).
//!
//! **Every Manifest tool takes an optional `path` and resolves a workspace per
//! call.** The server is started once, from wherever the thing that registered
//! it happened to be, and the orchestrator then works across several worktrees —
//! so a workspace resolved at startup would be the wrong one for most requests.
//! That is the same statelessness the base protocol has: the request carries
//! what it needs.

use armada_core::ctx::Ctx;
use armada_core::error::{ArmadaError, ErrClass};
use armada_core::lifecycle::Direction;
use armada_core::scope::Lens;
use armada_manifest::clock::SystemClock;
use armada_manifest::discovery;
use armada_manifest::net::RealFetch;
use armada_manifest::process::RealRun;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, CancelTaskParams, CreateTaskResult,
    GetTaskParams, GetTaskResult, ServerCapabilities, ServerInfo, UpdateTaskParams,
};
use rmcp::service::RequestContext;
use rmcp::task_manager::{TaskExit, TaskManager, TaskOptions};
use rmcp::{tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler};
use serde::Deserialize;
use std::path::PathBuf;

use super::answer;
use super::world::World;
use crate::app::App;
use crate::args::{Check, Common, Spawn};
use crate::render::progress::Silent;
use crate::verbs::{self, fleet, Output};

/// Every tool the orchestrator is offered, in the order
/// [`docs/commands/helm/mcp.md`](../../../../docs/commands/helm/mcp.md) lists
/// them.
///
/// **`manifest.explain` is documented and is not here**, because
/// `armada manifest explain` is not built — the parser answers "not built yet"
/// for it. A tool wrapping a verb that does not exist would have to reimplement
/// it, which is the one thing this module may not do; it lands when the verb
/// does.
pub const TOOLS: [&str; 12] = [
    "fleet.spawn",
    "fleet.status",
    "fleet.probe",
    "fleet.answer",
    "fleet.kill",
    "manifest.check",
    "manifest.up",
    "manifest.down",
    "manifest.status",
    "manifest.clean",
    "manifest.skills",
    "manifest.skill",
];

/// The tools that can outlast a request, and therefore run as tasks.
///
/// **`manifest.check` is the one this was built for.** A real run *"runs well
/// past ten minutes"* (`commands/helm/mcp.md`), which is longer than any client
/// will hold a `tools/call` open — and a bespoke "start it and poll me" pair of
/// tools is exactly what the Tasks extension exists to stop anyone inventing.
///
/// **`fleet.spawn` is here for a different reason and it is not "it is slow".**
/// It classifies with a model call and then runs `armada manifest init` inside a
/// fresh worktree, which is tens of seconds of somebody else's timeouts. It
/// returns without waiting for the Drone, so what a task buys is a handle for
/// the preparation rather than for the work.
///
/// **Everything else answers inline**, because a task handle for a read that
/// takes forty milliseconds is two round trips to save none.
pub const SLOW: [&str; 2] = ["manifest.check", "fleet.spawn"];

/// How long a finished task's result is kept for collection.
///
/// **An hour, against the extension's five-minute default.** The caller of a
/// forty-minute `manifest.check` may not be looking when it lands — that is the
/// whole reason it is a task — and a result swept before anybody read it turns a
/// long run into a wasted one.
pub const TASK_TTL_MS: u64 = 3_600_000;

/// How often the client is told to ask again.
pub const TASK_POLL_MS: u64 = 5_000;

/// What the orchestrator holds between requests.
///
/// **The world, which the entrypoint read, and the tasks it has in flight.**
/// The second is not session state in the sense the base protocol dropped: a
/// task is a *durable handle the client was given*, named in the answer and
/// polled by id, which is the extension's whole shape. Nothing here is implicit
/// context that a request would have to have established earlier.
#[derive(Clone)]
pub struct Toolbelt {
    world: World,
    tasks: TaskManager,
}

impl Toolbelt {
    /// The belt, over a world.
    pub fn new(world: World) -> Toolbelt {
        Toolbelt {
            world,
            tasks: TaskManager::new(),
        }
    }
}

// --------------------------------------------------------------- arguments
//
// **One struct per tool, and the doc comment on each field is what the model
// reads.** `schemars` puts it into the tool's `inputSchema`, so a field
// described here is described in the tool list — which is why these read as
// instructions rather than as notes to a maintainer.

/// `fleet.spawn`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct SpawnArgs {
    /// What the Job should do, in full. It becomes the Drone's prompt and is
    /// what classification reads, so a sentence beats three words.
    pub task: String,
    /// Name the workflow instead of letting classification pick: `design`,
    /// `plan`, `feature` or `bug`.
    #[serde(default)]
    pub workflow: Option<String>,
    /// The Job's handle. Derived from the task when absent.
    #[serde(default)]
    pub name: Option<String>,
    /// Ceiling overrides, as `key=value` — `max_iterations=6`,
    /// `max_wall_clock=20m`.
    #[serde(default)]
    pub budget: Vec<String>,
    /// Which repository to branch from. Defaults to where the server started.
    #[serde(default)]
    pub at: Option<String>,
    /// Below this confidence, refuse rather than spawn. Defaults to Armada's
    /// own threshold; `0` never refuses.
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Report the classification, the worktree, the port block and the budget,
    /// and start nothing.
    #[serde(default)]
    pub dry_run: bool,
    /// What this task's `${task.<key>}` placeholders mean, as `key=value`.
    ///
    /// The shipped `bug` workflow gates its first step on `test: ${task.test}`,
    /// so a bug Job that should run to completion without stopping to ask wants
    /// `set: ["test=<the test you want written>"]`.
    #[serde(default)]
    pub set: Vec<String>,
}

/// `fleet.status`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct StatusArgs {
    /// Include Jobs that are over. Off by default, because the question is
    /// almost always about what is running.
    #[serde(default)]
    pub all: bool,
    /// Only the Jobs waiting on a person.
    #[serde(default)]
    pub needs_attention: bool,
}

/// `fleet.probe`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct ProbeArgs {
    /// The Job, by name or by uuid.
    pub job: String,
}

/// `fleet.answer`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct AnswerArgs {
    /// The Job, by name or by uuid.
    pub job: String,
    /// The answer. It resumes the Job's session as a new turn.
    pub answer: String,
}

/// `fleet.kill`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct KillArgs {
    /// The Job, by name or by uuid. Omit to end every Job that is over.
    #[serde(default)]
    pub job: Option<String>,
    /// Leave the branch behind.
    #[serde(default)]
    pub keep_branch: bool,
    /// Leave the worktree behind.
    #[serde(default)]
    pub keep_worktree: bool,
}

/// The argument every Manifest tool takes.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceArgs {
    /// A directory inside the workspace to act on. Defaults to where the server
    /// started — name a Job's worktree to act on that Job's checkout.
    #[serde(default)]
    pub path: Option<String>,
}

/// `manifest.check`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct CheckArgs {
    /// A directory inside the workspace to check.
    #[serde(default)]
    pub path: Option<String>,
    /// Which checks, as `component:check` with `*` allowed — `api:*`,
    /// `*:lint`. Omit for every check.
    #[serde(default)]
    pub selector: Option<String>,
    /// Only this component's checks.
    #[serde(default)]
    pub component: Option<String>,
    /// Limit the checks to these files, for the checks that take a file list.
    #[serde(default)]
    pub files: Vec<String>,
    /// Run the fixing form of every check that has one.
    #[serde(default)]
    pub fix: bool,
    /// Report the plan and run nothing.
    #[serde(default)]
    pub dry_run: bool,
}

/// `manifest.up` and `manifest.down`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct ServicesArgs {
    /// A directory inside the workspace.
    #[serde(default)]
    pub path: Option<String>,
    /// One component, or omit for all of them.
    #[serde(default)]
    pub component: Option<String>,
    /// Report the argv and the ready-checks, and change nothing.
    #[serde(default)]
    pub dry_run: bool,
}

/// `manifest.status`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceStatusArgs {
    /// A directory inside the workspace.
    #[serde(default)]
    pub path: Option<String>,
    /// Every workspace of this repository, not just this checkout.
    #[serde(default)]
    pub project: bool,
    /// Every workspace on the machine. **The one lens that needs no workspace**,
    /// so it answers from anywhere.
    #[serde(default)]
    pub all: bool,
}

/// `manifest.clean`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct CleanArgs {
    /// A directory inside the workspace.
    #[serde(default)]
    pub path: Option<String>,
    /// Every workspace of this repository.
    #[serde(default)]
    pub project: bool,
    /// Every workspace on the machine.
    #[serde(default)]
    pub all: bool,
    /// Also remove the declared `owns.files`.
    #[serde(default)]
    pub artifacts: bool,
    /// Only workspaces whose directory no longer exists.
    #[serde(default)]
    pub orphaned: bool,
    /// Report what would be released and release nothing.
    #[serde(default)]
    pub dry_run: bool,
}

/// `manifest.skill`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct SkillArgs {
    /// A directory inside the workspace.
    #[serde(default)]
    pub path: Option<String>,
    /// Which skill.
    pub name: String,
}

// ------------------------------------------------------------------- tools

#[tool_router]
impl Toolbelt {
    /// `armada fleet spawn`.
    #[tool(
        name = "fleet.spawn",
        description = "Start a Job: classify the task, cut a worktree, claim its ports, and \
                       start a Drone on it. Returns as soon as the Drone is running — it does \
                       not wait for the work. Name a workflow only when you are sure; \
                       classification is cheap and is the default."
    )]
    async fn fleet_spawn(&self, Parameters(args): Parameters<SpawnArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("fleet spawn", move || {
            fleet::spawn(
                &RealRun,
                &SystemClock,
                &world.place(),
                &Spawn {
                    json: true,
                    task: args.task,
                    workflow: args.workflow,
                    name: args.name,
                    budget: args.budget,
                    at: args.at,
                    confidence: args.confidence,
                    dry_run: args.dry_run,
                    set: args
                        .set
                        .iter()
                        .filter_map(|pair| pair.split_once('='))
                        .map(|(key, value)| (key.trim().to_string(), value.to_string()))
                        .collect(),
                },
                // **Nobody to ask, and that is the correct answer here.** A
                // low-confidence spawn refuses instead of proceeding, which is
                // PLAN.md §15.4's "confirm when confidence is low" arriving as
                // something an orchestrator can act on: it names the workflow
                // and calls again. Prompting would mean an elicitation round
                // trip on the path of every spawn, to ask a question the caller
                // is better placed to answer than its user is.
                None,
                // **Silent, because stdout is the transport.** A live table
                // drawn here would corrupt the JSON-RPC stream, and there is no
                // terminal on the other end of an MCP call to draw one on.
                &mut Silent,
            )
        })
        .await
    }

    /// `armada fleet ls`, as structured data.
    #[tool(
        name = "fleet.status",
        description = "What every Job is doing, how long it has run, what it has spent, and \
                       which ones are waiting on a person. Read-only."
    )]
    async fn fleet_status(&self, Parameters(args): Parameters<StatusArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("fleet ls", move || {
            fleet::ls(
                &RealRun,
                &SystemClock,
                &world.place(),
                args.all,
                args.needs_attention,
            )
        })
        .await
    }

    /// One Job's transcript, summarised.
    #[tool(
        name = "fleet.probe",
        description = "Summarise one Job's transcript with a cheap model: what it is working \
                       on, what it has finished, and anything that looks stuck. Read-only — it \
                       never messages, resumes or interrupts the Drone, because asking a busy \
                       agent how it is going costs you the thing you were measuring."
    )]
    async fn fleet_probe(&self, Parameters(args): Parameters<ProbeArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("fleet probe", move || {
            fleet::probe(&RealRun, &world.place(), &args.job)
        })
        .await
    }

    /// `armada fleet answer`.
    #[tool(
        name = "fleet.answer",
        description = "Answer a Job that raised a question, which resumes it as a new turn. \
                       The budget is not reset: an answer is a continuation, not a new run."
    )]
    async fn fleet_answer(&self, Parameters(args): Parameters<AnswerArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("fleet answer", move || {
            fleet::answer(
                &RealRun,
                &SystemClock,
                &world.place(),
                &args.job,
                &args.answer,
            )
        })
        .await
    }

    /// `armada fleet kill`.
    #[tool(
        name = "fleet.kill",
        description = "End a Job: stop its Drone, release its ports, containers and processes, \
                       then drop its worktree and branch. The transcript is kept."
    )]
    async fn fleet_kill(&self, Parameters(args): Parameters<KillArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("fleet kill", move || {
            fleet::kill(
                &RealRun,
                &SystemClock,
                &world.place(),
                args.job.as_deref(),
                args.keep_branch,
                args.keep_worktree,
            )
        })
        .await
    }

    /// `armada manifest check`.
    #[tool(
        name = "manifest.check",
        description = "Run this workspace's checks and report each one's verdict, duration and \
                       log. This is the objective gate: a step verdict of PASS is only \
                       legitimate if it carries exit codes from here. A real run can last well \
                       past ten minutes."
    )]
    async fn manifest_check(&self, Parameters(args): Parameters<CheckArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("check", move || {
            let mut app = app(&world, args.path.as_deref())?;
            verbs::check::run(
                &mut app,
                &Check {
                    json: true,
                    dry_run: args.dry_run,
                    selector: args.selector,
                    component: args.component,
                    files: args.files,
                    all_files: false,
                    fix: args.fix,
                    jobs: None,
                    wait: false,
                    // **The tool runs the check and answers with its verdict.**
                    // `--detach` exists for a caller that has somewhere else to
                    // be; an MCP call already has one turn of an agent waiting
                    // on it, and handing that agent a run id to poll over a
                    // second tool call would be a worse version of the wait it
                    // is already doing.
                    detach: false,
                    status: false,
                    run: None,
                },
                &mut Silent,
            )
        })
        .await
    }

    /// `armada manifest up`.
    #[tool(
        name = "manifest.up",
        description = "Start this workspace's services on its own ports, and wait for each \
                       one's ready-check."
    )]
    async fn manifest_up(&self, Parameters(args): Parameters<ServicesArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("up", move || {
            let mut app = app(&world, args.path.as_deref())?;
            verbs::services::run(
                &mut app,
                Direction::Up,
                args.component.as_deref(),
                args.dry_run,
            )
        })
        .await
    }

    /// `armada manifest down`.
    #[tool(
        name = "manifest.down",
        description = "Stop this workspace's services. The port block is kept, so bringing it \
                       back up lands on the same ports."
    )]
    async fn manifest_down(&self, Parameters(args): Parameters<ServicesArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("down", move || {
            let mut app = app(&world, args.path.as_deref())?;
            verbs::services::run(
                &mut app,
                Direction::Down,
                args.component.as_deref(),
                args.dry_run,
            )
        })
        .await
    }

    /// `armada manifest status`.
    #[tool(
        name = "manifest.status",
        description = "What a workspace holds right now: its port block, what is listening, \
                       what Armada owns for it, and what it has not been able to reclaim. \
                       Read-only."
    )]
    async fn manifest_status(
        &self,
        Parameters(args): Parameters<WorkspaceStatusArgs>,
    ) -> CallToolResult {
        let world = self.world.clone();
        run("status", move || {
            let lens = lens(args.project, args.all)?;
            let mut app = scoped(&world, args.path.as_deref(), lens)?;
            verbs::status::run(
                &mut app,
                Common {
                    json: true,
                    dry_run: false,
                    lens,
                },
            )
        })
        .await
    }

    /// `armada manifest clean`.
    #[tool(
        name = "manifest.clean",
        description = "Release what a workspace holds: containers, volumes, process groups, \
                       ports and leases. Use dry_run first if you are not certain what it owns."
    )]
    async fn manifest_clean(&self, Parameters(args): Parameters<CleanArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("clean", move || {
            let lens = lens(args.project, args.all)?;
            let mut app = scoped(&world, args.path.as_deref(), lens)?;
            verbs::clean::run(
                &mut app,
                Common {
                    json: true,
                    dry_run: args.dry_run,
                    lens,
                },
                verbs::clean::Filters {
                    artifacts: args.artifacts,
                    orphaned: args.orphaned,
                    // **Never forced from here.** `--force` overrides the
                    // liveness guard, which is the guard that stops Armada
                    // reclaiming resources a live run is using — and an agent
                    // that could set it would eventually set it on a workspace
                    // one of its own Jobs was working in.
                    force: false,
                    force_rebuild: false,
                },
            )
        })
        .await
    }

    /// `armada manifest skills`.
    #[tool(
        name = "manifest.skills",
        description = "What this repository knows about itself, as a routing table: each \
                       skill's name and one-line summary, without the documents. Read this to \
                       decide which one a Job needs."
    )]
    async fn manifest_skills(&self, Parameters(args): Parameters<WorkspaceArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("skills", move || {
            let mut app = app(&world, args.path.as_deref())?;
            verbs::skills::run(&mut app, None)
        })
        .await
    }

    /// `armada manifest skills show <name>`.
    #[tool(
        name = "manifest.skill",
        description = "One skill, resolved, including its document. This is the expensive one: \
                       an orchestrator wants manifest.skills and a routing table, and a Drone \
                       wants this."
    )]
    async fn manifest_skill(&self, Parameters(args): Parameters<SkillArgs>) -> CallToolResult {
        let world = self.world.clone();
        run("skills", move || {
            let mut app = app(&world, args.path.as_deref())?;
            verbs::skills::run(&mut app, Some(&args.name))
        })
        .await
    }
}

#[tool_handler]
impl ServerHandler for Toolbelt {
    /// **The tasks extension is advertised by the server rather than by a
    /// tool.** `manifest.check` is on this belt and a real run lasts well past
    /// ten minutes, which is exactly what the extension exists for —
    /// *"asynchronous execution of long-running operations, with polling,
    /// mid-flight input, and durable handles"* — and a client negotiating it per
    /// request gets the same answer whichever tool it then calls.
    /// **Long operations become tasks, and only when the caller can read one.**
    /// SEP-2663 forbids returning a `CreateTaskResult` to a client that did not
    /// declare the extension — it would have no way to parse the handle — so the
    /// same tool answers inline for a client that cannot poll. That is one
    /// implementation with two answers rather than two tools, which is what
    /// keeps `manifest.check` a single name in the tool list.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let can_poll = context
            .client_capabilities()
            .is_some_and(|caps| caps.supports_tasks());
        if !can_poll || !SLOW.contains(&request.name.as_ref()) {
            return Self::tool_router()
                .call(ToolCallContext::new(self, request, context))
                .await;
        }

        let name = request.name.to_string();
        let belt = self.clone();
        let task = self.tasks.spawn(
            TaskOptions::default()
                .with_ttl_ms(TASK_TTL_MS)
                .with_poll_interval_ms(TASK_POLL_MS)
                .with_status_message(format!("`{name}` is running")),
            move |_task| {
                Box::pin(async move {
                    // **The task runs the same router the inline path does.**
                    // A second dispatch here would be a second answer to "what
                    // does this tool do", and the two would drift.
                    let response = Self::tool_router()
                        .call(ToolCallContext::new(&belt, request, context))
                        .await
                        .map_err(TaskExit::Error)?;
                    match response {
                        CallToolResponse::Complete(result) => Ok(result),
                        // No tool here asks the client anything mid-flight, so
                        // this is unreachable rather than unhandled — and it is
                        // written out because a `_ =>` would silently swallow
                        // the day one does.
                        other => Err(TaskExit::Error(ErrorData::internal_error(
                            format!("`{name}` answered a shape tasks cannot carry: {other:?}"),
                            None,
                        ))),
                    }
                })
            },
        );
        Ok(CallToolResponse::Task(CreateTaskResult::new(task)))
    }

    /// `tasks/get` — the state of one operation, and its result once there is
    /// one.
    async fn get_task(
        &self,
        request: GetTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetTaskResult, ErrorData> {
        self.tasks
            .get_task(&request.task_id)
            .map(GetTaskResult::new)
    }

    /// `tasks/update` — mid-flight input, which nothing here asks for yet.
    ///
    /// **Answered rather than refused.** The extension is advertised as a whole,
    /// and a client that sends an update for a task with no outstanding request
    /// gets the spec's behaviour — the keys are ignored — rather than a method
    /// that does not exist.
    async fn update_task(
        &self,
        request: UpdateTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        self.tasks
            .update_task(&request.task_id, request.input_responses)
    }

    /// `tasks/cancel` — cooperative, and it is honest about that.
    ///
    /// It records the intent and acknowledges; it does not abort the verb
    /// underneath. A `manifest check` mid-run owns process groups and a lease,
    /// and killing its future would leak both — the run's own interrupt path is
    /// what ends it properly.
    async fn cancel_task(
        &self,
        request: CancelTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        self.tasks.cancel_task(&request.task_id)
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tasks()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "armada",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Armada's orchestrator toolbelt. Every tool answers the same JSON envelope the \
             CLI does: read `status`, then `error`, then iterate `data.results[]`. \
             `fleet.probe` summarises a Job's transcript and never interrupts it. Decompose, \
             delegate, aggregate, report — do not do the work yourself.",
        )
    }
}

// ------------------------------------------------------------------ plumbing

/// Run one verb off the reactor and render its envelope.
///
/// **`spawn_blocking`, not a direct call.** Every verb underneath is blocking by
/// design — a check is a subprocess, a spawn is a `git worktree add` — and this
/// server runs on a current-thread runtime. Calling one inline would stall the
/// reactor, which is survivable for `fleet.status` and fatal for the tasks
/// extension: `tasks/get` has to answer *while* the operation it describes is
/// still going, and that is the entire point of the extension.
pub async fn run<F>(verb: &str, work: F) -> CallToolResult
where
    F: FnOnce() -> Result<Output, ArmadaError> + Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(outcome) => answer::from(verb, outcome),
        // The worker panicked. There is no envelope to render from a verb that
        // did not return, so one is built for the class that describes it: a
        // panic is Armada's bug and nobody else's.
        Err(joined) => answer::from(
            verb,
            Err(ArmadaError {
                class: ErrClass::ArmadaBug,
                r#where: verb.to_string(),
                message: format!("`{verb}` panicked: {joined}"),
                next_action: Some("report this, with what you asked for".to_string()),
            }),
        ),
    }
}

/// The two lens flags, as PLAN.md §3.3 pairs them.
///
/// **Both together is refused rather than resolved.** `--project` and `--all`
/// are the CLI's spelling of the same choice, and the parser refuses the pair;
/// answering one of them here would make the tool and the command disagree about
/// a line that means nothing.
fn lens(project: bool, all: bool) -> Result<Lens, ArmadaError> {
    Lens::from_flags(project, all).ok_or_else(|| ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: "project".to_string(),
        message: "`project` and `all` are two lenses and only one applies".to_string(),
        next_action: Some("pick one, or neither for this workspace alone".to_string()),
    })
}

/// Build the runtime a Manifest verb needs, for a workspace that must exist.
pub(super) fn app(
    world: &World,
    path: Option<&str>,
) -> Result<App<RealRun, SystemClock, RealFetch>, ArmadaError> {
    scoped(world, path, Lens::Workspace)
}

/// The same, under a lens that may not need a workspace at all.
///
/// **`all` is the one lens that answers from anywhere** — it is about the
/// machine rather than about a checkout, and nothing else on the machine reaps
/// orphaned ports and containers. A rule that resolved a workspace first would
/// fail before the one job only it does. This is `main`'s rule, reached from the
/// other caller (`ARCHITECTURE.md` §1.3).
fn scoped(
    world: &World,
    path: Option<&str>,
    lens: Lens,
) -> Result<App<RealRun, SystemClock, RealFetch>, ArmadaError> {
    let cwd = path.map_or_else(|| world.cwd.clone(), PathBuf::from);
    let workspace = match discovery::resolve(&RealRun, &cwd) {
        Ok(workspace) => Some(workspace),
        Err(_) if lens == Lens::All => None,
        Err(error) => return Err(error),
    };
    let ctx = Ctx {
        workspace,
        run: RealRun,
        now: SystemClock,
        fetch: RealFetch,
    };
    crate::app::build(ctx, &world.home, world.inherited.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The documented toolbelt, asserted against the code that serves it.
    #[test]
    fn every_tool_is_namespaced_by_its_module() {
        for tool in TOOLS {
            assert!(
                tool.starts_with("fleet.") || tool.starts_with("manifest."),
                "`{tool}` is namespaced by nothing, so the tool list has a different shape \
                 from the system"
            );
        }
    }

    /// **Every slow tool is a tool.** A name in [`SLOW`] that no longer exists
    /// would silently stop being run as a task, and the symptom would be a
    /// forty-minute request held open rather than an error.
    #[test]
    fn the_tools_that_become_tasks_are_tools_this_belt_has() {
        for slow in SLOW {
            assert!(TOOLS.contains(&slow), "`{slow}` is not on this belt");
        }
    }

    /// **Both lenses at once is the one combination that means nothing**, and
    /// the tool refuses it exactly as the parser does.
    #[test]
    fn the_two_lenses_are_a_choice_and_not_a_pair() {
        assert_eq!(lens(false, false).unwrap(), Lens::Workspace);
        assert_eq!(lens(true, false).unwrap(), Lens::Project);
        assert_eq!(lens(false, true).unwrap(), Lens::All);
        let error = lens(true, true).unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(error.next_action.is_some());
    }
}
