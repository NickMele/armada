//! The headless agent CLI: what a Drone is started as.
//!
//! **This is the only file in the workspace that knows how that CLI spells
//! anything.** A capability arrives here as a `Grant` and leaves as a string in
//! an argument list; a guarantee arrives as a type with one inhabitant and
//! leaves as a flag. Nothing above this crate learns either spelling.
//!
//! # The argument list is the permission model
//!
//! Not a metaphor. What a Drone may run unattended, whether it is asked before
//! it is refused, and whether the operator's own servers come along are all
//! granted and withheld here, in argv, at spawn. Three of the flags below are
//! doing work that has no runtime check behind it anywhere:
//!
//! | Flag | What its absence does |
//! | --- | --- |
//! | `--strict-mcp-config` | The session comes up holding every MCP server the operator has connected. Measured: seven servers, ninety-five tools, personal accounts. **This is the v1 defect this step exists for** |
//! | `--permission-mode` | The mode falls back to the operator's own configured default, which was measured as `auto` — a Drone that approves itself |
//! | `--allowedTools` | Every built-in tool is callable. It is a permission allowlist and **not** a toolset: it removed none of the thirty built-ins in any of the three spike runs |
//!
//! The last row is why confinement here is a floor rather than a fence, and it
//! is written down as an open question on Drone rather than papered over: the
//! built-in tools are bounded by what the Drone can reach — a worktree, an
//! empty environment — not by this list.
//!
//! # Nothing readable goes in argv
//!
//! No prompt text, no task, nothing brokered. `ps` prints a same-uid child's
//! argument list on darwin 27 and does **not** print its environment, measured
//! for this step, so argv is the one channel that is public by construction.
//! The prompt goes in on stdin as the session's first turn, which is also the
//! channel a later turn is injected through — one path, not two.
//!
//! # A denial is silent to the Drone and loud here
//!
//! `--permission-mode dontAsk` refuses without prompting, which it must: a
//! detached Drone has no terminal, so a prompt would hang the Job until its
//! timeout. The Drone is told about that in the baseline prompt, and every
//! refusal is a `DroneEvent::Refused` in the transcript, so a Job that goes
//! quiet is diagnosable as an argument-list problem rather than a prompt one.

use std::error::Error;
use std::fmt;

use adapter_traits::{
    AgentHarness, AmbientServers, DroneEvent, DroneSpawnConfig, Grant, Launch, Prompting,
};

use crate::mcp::EVIDENCE_SERVER;
use crate::transcript;

/// The tools the Evidence server exposes, as the harness names them: the
/// server's registered name and each tool's own, joined the way MCP tools are.
///
/// **All four in every toolbelt.** They are prepended here rather than being a
/// `Grant`, because a list is something a caller can build empty — and a Drone
/// denied one of them is denied silently, which reads as a Drone that went
/// quiet rather than as an argument-list fault.
///
/// **The third one is why this table matters.** A Drone on the `tests` step of
/// Job `01M14HZ8ND001FYT6264WZJFPB` was granted `Bash(cargo fmt --all:*)` and
/// `Bash(cargo xtask verify-foundations:*)` and nothing else, so every way of
/// finding out whether its work held up was denied by the row above — silently,
/// which reads to a Drone as a tool that does not work. It hand-checked, said
/// so honestly, and failed a Check it had no way to see coming. The fix is not
/// a wider `Bash` grant, which would run a command the workflow froze against a
/// Manifest that may have changed; it is a tool through which Fleet runs the
/// Checks itself.
const EVIDENCE_TOOL: &str = "mcp__armada__submit_evidence";
const SCOPE_TOOL: &str = "mcp__armada__declare_scope";
const CHECKS_TOOL: &str = "mcp__armada__run_checks";

/// **In every toolbelt, and not a `Grant`.** Asking costs nothing and creates
/// nothing — what it produces is a question on a Board — so there is no spend to
/// gate it behind, which is the whole of what separates it from
/// [`DISPATCH_TOOL`] below.
///
/// A Drone denied this one is denied in the way the comment above describes, and
/// the shape it takes here is worse than usual: the alternative to asking is
/// **guessing**, so a Drone whose `ask_question` is silently refused does not go
/// quiet. It carries on and produces work nobody chose.
const ASK_TOOL: &str = "mcp__armada__ask_question";

/// **Not in the table above, because it is not in every toolbelt.** It is the
/// one Armada tool that is granted rather than given: a Drone that may create
/// Jobs is a Drone one approval bought several Drones' worth of spend from, so
/// it is rendered only where [`Grant::DispatchAJob`] was granted. A Drone that
/// calls it without the grant is denied by the CLI silently, which is why Fleet
/// refuses the same call in words on its own side.
const DISPATCH_TOOL: &str = "mcp__armada__dispatch_job";

/// The program name `crates/config/settings.toml` gives as the default for the
/// AgentHarness binary path: `claude (on PATH)`.
///
/// **The vendor's spelling, and this file is where it is allowed to be.** A
/// composition root that carried its own default would be the boundary having
/// leaked — which is why the composition root calls [`HeadlessAgent::on_path`] rather
/// than passing a string it wrote down.
const ON_PATH: &str = "claude";

/// The models a Job may name, in the order a picker offers them.
///
/// **`crates/config/settings.toml` names none.** Two rows bear on this —
/// `kit-level-allowed-default-models-list`, which is "the set that Default
/// model per Job type and Judge model select from", and
/// `default-model-per-job-type` — and *neither carries a `default` value*,
/// unlike the AgentHarness binary row above it which carries one. So there is
/// no configured roster to read, and the roster has to come from somewhere or
/// the picker has no source.
///
/// It comes from here, for [`ON_PATH`]'s reason: these are the aliases this
/// vendor's CLI accepts on `--model`, so they are the adapter's knowledge and
/// nowhere else's. **That is a stand-in, not the answer** — the settings rows
/// are the answer, and until they carry values this list is the adapter
/// stating what its own binary will take.
const MODELS: &[&str] = &["opus", "sonnet", "haiku"];

/// The model a Job gets when nothing names one. The middle of [`MODELS`]:
/// capable enough to run a workflow and not the one that empties a budget.
const DEFAULT_MODEL: &str = "sonnet";

/// The headless agent CLI, at a path somebody configured.
///
/// The path is a value rather than a constant so that Doctor can probe a
/// missing binary and a machine can put it somewhere unusual — the alternative
/// is a hardcoded string at a call site, invisible to configuration and to
/// health.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadlessAgent {
    program: String,
}

impl HeadlessAgent {
    /// The CLI at `program`.
    pub fn at(program: impl Into<String>) -> HeadlessAgent {
        HeadlessAgent {
            program: program.into(),
        }
    }

    /// The CLI as installed: the settings default, resolved by the `PATH` the
    /// Drone is given rather than by an absolute path Fleet invented.
    ///
    /// **This is what makes the binary an override rather than a requirement.**
    /// A machine with the CLI installed the ordinary way needs no environment
    /// variable, and a machine that put it somewhere unusual names it — and is
    /// refused before the bind if that name is wrong.
    pub fn on_path() -> HeadlessAgent {
        HeadlessAgent::at(ON_PATH)
    }

    /// The program that will be executed. **The one way to ask what this
    /// harness is, without spelling it** — the name is a vendor's and lives in
    /// this crate, so the composition root probes it through here rather than
    /// carrying a literal it is not allowed to write.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The models a Job may name, for the composer's picker.
    ///
    /// Stated here rather than read from configuration because configuration
    /// states none — see [`MODELS`]. The caller passes it up the seam; nothing
    /// below the adapter boundary learns the spellings.
    pub fn models() -> &'static [&'static str] {
        MODELS
    }

    /// The model a proposal that names none is given.
    ///
    /// **This is what stops a Job dying at dispatch.** A proposal with no model
    /// used to be accepted, stored and shown on the board, and refused at spawn
    /// with "no model was named". Fleet now fills the value in at creation, and
    /// refuses there when it cannot.
    pub fn default_model() -> &'static str {
        DEFAULT_MODEL
    }
}

impl AgentHarness for HeadlessAgent {
    type Error = HarnessRefused;

    fn render(&self, config: &DroneSpawnConfig) -> Result<Launch, HarnessRefused> {
        // Headless, with the session on stdin and the transcript on stdout.
        // `--replay-user-messages` re-emits an injected turn at the moment it
        // is consumed, which is the only acknowledgement the stream offers that
        // a turn landed — without it, Fleet has told the Drone something and
        // has no way to know it arrived.
        let mut args: Vec<String> = vec![
            "-p".into(),
            "--input-format".into(),
            "stream-json".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--replay-user-messages".into(),
            "--model".into(),
            config.model().as_str().into(),
        ];

        match config.prompting() {
            Prompting::Never => {
                args.push("--permission-mode".into());
                args.push("dontAsk".into());
            }
        }

        // The one arm this enum has, and writing it is what puts the strict
        // flag on. The flag and the file go on together because they come off
        // one field: a config naming a file is a config excluding everything
        // else, and there is no value of that field that means otherwise.
        match config.mcp().ambient_servers() {
            AmbientServers::Excluded => {
                args.push("--strict-mcp-config".into());
                args.push("--mcp-config".into());
                args.push(config.mcp().path().into());
            }
        }

        args.push("--allowedTools".into());
        args.push(allowlist(config)?);

        Ok(Launch::rendered(config, &self.program, args))
    }

    fn read(&self, line: &str) -> Vec<DroneEvent> {
        transcript::read(line)
    }
}

/// The `--allowedTools` value: Armada's own tools first, then each grant.
///
/// They lead deliberately. They are the entries that are there on every spawn,
/// and a reader checking an argument list by eye sees them before anything that
/// varies.
fn allowlist(config: &DroneSpawnConfig) -> Result<String, HarnessRefused> {
    let mut allowed = vec![
        String::from(EVIDENCE_TOOL),
        String::from(SCOPE_TOOL),
        String::from(CHECKS_TOOL),
        String::from(ASK_TOOL),
    ];
    for grant in config.toolbelt().granted() {
        match grant {
            // Reading is three tools, because a search and a listing are reads
            // the CLI names separately, and a Drone denied them reads whole
            // files to find a line.
            Grant::ReadTheWorktree => {
                allowed.push("Read".into());
                allowed.push("Glob".into());
                allowed.push("Grep".into());
            }
            Grant::ChangeTheWorktree => {
                allowed.push("Edit".into());
                allowed.push("Write".into());
            }
            Grant::RunADeclaredCommand(run) => allowed.push(command_rule(run)?),
            Grant::DispatchAJob => allowed.push(DISPATCH_TOOL.into()),
        }
    }
    Ok(allowed.join(","))
}

/// One declared command, as a prefix rule.
///
/// `cargo test` becomes `Bash(cargo test:*)` — the command and anything after
/// it, which is what a Check's own invocation needs.
///
/// **A run string that would break the rule's own syntax is refused here, at
/// render, rather than reaching a spawn.** A malformed rule does not fail: it
/// silently allows nothing, the Drone is denied a command it was told it had,
/// and the Job goes quiet. That failure looks exactly like a bad prompt and is
/// not one.
fn command_rule(run: &str) -> Result<String, HarnessRefused> {
    let run = run.trim();
    if run.is_empty() {
        return Err(HarnessRefused::CommandEmpty);
    }
    if let Some(found) = run.chars().find(|c| matches!(c, '(' | ')' | ',' | '\n')) {
        return Err(HarnessRefused::CommandNotExpressibleAsARule {
            run: String::from(run),
            found,
        });
    }
    if would_push(run) {
        return Err(HarnessRefused::CommandWouldPush {
            run: String::from(run),
        });
    }
    Ok(format!("Bash({run}:*)"))
}

/// Whether a declared command would put a branch somewhere a person can see it.
///
/// **The last place a push can be expressed, and it is refused here.** No
/// `Grant` names one and no type a Drone is handed carries one, so a declared
/// command is the only remaining spelling — and a repository that declares one
/// is a repository whose Drone would try. The refusal is a rendering failure
/// rather than a silent omission because a rule that is quietly dropped denies
/// without telling anyone, and a denied Drone goes quiet.
///
/// This is belt to the environment's braces: a Drone's environment carries no
/// credential and no agent socket, so a push that reached the network would
/// have nothing to authenticate with. Two mechanisms, because the second one
/// fails at the point of use where this one fails at the point of declaration.
fn would_push(run: &str) -> bool {
    let mut words = run.split_whitespace();
    let Some(program) = words.next() else {
        return false;
    };
    let program = program.rsplit('/').next().unwrap_or(program);
    program == "git" && words.any(|word| word == "push")
}

/// Why a Drone could not be rendered.
///
/// **Every variant is fatal to the Job and none of them leaves a process
/// running**, because nothing has been started when this is raised. Each is a
/// different thing to go and do: fix the `commands.<name>.run` in the
/// `armada.yml`, or remove the entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HarnessRefused {
    /// A declared command with no command in it.
    CommandEmpty,
    /// A declared command holding a character the allowlist's own syntax uses.
    CommandNotExpressibleAsARule { run: String, found: char },
    /// A declared command that would push. **Refused, never granted quietly.**
    CommandWouldPush { run: String },
}

impl fmt::Display for HarnessRefused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessRefused::CommandEmpty => {
                out.write_str("a declared command is empty, so nothing can be allowed for it")
            }
            HarnessRefused::CommandNotExpressibleAsARule { run, found } => write!(
                out,
                "the declared command `{run}` holds `{found}`, which the tool \
                 allowlist uses for its own syntax — a rule built from it would \
                 allow nothing and the Drone would be denied without being told. \
                 Move the command into a script and declare the script"
            ),
            HarnessRefused::CommandWouldPush { run } => write!(
                out,
                "the declared command `{run}` would push, and a Drone cannot: it \
                 commits locally inside its own worktree, and push, pull request \
                 and merge are Fleet's, with credentials a Drone is never given. \
                 Remove it from the Manifest — a Drone granted it would be denied \
                 at the point of use with nothing to say why"
            ),
        }
    }
}

impl Error for HarnessRefused {}

/// The tool name the baseline prompt is describing, for a caller that needs to
/// name it — a health probe, or a test asserting the toolbelt is not empty.
///
/// The prompt itself deliberately does **not** carry it: the tool's own
/// description carries its name, so a described tool and a named tool cannot
/// drift apart.
pub fn evidence_tool() -> &'static str {
    EVIDENCE_TOOL
}

/// The scope tool's name, for the same callers. **All four are needed**: a
/// toolbelt missing any one of them denies the Drone silently, and a Job that
/// goes quiet is an argument-list fault rather than a prompt one.
pub fn scope_tool() -> &'static str {
    SCOPE_TOOL
}

/// The dry-run tool's name. The one this file's [`EVIDENCE_TOOL`] comment is
/// about: a Drone that cannot ask whether its work passes has no way to find
/// out except by submitting.
pub fn checks_tool() -> &'static str {
    CHECKS_TOOL
}

/// The asking tool's name. In every toolbelt like the three above, and the one
/// whose silent denial is not silence: a Drone that cannot ask guesses.
pub fn ask_tool() -> &'static str {
    ASK_TOOL
}

/// The dispatch tool's name, for a caller that needs to assert it is *absent*
/// from a toolbelt. The four above are in every one; this is the only Armada
/// tool a spawn can be built without.
pub fn dispatch_tool() -> &'static str {
    DISPATCH_TOOL
}

/// The server every tool above is served from.
pub fn evidence_server() -> &'static str {
    EVIDENCE_SERVER
}
