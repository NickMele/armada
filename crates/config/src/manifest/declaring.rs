//! One more command in an `armada.yml` a person owns, and nothing else moved.
//!
//! **Text in, text out, and the parser is the judge.** A person's file carries
//! comments and an order that a YAML round trip throws away, so the entry is
//! written into the text where `commands:` already is. The result is read back
//! with [`Manifest::parse`] before anyone may keep it, and one that does not
//! read back declaring exactly this command is refused rather than written.

use std::fmt;
use std::path::Path;

use crate::error::LoadError;
use crate::manifest::Manifest;

/// An `armada.yml` with the command declared in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The whole file, as it should now be written.
    pub text: String,
    /// The name the command is declared under.
    pub name: String,
    /// The file already declared this exact command, so `text` is unchanged.
    pub already: bool,
}

/// Why the command could not be added.
#[derive(Debug)]
pub enum NotDeclared {
    /// The file did not read before anything was added to it.
    Unreadable(LoadError),
    /// `commands:` is written as an inline map, which cannot be extended
    /// without rewriting the person's own line.
    Inline,
    /// The command is more than one line, which a `run:` cannot hold.
    MoreThanOneLine,
    /// What would have been written does not read.
    WouldNotRead(LoadError),
    /// What would have been written reads, and does not declare this command
    /// under this name.
    Misread { name: String },
}

impl fmt::Display for NotDeclared {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotDeclared::Unreadable(cause) => {
                write!(out, "armada.yml does not read as it stands: {cause}")
            }
            NotDeclared::Inline => write!(
                out,
                "armada.yml writes `commands:` on one line, so a command cannot be \
                 added without rewriting it. Add it by hand"
            ),
            NotDeclared::MoreThanOneLine => {
                write!(out, "the command is more than one line, and a `run:` holds one")
            }
            NotDeclared::WouldNotRead(cause) => write!(
                out,
                "armada.yml would not read with the command added, so it was left \
                 as it was: {cause}"
            ),
            NotDeclared::Misread { name } => write!(
                out,
                "armada.yml read back without `commands.{name}` running this \
                 command, so it was left as it was"
            ),
        }
    }
}

impl std::error::Error for NotDeclared {}

impl Manifest {
    /// `text` with `run` declared under `commands`, under a name derived from
    /// it that nothing in the file already uses.
    ///
    /// **Declared, never destructive.** The key is not written, so it reads as
    /// false; a person allowing a command is allowing it to run unattended.
    pub fn declaring_command(path: &Path, text: &str, run: &str) -> Result<Declared, NotDeclared> {
        let run = run.trim();
        if run.contains('\n') || run.contains('\r') {
            return Err(NotDeclared::MoreThanOneLine);
        }
        let before = Manifest::parse(path, text).map_err(NotDeclared::Unreadable)?;
        let declared = before.command_names().into_iter().find(|name| {
            before
                .command(name)
                .is_some_and(|command| command.run() == run)
        });
        if let Some(name) = declared {
            return Ok(Declared {
                text: text.to_string(),
                name,
                already: true,
            });
        }
        let name = unused_name(run, &before);
        let text = inserted(text, &name, run)?;
        let after = Manifest::parse(path, &text).map_err(NotDeclared::WouldNotRead)?;
        match after.command(&name) {
            Some(command) if command.run() == run && !command.is_destructive() => Ok(Declared {
                text,
                name,
                already: false,
            }),
            _ => Err(NotDeclared::Misread { name }),
        }
    }
}

/// A name from the command's first words, numbered until neither registry has
/// it: a name in both `checks` and `commands` is refused at load.
fn unused_name(run: &str, manifest: &Manifest) -> String {
    let words: Vec<String> = run
        .split_whitespace()
        .take(3)
        .map(|word| {
            word.rsplit('/')
                .next()
                .unwrap_or(word)
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_string()
        })
        .filter(|word| !word.is_empty())
        .collect();
    let stem = if words.is_empty() {
        "allowed".to_string()
    } else {
        words.join("-")
    };
    let taken = |name: &str| manifest.command(name).is_some() || manifest.check(name).is_some();
    let mut name = stem.clone();
    let mut n = 2;
    while taken(&name) {
        name = format!("{stem}-{n}");
        n += 1;
    }
    name
}

/// The entry, written after the last line of the `commands:` block at the
/// block's own indentation, or a new block at the end of the file.
fn inserted(text: &str, name: &str, run: &str) -> Result<String, NotDeclared> {
    let entry = |indent: &str| format!("{indent}{name}:\n{indent}{indent}run: {}\n", quoted(run));
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let Some(at) = lines.iter().position(|line| line.starts_with("commands:")) else {
        let mut out = text.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\ncommands:\n");
        out.push_str(&entry("  "));
        return Ok(out);
    };
    let rest = lines[at].trim_end()["commands:".len()..].trim_start();
    if !(rest.is_empty() || rest.starts_with('#')) {
        return Err(NotDeclared::Inline);
    }
    // The block is every indented line up to the next key at column zero. A
    // comment at column zero inside it is skipped rather than read as its end.
    let mut last = at;
    let mut indent = None;
    for (i, line) in lines.iter().enumerate().skip(at + 1) {
        let bare = line.trim_end_matches(['\n', '\r']);
        if bare.trim().is_empty() {
            continue;
        }
        let lead = bare.len() - bare.trim_start().len();
        if lead == 0 {
            if bare.starts_with('#') {
                continue;
            }
            break;
        }
        if indent.is_none() && !bare.trim_start().starts_with('#') {
            indent = Some(bare[..lead].to_string());
        }
        last = i;
    }
    let indent = indent.unwrap_or_else(|| "  ".to_string());
    let mut out = String::with_capacity(text.len() + 64);
    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        if i == last {
            if !line.ends_with('\n') {
                out.push('\n');
            }
            if last != at {
                out.push('\n');
            }
            out.push_str(&entry(&indent));
        }
    }
    Ok(out)
}

/// A YAML double-quoted scalar, so no command can be read as anything else.
fn quoted(run: &str) -> String {
    format!("\"{}\"", run.replace('\\', "\\\\").replace('"', "\\\""))
}
