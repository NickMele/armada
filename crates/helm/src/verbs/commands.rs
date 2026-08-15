//! `armada manifest commands` — the verbs this repository declares.
//!
//! **The gap it closes, and it is the third of the same one.** `armada manifest
//! skills` listed `skills:`, `armada manifest components` listed `components:`,
//! and `commands:` was left: the only way to learn a repository's own verbs was
//! to open `armada.yml` and read them off. That is the question a newcomer asks
//! first — human or agent — and parsing YAML to answer it is exactly the work
//! Armada exists to remove (PLAN.md §4.5).
//!
//! **The same three columns `skills` and `components` use**, because a reader
//! who has met one has met the others. The name is what `armada manifest
//! <name>` takes; the detail is the entry's `help:`, which is the one line the
//! repository wrote to say what its verb is for.
//!
//! **How many entries can reach a secret is a count in the summary, not a
//! status word**, exactly as `skills` counts unresolved references there. It is
//! a fact about the listing rather than a state of any row, and a status column
//! that changed meaning per verb is how a status column stops being readable.
//!
//! **An entry with no `help:` shows its `cmd:` instead.** A blank cell would
//! read as a defect, and the command string is the only other thing Armada
//! knows about the entry — it is worse than a sentence and much better than
//! nothing. The `--json` payload carries both unconditionally, so an agent
//! never has to work out which one it got.
//!
//! **A read verb**, like `status`, `skills` and `components`: it takes no
//! lease, mutates nothing, and its exit code describes the query rather than
//! the repository.

use armada_core::config::Stdio;
use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{CommandView, CommandsData, Envelope, ResultRow};
use armada_core::error::{ArmadaError, Status};

use crate::app::App;
use crate::verbs::{load_config, Output};

/// List every `commands:` entry this workspace declares.
pub fn run<R: Run, C: Clock, F: Fetch>(app: &mut App<R, C, F>) -> Result<Output, ArmadaError> {
    let (workspace, config) = load_config(app)?;

    let commands: Vec<CommandView> = config
        .commands
        .iter()
        .map(|(name, entry)| CommandView {
            name: name.clone(),
            cmd: entry.cmd.clone(),
            help: entry.help.clone(),
            stdio: match entry.stdio {
                Stdio::Inherit => "inherit".to_string(),
                Stdio::Pipe => "pipe".to_string(),
            },
            secrets: entry.secrets.clone(),
        })
        .collect();

    let results = commands
        .iter()
        .map(|command| {
            // **`OK` and never a verdict**, the rule `skills` and `components`
            // already keep: listing an entry says the repository declares it,
            // not that it runs, that `argv[0]` exists, or that its grant
            // resolves. That is `armada manifest config verify`'s answer, on a
            // different command.
            let mut row = ResultRow::new(command.name.clone(), Status::Ok);
            row.reason = Some(detail(command).to_string());
            row
        })
        .collect();

    Ok(Output::Commands(Box::new(Envelope::ok(
        "commands",
        Some(workspace.id.clone()),
        Status::Ok,
        CommandsData { results, commands },
    ))))
}

/// The one line that says what an entry is for, falling back to what it runs.
///
/// **`help:` is optional in the schema and this is the cost of that.** A
/// repository that wrote one gets a sentence; one that did not gets its own
/// command string back, which at least says what will happen. Nothing is
/// invented, and no row is ever blank.
pub fn detail(command: &CommandView) -> &str {
    match command.help.as_deref() {
        Some(help) => help,
        None => command.cmd.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(name: &str, cmd: &str, help: Option<&str>) -> CommandView {
        CommandView {
            name: name.to_string(),
            cmd: cmd.to_string(),
            help: help.map(str::to_string),
            stdio: "inherit".to_string(),
            secrets: Vec::new(),
        }
    }

    /// The sentence the repository wrote wins, because it is the one that says
    /// what the verb is *for* rather than what it executes.
    #[test]
    fn the_detail_is_the_declared_help_when_there_is_one() {
        assert_eq!(
            detail(&view(
                "tickets",
                "uv run scripts/tickets.py",
                Some("Report stale tickets")
            )),
            "Report stale tickets"
        );
    }

    /// An entry with no `help:` is common and legal, and a blank cell would
    /// read as a defect rather than as an omission in the config.
    #[test]
    fn an_entry_without_help_falls_back_to_what_it_runs() {
        assert_eq!(
            detail(&view("seed", "pnpm prisma db seed", None)),
            "pnpm prisma db seed"
        );
    }
}
