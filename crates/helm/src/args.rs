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

use armada_core::error::{CharError, ErrClass};
use armada_core::scope::Lens;

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// `armada --version`.
    Version,
    /// `armada --help`, or `armada` with nothing at all.
    Help,
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
        /// Rebuild an unreadable `char.db` from labels alone.
        force_rebuild: bool,
    },
    /// `armada manifest status`.
    Status(Common),
    /// `armada manifest check`.
    Check(Box<Check>),
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
    pub error: CharError,
    /// How to report it.
    pub json: bool,
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
    /// `--jobs N`: this run's CPU budget, overriding the machine's.
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
pub const BUILTIN_VERBS: [&str; 9] = [
    "init",
    "up",
    "down",
    "check",
    "clean",
    "status",
    "config",
    "agents-md",
    "explain",
];

/// The module names, plus the two top-level verbs, that `armada` claims.
///
/// Only `manifest` is built. The rest are claimed for the same reason
/// [`BUILTIN_VERBS`] are: a name that is going to mean one thing must not mean
/// something else for a release first. Each carries the milestone that builds
/// it (PHASES.md §8).
const RESERVED_TOP_LEVEL: [(&str, &str); 6] = [
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

/// Parse an argument vector, excluding `argv[0]`.
pub fn parse(args: &[String]) -> Result<Invocation, ParseFailure> {
    let mut json = false;
    let mut index = 0;

    // Global flags come first, before the module. After a `commands:` name
    // nothing is Armada's, so this is the only place a global flag can be given
    // for a dispatched command — and that is stated in the help rather than
    // inferred from position.
    while index < args.len() {
        match args[index].as_str() {
            "--version" | "-V" => return Ok(Invocation::Version),
            "--help" | "-h" => return Ok(Invocation::Help),
            "--json" => {
                json = true;
                index += 1;
            }
            flag if flag.starts_with('-') => return Err(failure(unknown_flag(flag), json)),
            _ => break,
        }
    }

    let Some(module) = args.get(index) else {
        return Ok(Invocation::Help);
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
            CharError {
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

    let Some(verb) = args.get(index + 1) else {
        return Ok(Invocation::Help);
    };
    let rest = &args[index + 2..];

    match verb.as_str() {
        "init" => Ok(Invocation::Init(common(rest, json, &["--dry-run"])?)),
        "status" => {
            let common = common(rest, json, &[])?;
            if common.dry_run {
                return Err(failure(
                    CharError {
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
        "check" => Ok(Invocation::Check(Box::new(check(rest, json)?))),
        "clean" => {
            let common = common(
                rest,
                json,
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
            CharError {
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
fn check(rest: &[String], json: bool) -> Result<Check, ParseFailure> {
    let mut parsed = Check {
        // Settled before the loop, so that how a failure is *reported* does not
        // depend on where in the line the offending flag sits: `armada manifest check
        // --detach --json` and `armada manifest check --json --detach` are the same
        // failure. `common` does the same, for the same reason.
        json: json || rest.iter().any(|a| a == "--json"),
        ..Default::default()
    };
    let mut positionals: Vec<String> = Vec::new();
    let mut index = 0;

    while index < rest.len() {
        let arg = rest[index].as_str();
        index += 1;
        match arg {
            "--json" => parsed.json = true,
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
            "--jobs" => match rest.get(index).and_then(|n| n.parse::<u32>().ok()) {
                Some(jobs) if jobs > 0 => {
                    parsed.jobs = Some(jobs);
                    index += 1;
                }
                _ => return Err(failure(needs_a_value("--jobs"), parsed.json)),
            },
            // Reserved by PLAN.md §3 and not built in this phase. Refused by
            // name rather than falling through to "unknown flag", because the
            // flag *is* known and the honest answer is that Armada cannot do it
            // yet — an agent told "unknown flag" would go looking for a typo.
            "--detach" | "--status" => {
                return Err(failure(
                    CharError {
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
                CharError {
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

fn needs_a_value(flag: &str) -> CharError {
    CharError {
        class: ErrClass::BadInvocation,
        r#where: flag.to_string(),
        message: format!("`{flag}` needs a value"),
        next_action: Some("`armada --help` lists what each verb takes".to_string()),
    }
}

fn common(rest: &[String], json: bool, allowed: &[&str]) -> Result<Common, ParseFailure> {
    let mut common = Common {
        // Settled before the loop, so that how a failure is *reported* does not
        // depend on where in the line the offending flag sits: `armada manifest init
        // --turbo --json` and `armada manifest init --json --turbo` are the same failure.
        json: json || rest.iter().any(|a| a == "--json"),
        ..Default::default()
    };
    let mut project = false;
    let mut all = false;

    for arg in rest {
        match arg.as_str() {
            "--json" => {}
            "--project" => project = true,
            "--all" => all = true,
            "--dry-run" if allowed.contains(&"--dry-run") => common.dry_run = true,
            flag if allowed.contains(&flag) => {}
            other => return Err(failure(unknown_flag(other), common.json)),
        }
    }

    common.lens = Lens::from_flags(project, all).ok_or_else(|| {
        failure(
            CharError {
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

fn failure(error: CharError, json: bool) -> ParseFailure {
    ParseFailure { error, json }
}

fn unknown_flag(flag: &str) -> CharError {
    CharError {
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
        assert_eq!(parse(&[]).unwrap(), Invocation::Help);
    }

    #[test]
    fn init_takes_json_and_dry_run() {
        let Invocation::Init(common) =
            parse(&args(&["manifest", "init", "--json", "--dry-run"])).unwrap()
        else {
            panic!()
        };
        assert!(common.json && common.dry_run);
    }

    #[test]
    fn the_scope_lens_is_the_same_flag_on_status_and_clean() {
        let Invocation::Status(status) =
            parse(&args(&["manifest", "status", "--project"])).unwrap()
        else {
            panic!()
        };
        let Invocation::Clean { common, .. } =
            parse(&args(&["manifest", "clean", "--project"])).unwrap()
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
        .unwrap();
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
            parse(&args(&["--json", "manifest", "worktrees", "--json"])).unwrap()
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
                !matches!(parsed, Ok(Invocation::Dispatch { .. })),
                "`{verb}` dispatched"
            );
        }
    }

    #[test]
    fn status_refuses_a_dry_run_because_it_changes_nothing() {
        assert!(parse(&args(&["manifest", "status", "--dry-run"])).is_err());
    }

    /// The module name is a grammar level, so a bare module is as incomplete as
    /// a bare `armada` and gets the same answer rather than an error.
    #[test]
    fn a_module_with_no_verb_is_help() {
        assert_eq!(parse(&args(&["manifest"])).unwrap(), Invocation::Help);
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
        assert_eq!(parse(&args(&["--version"])).unwrap(), Invocation::Version);
        assert_eq!(
            parse(&args(&["--json", "--help"])).unwrap(),
            Invocation::Help
        );
    }
}
