//! What was typed, turned into one of four things it could have been.
//!
//! # Parsed by hand, and that is not a stopgap
//!
//! There is no argument-parsing dependency and no workspace dependency table to
//! add one to. A handful of verbs, one optional positional and two flags is less surface
//! than the derive macro that would read it, and a crate added here is added to
//! the binary every other crate links into.
//!
//! # Every fault, and never the first one
//!
//! `config` reports every refusal in a Manifest rather than stopping, and a
//! command line is read the same way: an unknown flag beside a missing name is
//! two lines, so one correction fixes both.

use std::fmt;
use std::path::PathBuf;

/// What the caller asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    /// The daemon. Everything below it needs a port, a store and a process.
    /// `repository` is added at start; none given serves what Fleet remembers.
    Serve { repository: Option<PathBuf> },
    /// One Check the Manifest declares, by name.
    Check { name: String },
    /// One Command the Manifest declares, by name.
    Run { name: String },
    /// The Checks a change hits, from the paths read on stdin.
    Covers,
    /// Worktrees, branches and Jobs, given back.
    Clean { everything: bool, force: bool },
    /// The agent's door, spoken on stdin and stdout for an agent standing in
    /// this repository. Started by an agent's MCP client, never by a person.
    Mcp,
    /// The merge line — `docs/capabilities/merge-line.md`.
    Land(LandAct),
    /// What the verbs are.
    Help,
}

/// Which of `armada land`'s forms was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LandAct {
    /// `armada land` — join the line, and return at once.
    Join,
    /// `armada land preflight` — ready this branch, and stamp its tree.
    Preflight,
    /// `armada land --status [branch]` — where it is, from disk.
    Status { branch: Option<String> },
    /// `armada land --runner <common-git-dir>` — hidden: the detached
    /// runner's own entry point, started by `ensure_runner` rather than
    /// typed.
    Runner { common_git_dir: PathBuf },
}

/// The verbs, in the order the usage prints them, each with what it is for.
///
/// One list, read by both the parser and the usage text — a second copy is how
/// a verb gets added and stays undocumented.
const VERBS: &[(&str, &str)] = &[
    (
        "serve",
        "run Fleet against a repository until it is signalled",
    ),
    ("check", "run one Check the Manifest declares, by name"),
    ("run", "run one Command the Manifest declares, by name"),
    (
        COVERS,
        "name the Checks a change hits, from the changed paths on stdin",
    ),
    (
        "clean",
        "give this repository's worktrees, branches and Jobs back",
    ),
    (
        MCP,
        "relay this repository's agent door on stdin and stdout — an agent's client runs it",
    ),
    (LAND, "join, ready, or poll the merge line to `main`"),
];

/// The verb an agent's MCP configuration names.
///
/// **One value, read by the parser and written into the file.** The entry
/// `crate::mcp::publish` puts in a repository's `.mcp.json` names this verb, so
/// a rename that missed one of the two would publish a door nothing answers.
pub const MCP: &str = "mcp";

/// The verb that answers which Checks a change hits. `scripts/land` names it.
pub const COVERS: &str = "covers";

/// The merge line. `scripts/land` is now a thin shim over this verb.
pub const LAND: &str = "land";

/// Read the arguments after the program name.
pub fn read<I: IntoIterator<Item = String>>(args: I) -> Result<Verb, Misread> {
    let args: Vec<String> = args.into_iter().collect();
    let Some(verb) = args.first() else {
        return Err(Misread {
            faults: vec![Fault::NothingAsked],
        });
    };
    if verb == "help" || verb == "--help" || verb == "-h" {
        return Ok(Verb::Help);
    }

    let rest = &args[1..];
    let mut faults = Vec::new();
    let parsed = match verb.as_str() {
        "serve" => {
            let positional = positionals(rest, &[], &mut faults);
            at_most_one("serve", &positional, &mut faults);
            Some(Verb::Serve {
                repository: positional.first().map(PathBuf::from),
            })
        }
        "check" | "run" => {
            let positional = positionals(rest, &[], &mut faults);
            at_most_one(verb, &positional, &mut faults);
            match positional.first() {
                Some(name) if verb == "check" => Some(Verb::Check { name: name.clone() }),
                Some(name) => Some(Verb::Run { name: name.clone() }),
                None => {
                    faults.push(Fault::NoName { verb: verb.clone() });
                    None
                }
            }
        }
        COVERS => {
            let positional = positionals(rest, &[], &mut faults);
            if let Some(given) = positional.first() {
                faults.push(Fault::PathsComeOnStdin {
                    given: given.clone(),
                });
            }
            Some(Verb::Covers)
        }
        MCP => {
            let positional = positionals(rest, &[], &mut faults);
            at_most_one(MCP, &positional, &mut faults);
            if let Some(given) = positional.first() {
                faults.push(Fault::StandingTakesNoPath {
                    verb: verb.clone(),
                    given: given.clone(),
                });
            }
            Some(Verb::Mcp)
        }
        "clean" => {
            let positional = positionals(rest, &["--all", "--force"], &mut faults);
            at_most_one("clean", &positional, &mut faults);
            if let Some(given) = positional.first() {
                faults.push(Fault::StandingTakesNoPath {
                    verb: verb.clone(),
                    given: given.clone(),
                });
            }
            Some(Verb::Clean {
                everything: rest.iter().any(|arg| arg == "--all"),
                force: rest.iter().any(|arg| arg == "--force"),
            })
        }
        LAND => read_land(rest, &mut faults),
        _ => {
            faults.push(Fault::NoSuchVerb {
                given: verb.clone(),
            });
            None
        }
    };

    match parsed {
        Some(verb) if faults.is_empty() => Ok(verb),
        _ => Err(Misread { faults }),
    }
}

/// The arguments that are not flags, with every unrecognised flag recorded.
fn positionals(args: &[String], allowed: &[&str], faults: &mut Vec<Fault>) -> Vec<String> {
    let mut positional = Vec::new();
    for arg in args {
        if arg.starts_with('-') && !allowed.contains(&arg.as_str()) {
            faults.push(Fault::NoSuchFlag {
                given: arg.clone(),
                allowed: allowed.iter().map(|f| (*f).to_string()).collect(),
            });
        } else if !arg.starts_with('-') {
            positional.push(arg.clone());
        }
    }
    positional
}

/// `land`'s own shape: an optional `preflight` positional, `--status` with
/// an optional value, and the hidden `--runner <common-git-dir>` —
/// different enough from every other verb's flags (a value, not a bare
/// switch) that it is read by hand rather than through [`positionals`].
/// Precedence where more than one is given — `--runner`, then `--status`,
/// then `preflight`, then joining the line — matches `argparse`'s own
/// dispatch in `scripts/land`'s `main()`.
fn read_land(rest: &[String], faults: &mut Vec<Fault>) -> Option<Verb> {
    let mut positional = Vec::new();
    let mut status: Option<Option<String>> = None;
    let mut runner = None;
    let mut i = 0;
    while i < rest.len() {
        let arg = &rest[i];
        if arg == "--status" {
            let value = rest
                .get(i + 1)
                .filter(|next| !next.starts_with('-'))
                .cloned();
            i += usize::from(value.is_some());
            status = Some(value);
        } else if arg == "--runner" {
            match rest.get(i + 1) {
                Some(value) => {
                    runner = Some(value.clone());
                    i += 1;
                }
                None => faults.push(Fault::FlagNeedsAValue { flag: arg.clone() }),
            }
        } else if arg.starts_with('-') {
            faults.push(Fault::NoSuchFlag {
                given: arg.clone(),
                allowed: vec!["--status".to_string()],
            });
        } else {
            positional.push(arg.clone());
        }
        i += 1;
    }
    at_most_one(LAND, &positional, faults);
    if let Some(first) = positional.first() {
        if first != "preflight" {
            faults.push(Fault::LandActionUnknown {
                given: first.clone(),
            });
        }
    }

    if let Some(common_git_dir) = runner {
        return Some(Verb::Land(LandAct::Runner {
            common_git_dir: PathBuf::from(common_git_dir),
        }));
    }
    if let Some(branch) = status {
        return Some(Verb::Land(LandAct::Status {
            branch: branch.filter(|branch| !branch.is_empty()),
        }));
    }
    if positional.first().map(String::as_str) == Some("preflight") {
        return Some(Verb::Land(LandAct::Preflight));
    }
    positional.is_empty().then_some(Verb::Land(LandAct::Join))
}

fn at_most_one(verb: &str, positional: &[String], faults: &mut Vec<Fault>) {
    if positional.len() > 1 {
        faults.push(Fault::TooMany {
            verb: verb.to_string(),
            extra: positional[1..].to_vec(),
        });
    }
}

/// Everything wrong with what was typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Misread {
    pub faults: Vec<Fault>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// `armada`, with nothing after it.
    NothingAsked,
    NoSuchVerb {
        given: String,
    },
    /// `check` and `run` are named at, always. There is no default Check.
    NoName {
        verb: String,
    },
    NoSuchFlag {
        given: String,
        allowed: Vec<String>,
    },
    TooMany {
        verb: String,
        extra: Vec<String>,
    },
    /// The verb acts on the repository the caller is standing in. A path would
    /// let somebody clean a repository they are not looking at — and would let
    /// an agent ask about one it is not working in, which is the same fault one
    /// level over.
    StandingTakesNoPath {
        verb: String,
        given: String,
    },
    /// `covers` reads its paths on stdin, one per line, so a diff of any size
    /// fits and a path is never mistaken for a flag.
    PathsComeOnStdin {
        given: String,
    },
    /// `--status` or `--runner` at the end of the line, with no value after
    /// it.
    FlagNeedsAValue {
        flag: String,
    },
    /// `land`'s one positional is `preflight`; anything else named there.
    LandActionUnknown {
        given: String,
    },
}

impl fmt::Display for Misread {
    /// One line per fault, then the usage. The person reading this is at a
    /// terminal, and a set of problems folded onto one line sends them back to
    /// the shell to find out which word each one meant.
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        for fault in &self.faults {
            writeln!(out, "{fault}")?;
        }
        write!(out, "{}", Usage)
    }
}

impl std::error::Error for Misread {}

impl fmt::Display for Fault {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::NothingAsked => write!(out, "armada was asked for nothing"),
            Fault::NoSuchVerb { given } => write!(
                out,
                "`{given}` is not one of armada's verbs — they are {}",
                listed(&VERBS.iter().map(|(name, _)| *name).collect::<Vec<_>>())
            ),
            Fault::NoName { verb } => write!(
                out,
                "`armada {verb}` needs the name of one thing the Manifest declares"
            ),
            Fault::NoSuchFlag { given, allowed } if allowed.is_empty() => {
                write!(out, "`{given}` is a flag this verb does not take")
            }
            Fault::NoSuchFlag { given, allowed } => {
                let names: Vec<&str> = allowed.iter().map(String::as_str).collect();
                write!(
                    out,
                    "`{given}` is not a flag this verb takes — it takes {}",
                    listed(&names)
                )
            }
            Fault::TooMany { verb, extra } => write!(
                out,
                "`armada {verb}` takes one argument, and {} came after it",
                listed(&extra.iter().map(String::as_str).collect::<Vec<_>>())
            ),
            Fault::StandingTakesNoPath { verb, given } => write!(
                out,
                "`armada {verb}` acts on the repository you are standing in, so `{given}` \
                 is an argument it has nowhere to put"
            ),
            Fault::PathsComeOnStdin { given } => write!(
                out,
                "`armada {COVERS}` reads the changed paths on stdin, one per line, so `{given}` \
                 has nowhere to go — `git diff --name-only main | armada {COVERS}`"
            ),
            Fault::FlagNeedsAValue { flag } => {
                write!(out, "`{flag}` needs a value after it")
            }
            Fault::LandActionUnknown { given } => write!(
                out,
                "`armada {LAND} {given}` is not a form this verb takes — they are `{LAND}`, \
                 `{LAND} preflight`, `{LAND} --status [branch]`"
            ),
        }
    }
}

/// The verbs, printed after any refusal and by `armada help`.
pub struct Usage;

impl fmt::Display for Usage {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(out, "armada — Fleet, and what a repository's Manifest says")?;
        writeln!(out)?;
        for (verb, what) in VERBS {
            if *verb == LAND {
                writeln!(
                    out,
                    "  armada {LAND}                     join the merge line, and return at once"
                )?;
                writeln!(
                    out,
                    "  armada {LAND} preflight            ready this branch, and stamp its tree"
                )?;
                writeln!(
                    out,
                    "  armada {LAND} --status [<branch>]  where it is, answered from disk"
                )?;
                continue;
            }
            let shape = match *verb {
                "serve" => "serve [<path>]".to_string(),
                "clean" => "clean [--all]".to_string(),
                MCP => MCP.to_string(),
                COVERS => format!("{COVERS} < <paths>"),
                named => format!("{named} <name>"),
            };
            writeln!(out, "  armada {shape:<16}  {what}")?;
        }
        writeln!(out)?;
        writeln!(
            out,
            "  clean keeps a branch whose commits are not on the base branch, and names"
        )?;
        writeln!(
            out,
            "  it. --force deletes it, and the work on it, along with the rest."
        )?;
        Ok(())
    }
}

fn listed(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
