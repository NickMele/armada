//! Argument parsing, and nothing else.
//!
//! **Hand-rolled rather than a parser crate**, for one reason that decides it:
//! a `commands:` entry's remaining argv must reach the child **untouched**
//! (PLAN.md §4.5), including flags char itself defines. `char worktrees prune
//! --dry-run` runs `… prune --dry-run`, and `--dry-run` there is the child's.
//! A general parser has to be told to stop parsing, and being told wrongly is
//! silent.
//!
//! So the grammar is stated rather than inferred:
//!
//! ```text
//! char [global flags] <verb> [verb flags]        for the verbs char owns
//! char [global flags] <name> [anything at all]   for a commands: entry
//! ```
//!
//! Everything after a `commands:` name is the child's, whatever it looks like.

use charkit_core::error::{CharError, ErrClass};
use charkit_core::scope::Lens;

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// `char --version`.
    Version,
    /// `char --help`, or `char` with nothing at all.
    Help,
    /// `char init`.
    Init(Common),
    /// `char clean`.
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
    /// `char status`.
    Status(Common),
    /// `char check`.
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
/// second grammar: it cannot tell char's own `--json` from one belonging to a
/// `commands:` child, which is the distinction this whole module exists to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFailure {
    /// What went wrong.
    pub error: CharError,
    /// How to report it.
    pub json: bool,
}

/// `char check`, with the flags PLAN.md §3.2 gives it.
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

/// The verbs char owns. A `commands:` entry may not shadow one — the schema
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

/// Parse an argument vector, excluding `argv[0]`.
pub fn parse(args: &[String]) -> Result<Invocation, ParseFailure> {
    let mut json = false;
    let mut index = 0;

    // Global flags come first, before the verb. After a `commands:` name
    // nothing is char's, so this is the only place a global flag can be given
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

    let Some(verb) = args.get(index) else {
        return Ok(Invocation::Help);
    };
    let rest = &args[index + 1..];

    match verb.as_str() {
        "init" => Ok(Invocation::Init(common(rest, json, &["--dry-run"])?)),
        "status" => {
            let common = common(rest, json, &[])?;
            if common.dry_run {
                return Err(failure(
                    CharError {
                        class: ErrClass::BadInvocation,
                        r#where: "status".to_string(),
                        message: "`char status` reads; there is nothing to dry-run".to_string(),
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
        // not implement. Otherwise `char check` in a repo declaring a `check:`
        // command would dispatch to it — and the one guarantee the project
        // exists to provide is that the verbs mean the same thing everywhere.
        name if BUILTIN_VERBS.contains(&name) => Err(failure(
            CharError {
                class: ErrClass::BadInvocation,
                r#where: verb.clone(),
                message: format!("`char {verb}` is not built yet"),
                next_action: Some(
                    "phase 2 ships init, clean and status, plus the repo's own commands:"
                        .to_string(),
                ),
            },
            // The name is a built-in, so the rest is char's own argv and not a
            // child's: `char check --json` asks for the envelope just as
            // `char --json check` does.
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
/// let a caller believe they had asked for something char never does.
fn check(rest: &[String], json: bool) -> Result<Check, ParseFailure> {
    let mut parsed = Check {
        // Settled before the loop, so that how a failure is *reported* does not
        // depend on where in the line the offending flag sits: `char check
        // --detach --json` and `char check --json --detach` are the same
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
            // flag *is* known and the honest answer is that char cannot do it
            // yet — an agent told "unknown flag" would go looking for a typo.
            "--detach" | "--status" => {
                return Err(failure(
                    CharError {
                        class: ErrClass::BadInvocation,
                        r#where: arg.to_string(),
                        message: format!("`{arg}` is not built yet"),
                        next_action: Some(
                            "run `char check` in the foreground; `--wait` queues behind another run"
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
    // two different questions — `char check api lint` might mean the component
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
                    message: "`char check` takes one selector, or several paths".to_string(),
                    next_action: Some(
                        "`char check <component>:<check>`, or `--files a.py b.py`".to_string(),
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
        next_action: Some("`char --help` lists what each verb takes".to_string()),
    }
}

fn common(rest: &[String], json: bool, allowed: &[&str]) -> Result<Common, ParseFailure> {
    let mut common = Common {
        // Settled before the loop, so that how a failure is *reported* does not
        // depend on where in the line the offending flag sits: `char init
        // --turbo --json` and `char init --json --turbo` are the same failure.
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
        next_action: Some("`char --help` lists what each verb takes".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bare_char_is_help_rather_than_an_error() {
        assert_eq!(parse(&[]).unwrap(), Invocation::Help);
    }

    #[test]
    fn init_takes_json_and_dry_run() {
        let Invocation::Init(common) = parse(&args(&["init", "--json", "--dry-run"])).unwrap()
        else {
            panic!()
        };
        assert!(common.json && common.dry_run);
    }

    #[test]
    fn the_scope_lens_is_the_same_flag_on_status_and_clean() {
        let Invocation::Status(status) = parse(&args(&["status", "--project"])).unwrap() else {
            panic!()
        };
        let Invocation::Clean { common, .. } = parse(&args(&["clean", "--project"])).unwrap()
        else {
            panic!()
        };
        assert_eq!(status.lens, Lens::Project);
        assert_eq!(common.lens, Lens::Project);
    }

    #[test]
    fn project_and_all_together_is_bad_invocation() {
        let err = parse(&args(&["status", "--project", "--all"]))
            .unwrap_err()
            .error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert_eq!(err.class.exit_code(), 2);
    }

    #[test]
    fn an_unknown_flag_is_bad_invocation_and_says_where_to_look() {
        let err = parse(&args(&["init", "--turbo"])).unwrap_err().error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.next_action.is_some());
    }

    /// The done-when: subcommands and flags reach the child untouched. Note
    /// `--dry-run` here is the child's, even though char defines a flag by
    /// that name.
    #[test]
    fn a_dispatched_commands_entry_keeps_every_argument_it_was_given() {
        let parsed = parse(&args(&["worktrees", "prune", "--dry-run", "--", "-x"])).unwrap();
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
    fn a_global_json_before_the_command_name_is_chars() {
        let Invocation::Dispatch { name, argv, json } =
            parse(&args(&["--json", "worktrees", "--json"])).unwrap()
        else {
            panic!()
        };
        assert!(json, "the leading one is char's");
        assert_eq!(name, "worktrees");
        assert_eq!(argv, args(&["--json"]), "the trailing one is the child's");
    }

    #[test]
    fn a_verb_that_is_not_built_yet_says_so_rather_than_dispatching_it() {
        let err = parse(&args(&["up"])).unwrap_err().error;
        assert_eq!(err.class, ErrClass::BadInvocation);
        assert!(err.message.contains("not built yet"));
    }

    /// A failure before a verb exists still has to be reportable in the
    /// envelope, so the `--json` the parser saw rides out with the error —
    /// however it was spelled, and wherever the offending flag sits.
    #[test]
    fn a_parse_failure_carries_out_the_json_it_had_already_seen() {
        for words in [
            &["--json", "up"][..],
            &["up", "--json"][..],
            &["--json", "init", "--turbo"][..],
            &["init", "--turbo", "--json"][..],
        ] {
            let failure = parse(&args(words)).unwrap_err();
            assert!(failure.json, "`char {}` lost --json", words.join(" "));
            assert_eq!(failure.error.class, ErrClass::BadInvocation);
        }
        assert!(!parse(&args(&["up"])).unwrap_err().json);
    }

    /// Every built-in name is claimed, including the ones phase 2 does not
    /// implement — otherwise `char check` in a repo declaring a `check:`
    /// command would dispatch to it and mean something different here than
    /// everywhere else.
    #[test]
    fn no_builtin_verb_can_be_reached_by_dispatch() {
        for verb in BUILTIN_VERBS {
            let parsed = parse(&args(&[verb]));
            assert!(
                !matches!(parsed, Ok(Invocation::Dispatch { .. })),
                "`{verb}` dispatched"
            );
        }
    }

    #[test]
    fn status_refuses_a_dry_run_because_it_changes_nothing() {
        assert!(parse(&args(&["status", "--dry-run"])).is_err());
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
