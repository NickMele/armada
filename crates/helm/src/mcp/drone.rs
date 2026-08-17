//! **A Drone's toolbelt**: report, ask, and emit a verdict.
//!
//! Three tools, and the absence of the fourth is the design.
//! [`docs/commands/helm/mcp.md`](../../../../docs/commands/helm/mcp.md) states
//! it plainly: *"A Drone able to spawn Drones is a fork bomb with a budget."* A
//! Job has a budget, a worktree and a ceiling; a Drone that could mint another
//! Job could mint a hundred, each with its own budget, and the ceiling that was
//! meant to bound the work would bound one branch of a tree.
//!
//! **The enforcement is that this is a different type.** There is no list of all
//! the tools with `spawn` filtered out of it, because a filter is a line
//! somebody eventually moves, and moving it has no visible consequence until a
//! machine is full of Drones. [`Toolbelt`] and [`super::helm::Toolbelt`] have
//! separate routers built from separate `#[tool]` functions, so `fleet.spawn` is
//! not reachable from here — not blocked from here, *absent*.
//!
//! **A Drone acts on its own Job and can name no other.** Every tool takes the
//! handle from `ARMADA_JOB`, which [`armada_fleet::drone::job_env`] set on the
//! process and which is therefore not a parameter the model chooses. A worker
//! able to write another worker's record could rewrite the evidence somebody
//! else's verdict rests on.

use armada_core::error::{ArmadaError, ErrClass};
use armada_core::fleet::{DroneStatus, Subject};
use armada_manifest::clock::SystemClock;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde::Deserialize;

use super::helm::run;
use super::world::World;
use crate::verbs::fleet;

/// Every tool a Drone is offered. **`fleet.spawn` is not among them and there is
/// no path from here to it.**
///
/// **`fleet.propose` is the fourth, added by `docs/reserved/008`**, and it is
/// not a widening of what a Drone may *do*: it writes an inbox entry and changes
/// nothing. The rule the list is built on is unchanged — a Drone acts on its own
/// Job and can name no other.
pub const TOOLS: [&str; 4] = [
    "fleet.report",
    "fleet.ask_human",
    "fleet.propose",
    "fleet.verdict",
];

/// How long `fleet.ask_human` waits before answering "still open".
///
/// **Five minutes, and the entry outlives it.** A Drone that held a request open
/// until its person came back from lunch would be a Drone spending wall clock
/// against its own ceiling to do nothing. The inbox entry stays; the tool can be
/// called again, or the Job can be resumed by `armada fleet answer` whenever the
/// answer arrives.
pub const ASK_WAIT_MS: u64 = 300_000;

/// What a Drone holds between requests: the world, and the Job it is.
#[derive(Clone)]
pub struct Toolbelt {
    world: World,
}

impl Toolbelt {
    /// The belt, over a world.
    pub fn new(world: World) -> Toolbelt {
        Toolbelt { world }
    }

    /// Which Job this process is.
    ///
    /// **It cannot be `None` in practice** — [`super::Belt::decide`] only chose
    /// this belt because `ARMADA_JOB` was set — but the refusal is written
    /// rather than unwrapped, because a panic inside a tool handler is an
    /// envelope the caller never sees.
    fn job(&self) -> Result<String, ArmadaError> {
        self.world
            .job()
            .map(str::to_string)
            .ok_or_else(|| ArmadaError {
                class: ErrClass::Environment,
                r#where: "ARMADA_JOB".to_string(),
                message: "this process is not a Job's, so it has no record to write".to_string(),
                next_action: Some("this toolbelt is served to a Drone only".to_string()),
            })
    }
}

/// `fleet.report`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct ReportArgs {
    /// What was finished, tried, or found. One or two sentences.
    pub body: String,
    /// The step boundary this note crosses, if it crosses one: `entered` when
    /// you begin a step, `attempted` when you have stopped working on it.
    ///
    /// **`completed` and `failed` are refused here**, and the refusal says where
    /// they go: a step is done when its `verify:` predicate holds, which is
    /// the gate's to decide.
    #[serde(default)]
    pub event: Option<String>,
}

/// `fleet.ask_human`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct AskArgs {
    /// The question, in full. It is what a person reads in the inbox, out of
    /// context and possibly hours later.
    pub question: String,
}

/// `fleet.propose`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct ProposeArgs {
    /// What the proposal is about: `manifest` or `guild`.
    pub subject: String,
    /// What you would change, and what you saw that says so. It is read out of
    /// context and possibly days later, so it has to stand on its own.
    pub proposal: String,
}

/// `fleet.verdict`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct VerdictArgs {
    /// Whether you finished your work or hit a blocker: `done` or `stuck`.
    /// The Job will verify that the step is complete and advance or retry
    /// based on the predicate outcome.
    pub status: String,
}

#[tool_router]
impl Toolbelt {
    /// Append progress to this Job's own record.
    #[tool(
        name = "fleet.report",
        description = "Append a progress note to your own Job record, and say which step \
                       boundary it crosses. Report `entered` when you start a step and \
                       `attempted` when you stop working on it — that is what makes the step \
                       you are on, and how long you have been on it, visible to the person \
                       running the fleet. It does not interrupt anyone and does not need an \
                       answer. You cannot report a step `completed`: a step is done when its \
                       workflow predicate holds, which is fleet.verdict's to record."
    )]
    async fn fleet_report(&self, Parameters(args): Parameters<ReportArgs>) -> CallToolResult {
        let world = self.world.clone();
        let job = match self.job() {
            Ok(job) => job,
            Err(error) => return super::answer::from("fleet report", Err(error)),
        };
        run("fleet report", move || {
            fleet::report(
                &SystemClock,
                &world.place(),
                &job,
                &args.body,
                None,
                args.event.as_deref(),
            )
        })
        .await
    }

    /// Raise a question to the person, and wait for an answer.
    #[tool(
        name = "fleet.ask_human",
        description = "Raise a question to the person running this fleet and wait for an answer. \
                       Use it only for a judgement that is genuinely theirs. It returns with the \
                       answer, or with the entry still open if nobody has replied yet — the \
                       question is not lost either way."
    )]
    async fn fleet_ask_human(&self, Parameters(args): Parameters<AskArgs>) -> CallToolResult {
        let world = self.world.clone();
        let job = match self.job() {
            Ok(job) => job,
            Err(error) => return super::answer::from("fleet ask_human", Err(error)),
        };
        run("fleet ask_human", move || {
            fleet::ask_human(
                &SystemClock,
                &world.place(),
                &job,
                &args.question,
                ASK_WAIT_MS,
            )
        })
        .await
    }

    /// Propose a change to the manifest or the guild, and carry on.
    ///
    /// **The description is the load-bearing half of `docs/reserved/008`.**
    /// [`armada_core::skill::BODY`] tells a Drone that noticing is part of the
    /// job; this is what it reads at the moment it reaches for a tool, so it has
    /// to say the two things that decide between this and `fleet.ask_human`: it
    /// does not wait, and it changes nothing.
    #[tool(
        name = "fleet.propose",
        description = "Raise a change you think should be made to this repository's armada.yml \
                       (subject `manifest`) or to the person's own guild (subject `guild`), \
                       because you learned something it does not say. Do not edit armada.yml \
                       yourself. It files one inbox entry with an id and returns at once — it \
                       does not wait for an answer and you are not blocked on it, which is what \
                       makes it different from mcp__armada__fleet_ask_human. A proposal is not a \
                       change: nothing happens until the person decides."
    )]
    async fn fleet_propose(&self, Parameters(args): Parameters<ProposeArgs>) -> CallToolResult {
        let world = self.world.clone();
        let job = match self.job() {
            Ok(job) => job,
            Err(error) => return super::answer::from("fleet propose", Err(error)),
        };
        let subject = match Subject::parse(&args.subject) {
            Some(subject) => subject,
            None => {
                return super::answer::from(
                    "fleet propose",
                    Err(ArmadaError {
                        class: ErrClass::BadInvocation,
                        r#where: "subject".to_string(),
                        message: format!("`{}` is not something to propose about", args.subject),
                        next_action: Some("one of manifest, guild".to_string()),
                    }),
                )
            }
        };
        run("fleet propose", move || {
            fleet::propose(&SystemClock, &world.place(), &job, subject, &args.proposal)
        })
        .await
    }

    /// Report step completion or blockage.
    #[tool(
        name = "fleet.verdict",
        description = "Record whether you finished your work (`done`) or hit a blocker (`stuck`). \
                       The Job will verify that the step is complete and advance or retry based on \
                       the outcome of its predicate."
    )]
    async fn fleet_verdict(&self, Parameters(args): Parameters<VerdictArgs>) -> CallToolResult {
        let world = self.world.clone();
        let job = match self.job() {
            Ok(job) => job,
            Err(error) => return super::answer::from("fleet verdict", Err(error)),
        };
        let status = match word_to_status(&args.status) {
            Ok(status) => status,
            Err(error) => return super::answer::from("fleet verdict", Err(error)),
        };
        run("fleet verdict", move || {
            fleet::verdict(&SystemClock, &world.place(), &job, status)
        })
        .await
    }
}

#[tool_handler]
impl ServerHandler for Toolbelt {
    /// **No tasks extension on this belt.** Its three tools are a file append,
    /// an inbox write and a bounded wait; none of them is the ten-minute
    /// operation the extension exists for, and advertising a capability nothing
    /// here uses would be a claim a client could act on.
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "armada",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Armada's Drone toolbelt. You may report progress, ask the human a question, \
                 propose a change to the manifest or the guild, and record a step verdict — all \
                 against your own Job and no other. You cannot spawn work; if the task needs \
                 decomposing, say so with mcp__armada__fleet_ask_human. Do not edit armada.yml \
                 to match something you learned: propose it.",
            )
    }
}

/// Parse the status word a caller wrote.
///
/// **Case-insensitive** — a model writing `done` means `Done`.
/// A word outside the two is refused rather than guessed at.
fn word_to_status(word: &str) -> Result<DroneStatus, ArmadaError> {
    match word.trim().to_ascii_lowercase().as_str() {
        "done" => Ok(DroneStatus::Done),
        "stuck" => Ok(DroneStatus::Stuck),
        other => Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "status".to_string(),
            message: format!("`{other}` is not a valid status"),
            next_action: Some("one of done, stuck".to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The Drone's brief names this belt and no other.**
    ///
    /// [`armada_core::fleet::drone::BRIEF`] is prose telling a Drone which tools
    /// its account of the work is read through. Two ways that prose goes wrong
    /// silently, and this is where both are caught, because this is the only
    /// place the router and the words are visible at once.
    ///
    /// **The spelling.** The server advertises `fleet.report`; Claude Code
    /// exposes it to the model as `mcp__armada__fleet_report` (`docs/traps.md`,
    /// *Claude Code renames a dotted tool*). A brief written with the documented
    /// name matches nothing and the model answers that it has no such tool — the
    /// same class of inert instruction that made Helm's persona ask for files it
    /// had no `Read` to open.
    ///
    /// **The absence.** `fleet.spawn` is not on this belt and there is no path
    /// from here to it; a brief that mentioned it would be inviting a Drone to
    /// call a tool that does not exist, which costs a turn and teaches it
    /// nothing.
    #[test]
    fn the_brief_names_every_tool_on_this_belt_as_the_model_sees_it() {
        // **The whole appended prompt, not one constant of it.** `fleet.propose`
        // is named by `armada_core::skill::BODY` and the other three by
        // `BRIEF`; they ride in one `--append-system-prompt` because the flag is
        // singular, and a test reading half of it would go green against a
        // Drone offered a tool nothing had told it about.
        let brief = armada_core::fleet::drone::brief();
        let brief = brief.as_str();
        for tool in TOOLS {
            let seen = format!(
                "mcp__{}__{}",
                armada_core::helm::SERVER,
                tool.replace('.', "_")
            );
            assert!(brief.contains(&seen), "the brief never names `{seen}`");
            assert!(
                !brief.contains(tool),
                "the brief spells `{tool}` the way the wire does, which matches \
                 nothing the model can call"
            );
        }
        // It says a Drone cannot spawn, and it names no tool for doing so —
        // those are different claims and only the second is a bug.
        assert!(brief.contains("cannot spawn"), "{brief}");
        assert!(
            !brief.contains(&format!("mcp__{}__fleet_spawn", armada_core::helm::SERVER)),
            "the brief offers a Drone a tool this belt does not serve"
        );
    }

}
