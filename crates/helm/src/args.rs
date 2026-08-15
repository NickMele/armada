//! Argument parsing, and nothing else.
//!
//! **Hand-rolled rather than a parser crate**, for one reason that decides it:
//! a `commands:` entry's remaining argv must reach the child **untouched**
//! (PLAN.md §4.5), including flags Armada itself defines. `armada manifest
//! worktrees prune --dry-run` runs `… prune --dry-run`, and `--dry-run` there
//! is the child's. A general parser has to be told to stop parsing, and being
//! told wrongly is silent.
//!
//! So the grammar is stated rather than inferred:
//!
//! ```text
//! armada [global flags] manifest <verb> [verb flags]      the verbs Manifest owns
//! armada [global flags] manifest <name> [anything at all] a commands: entry
//! ```
//!
//! Everything after a `commands:` name is the child's, whatever it looks like.
//!
//! **The module name is a level of the grammar, not a prefix on a verb name.**
//! `armada manifest check` is Manifest's `check`; `armada fleet ls` will be
//! Fleet's `ls`, and the two never have to disambiguate a shared word
//! ([`glossary.md`](../../../docs/glossary.md), `ARCHITECTURE.md` §1.9). The
//! cost is that the most-used verbs got longer, which PHASES.md §8.3 records as
//! an intended trade with a reserved-not-built resolution (PLAN.md §3).

use armada_core::error::{ArmadaError, ErrClass};
use armada_core::scope::Lens;

use crate::render::help::Topic;
use crate::render::style::ColorChoice;

/// What the whole line asked for: one verb, and the one global rendering
/// decision that outlives it.
///
/// **`color` sits here rather than on each variant** because it is decided once
/// for the process (PLAN.md §3.1.1) and applies to output no verb produces —
/// `--version`, `--help`, and the error from a line that never named a verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The verb, and its own flags.
    pub invocation: Invocation,
    /// `--color auto|always|never`.
    pub color: ColorChoice,
}

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// `armada --version`.
    Version,
    /// `armada --help`, `armada manifest --help`, `armada manifest check
    /// --help`, or `armada` with nothing at all — which is a different page
    /// (`docs/commands/render.md`: the wordmark shows there and nowhere else).
    Help(Topic),
    /// `armada manifest init`.
    Init(Common),
    /// `armada manifest up`, or `armada manifest down`.
    ///
    /// **One variant for two verbs**, because they take the same line: a
    /// component selector, `--json`, and `--dry-run` on the one that changes
    /// something. `direction` is what they differ by, and that is the whole
    /// difference (PLAN.md §3).
    Services {
        /// Start, or stop.
        direction: armada_core::lifecycle::Direction,
        /// The component to act on, or `None` for all of them.
        selector: Option<String>,
        /// Emit the envelope rather than human output.
        json: bool,
        /// Change nothing; report the argv and the ready-checks.
        dry_run: bool,
    },
    /// `armada manifest clean`.
    Clean {
        /// The flags every verb shares.
        common: Common,
        /// Also remove declared `owns.files`.
        artifacts: bool,
        /// Only touch workspaces whose directory no longer exists.
        orphaned: bool,
        /// Override the liveness guard.
        force: bool,
        /// Rebuild an unreadable `manifest.db` from labels alone.
        force_rebuild: bool,
    },
    /// `armada manifest status`.
    Status(Common),
    /// `armada manifest check`.
    Check(Box<Check>),
    /// `armada manifest skills`, or `armada manifest skills show <name>`.
    Skills {
        /// The skill to resolve, or `None` to list them all.
        show: Option<String>,
        /// Emit the envelope rather than human output.
        json: bool,
    },
    /// `armada manifest config <scan|verify>`.
    Config {
        /// Which half of the sandwich (PLAN.md §5).
        sub: ConfigSub,
        /// Emit the envelope rather than human output.
        json: bool,
    },
    /// A `commands:` entry, with everything after its name.
    Dispatch {
        /// The entry's name.
        name: String,
        /// Passed through untouched.
        argv: Vec<String>,
        /// `--json` forces `pipe` and makes stdout carry the envelope alone.
        json: bool,
    },
    /// `armada init` — **this machine**, not a workspace.
    MachineInit(Box<MachineInit>),
    /// `armada doctor`.
    Doctor {
        /// Emit the envelope.
        json: bool,
        /// Repair what is safely repairable.
        fix: bool,
    },
    /// `armada guild <verb>`.
    Guild(Box<GuildInvocation>),
    /// `armada fleet <verb>`.
    Fleet(Box<FleetInvocation>),
}

/// One of Fleet's verbs, and its own flags.
///
/// **Six variants rather than one with a mode field**, because they share
/// almost nothing: `spawn` takes a task and four ways to override the plan,
/// `board` takes one Job, and `ls` takes two lenses. A single struct would be
/// twelve optional fields with a comment saying which combinations are legal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetInvocation {
    /// `armada fleet spawn "<task>"`.
    Spawn(Box<Spawn>),
    /// `armada fleet ls`.
    Ls {
        /// Emit the envelope.
        json: bool,
        /// Include finished and killed Jobs, not just live ones.
        all: bool,
        /// Only Jobs waiting on you.
        needs_attention: bool,
    },
    /// `armada fleet board <job>`.
    Board {
        /// Emit the envelope.
        json: bool,
        /// Which Job.
        job: String,
        /// Change directory and exec `claude --resume`, replacing this process.
        exec: bool,
    },
    /// `armada fleet kill <job>`.
    Kill {
        /// Emit the envelope.
        json: bool,
        /// Which Job, or `None` under `--all-finished`.
        job: Option<String>,
        /// Do not delete the branch.
        keep_branch: bool,
        /// Release resources but leave the directory. Implies `--keep-branch`.
        keep_worktree: bool,
        /// Kill every Job whose workflow has terminated.
        all_finished: bool,
    },
    /// `armada fleet answer <job> "<answer>"`.
    Answer {
        /// Emit the envelope.
        json: bool,
        /// Which Job.
        job: String,
        /// What to tell it.
        answer: String,
    },
    /// `armada fleet inbox`.
    Inbox {
        /// Emit the envelope.
        json: bool,
        /// Only this Job's entries.
        job: Option<String>,
        /// Include entries already answered.
        all: bool,
    },
}

impl FleetInvocation {
    /// Whether this invocation asked for the envelope.
    pub fn json(&self) -> bool {
        match self {
            FleetInvocation::Spawn(spawn) => spawn.json,
            FleetInvocation::Ls { json, .. }
            | FleetInvocation::Board { json, .. }
            | FleetInvocation::Kill { json, .. }
            | FleetInvocation::Answer { json, .. }
            | FleetInvocation::Inbox { json, .. } => *json,
        }
    }
}

/// `armada fleet spawn`, with the flags `commands/fleet/spawn.md` gives it.
///
/// A struct rather than variant fields, for the same reason [`Check`] is one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Spawn {
    /// Emit the envelope.
    pub json: bool,
    /// What to do, in your words.
    pub task: String,
    /// Override classification.
    pub workflow: Option<String>,
    /// The Job's handle. Derived from the task when absent.
    pub name: Option<String>,
    /// `--budget k=v`, repeatable.
    pub budget: Vec<String>,
    /// Which repository to branch from.
    pub at: Option<String>,
    /// Report the classification, worktree path, port block and budget. Starts
    /// nothing.
    pub dry_run: bool,
}

/// `armada init`, with the flags `docs/commands/init.md` gives it.
///
/// A struct rather than variant fields for the same reason [`Check`] is one:
/// five flags inline makes every `match` on [`Invocation`] unreadable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MachineInit {
    /// Emit the envelope.
    pub json: bool,
    /// Skip the prompt and clone the guild from here.
    pub guild: Option<String>,
    /// Skip the prompt and unpack this bundle.
    pub bundle: Option<String>,
    /// Take the default answer to every interview question.
    pub defaults: bool,
    /// Re-run against an existing `~/.armada/`.
    pub force: bool,
}

/// One of Guild's verbs, and its own flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildInvocation {
    /// `armada guild init`.
    Init {
        /// Emit the envelope.
        json: bool,
        /// Where to import an existing setup from.
        from: Option<String>,
        /// Start empty.
        no_import: bool,
        /// Set the sync remote without being asked.
        remote: Option<String>,
        /// Take every default answer.
        defaults: bool,
        /// Overwrite an existing guild.
        force: bool,
    },
    /// `armada guild push`.
    Push {
        /// Emit the envelope.
        json: bool,
        /// Force-push. Refused unless the remote is strictly behind.
        force: bool,
    },
    /// `armada guild pull`.
    ///
    /// **Takes nothing but `--json`.** Pulling is not a decision with options;
    /// the decisions are what to do when it will not fast-forward, and those
    /// are reported rather than flagged (`guild/pull.md`).
    Pull {
        /// Emit the envelope.
        json: bool,
    },
    /// `armada guild export`.
    Export {
        /// Emit the envelope.
        json: bool,
        /// Where to write.
        out: Option<String>,
        /// Include `machine.yml`. **Off by default** — the whole point of that
        /// file is that it does not travel.
        include_secrets: bool,
    },
    /// `armada guild import`.
    Import {
        /// Emit the envelope.
        json: bool,
        /// The bundle. Required.
        path: String,
        /// Merge rather than replace; conflicts are reported and skipped.
        merge: bool,
        /// Replace an existing guild.
        force: bool,
    },
}

impl GuildInvocation {
    /// Whether this invocation asked for the envelope.
    pub fn json(&self) -> bool {
        match self {
            GuildInvocation::Init { json, .. }
            | GuildInvocation::Push { json, .. }
            | GuildInvocation::Pull { json }
            | GuildInvocation::Export { json, .. }
            | GuildInvocation::Import { json, .. } => *json,
        }
    }
}

/// Which layer of the bootstrap sandwich `config` was asked for (PLAN.md §5).
///
/// **Two subcommands with one purpose between them**: let an agent produce a
/// working config for a repository it has never seen, with no human in the
/// loop. Layer 2 — the authoring — is deliberately not a subcommand, because it
/// is not Armada's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSub {
    /// Layer 1: report facts, decide nothing.
    Scan,
    /// Layer 3: pass 1 static, then pass 2 for real.
    Verify,
}

/// A parse failure, carrying the `--json` the parser had **already seen** when
/// it failed.
///
/// Every verb takes `--json` (PLAN.md §3), including the ones this phase
/// answers with "not built yet" — so a failure before a verb exists still has
/// to answer in the envelope. The flag rides out with the error rather than
/// being re-scanned from argv by the caller, because a second scan would be a
/// second grammar: it cannot tell Armada's own `--json` from one belonging to a
/// `commands:` child, which is the distinction this whole module exists to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFailure {
    /// What went wrong.
    pub error: ArmadaError,
    /// How to report it.
    pub json: bool,
    /// And, when it is reported as text, whether to paint it. Carried out for
    /// the same reason `json` is: re-scanning argv would be a second grammar.
    pub color: ColorChoice,
}

/// `armada manifest check`, with the flags PLAN.md §3.2 gives it.
///
/// A struct rather than eight variant fields, because `check` takes more flags
/// than every other verb put together and a variant that wide makes every match
/// on `Invocation` unreadable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Check {
    /// Emit the envelope rather than human output.
    pub json: bool,
    /// Change nothing; report what would run.
    pub dry_run: bool,
    /// The bare positional, if there was one.
    pub selector: Option<String>,
    /// `--component NAME`.
    pub component: Option<String>,
    /// `--files a.py b.py`.
    pub files: Vec<String>,
    /// `--all-files`: scope from each component's `match:` globs rather than
    /// from the diff.
    pub all_files: bool,
    /// `--fix`: run `fix:` instead of `cmd:`.
    pub fix: bool,
    /// `--concurrency N`: this run's CPU budget, overriding the machine's.
    pub jobs: Option<u32>,
    /// `--wait`: queue for the run lease instead of failing fast.
    pub wait: bool,
}

/// The flags more than one verb takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Common {
    /// Emit the envelope rather than human output.
    pub json: bool,
    /// Change nothing; report what would happen.
    pub dry_run: bool,
    /// How wide to reach.
    pub lens: Lens,
}

/// The verbs Manifest owns. A `commands:` entry may not shadow one — the schema
/// rejects that, because without the rule a repo can silently break the one
/// guarantee the project exists to provide.
pub const BUILTIN_VERBS: [&str; 11] = [
    "init",
    "up",
    "down",
    "check",
    "clean",
    "status",
    "config",
    "skills",
    "render",
    "agents-md",
    "explain",
];

/// The module names, plus the top-level verbs, that `armada` claims and has
/// **not** built.
///
/// They are claimed for the same reason [`BUILTIN_VERBS`] are: a name that is
/// going to mean one thing must not mean something else for a release first.
/// Each carries the milestone that builds it (PHASES.md §8).
///
/// **M2 emptied three rows out of this table** — `init`, `doctor` and `guild`
/// are built, and moved to [`TOP_LEVEL_VERBS`] and [`GUILD_VERBS`]. **M3's first
/// third emptied a fourth**: `fleet` is built, and its verbs are in
/// [`FLEET_VERBS`].
pub const RESERVED_TOP_LEVEL: [(&str, &str); 2] = [
    ("helm", "M3 — the one agent you do talk to"),
    ("bridge", "M3 — the live screen"),
];

/// Fleet's verbs, and what each is for.
///
/// **All six are built.** Fleet is usable from a shell before the MCP server or
/// Helm exists, which is the whole point of building it first (PHASES.md §8.5).
pub const FLEET_VERBS: [(&str, &str); 6] = [
    ("spawn", "start an isolated agent Job on a task"),
    (
        "ls",
        "what is running, what it has spent, and who needs you",
    ),
    (
        "board",
        "take a Job over yourself: worktree and resume command",
    ),
    (
        "answer",
        "give a waiting Job your decision; let it continue",
    ),
    ("inbox", "what the fleet needs from you"),
    ("kill", "end a Job and release everything it owns"),
];

/// The top-level verbs that are built, and what each is for.
///
/// **`init` here is a different verb from `manifest init`, and the help says
/// so**: this one sets up *you, here*; that one claims a workspace.
pub const TOP_LEVEL_VERBS: [(&str, &str); 2] = [
    (
        "init",
        "set up this machine: checks, ~/.armada/, and your guild",
    ),
    ("doctor", "what this machine is missing, and what fixes it"),
];

/// Guild's verbs, built and unbuilt alike.
///
/// The unbuilt two are claimed for the same reason every other reserved name
/// is, and they answer by name rather than as an unknown verb — a caller told
/// "unknown" would go looking for a typo.
pub const GUILD_VERBS: [(&str, &str); 7] = [
    ("init", "build your guild by interviewing you"),
    ("pull", "bring this machine's guild up to date"),
    ("push", "send local guild changes to the remote"),
    ("export", "write the whole guild to one portable file"),
    ("import", "restore a guild from a bundle"),
    ("edit", "M2 — open a guild file, validate it, commit it"),
    ("verify", "M2 — cross-check every workflow, skill and scope"),
];

/// The Guild verbs this milestone built. The rest answer "not built yet".
const GUILD_BUILT: [&str; 5] = ["init", "pull", "push", "export", "import"];

/// The verbs with a help page of their own — the ones Manifest has built.
///
/// A separate list from [`BUILTIN_VERBS`], because that one claims names,
/// several of which answer "not built yet": giving `armada manifest up --help` a
/// page would promise a verb that does not exist.
const BUILT_PAGES: [&str; 8] = [
    "init", "up", "down", "status", "check", "clean", "config", "skills",
];

/// `--help` in either spelling.
fn is_help(arg: &str) -> bool {
    arg == "--help" || arg == "-h"
}

/// Parse an argument vector, excluding `argv[0]`.
///
/// **`--color` is settled even on the failure path.** A line that Armada refuses
/// still has its refusal rendered, and rendering it in a colour the caller
/// turned off is the same bug as ignoring `--json` on a parse error.
pub fn parse(args: &[String]) -> Result<Parsed, ParseFailure> {
    let mut color = ColorChoice::default();
    match parse_into(args, &mut color) {
        Ok(invocation) => Ok(Parsed { invocation, color }),
        Err(mut failure) => {
            failure.color = color;
            Err(failure)
        }
    }
}

fn parse_into(args: &[String], color: &mut ColorChoice) -> Result<Invocation, ParseFailure> {
    let mut json = false;
    let mut index = 0;

    // Global flags come first, before the module. After a `commands:` name
    // nothing is Armada's, so this is the only place a global flag can be given
    // for a dispatched command — and that is stated in the help rather than
    // inferred from position.
    while index < args.len() {
        match args[index].as_str() {
            "--version" | "-V" => return Ok(Invocation::Version),
            "--help" | "-h" => return Ok(Invocation::Help(Topic::Root)),
            "--json" => {
                json = true;
                index += 1;
            }
            flag if flag == "--color" || flag.starts_with("--color=") => {
                let (choice, consumed) = color_value(args, index).map_err(|e| failure(e, json))?;
                *color = choice;
                index += consumed;
            }
            flag if flag.starts_with('-') => return Err(failure(unknown_flag(flag), json)),
            _ => break,
        }
    }

    // **Bare `armada` is its own page**, not `--help`. Both list the same
    // things, but only one of them is the moment of orientation the wordmark
    // belongs to (`docs/commands/render.md`) — a banner above the page you
    // reached for because you are in a hurry is a banner in the way.
    let Some(module) = args.get(index) else {
        return Ok(Invocation::Help(Topic::Bare));
    };

    if module != "manifest" {
        let name = module.as_str();
        if name.starts_with('-') {
            return Err(failure(unknown_flag(name), json));
        }
        let rest = &args[index + 1..];
        // **The two machine verbs and the Guild module, before the fallthrough.**
        // `armada init` is a different verb from `armada manifest init` and the
        // grammar keeps them apart by the level they sit at, not by a prefix.
        match name {
            "init" => return machine_init(rest, json, color),
            "doctor" => return doctor(rest, json, color),
            "guild" => return guild(rest, json, color),
            "fleet" => return fleet(rest, json, color),
            _ => {}
        }
        let json = json || rest.iter().any(|a| a == "--json");
        let message = match RESERVED_TOP_LEVEL.iter().find(|(n, _)| *n == name) {
            Some((_, milestone)) => format!("`armada {name}` is not built yet — {milestone}"),
            None => format!("unknown command `{name}`"),
        };
        return Err(failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: name.to_string(),
                message,
                next_action: Some(
                    "`armada --help` lists the modules and verbs that are built".to_string(),
                ),
            },
            json,
        ));
    }

    // A module with no verb is as incomplete as a bare `armada`, and gets that
    // module's page rather than an error.
    let Some(verb) = args.get(index + 1) else {
        return Ok(Invocation::Help(Topic::Manifest));
    };
    if is_help(verb) {
        return Ok(Invocation::Help(Topic::Manifest));
    }
    let rest = &args[index + 2..];

    // **A page per verb, and only for a verb Armada owns.** `armada manifest
    // worktrees --help` is the *child's* `--help`, exactly as its `--dry-run` is
    // the child's — the rule this whole module exists to keep (PLAN.md §4.5).
    if let Some(page) = BUILT_PAGES
        .iter()
        .find(|name| *name == verb)
        .filter(|_| rest.iter().any(|arg| is_help(arg)))
    {
        return Ok(Invocation::Help(Topic::Verb(page)));
    }

    match verb.as_str() {
        "init" => Ok(Invocation::Init(common(rest, json, color, &["--dry-run"])?)),
        "up" => services(rest, json, color, armada_core::lifecycle::Direction::Up),
        "down" => services(rest, json, color, armada_core::lifecycle::Direction::Down),
        "status" => {
            let common = common(rest, json, color, &[])?;
            if common.dry_run {
                return Err(failure(
                    ArmadaError {
                        class: ErrClass::BadInvocation,
                        r#where: "status".to_string(),
                        message: "`armada manifest status` reads; there is nothing to dry-run"
                            .to_string(),
                        next_action: Some("drop --dry-run".to_string()),
                    },
                    common.json,
                ));
            }
            Ok(Invocation::Status(common))
        }
        "check" => Ok(Invocation::Check(Box::new(check(rest, json, color)?))),
        "config" => config(rest, json, color),
        "skills" => skills(rest, json, color),
        "clean" => {
            let common = common(
                rest,
                json,
                color,
                &[
                    "--dry-run",
                    "--artifacts",
                    "--orphaned",
                    "--force",
                    "--force-rebuild",
                ],
            )?;
            Ok(Invocation::Clean {
                common,
                artifacts: rest.iter().any(|a| a == "--artifacts"),
                orphaned: rest.iter().any(|a| a == "--orphaned"),
                force: rest.iter().any(|a| a == "--force"),
                force_rebuild: rest.iter().any(|a| a == "--force-rebuild"),
            })
        }
        // Every built-in name is claimed, including the ones this phase does
        // not implement. Otherwise `armada manifest check` in a repo declaring a `check:`
        // command would dispatch to it — and the one guarantee the project
        // exists to provide is that the verbs mean the same thing everywhere.
        name if BUILTIN_VERBS.contains(&name) => Err(failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: verb.clone(),
                message: format!("`armada manifest {verb}` is not built yet"),
                next_action: Some(
                    "phase 2 ships init, clean and status, plus the repo's own commands:"
                        .to_string(),
                ),
            },
            // The name is a built-in, so the rest is Armada's own argv and not a
            // child's: `armada manifest check --json` asks for the envelope just as
            // `armada --json manifest check` does.
            json || rest.iter().any(|a| a == "--json"),
        )),
        name if name.starts_with('-') => Err(failure(unknown_flag(name), json)),
        name => Ok(Invocation::Dispatch {
            name: name.to_string(),
            argv: rest.to_vec(),
            json,
        }),
    }
}

/// `check`'s own parser.
///
/// **Not routed through [`common`]**, because `check` takes no scope lens: a run
/// is this workspace's by definition, and accepting `--project` silently would
/// let a caller believe they had asked for something Armada never does.
fn check(rest: &[String], json: bool, color: &mut ColorChoice) -> Result<Check, ParseFailure> {
    let mut parsed = Check {
        // Settled before the loop, so that how a failure is *reported* does not
        // depend on where in the line the offending flag sits: `armada manifest check
        // --detach --json` and `armada manifest check --json --detach` are the same
        // failure. `common` does the same, for the same reason.
        json: json || rest.iter().any(|a| a == "--json"),
        ..Default::default()
    };
    // The same rule, for the same reason, on the other rendering flag.
    *color = color_in(rest, *color).map_err(|e| failure(e, parsed.json))?;
    let mut positionals: Vec<String> = Vec::new();
    let mut index = 0;

    while index < rest.len() {
        let arg = rest[index].as_str();
        index += 1;
        match arg {
            "--json" => parsed.json = true,
            // Already read by `color_in` above; here only to consume its value
            // so it is not mistaken for a positional.
            "--color" => index += 1,
            flag if flag.starts_with("--color=") => {}
            "--dry-run" => parsed.dry_run = true,
            "--all-files" => parsed.all_files = true,
            "--fix" => parsed.fix = true,
            "--wait" => parsed.wait = true,
            // A list, so it consumes until the next flag. `--files` exists
            // precisely for names a shell would mangle as positionals.
            "--files" => {
                while index < rest.len() && !rest[index].starts_with("--") {
                    parsed.files.push(rest[index].clone());
                    index += 1;
                }
                if parsed.files.is_empty() {
                    return Err(failure(needs_a_value("--files"), parsed.json));
                }
            }
            "--component" => match rest.get(index) {
                Some(name) if !name.starts_with("--") => {
                    parsed.component = Some(name.clone());
                    index += 1;
                }
                _ => return Err(failure(needs_a_value("--component"), parsed.json)),
            },
            "--concurrency" => match rest.get(index).and_then(|n| n.parse::<u32>().ok()) {
                Some(jobs) if jobs > 0 => {
                    parsed.jobs = Some(jobs);
                    index += 1;
                }
                _ => return Err(failure(needs_a_value("--concurrency"), parsed.json)),
            },
            // Reserved by PLAN.md §3 and not built in this phase. Refused by
            // name rather than falling through to "unknown flag", because the
            // flag *is* known and the honest answer is that Armada cannot do it
            // yet — an agent told "unknown flag" would go looking for a typo.
            "--detach" | "--status" => {
                return Err(failure(
                    ArmadaError {
                        class: ErrClass::BadInvocation,
                        r#where: arg.to_string(),
                        message: format!("`{arg}` is not built yet"),
                        next_action: Some(
                            "run `armada manifest check` in the foreground; `--wait` queues behind another run"
                                .to_string(),
                        ),
                    },
                    parsed.json,
                ));
            }
            flag if flag.starts_with('-') => return Err(failure(unknown_flag(flag), parsed.json)),
            word => positionals.push(word.to_string()),
        }
    }

    // **One selector, or several paths.** Two bare words that are not paths are
    // two different questions — `armada manifest check api lint` might mean the component
    // or the check — and guessing is the thing §3.2's grammar exists to avoid.
    match positionals.len() {
        0 => {}
        1 => parsed.selector = positionals.pop(),
        _ if positionals
            .iter()
            .all(|word| word.contains('/') || word.contains('.')) =>
        {
            parsed.files.extend(positionals);
        }
        _ => {
            return Err(failure(
                ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: positionals.join(" "),
                    message: "`armada manifest check` takes one selector, or several paths"
                        .to_string(),
                    next_action: Some(
                        "`armada manifest check <component>:<check>`, or `--files a.py b.py`"
                            .to_string(),
                    ),
                },
                parsed.json,
            ))
        }
    }

    Ok(parsed)
}

// ---------------------------------------------------------------- M2: the
// machine, and the guild

/// A small flag reader for the M2 verbs.
///
/// **Shared rather than five near-copies**, because five hand-written flag
/// loops is five places to forget that `--json` and `--color` are Armada's
/// everywhere. `takes_value` names the flags whose next argument is theirs;
/// everything else is a switch, and an unrecognised flag is refused rather than
/// ignored.
struct Flags {
    /// Switches that were present.
    switches: Vec<String>,
    /// Flags with values, in the order given.
    values: Vec<(String, String)>,
    /// Bare words, in order.
    positionals: Vec<String>,
    /// Whether the envelope was asked for.
    json: bool,
}

impl Flags {
    fn on(&self, flag: &str) -> bool {
        self.switches.iter().any(|s| s == flag)
    }

    fn value(&self, flag: &str) -> Option<String> {
        self.values
            .iter()
            .find(|(name, _)| name == flag)
            .map(|(_, value)| value.clone())
    }

    /// Every occurrence, for a flag that repeats — `--budget k=v` is the one.
    fn every(&self, flag: &str) -> Vec<String> {
        self.values
            .iter()
            .filter(|(name, _)| name == flag)
            .map(|(_, value)| value.clone())
            .collect()
    }
}

fn flags(
    rest: &[String],
    json: bool,
    color: &mut ColorChoice,
    r#where: &str,
    allowed: &[&str],
    takes_value: &[&str],
) -> Result<Flags, ParseFailure> {
    // Settled before the loop, so that how a failure is *reported* does not
    // depend on where in the line the offending flag sits — the same rule
    // `common` and `check` follow.
    let json = json || rest.iter().any(|a| a == "--json");
    *color = color_in(rest, *color).map_err(|e| failure(e, json))?;

    let mut parsed = Flags {
        switches: Vec::new(),
        values: Vec::new(),
        positionals: Vec::new(),
        json,
    };
    let mut index = 0;
    while index < rest.len() {
        let arg = rest[index].as_str();
        index += 1;
        match arg {
            "--json" => {}
            // Already read by `color_in`; here only to consume its value so it
            // is not mistaken for a positional.
            "--color" => index += 1,
            flag if flag.starts_with("--color=") => {}
            flag if takes_value.contains(&flag) => match rest.get(index) {
                Some(value) if !value.starts_with("--") => {
                    parsed.values.push((flag.to_string(), value.clone()));
                    index += 1;
                }
                _ => return Err(failure(needs_a_value(flag), json)),
            },
            flag if allowed.contains(&flag) => parsed.switches.push(flag.to_string()),
            flag if flag.starts_with('-') => return Err(failure(unknown_flag(flag), json)),
            word => parsed.positionals.push(word.to_string()),
        }
    }
    let _ = r#where;
    Ok(parsed)
}

/// `armada init` — **this machine**.
fn machine_init(
    rest: &[String],
    json: bool,
    color: &mut ColorChoice,
) -> Result<Invocation, ParseFailure> {
    let parsed = flags(
        rest,
        json,
        color,
        "init",
        &["--defaults", "--force"],
        &["--guild", "--bundle"],
    )?;
    let guild = parsed.value("--guild");
    let bundle = parsed.value("--bundle");
    // **Mutually exclusive, and refused rather than ordered.** Both name where
    // an existing guild comes from, and picking one for the caller would be
    // guessing at the question the flag exists to answer.
    if guild.is_some() && bundle.is_some() {
        return Err(failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: "--guild --bundle".to_string(),
                message: "--guild and --bundle are two different sources for one guild".to_string(),
                next_action: Some("pass one of them".to_string()),
            },
            parsed.json,
        ));
    }
    Ok(Invocation::MachineInit(Box::new(MachineInit {
        json: parsed.json,
        guild,
        bundle,
        defaults: parsed.on("--defaults"),
        force: parsed.on("--force"),
    })))
}

fn doctor(
    rest: &[String],
    json: bool,
    color: &mut ColorChoice,
) -> Result<Invocation, ParseFailure> {
    let parsed = flags(rest, json, color, "doctor", &["--fix"], &[])?;
    Ok(Invocation::Doctor {
        json: parsed.json,
        fix: parsed.on("--fix"),
    })
}

/// `armada guild <verb>`.
///
/// **The module name is a level of the grammar**, exactly as `manifest` is, so
/// a bare `armada guild` is as incomplete as a bare `armada` and gets that
/// module's page rather than an error.
fn guild(rest: &[String], json: bool, color: &mut ColorChoice) -> Result<Invocation, ParseFailure> {
    let Some(verb) = rest.first() else {
        return Ok(Invocation::Help(Topic::Guild));
    };
    if is_help(verb) {
        return Ok(Invocation::Help(Topic::Guild));
    }
    let tail = &rest[1..];
    let name = verb.as_str();

    if !GUILD_BUILT.contains(&name) {
        let json = json || tail.iter().any(|a| a == "--json");
        let message = match GUILD_VERBS.iter().find(|(n, _)| *n == name) {
            Some((_, summary)) => format!("`armada guild {name}` is not built yet — {summary}"),
            None => format!("unknown verb `armada guild {name}`"),
        };
        return Err(failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: format!("guild {name}"),
                message,
                next_action: Some("`armada guild --help` lists what is built".to_string()),
            },
            json,
        ));
    }

    let invocation = match name {
        "init" => {
            let parsed = flags(
                tail,
                json,
                color,
                "guild init",
                &["--no-import", "--defaults", "--force"],
                &["--from", "--remote"],
            )?;
            GuildInvocation::Init {
                json: parsed.json,
                from: parsed.value("--from"),
                no_import: parsed.on("--no-import"),
                remote: parsed.value("--remote"),
                defaults: parsed.on("--defaults"),
                force: parsed.on("--force"),
            }
        }
        "pull" => {
            let parsed = flags(tail, json, color, "guild pull", &[], &[])?;
            GuildInvocation::Pull { json: parsed.json }
        }
        "push" => {
            let parsed = flags(tail, json, color, "guild push", &["--force"], &[])?;
            GuildInvocation::Push {
                json: parsed.json,
                force: parsed.on("--force"),
            }
        }
        "export" => {
            let parsed = flags(
                tail,
                json,
                color,
                "guild export",
                &["--include-secrets"],
                &["--out"],
            )?;
            GuildInvocation::Export {
                json: parsed.json,
                out: parsed.value("--out"),
                include_secrets: parsed.on("--include-secrets"),
            }
        }
        _ => {
            let parsed = flags(
                tail,
                json,
                color,
                "guild import",
                &["--merge", "--force"],
                &[],
            )?;
            let Some(path) = parsed.positionals.first().cloned() else {
                return Err(failure(
                    ArmadaError {
                        class: ErrClass::BadInvocation,
                        r#where: "guild import".to_string(),
                        message: "`armada guild import` needs the bundle to import".to_string(),
                        next_action: Some("`armada guild import ./guild.tar.zst`".to_string()),
                    },
                    parsed.json,
                ));
            };
            GuildInvocation::Import {
                json: parsed.json,
                path,
                merge: parsed.on("--merge"),
                force: parsed.on("--force"),
            }
        }
    };
    Ok(Invocation::Guild(Box::new(invocation)))
}

/// `armada fleet <verb>`.
///
/// **The module name is a level of the grammar**, exactly as `manifest` and
/// `guild` are, so a bare `armada fleet` is as incomplete as a bare `armada` and
/// gets that module's page rather than an error.
fn fleet(rest: &[String], json: bool, color: &mut ColorChoice) -> Result<Invocation, ParseFailure> {
    let Some(verb) = rest.first() else {
        return Ok(Invocation::Help(Topic::Fleet));
    };
    if is_help(verb) {
        return Ok(Invocation::Help(Topic::Fleet));
    }
    let tail = &rest[1..];
    let name = verb.as_str();

    if !FLEET_VERBS.iter().any(|(known, _)| *known == name) {
        let json = json || tail.iter().any(|a| a == "--json");
        return Err(failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: format!("fleet {name}"),
                message: format!("unknown verb `armada fleet {name}`"),
                next_action: Some("`armada fleet --help` lists the six".to_string()),
            },
            json,
        ));
    }

    let invocation = match name {
        "spawn" => {
            let parsed = flags(
                tail,
                json,
                color,
                "fleet spawn",
                &["--dry-run"],
                &["--workflow", "--name", "--budget", "-C"],
            )?;
            // **The task is required and is not defaulted.** A `spawn` with no
            // task would classify an empty string and burn a worktree, a port
            // block and a model call on a Job nobody described.
            let Some(task) = one_positional(
                &parsed,
                "fleet spawn",
                "the task to work on",
                "`armada fleet spawn \"add rate limiting to the API\"`",
            )?
            else {
                return Err(needs_positional(
                    "fleet spawn",
                    "`armada fleet spawn` needs a task",
                    "`armada fleet spawn \"add rate limiting to the API\"`",
                    parsed.json,
                ));
            };
            FleetInvocation::Spawn(Box::new(Spawn {
                json: parsed.json,
                task,
                workflow: parsed.value("--workflow"),
                name: parsed.value("--name"),
                budget: parsed.every("--budget"),
                at: parsed.value("-C"),
                dry_run: parsed.on("--dry-run"),
            }))
        }
        "ls" => {
            let parsed = flags(
                tail,
                json,
                color,
                "fleet ls",
                &["--all", "--needs-attention"],
                &[],
            )?;
            FleetInvocation::Ls {
                json: parsed.json,
                all: parsed.on("--all"),
                needs_attention: parsed.on("--needs-attention"),
            }
        }
        "board" => {
            // `--print` is the default and is accepted so that writing it out
            // is not an error — a caller being explicit about a default should
            // never be refused.
            let parsed = flags(
                tail,
                json,
                color,
                "fleet board",
                &["--print", "--exec"],
                &[],
            )?;
            let Some(job) = one_positional(
                &parsed,
                "fleet board",
                "which Job",
                "`armada fleet ls` lists them",
            )?
            else {
                return Err(needs_positional(
                    "fleet board",
                    "`armada fleet board` needs a Job",
                    "`armada fleet ls` lists them",
                    parsed.json,
                ));
            };
            FleetInvocation::Board {
                json: parsed.json,
                job,
                exec: parsed.on("--exec"),
            }
        }
        "kill" => {
            let parsed = flags(
                tail,
                json,
                color,
                "fleet kill",
                &["--keep-branch", "--keep-worktree", "--all-finished"],
                &[],
            )?;
            let all_finished = parsed.on("--all-finished");
            let job = one_positional(
                &parsed,
                "fleet kill",
                "which Job",
                "`armada fleet ls` lists them",
            )?;
            // **One or the other, and refused rather than ordered.** Naming a
            // Job *and* `--all-finished` asks two different questions, and
            // picking one for the caller could kill four Jobs they did not name.
            if job.is_some() == all_finished {
                return Err(failure(
                    ArmadaError {
                        class: ErrClass::BadInvocation,
                        r#where: "fleet kill".to_string(),
                        message: match all_finished {
                            true => "`armada fleet kill` takes a Job or --all-finished, not both"
                                .to_string(),
                            false => {
                                "`armada fleet kill` needs a Job, or --all-finished".to_string()
                            }
                        },
                        next_action: Some("`armada fleet ls --all` lists them".to_string()),
                    },
                    parsed.json,
                ));
            }
            FleetInvocation::Kill {
                json: parsed.json,
                job,
                // **`--keep-worktree` implies `--keep-branch`.** A directory
                // left behind whose branch was deleted is a worktree pointing
                // at nothing, which is worse than either half on its own.
                keep_branch: parsed.on("--keep-branch") || parsed.on("--keep-worktree"),
                keep_worktree: parsed.on("--keep-worktree"),
                all_finished,
            }
        }
        "answer" => {
            let parsed = flags(tail, json, color, "fleet answer", &[], &[])?;
            match parsed.positionals.as_slice() {
                [job, answer] => FleetInvocation::Answer {
                    json: parsed.json,
                    job: job.clone(),
                    answer: answer.clone(),
                },
                _ => {
                    return Err(needs_positional(
                        "fleet answer",
                        "`armada fleet answer` needs a Job and what to tell it",
                        "`armada fleet answer nightly-flake \"yes, raise it to 90s\"`",
                        parsed.json,
                    ))
                }
            }
        }
        _ => {
            let parsed = flags(tail, json, color, "fleet inbox", &["--all"], &["--job"])?;
            FleetInvocation::Inbox {
                json: parsed.json,
                job: parsed.value("--job"),
                all: parsed.on("--all"),
            }
        }
    };
    Ok(Invocation::Fleet(Box::new(invocation)))
}

/// The one bare word a Fleet verb takes, or `None`.
///
/// Two is refused rather than joined: `armada fleet answer` took a quoted string
/// and lost its quotes is the failure this catches, and guessing that two words
/// were meant as one sentence would make the refusal impossible.
fn one_positional(
    parsed: &Flags,
    r#where: &str,
    what: &str,
    next_action: &str,
) -> Result<Option<String>, ParseFailure> {
    match parsed.positionals.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some(only.clone())),
        several => Err(failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: r#where.to_string(),
                message: format!(
                    "`armada {where}` takes {what}, and was given {}",
                    several.len()
                ),
                next_action: Some(next_action.to_string()),
            },
            parsed.json,
        )),
    }
}

fn needs_positional(r#where: &str, message: &str, next_action: &str, json: bool) -> ParseFailure {
    failure(
        ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: r#where.to_string(),
            message: message.to_string(),
            next_action: Some(next_action.to_string()),
        },
        json,
    )
}

/// `armada manifest up [<selector>]` and `armada manifest down [<selector>]`.
///
/// **No scope lens, and that is deliberate.** `--project` and `--all` reach
/// workspaces that are not this one, and starting another agent's services is
/// the opposite of what the flat-siblings model promises (PLAN.md §2.2). They
/// are refused by [`only_flags`] like any other unknown flag.
///
/// **`down` takes no `--dry-run`.** Its preview would be *"the services this
/// workspace declares"*, which `armada manifest status` already answers and
/// answers better, because it probes the ports rather than reading the config.
fn services(
    rest: &[String],
    json: bool,
    color: &mut ColorChoice,
    direction: armada_core::lifecycle::Direction,
) -> Result<Invocation, ParseFailure> {
    let json = json || rest.iter().any(|a| a == "--json");
    *color = color_in(rest, *color).map_err(|e| failure(e, json))?;

    let up = direction == armada_core::lifecycle::Direction::Up;
    let allowed: &[&str] = if up { &["--dry-run"] } else { &[] };
    only_flags(rest, json, allowed)?;

    let words = positional(rest);
    let verb = if up { "up" } else { "down" };
    if words.len() > 1 {
        return Err(failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: words.join(" "),
                message: format!("`armada manifest {verb}` takes one component, or none"),
                next_action: Some(format!(
                    "`armada manifest {verb}` acts on every service; \
                     `armada manifest {verb} <component>` on one"
                )),
            },
            json,
        ));
    }

    Ok(Invocation::Services {
        direction,
        selector: words.into_iter().next(),
        json,
        dry_run: up && rest.iter().any(|a| a == "--dry-run"),
    })
}

/// `armada manifest config <scan|verify>`.
///
/// **The subcommand is required and is not defaulted.** `scan` reports and
/// `verify` runs the check suite for real; guessing which one a bare `config`
/// meant would, on the wrong guess, be a full build nobody asked for.
fn config(
    rest: &[String],
    json: bool,
    color: &mut ColorChoice,
) -> Result<Invocation, ParseFailure> {
    let json = json || rest.iter().any(|a| a == "--json");
    *color = color_in(rest, *color).map_err(|e| failure(e, json))?;

    let sub = match positional(rest).first().map(String::as_str) {
        Some("scan") => ConfigSub::Scan,
        Some("verify") => ConfigSub::Verify,
        other => {
            return Err(failure(
                ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: other.unwrap_or("config").to_string(),
                    message: match other {
                        Some(word) => {
                            format!("`armada manifest config {word}` is not a subcommand")
                        }
                        None => "`armada manifest config` needs a subcommand".to_string(),
                    },
                    next_action: Some(
                        "`armada manifest config scan` reports evidence; \
                         `armada manifest config verify` validates a written one"
                            .to_string(),
                    ),
                },
                json,
            ))
        }
    };
    only_flags(rest, json, &[])?;
    Ok(Invocation::Config { sub, json })
}

/// `armada manifest skills`, and `armada manifest skills show <name>`.
///
/// **There is no `run`, and its absence is the design** (PLAN.md §4.8). "Add a
/// migration" has no deterministic expansion, and a runner would mean Armada
/// choosing arguments on the user's behalf.
fn skills(
    rest: &[String],
    json: bool,
    color: &mut ColorChoice,
) -> Result<Invocation, ParseFailure> {
    let json = json || rest.iter().any(|a| a == "--json");
    *color = color_in(rest, *color).map_err(|e| failure(e, json))?;
    let words = positional(rest);

    let show = match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => None,
        ["show", name] => Some((*name).to_string()),
        ["show"] => {
            return Err(failure(
                ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: "show".to_string(),
                    message: "`armada manifest skills show` needs a skill name".to_string(),
                    next_action: Some(
                        "`armada manifest skills` lists the names this repo declares".to_string(),
                    ),
                },
                json,
            ))
        }
        _ => {
            return Err(failure(
                ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: words.join(" "),
                    message: "`armada manifest skills` takes nothing, or `show <name>`".to_string(),
                    next_action: Some(
                        "there is deliberately no way to run a skill (PLAN.md §4.8)".to_string(),
                    ),
                },
                json,
            ))
        }
    };
    only_flags(rest, json, &[])?;
    Ok(Invocation::Skills { show, json })
}

/// The bare words of a verb's own argv, with the flags Armada owns removed.
fn positional(rest: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < rest.len() {
        let arg = rest[index].as_str();
        index += 1;
        match arg {
            "--color" => index += 1,
            flag if flag.starts_with('-') => {}
            word => out.push(word.to_string()),
        }
    }
    out
}

/// Refuse any flag beyond the shared ones and `allowed`.
fn only_flags(rest: &[String], json: bool, allowed: &[&str]) -> Result<(), ParseFailure> {
    for arg in rest {
        let arg = arg.as_str();
        if !arg.starts_with('-')
            || arg == "--json"
            || arg == "--color"
            || arg.starts_with("--color=")
            || allowed.contains(&arg)
        {
            continue;
        }
        return Err(failure(unknown_flag(arg), json));
    }
    Ok(())
}

fn needs_a_value(flag: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: flag.to_string(),
        message: format!("`{flag}` needs a value"),
        next_action: Some("`armada --help` lists what each verb takes".to_string()),
    }
}

fn common(
    rest: &[String],
    json: bool,
    color: &mut ColorChoice,
    allowed: &[&str],
) -> Result<Common, ParseFailure> {
    let mut common = Common {
        // Settled before the loop, so that how a failure is *reported* does not
        // depend on where in the line the offending flag sits: `armada manifest init
        // --turbo --json` and `armada manifest init --json --turbo` are the same failure.
        json: json || rest.iter().any(|a| a == "--json"),
        ..Default::default()
    };
    // The same rule, for the same reason, on the other rendering flag.
    *color = color_in(rest, *color).map_err(|e| failure(e, common.json))?;
    let mut project = false;
    let mut all = false;
    let mut index = 0;

    while index < rest.len() {
        let arg = rest[index].as_str();
        index += 1;
        match arg {
            "--json" => {}
            // Already read by `color_in` above; here only to consume its value.
            "--color" => index += 1,
            flag if flag.starts_with("--color=") => {}
            "--project" => project = true,
            "--all" => all = true,
            "--dry-run" if allowed.contains(&"--dry-run") => common.dry_run = true,
            flag if allowed.contains(&flag) => {}
            other => return Err(failure(unknown_flag(other), common.json)),
        }
    }

    common.lens = Lens::from_flags(project, all).ok_or_else(|| {
        failure(
            ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: "scope".to_string(),
                message: "--project and --all are two different scopes".to_string(),
                next_action: Some("pass one of them".to_string()),
            },
            common.json,
        )
    })?;
    Ok(common)
}

fn failure(error: ArmadaError, json: bool) -> ParseFailure {
    // The colour is stamped on by [`parse`] once the whole line has been read,
    // so that a `--color` sitting after the offending flag still counts.
    ParseFailure {
        error,
        json,
        color: ColorChoice::default(),
    }
}

/// The last `--color` in a verb's own flags, or `current` if it names none.
///
/// **Scanned before the flag loop**, for the same reason `--json` is: how a
/// failure is *rendered* must not depend on where in the line the offending flag
/// sits.
fn color_in(rest: &[String], current: ColorChoice) -> Result<ColorChoice, ArmadaError> {
    let mut found = current;
    let mut index = 0;
    while index < rest.len() {
        let arg = rest[index].as_str();
        if arg == "--color" || arg.starts_with("--color=") {
            let (choice, consumed) = color_value(rest, index)?;
            found = choice;
            index += consumed;
        } else {
            index += 1;
        }
    }
    Ok(found)
}

/// `--color never` and `--color=never` are one flag with two spellings.
///
/// Returns the choice and how many argv slots it used, so a caller stepping
/// through a line does not have to know which spelling it just saw.
fn color_value(args: &[String], index: usize) -> Result<(ColorChoice, usize), ArmadaError> {
    let (word, consumed) = match args[index].split_once('=') {
        Some((_, value)) => (value, 1),
        None => match args.get(index + 1) {
            Some(value) if !value.starts_with('-') => (value.as_str(), 2),
            _ => return Err(bad_color(None)),
        },
    };
    match ColorChoice::parse(word) {
        Some(choice) => Ok((choice, consumed)),
        None => Err(bad_color(Some(word))),
    }
}

fn bad_color(given: Option<&str>) -> ArmadaError {
    let values = ColorChoice::VALUES.join(", ");
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: "--color".to_string(),
        message: match given {
            Some(word) => format!("`--color {word}` is not one of: {values}"),
            None => format!("`--color` needs one of: {values}"),
        },
        next_action: Some(
            "`--color auto` is the default: colour at a terminal, none through a pipe".to_string(),
        ),
    }
}

fn unknown_flag(flag: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: flag.to_string(),
        message: format!("unknown flag `{flag}`"),
        next_action: Some("`armada --help` lists what each verb takes".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bare_armada_is_help_rather_than_an_error() {
        assert_eq!(
            parse(&[]).unwrap().invocation,
            Invocation::Help(Topic::Bare)
        );
    }

    #[test]
    fn init_takes_json_and_dry_run() {
        let Invocation::Init(common) = parse(&args(&["manifest", "init", "--json", "--dry-run"]))
            .unwrap()
            .invocation
        else {
            panic!()
        };
        assert!(common.json && common.dry_run);
    }

    #[test]
    fn the_scope_lens_is_the_same_flag_on_status_and_clean() {
        let Invocation::Status(status) = parse(&args(&["manifest", "status", "--project"]))
            .unwrap()
            .invocation
        else {
            panic!()
        };
        let Invocation::Clean { common, .. } = parse(&args(&["manifest", "clean", "--project"]))
            .unwrap()
            .invocation
        else {
            panic!()
        };
        assert_eq!(status.lens, Lens::Project);
        assert_eq!(common.lens, Lens::Project);
    }

    #[test]
    fn project_and_all_together_is_bad_invocation() {
        let err = parse(&args(&["manifest", "status", "--project", "--all"]))
            .unwrap_err()
            .error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert_eq!(err.class.exit_code(), 2);
    }

    #[test]
    fn an_unknown_flag_is_bad_invocation_and_says_where_to_look() {
        let err = parse(&args(&["manifest", "init", "--turbo"]))
            .unwrap_err()
            .error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.next_action.is_some());
    }

    /// The done-when: subcommands and flags reach the child untouched. Note
    /// `--dry-run` here is the child's, even though Armada defines a flag by
    /// that name.
    #[test]
    fn a_dispatched_commands_entry_keeps_every_argument_it_was_given() {
        let parsed = parse(&args(&[
            "manifest",
            "worktrees",
            "prune",
            "--dry-run",
            "--",
            "-x",
        ]))
        .unwrap()
        .invocation;
        assert_eq!(
            parsed,
            Invocation::Dispatch {
                name: "worktrees".to_string(),
                argv: args(&["prune", "--dry-run", "--", "-x"]),
                json: false,
            }
        );
    }

    #[test]
    fn a_global_json_before_the_command_name_is_armadas() {
        let Invocation::Dispatch { name, argv, json } =
            parse(&args(&["--json", "manifest", "worktrees", "--json"]))
                .unwrap()
                .invocation
        else {
            panic!()
        };
        assert!(json, "the leading one is Armada's");
        assert_eq!(name, "worktrees");
        assert_eq!(argv, args(&["--json"]), "the trailing one is the child's");
    }

    #[test]
    fn a_verb_that_is_not_built_yet_says_so_rather_than_dispatching_it() {
        let err = parse(&args(&["manifest", "explain"])).unwrap_err().error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.message.contains("not built yet"));
    }

    /// A failure before a verb exists still has to be reportable in the
    /// envelope, so the `--json` the parser saw rides out with the error —
    /// however it was spelled, and wherever the offending flag sits.
    #[test]
    fn a_parse_failure_carries_out_the_json_it_had_already_seen() {
        for words in [
            &["--json", "manifest", "explain"][..],
            &["manifest", "explain", "--json"][..],
            &["--json", "manifest", "init", "--turbo"][..],
            &["manifest", "init", "--turbo", "--json"][..],
        ] {
            let failure = parse(&args(words)).unwrap_err();
            assert!(failure.json, "`armada {}` lost --json", words.join(" "));
            assert_eq!(failure.error.class, ErrClass::BadInvocation);
        }
        assert!(!parse(&args(&["manifest", "explain"])).unwrap_err().json);
    }

    /// Every built-in name is claimed, including the ones phase 2 does not
    /// implement — otherwise `armada manifest check` in a repo declaring a `check:`
    /// command would dispatch to it and mean something different here than
    /// everywhere else.
    #[test]
    fn no_builtin_verb_can_be_reached_by_dispatch() {
        for verb in BUILTIN_VERBS {
            let parsed = parse(&args(&["manifest", verb]));
            assert!(
                !matches!(
                    parsed,
                    Ok(Parsed {
                        invocation: Invocation::Dispatch { .. },
                        ..
                    })
                ),
                "`{verb}` dispatched"
            );
        }
    }

    #[test]
    fn status_refuses_a_dry_run_because_it_changes_nothing() {
        assert!(parse(&args(&["manifest", "status", "--dry-run"])).is_err());
    }

    #[test]
    fn config_takes_one_of_its_two_subcommands() {
        for (word, expected) in [("scan", ConfigSub::Scan), ("verify", ConfigSub::Verify)] {
            let Invocation::Config { sub, json } = parse(&args(&["manifest", "config", word]))
                .unwrap()
                .invocation
            else {
                panic!("`config {word}` did not parse")
            };
            assert_eq!(sub, expected);
            assert!(!json);
        }
        assert!(
            parse(&args(&["manifest", "config", "scan", "--json"]))
                .unwrap()
                .invocation
                == Invocation::Config {
                    sub: ConfigSub::Scan,
                    json: true
                }
        );
    }

    /// **The subcommand is required and is not defaulted.** `verify` runs the
    /// check suite for real, so guessing which one a bare `config` meant would,
    /// on the wrong guess, be a full build nobody asked for.
    #[test]
    fn a_bare_config_is_refused_rather_than_defaulted() {
        for words in [
            &["manifest", "config"][..],
            &["manifest", "config", "validate"][..],
        ] {
            let err = parse(&args(words)).unwrap_err().error;
            assert_eq!(err.class, ErrClass::BadInvocation);
            assert!(err.next_action.unwrap().contains("config scan"));
        }
    }

    #[test]
    fn config_refuses_a_flag_it_does_not_take() {
        let failure = parse(&args(&["manifest", "config", "scan", "--all", "--json"])).unwrap_err();
        assert_eq!(failure.error.class, ErrClass::BadInvocation);
        assert!(failure.json, "the envelope is still owed an answer");
    }

    #[test]
    fn skills_lists_by_default_and_resolves_one_with_show() {
        assert_eq!(
            parse(&args(&["manifest", "skills"])).unwrap().invocation,
            Invocation::Skills {
                show: None,
                json: false
            }
        );
        assert_eq!(
            parse(&args(&[
                "manifest",
                "skills",
                "show",
                "add-migration",
                "--json"
            ]))
            .unwrap()
            .invocation,
            Invocation::Skills {
                show: Some("add-migration".to_string()),
                json: true
            }
        );
    }

    /// **There is deliberately no way to run a skill** (PLAN.md §4.8). "Add a
    /// migration" has no deterministic expansion, and a runner would mean
    /// Armada choosing arguments on the user's behalf. The parser is where a
    /// third subcommand would have to appear, so this is where its absence is
    /// asserted.
    #[test]
    fn there_is_no_third_subcommand_that_runs_a_skill() {
        for words in [
            &["manifest", "skills", "run", "add-migration"][..],
            &["manifest", "skills", "add-migration"][..],
            &["manifest", "skills", "show"][..],
        ] {
            let err = parse(&args(words)).unwrap_err().error;
            assert_eq!(
                err.class,
                ErrClass::BadInvocation,
                "`armada {}` was accepted",
                words.join(" ")
            );
            assert!(err.next_action.is_some());
        }
    }

    /// The module name is a grammar level, so a bare module is as incomplete as
    /// a bare `armada` and gets the same answer rather than an error.
    #[test]
    fn a_module_with_no_verb_is_help() {
        assert_eq!(
            parse(&args(&["manifest"])).unwrap().invocation,
            Invocation::Help(Topic::Manifest)
        );
    }

    /// A module that does not exist yet says which milestone builds it, rather
    /// than falling through to Manifest's `commands:` dispatch — which is what
    /// would silently give a repo the power to define `armada fleet`.
    #[test]
    fn a_module_that_is_not_built_yet_names_its_milestone() {
        for (name, _) in RESERVED_TOP_LEVEL {
            let err = parse(&args(&[name])).unwrap_err().error;
            assert_eq!(err.class, ErrClass::BadInvocation);
            assert!(err.message.contains("not built yet"), "`{name}`");
        }
    }

    /// `--color` is Armada's wherever Armada's own flags are read: before the
    /// module, and among a built-in verb's flags. Both spellings, both places.
    #[test]
    fn color_is_read_before_the_module_and_among_a_verbs_own_flags() {
        for words in [
            &["--color", "never", "manifest", "status"][..],
            &["--color=never", "manifest", "status"][..],
            &["manifest", "status", "--color", "never"][..],
            &["manifest", "status", "--color=never"][..],
        ] {
            assert_eq!(
                parse(&args(words)).unwrap().color,
                ColorChoice::Never,
                "`armada {}` lost --color",
                words.join(" ")
            );
        }
        assert_eq!(
            parse(&args(&["manifest", "status"])).unwrap().color,
            ColorChoice::Auto,
            "auto is the default"
        );
    }

    /// A `commands:` child's `--color` is the child's, exactly as its `--json`
    /// is. Nothing after the entry's name is Armada's.
    #[test]
    fn a_color_after_a_commands_name_belongs_to_the_child() {
        let parsed = parse(&args(&["manifest", "worktrees", "--color", "always"])).unwrap();
        assert_eq!(parsed.color, ColorChoice::Auto, "Armada's own is untouched");
        let Invocation::Dispatch { argv, .. } = parsed.invocation else {
            panic!()
        };
        assert_eq!(argv, args(&["--color", "always"]));
    }

    /// A refusal is still rendered, so the flag that decides how rides out with
    /// it — including when it sits *after* the flag that caused the refusal.
    #[test]
    fn a_parse_failure_carries_out_the_color_it_had_already_seen() {
        for words in [
            &["--color", "never", "manifest", "explain"][..],
            &["manifest", "init", "--turbo", "--color", "never"][..],
            &["manifest", "check", "--detach", "--color", "never"][..],
        ] {
            assert_eq!(
                parse(&args(words)).unwrap_err().color,
                ColorChoice::Never,
                "`armada {}` lost --color",
                words.join(" ")
            );
        }
    }

    /// A value that is not one of the three is a bad invocation rather than a
    /// silent fallback to `auto` — a caller who typed `--color yes` asked a
    /// question, and answering it with the default is how they never find out.
    #[test]
    fn an_unrecognised_color_is_refused_and_lists_the_three() {
        for words in [
            &["--color", "yes", "manifest", "status"][..],
            &["manifest", "status", "--color=yes"][..],
            &["manifest", "status", "--color"][..],
        ] {
            let err = parse(&args(words)).unwrap_err().error;
            assert_eq!(err.class, ErrClass::BadInvocation);
            assert_eq!(err.r#where, "--color");
            assert!(
                err.message.contains("auto, always, never"),
                "{}",
                err.message
            );
        }
    }

    /// Only Manifest dispatches to a repo's `commands:`. A bare word at the top
    /// level is a module name, and an unknown one is an unknown command.
    #[test]
    fn a_repo_command_is_not_reachable_without_its_module() {
        let err = parse(&args(&["worktrees", "prune"])).unwrap_err().error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.message.contains("unknown command"), "{}", err.message);
    }

    // ------------------------------------------------------------------ M2

    /// **`armada init` and `armada manifest init` are two different verbs**,
    /// and the grammar keeps them apart by the level they sit at. One claims a
    /// workspace; the other sets up the machine.
    #[test]
    fn the_two_inits_are_two_verbs_at_two_levels() {
        assert!(matches!(
            parse(&args(&["init"])).unwrap().invocation,
            Invocation::MachineInit(_)
        ));
        assert!(matches!(
            parse(&args(&["manifest", "init"])).unwrap().invocation,
            Invocation::Init(_)
        ));
    }

    #[test]
    fn init_takes_the_flags_its_page_documents() {
        let Invocation::MachineInit(init) = parse(&args(&[
            "init",
            "--guild",
            "git@example.com:me/guild.git",
            "--defaults",
            "--force",
            "--json",
        ]))
        .unwrap()
        .invocation
        else {
            panic!()
        };
        assert_eq!(init.guild.as_deref(), Some("git@example.com:me/guild.git"));
        assert!(init.defaults && init.force && init.json);
    }

    /// **Two sources for one guild is refused, not ordered.** Picking one for
    /// the caller would be guessing at the question the flag exists to answer.
    #[test]
    fn a_remote_and_a_bundle_together_is_bad_invocation() {
        let err = parse(&args(&["init", "--guild", "u", "--bundle", "b"]))
            .unwrap_err()
            .error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert_eq!(err.class.exit_code(), 2);
    }

    /// The module name is a grammar level, so a bare `armada guild` is as
    /// incomplete as a bare `armada` and gets that module's page.
    #[test]
    fn a_bare_guild_is_its_module_page() {
        assert_eq!(
            parse(&args(&["guild"])).unwrap().invocation,
            Invocation::Help(Topic::Guild)
        );
        assert_eq!(
            parse(&args(&["guild", "--help"])).unwrap().invocation,
            Invocation::Help(Topic::Guild)
        );
    }

    #[test]
    fn every_built_guild_verb_parses_with_its_own_flags() {
        for (line, expected) in [
            (vec!["guild", "pull"], GuildInvocation::Pull { json: false }),
            (
                vec!["guild", "push", "--force"],
                GuildInvocation::Push {
                    json: false,
                    force: true,
                },
            ),
            (
                vec!["guild", "export", "--include-secrets"],
                GuildInvocation::Export {
                    json: false,
                    out: None,
                    include_secrets: true,
                },
            ),
            (
                vec!["guild", "import", "./g.tar.zst", "--merge"],
                GuildInvocation::Import {
                    json: false,
                    path: "./g.tar.zst".to_string(),
                    merge: true,
                    force: false,
                },
            ),
        ] {
            let Invocation::Guild(parsed) = parse(&args(&line)).unwrap().invocation else {
                panic!("`armada {}` did not parse as a guild verb", line.join(" "))
            };
            assert_eq!(*parsed, expected, "`armada {}`", line.join(" "));
        }
    }

    /// **`pull` takes nothing but `--json`.** Pulling is not a decision with
    /// options; the decisions are what to do when it will not fast-forward, and
    /// those are reported rather than flagged.
    #[test]
    fn pull_refuses_a_flag_it_does_not_have() {
        let err = parse(&args(&["guild", "pull", "--force"]))
            .unwrap_err()
            .error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.message.contains("--force"), "{}", err.message);
    }

    /// `import` needs the bundle, and says which invocation it wanted rather
    /// than which flag was missing.
    #[test]
    fn import_without_a_bundle_says_what_it_needed() {
        let err = parse(&args(&["guild", "import"])).unwrap_err().error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.next_action.unwrap().contains("guild.tar.zst"));
    }

    /// **An unbuilt Guild verb answers by name.** A caller told "unknown" would
    /// go looking for a typo — the same rule `manifest up` already follows.
    #[test]
    fn an_unbuilt_guild_verb_names_itself_rather_than_reading_as_a_typo() {
        for verb in ["edit", "verify"] {
            let err = parse(&args(&["guild", verb])).unwrap_err().error;
            assert!(err.message.contains("not built yet"), "`{verb}`");
            assert!(err.message.contains(verb), "`{verb}`");
        }
        let unknown = parse(&args(&["guild", "frobnicate"])).unwrap_err().error;
        assert!(unknown.message.contains("unknown verb"), "{unknown:?}");
    }

    /// Every M2 verb answers `--json`, including on the failure path — the same
    /// guarantee every other verb makes.
    #[test]
    fn the_machine_verbs_carry_json_wherever_it_is_spelled() {
        for line in [
            &["--json", "doctor"][..],
            &["doctor", "--json"][..],
            &["--json", "guild", "pull"][..],
            &["guild", "pull", "--json"][..],
        ] {
            let parsed = parse(&args(line)).unwrap().invocation;
            let json = match parsed {
                Invocation::Doctor { json, .. } => json,
                Invocation::Guild(guild) => guild.json(),
                _ => panic!("`armada {}` did not parse", line.join(" ")),
            };
            assert!(json, "`armada {}` lost --json", line.join(" "));
        }
        let failure = parse(&args(&["guild", "edit", "--json"])).unwrap_err();
        assert!(
            failure.json,
            "a refusal lost the --json it had already seen"
        );
    }

    /// Nothing claimed here is claimed twice: a name that is built must not
    /// also answer "not built yet".
    #[test]
    fn no_name_is_both_built_and_reserved() {
        for (built, _) in TOP_LEVEL_VERBS {
            assert!(
                !RESERVED_TOP_LEVEL.iter().any(|(name, _)| *name == built),
                "`{built}` is both built and reserved"
            );
        }
        for built in GUILD_BUILT {
            let (_, summary) = GUILD_VERBS
                .iter()
                .find(|(name, _)| *name == built)
                .unwrap_or_else(|| panic!("`guild {built}` is built and unlisted"));
            assert!(
                !summary.starts_with("M2 "),
                "`guild {built}` is built and listed as unbuilt"
            );
        }
    }

    // ------------------------------------------------------------------ Fleet

    fn fleet_verb(words: &[&str]) -> FleetInvocation {
        match parse(&args(words)).unwrap().invocation {
            Invocation::Fleet(fleet) => *fleet,
            other => panic!("{words:?} did not parse as a Fleet verb: {other:?}"),
        }
    }

    /// **The module name is a level of the grammar**, so a bare `armada fleet`
    /// is as incomplete as a bare `armada` and gets that module's page.
    #[test]
    fn a_bare_fleet_is_the_module_page_rather_than_an_error() {
        assert_eq!(
            parse(&args(&["fleet"])).unwrap().invocation,
            Invocation::Help(Topic::Fleet)
        );
        assert_eq!(
            parse(&args(&["fleet", "--help"])).unwrap().invocation,
            Invocation::Help(Topic::Fleet)
        );
    }

    #[test]
    fn spawn_takes_a_task_and_four_ways_to_override_the_plan() {
        let FleetInvocation::Spawn(spawn) = fleet_verb(&[
            "fleet",
            "spawn",
            "add rate limiting",
            "--workflow",
            "feature",
            "--name",
            "rate-limit",
            "--budget",
            "max_tokens=200000",
            "--budget",
            "max_wall_clock=45m",
            "-C",
            "../api",
            "--json",
        ]) else {
            panic!("not a spawn")
        };
        assert_eq!(spawn.task, "add rate limiting");
        assert_eq!(spawn.workflow.as_deref(), Some("feature"));
        assert_eq!(spawn.name.as_deref(), Some("rate-limit"));
        // **Repeatable, and every occurrence survives.** A `--budget` that kept
        // only the last one would silently drop a ceiling the caller raised.
        assert_eq!(spawn.budget, ["max_tokens=200000", "max_wall_clock=45m"]);
        assert_eq!(spawn.at.as_deref(), Some("../api"));
        assert!(spawn.json);
    }

    /// **A `spawn` with no task is refused.** Classifying an empty string would
    /// burn a worktree, a port block and a model call on a Job nobody described.
    #[test]
    fn spawn_without_a_task_is_refused() {
        let failure = parse(&args(&["fleet", "spawn"])).unwrap_err();
        assert_eq!(failure.error.class, ErrClass::BadInvocation);
        assert!(failure.error.next_action.unwrap().contains("fleet spawn"));
    }

    /// Two bare words is two questions. `armada fleet spawn add rate limiting`
    /// is a quoted string that lost its quotes, and guessing that they were one
    /// sentence would make the refusal impossible.
    #[test]
    fn spawn_takes_one_task_and_not_several_words() {
        assert!(parse(&args(&["fleet", "spawn", "add", "rate", "limiting"])).is_err());
    }

    #[test]
    fn ls_takes_its_two_lenses() {
        assert_eq!(
            fleet_verb(&["fleet", "ls", "--all", "--needs-attention"]),
            FleetInvocation::Ls {
                json: false,
                all: true,
                needs_attention: true,
            }
        );
    }

    #[test]
    fn board_takes_one_job_and_the_two_ways_to_enter_it() {
        assert_eq!(
            fleet_verb(&["fleet", "board", "rate-limit", "--exec"]),
            FleetInvocation::Board {
                json: false,
                job: "rate-limit".to_string(),
                exec: true,
            }
        );
        // `--print` is the default; a caller being explicit about it is never
        // refused.
        assert!(parse(&args(&["fleet", "board", "rate-limit", "--print"])).is_ok());
        assert!(parse(&args(&["fleet", "board"])).is_err());
    }

    /// **`--keep-worktree` implies `--keep-branch`.** A directory left behind
    /// whose branch was deleted is a worktree pointing at nothing.
    #[test]
    fn keeping_a_worktree_keeps_its_branch_without_being_asked_twice() {
        let FleetInvocation::Kill {
            keep_branch,
            keep_worktree,
            ..
        } = fleet_verb(&["fleet", "kill", "rate-limit", "--keep-worktree"])
        else {
            panic!("not a kill")
        };
        assert!(keep_worktree);
        assert!(keep_branch);
    }

    /// **A Job or `--all-finished`, and refused rather than ordered.** Naming a
    /// Job *and* the flag asks two questions, and picking one could kill four
    /// Jobs the caller did not name.
    #[test]
    fn kill_takes_a_job_or_all_finished_and_never_both_or_neither() {
        assert!(matches!(
            fleet_verb(&["fleet", "kill", "--all-finished"]),
            FleetInvocation::Kill { job: None, .. }
        ));
        for line in [
            args(&["fleet", "kill"]),
            args(&["fleet", "kill", "rate-limit", "--all-finished"]),
        ] {
            let failure = parse(&line).unwrap_err();
            assert_eq!(failure.error.class, ErrClass::BadInvocation, "{line:?}");
        }
    }

    #[test]
    fn answer_takes_a_job_and_what_to_tell_it() {
        assert_eq!(
            fleet_verb(&["fleet", "answer", "nightly-flake", "yes, raise it to 90s"]),
            FleetInvocation::Answer {
                json: false,
                job: "nightly-flake".to_string(),
                answer: "yes, raise it to 90s".to_string(),
            }
        );
        assert!(parse(&args(&["fleet", "answer", "nightly-flake"])).is_err());
    }

    #[test]
    fn inbox_takes_a_job_filter_and_the_answered_lens() {
        assert_eq!(
            fleet_verb(&["fleet", "inbox", "--job", "flake", "--all"]),
            FleetInvocation::Inbox {
                json: false,
                job: Some("flake".to_string()),
                all: true,
            }
        );
    }

    /// **`--json` is answered wherever it sits**, including before the module,
    /// because how a failure is reported must not depend on where in the line
    /// the flag was typed.
    #[test]
    fn every_fleet_verb_answers_the_envelope_flag_from_either_side() {
        for verb in FLEET_VERBS.map(|(name, _)| name) {
            let tail: &[&str] = match verb {
                "spawn" => &["a task"],
                "board" | "kill" => &["rate-limit"],
                "answer" => &["rate-limit", "go on"],
                _ => &[],
            };
            let mut before = args(&["--json", "fleet", verb]);
            before.extend(tail.iter().map(|s| s.to_string()));
            let mut after = args(&["fleet", verb]);
            after.extend(tail.iter().map(|s| s.to_string()));
            after.push("--json".to_string());

            for line in [before, after] {
                let Invocation::Fleet(fleet) = parse(&line).unwrap().invocation else {
                    panic!("{line:?} is not a Fleet verb")
                };
                assert!(fleet.json(), "{line:?} lost --json");
            }
        }
    }

    /// A verb Fleet does not have is a typo, and the message says where the six
    /// are listed rather than leaving the caller to guess.
    #[test]
    fn an_unknown_fleet_verb_is_refused_by_name() {
        let failure = parse(&args(&["fleet", "restart"])).unwrap_err();
        assert_eq!(failure.error.r#where, "fleet restart");
        assert!(failure.error.next_action.unwrap().contains("--help"));
    }

    /// **`fleet` has left the reserved table**, and nothing that is built may
    /// still be claimed as unbuilt — that is the one way this table goes wrong.
    #[test]
    fn no_built_module_is_still_listed_as_reserved() {
        for (name, _) in RESERVED_TOP_LEVEL {
            assert_ne!(name, "fleet", "fleet is built and still reserved");
            assert!(parse(&args(&[name])).is_err(), "`{name}` is not built");
        }
    }

    #[test]
    fn version_and_help_win_wherever_they_appear_among_the_global_flags() {
        assert_eq!(
            parse(&args(&["--version"])).unwrap().invocation,
            Invocation::Version
        );
        assert_eq!(
            parse(&args(&["--json", "--help"])).unwrap().invocation,
            Invocation::Help(Topic::Root)
        );
    }
}
