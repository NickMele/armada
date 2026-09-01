//! What a Drone is started as, in a crate that cannot start one.
//!
//! # The harness renders; Fleet starts
//!
//! [`AgentHarness`](super::AgentHarness) does not spawn. It turns a
//! [`DroneSpawnConfig`] into a [`Launch`] — a program, an argument list, an
//! environment and a directory — and Fleet starts that, detached, through the
//! one type in the workspace that can start anything.
//!
//! Three things fall out of the split, and each was a reason to take it.
//!
//! **Every confinement property becomes a pure assertion.** Whether the strict
//! flag is on the argument list, whether an ambient server can appear, whether
//! a credential is in the environment — all of it is a value a test reads,
//! with no process, no timing and no machine involved. The alternative is a
//! suite that can only check confinement by spawning the thing being confined.
//!
//! **The environment is built by Fleet, once, and not by whoever spawns.** v1's
//! Drone spawn inherited the operator's environment wholesale, and v1's own
//! design table rejected that channel in the same repository. Here there is no
//! inherit: [`Environment::nothing`] is where every Drone environment starts.
//!
//! **A harness cannot start an attached process**, because it cannot start one.
//!
//! # There is no escape hatch, at three levels
//!
//! [`DroneSpawnConfig`] has private fields, one constructor, no `Default`, no
//! setter and no raw argument builder. [`Launch`] can only be built from a
//! config, and takes its environment and its directory from that config rather
//! than from its own caller — so an implementation cannot render a Drone into a
//! different directory or a different environment than the one it was given.
//! And [`McpConfig`] answers [`McpConfig::ambient_servers`] with a one-variant
//! enum, so the strict flag is a consequence of the field's type rather than a
//! second field somebody could set to false.
//!
//! # Why there is no field for the prompt's text on the argument list
//!
//! The task text goes in on stdin as the session's first turn, not in argv.
//! Argv is world-readable through `ps` — measured on darwin 27, where the
//! environment of a same-uid child is *not* — so anything on it is public to
//! every process on the machine. Nothing brokered may be there, permanently,
//! and a Job's brief is not something to publish either.
//!
//! The same channel is what [`crate::AgentHarness`] needs for a live session:
//! stdin is held open, one JSON object per line, which is the mechanism spike 4
//! measured. So the prompt and every later injected turn take one path, and
//! there is not a second one to keep correct.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::Worktree;

/// The model a Drone runs on.
///
/// **This is `core_model::ModelName`'s string, and it is deliberately not that
/// type.** `cargo tree -p adapter-traits` shows only this crate, so the seam
/// every adapter implements cannot name the domain crate. The composition root
/// is the one place both types are in scope, and `ModelName::as_str` is the
/// only thing that should ever be handed to [`Model::named`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model(String);

impl Model {
    pub fn named(name: &str) -> Result<Model, SpawnConfigRefused> {
        if name.trim().is_empty() {
            return Err(SpawnConfigRefused::ModelUnnamed);
        }
        Ok(Model(String::from(name)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The assembled prompt: six layers of text, already in order.
///
/// This type does not assemble it and does not know what the layers are — that
/// is the Agent Prompt Contract's, and Fleet's. What it holds is the finished
/// string that becomes the session's first turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt(String);

impl Prompt {
    pub fn assembled(text: &str) -> Result<Prompt, SpawnConfigRefused> {
        if text.trim().is_empty() {
            return Err(SpawnConfigRefused::PromptEmpty);
        }
        Ok(Prompt(String::from(text)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a server Armada did not inject can appear in the Drone's session.
///
/// **One variant, and there is no second one to write.** v1 spawned without the
/// strict flag and its Drone inherited a 103-tool toolbelt including the
/// operator's personal accounts. That is not a check that was skipped; it is a
/// flag that was never passed, so the fix is a value with no other setting
/// rather than a boolean somebody could pass `false`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmbientServers {
    /// Nothing but what [`McpConfig`] names is reachable.
    Excluded,
}

/// The MCP configuration file Fleet wrote for one Drone.
///
/// Non-optional on [`DroneSpawnConfig`], because the Evidence tool is the only
/// sanctioned completion path and a Drone without it has no way to finish. The
/// two failures are not alike: a harness that cannot inject the server fails
/// immediately and visibly, while a harness that injects but cannot exclude
/// looks exactly like success — the Drone works, holding the operator's whole
/// toolbelt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpConfig {
    path: String,
}

impl McpConfig {
    /// The file at `path`, and nothing but the servers it names.
    ///
    /// The name says what the value means, so a call site reads as the
    /// guarantee rather than as a path being handed over.
    pub fn only_these(path: &str) -> Result<McpConfig, SpawnConfigRefused> {
        if path.trim().is_empty() {
            return Err(SpawnConfigRefused::McpConfigPathEmpty);
        }
        if !path.starts_with('/') {
            return Err(SpawnConfigRefused::McpConfigPathNotAbsolute {
                given: String::from(path),
            });
        }
        Ok(McpConfig {
            path: String::from(path),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// What happens to every server this file does not name.
    ///
    /// An implementation matching on this has one arm to write, and writing it
    /// is what puts the strict flag on the argument list.
    pub fn ambient_servers(&self) -> AmbientServers {
        AmbientServers::Excluded
    }
}

/// How a Drone is answered when it reaches for something it was not granted.
///
/// **One variant.** A detached Drone has no controlling terminal and its stdin
/// carries the session, so a prompt has nobody to answer it and would hang the
/// Job until its timeout. Leaving the mode off is worse still: it inherits
/// whatever the operator configured, which was measured as `auto`.
///
/// The cost is stated in the baseline prompt rather than hidden: a denial
/// arrives silently, so the Drone is told to notice one and stop rather than
/// route around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prompting {
    /// The Drone is never asked. A call outside the toolbelt is refused.
    Never,
}

/// One thing a Drone may do, named for the capability rather than for the tool.
///
/// **The vendor's spelling for each of these is `adapters`' business.** A Drone
/// does not link Rust, so the type that withholds a capability from it is this
/// enum and the argument list rendered from it — not a handle with methods
/// missing.
///
/// **There is no variant that can express a push, a pull request or a merge**,
/// and that is the Drone-facing version-control boundary: not a method absent
/// from a struct, but a capability with no spelling in the type that grants
/// capabilities. The second half is [`Environment`], which carries no
/// credential, so a shell that reached a push would have nothing to push with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Grant {
    /// Read any file inside the worktree.
    ReadTheWorktree,
    /// Create and change files inside the worktree.
    ChangeTheWorktree,
    /// Run one command the Manifest declared, spelled as its `run` string.
    ///
    /// The string comes from a `commands.<name>.run` in an `armada.yml` that a
    /// person committed, never from a Drone.
    RunADeclaredCommand(String),
    /// Create Jobs, as children of the one being worked.
    ///
    /// **The only grant whose effect outlives the Drone that holds it.** Every
    /// other capability here changes a worktree or reads one; this one makes
    /// records that get their own worktrees, their own Drones and their own
    /// bill. It is granted on one step of one workflow — the step after a
    /// person read the plan — and withheld everywhere else, which is what makes
    /// a Drone inventing work a call that is not on its list rather than a call
    /// somebody remembered to refuse.
    DispatchAJob,
}

/// What a Drone may call.
///
/// **The Evidence tool is in every toolbelt and there is no constructor that
/// omits it.** It is not in the [`Grant`] list either, because a list is
/// something a caller can build empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toolbelt {
    granted: Vec<Grant>,
}

impl Toolbelt {
    /// The Evidence tool, and nothing else. Where every toolbelt starts.
    pub fn evidence_only() -> Toolbelt {
        Toolbelt {
            granted: Vec::new(),
        }
    }

    /// One more capability. Additive only — there is no way to take one away,
    /// and nothing to take away the Evidence tool from.
    pub fn and(mut self, grant: Grant) -> Toolbelt {
        self.granted.push(grant);
        self
    }

    /// Everything granted beyond the Evidence tool, in the order granted.
    pub fn granted(&self) -> &[Grant] {
        &self.granted
    }
}

/// The Drone's whole environment, built one variable at a time.
///
/// **Nothing is inherited.** `env_clear` appears nowhere in v1's production
/// code, so a token exported in the operator's shell reached every Drone v1
/// spawned — the one place v1's Drone spawn was worse than its own check
/// spawn, and the thing its own handoff table said not to do. Here the starting
/// point is [`Environment::nothing`] and there is no method that adds the
/// caller's own environment to it.
///
/// **Nothing brokered goes in here yet.** Nothing in this workspace resolves a
/// secret, so there is no named exit through which a [`Secret`](crate::Secret)
/// becomes a value on this list. When one arrives it is one method, and this
/// type loses its `Debug` the day it does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    vars: Vec<(String, String)>,
}

impl Environment {
    /// An empty environment. **The only starting point there is.**
    pub fn nothing() -> Environment {
        Environment::default()
    }

    /// One variable, by name.
    ///
    /// A name that is empty, holds `=`, or holds a NUL is refused: the first
    /// two are not names the operating system can express, and a NUL truncates
    /// the string somebody else reads.
    pub fn and(mut self, name: &str, value: &str) -> Result<Environment, SpawnConfigRefused> {
        if name.is_empty() {
            return Err(SpawnConfigRefused::EnvNameEmpty);
        }
        if name.contains('=') || name.contains('\0') || value.contains('\0') {
            return Err(SpawnConfigRefused::EnvNameNotPortable {
                name: String::from(name),
            });
        }
        if self.vars.iter().any(|(held, _)| held == name) {
            return Err(SpawnConfigRefused::EnvNameTwice {
                name: String::from(name),
            });
        }
        self.vars.push((String::from(name), String::from(value)));
        Ok(self)
    }

    /// Every variable, in the order it was named.
    pub fn vars(&self) -> &[(String, String)] {
        &self.vars
    }

    /// The names only, never the values. What a log line or a failure journal
    /// is allowed to carry — v1's `EnvDelta::names()`, kept for the same reason.
    pub fn names(&self) -> Vec<&str> {
        self.vars.iter().map(|(name, _)| name.as_str()).collect()
    }
}

/// Everything one Drone is started with.
///
/// **Private fields, one constructor, no `Default`, no setter, no raw argument
/// builder and no escape-hatch constructor.** A future flag is a new field and
/// a new parameter, never a string that goes around the fields already here.
/// Every parameter of [`DroneSpawnConfig::spawn_in`] is a distinct type, so no
/// two of them can be passed in the wrong order.
///
/// The failure this refuses is the config that is correct today because
/// whoever built it remembered the flag, and wrong the day somebody builds one
/// by a different path. There is no different path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneSpawnConfig {
    worktree: Worktree,
    model: Model,
    prompt: Prompt,
    mcp: McpConfig,
    toolbelt: Toolbelt,
    environment: Environment,
}

impl DroneSpawnConfig {
    /// The only way to make one.
    pub fn spawn_in(
        worktree: &Worktree,
        model: Model,
        prompt: Prompt,
        mcp: McpConfig,
        toolbelt: Toolbelt,
        environment: Environment,
    ) -> DroneSpawnConfig {
        DroneSpawnConfig {
            worktree: worktree.clone(),
            model,
            prompt,
            mcp,
            toolbelt,
            environment,
        }
    }

    pub fn worktree(&self) -> &Worktree {
        &self.worktree
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The session's first turn. **Not an argument** — see this module's note
    /// on why nothing readable goes in argv.
    pub fn prompt(&self) -> &Prompt {
        &self.prompt
    }

    pub fn mcp(&self) -> &McpConfig {
        &self.mcp
    }

    pub fn toolbelt(&self) -> &Toolbelt {
        &self.toolbelt
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Whether the Drone can be asked to confirm something. It cannot.
    pub fn prompting(&self) -> Prompting {
        Prompting::Never
    }
}

/// A value that could not have produced a confined Drone, refused where it was
/// written rather than where it would have failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnConfigRefused {
    ModelUnnamed,
    PromptEmpty,
    McpConfigPathEmpty,
    /// A relative path would resolve against the Drone's working directory,
    /// which is the worktree — so the file Fleet wrote would not be the file
    /// the Drone read.
    McpConfigPathNotAbsolute {
        given: String,
    },
    EnvNameEmpty,
    EnvNameNotPortable {
        name: String,
    },
    /// The same name twice. Refused rather than resolved, because which one
    /// wins is a rule nobody would find and both callers believe theirs is set.
    EnvNameTwice {
        name: String,
    },
}

/// What a harness turns a [`DroneSpawnConfig`] into: a process, not yet started.
///
/// **There is no stdio on it.** Where the Drone's input, output and errors go
/// is fixed by whoever starts it — stdin holds the session, stdout carries the
/// transcript — and a field here would be a way to ask for something else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    program: String,
    args: Vec<String>,
    directory: String,
    environment: Environment,
}

impl Launch {
    /// Render a config into a startable process.
    ///
    /// **The directory and the environment come from the config**, not from the
    /// caller, so an implementation cannot put a Drone somewhere else or hand
    /// it something else. What an implementation supplies is the two things
    /// only it can know: which program, and how that program spells the
    /// config's guarantees.
    pub fn rendered(config: &DroneSpawnConfig, program: &str, args: Vec<String>) -> Launch {
        Launch {
            program: String::from(program),
            args,
            directory: String::from(config.worktree().path()),
            environment: config.environment().clone(),
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The worktree. A Drone's working directory is its own checkout and
    /// nothing else, which is what makes a relative path in its own output
    /// mean something.
    pub fn directory(&self) -> &str {
        &self.directory
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }
}

/// A Drone that is running: what Fleet knows about the process itself.
///
/// **The pid, and nothing that can act on it.** Killing a Drone is
/// `LiveSession::terminate` in `fleet`, which is held by the one caller allowed
/// to end one; a handle with a `kill` on it would put that call everywhere the
/// handle goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneHandle {
    pid: u32,
}

impl DroneHandle {
    /// Record a process that started. Called by whoever started it.
    pub fn started(pid: u32) -> DroneHandle {
        DroneHandle { pid }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl SpawnConfigRefused {
    /// A sentence for a person, built here rather than by each caller.
    pub fn said(&self) -> String {
        match self {
            SpawnConfigRefused::ModelUnnamed => "no model was named".to_string(),
            SpawnConfigRefused::PromptEmpty => "the assembled prompt is empty".to_string(),
            SpawnConfigRefused::McpConfigPathEmpty => {
                "no MCP configuration file was named".to_string()
            }
            SpawnConfigRefused::McpConfigPathNotAbsolute { given } => {
                let mut said = String::from("the MCP configuration path `");
                said.push_str(given);
                said.push_str("` is relative, and would resolve inside the worktree");
                said
            }
            SpawnConfigRefused::EnvNameEmpty => {
                "an environment variable was given no name".to_string()
            }
            SpawnConfigRefused::EnvNameNotPortable { name } => {
                let mut said = String::from("`");
                said.push_str(name);
                said.push_str("` is not a name the operating system can carry");
                said
            }
            SpawnConfigRefused::EnvNameTwice { name } => {
                let mut said = String::from("`");
                said.push_str(name);
                said.push_str("` was named twice, and which one wins is not decided here");
                said
            }
        }
    }
}
