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

/// The module names, plus the two top-level verbs, that `armada` claims.
///
/// Only `manifest` is built. The rest are claimed for the same reason
/// [`BUILTIN_VERBS`] are: a name that is going to mean one thing must not mean
/// something else for a release first. Each carries the milestone that builds
/// it (PHASES.md §8).
pub const RESERVED_TOP_LEVEL: [(&str, &str); 6] = [
    (
        "init",
        "M2 — machine setup; `armada manifest init` claims a workspace",
    ),
    ("doctor", "M2 — what this machine is missing"),
    ("guild", "M2 — your portable setup"),
    ("fleet", "M3 — the agents you do not talk to"),
    ("helm", "M3 — the one agent you do talk to"),
    ("bridge", "M3 — the live screen"),
];

/// The verbs with a help page of their own — the ones Manifest has built.
///
/// A separate list from [`BUILTIN_VERBS`], because that one claims names,
/// several of which answer "not built yet": giving `armada manifest up --help` a
/// page would promise a verb that does not exist.
const BUILT_PAGES: [&str; 6] = ["init", "status", "check", "clean", "config", "skills"];

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
        let json = json || args[index + 1..].iter().any(|a| a == "--json");
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
                    "`armada manifest <verb>` is the module that is built; `armada --help` lists it"
                        .to_string(),
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
        let err = parse(&args(&["manifest", "up"])).unwrap_err().error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.message.contains("not built yet"));
    }

    /// A failure before a verb exists still has to be reportable in the
    /// envelope, so the `--json` the parser saw rides out with the error —
    /// however it was spelled, and wherever the offending flag sits.
    #[test]
    fn a_parse_failure_carries_out_the_json_it_had_already_seen() {
        for words in [
            &["--json", "manifest", "up"][..],
            &["manifest", "up", "--json"][..],
            &["--json", "manifest", "init", "--turbo"][..],
            &["manifest", "init", "--turbo", "--json"][..],
        ] {
            let failure = parse(&args(words)).unwrap_err();
            assert!(failure.json, "`armada {}` lost --json", words.join(" "));
            assert_eq!(failure.error.class, ErrClass::BadInvocation);
        }
        assert!(!parse(&args(&["manifest", "up"])).unwrap_err().json);
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
            &["--color", "never", "manifest", "up"][..],
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
