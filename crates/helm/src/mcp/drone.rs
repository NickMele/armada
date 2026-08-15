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
use armada_core::fleet::Verdict;
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
pub const TOOLS: [&str; 3] = ["fleet.report", "fleet.ask_human", "fleet.verdict"];

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
}

/// `fleet.ask_human`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct AskArgs {
    /// The question, in full. It is what a person reads in the inbox, out of
    /// context and possibly hours later.
    pub question: String,
}

/// `fleet.verdict`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct VerdictArgs {
    /// Which step ended.
    pub step: String,
    /// How it ended: `PASS`, `FAILED`, `BLOCKED` or `NEEDS_HUMAN`.
    pub verdict: String,
    /// What the verdict rests on. **Required for a `PASS`** — an agent asserting
    /// that the tests pass is not evidence, an `armada manifest check` exit code
    /// is (PLAN.md §14.3).
    #[serde(default)]
    pub evidence: Vec<EvidenceArg>,
}

/// One piece of evidence, as a caller writes it.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct EvidenceArg {
    /// What produced it — `check`, `command`.
    pub kind: String,
    /// Which one: a check id from `manifest.check`'s `results[]`, a command
    /// name.
    pub scope: String,
    /// What it exited with.
    pub exit: i32,
}

#[tool_router]
impl Toolbelt {
    /// Append progress to this Job's own record.
    #[tool(
        name = "fleet.report",
        description = "Append a progress note to your own Job record. Use it when you finish \
                       something worth an orchestrator knowing about; it does not interrupt \
                       anyone and does not need an answer."
    )]
    async fn fleet_report(&self, Parameters(args): Parameters<ReportArgs>) -> CallToolResult {
        let world = self.world.clone();
        let job = match self.job() {
            Ok(job) => job,
            Err(error) => return super::answer::from("fleet report", Err(error)),
        };
        run("fleet report", move || {
            fleet::report(&SystemClock, &world.place(), &job, &args.body)
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

    /// Emit a step verdict.
    #[tool(
        name = "fleet.verdict",
        description = "Record how a step of your workflow ended: PASS, FAILED, BLOCKED or \
                       NEEDS_HUMAN. A PASS must carry evidence an external command produced — \
                       check ids and their exit codes — and is refused without it."
    )]
    async fn fleet_verdict(&self, Parameters(args): Parameters<VerdictArgs>) -> CallToolResult {
        let world = self.world.clone();
        let job = match self.job() {
            Ok(job) => job,
            Err(error) => return super::answer::from("fleet verdict", Err(error)),
        };
        let reached = match word_to_verdict(&args.verdict) {
            Ok(reached) => reached,
            Err(error) => return super::answer::from("fleet verdict", Err(error)),
        };
        let evidence = args
            .evidence
            .into_iter()
            .map(|e| armada_core::envelope::Evidence {
                kind: e.kind,
                scope: e.scope,
                exit: e.exit,
            })
            .collect();
        run("fleet verdict", move || {
            fleet::verdict(
                &SystemClock,
                &world.place(),
                &job,
                &args.step,
                reached,
                evidence,
            )
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
                 and record a step verdict — all against your own Job and no other. You cannot \
                 spawn work; if the task needs decomposing, say so with fleet.ask_human.",
            )
    }
}

/// The word a caller writes, as the enum PLAN.md §14.3 defines.
///
/// **One spelling, and it is the JSON spelling** — `FAILED`, never `FAIL`.
/// Accepted case-insensitively because a model writing `pass` meant `PASS`, and
/// refusing that would send an agent round a loop over capitalisation; a word
/// outside the four is refused rather than guessed at, for the reason
/// classification refuses a fifth label.
fn word_to_verdict(word: &str) -> Result<Verdict, ArmadaError> {
    match word.trim().to_ascii_uppercase().as_str() {
        "PASS" => Ok(Verdict::Pass),
        "FAILED" => Ok(Verdict::Failed),
        "BLOCKED" => Ok(Verdict::Blocked),
        "NEEDS_HUMAN" => Ok(Verdict::NeedsHuman),
        other => Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "verdict".to_string(),
            message: format!("`{other}` is not a verdict"),
            next_action: Some("one of PASS, FAILED, BLOCKED, NEEDS_HUMAN".to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_verdicts_are_read_in_any_case_and_nothing_else_is() {
        assert_eq!(word_to_verdict("PASS").unwrap(), Verdict::Pass);
        assert_eq!(word_to_verdict("pass").unwrap(), Verdict::Pass);
        assert_eq!(
            word_to_verdict(" needs_human ").unwrap(),
            Verdict::NeedsHuman
        );
        // The split PLAN.md §14.3 says Fleet must not reinvent.
        assert!(word_to_verdict("FAIL").is_err());
        assert!(word_to_verdict("ok").is_err());
    }

    #[test]
    fn a_refused_verdict_says_which_four_words_are_legal() {
        let error = word_to_verdict("done").unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        let next = error.next_action.expect("a next action");
        for word in ["PASS", "FAILED", "BLOCKED", "NEEDS_HUMAN"] {
            assert!(next.contains(word), "{next}");
        }
    }
}
