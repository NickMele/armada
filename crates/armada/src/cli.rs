//! What was typed, turned into one of four things it could have been.
//!
//! # Parsed by hand, and that is not a stopgap
//!
//! There is no argument-parsing dependency and no workspace dependency table to
//! add one to. Four verbs, one optional positional and one flag is less surface
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
    Serve { repository: Option<PathBuf> },
    /// One Check the Manifest declares, by name.
    Check { name: String },
    /// One Command the Manifest declares, by name.
    Run { name: String },
    /// Worktrees, branches and Jobs, given back.
    Clean { everything: bool, force: bool },
    /// What the four verbs are.
    Help,
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
        "clean",
        "give this repository's worktrees, branches and Jobs back",
    ),
];

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
        "clean" => {
            let positional = positionals(rest, &["--all", "--force"], &mut faults);
            at_most_one("clean", &positional, &mut faults);
            if let Some(given) = positional.first() {
                faults.push(Fault::CleanTakesNoPath {
                    given: given.clone(),
                });
            }
            Some(Verb::Clean {
                everything: rest.iter().any(|arg| arg == "--all"),
                force: rest.iter().any(|arg| arg == "--force"),
            })
        }
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
    /// `clean` acts on the repository the caller is standing in. A path would
    /// let somebody clean a repository they are not looking at.
    CleanTakesNoPath {
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
            Fault::CleanTakesNoPath { given } => write!(
                out,
                "`armada clean` cleans the repository you are standing in, so `{given}` \
                 is an argument it has nowhere to put"
            ),
        }
    }
}

/// The four verbs, printed after any refusal and by `armada help`.
pub struct Usage;

impl fmt::Display for Usage {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(out, "armada — Fleet, and what a repository's Manifest says")?;
        writeln!(out)?;
        for (verb, what) in VERBS {
            let shape = match *verb {
                "serve" => "serve [<path>]".to_string(),
                "clean" => "clean [--all]".to_string(),
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
