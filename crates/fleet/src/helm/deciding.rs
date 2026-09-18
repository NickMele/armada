//! What a Helm call is put to a person for, and what simply runs. `#1525`.
//!
//! **Fleet decides this and the person's own settings no longer do**, which
//! reverses `super::asking`'s old rule. The owner's three classes on 18 Sep
//! 2026 are the whole of it: destructive, pushes code to a shared space, writes
//! off this machine. `docs/concepts/helm.md`, *What Helm asks about*.
//!
//! **Their settings still decide what they decide.** The CLI reads them before
//! Fleet is called; what arrives here is the remainder.
//!
//! **Auto *unless*, so an unrecognised command runs.** The safety is in
//! recognising the risky set, and what runs unasked is on the record as
//! `HelmCallSettled::RanUnasked` — the audit trail is what catches a rule drawn
//! too wide.

use ipc::AskingToRun;

/// Why one call is being put to a person. **A reason and not a category**: it
/// is written to be read in an event, and on the card itself one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Because {
    /// It removes or overwrites something that does not come back.
    Destructive,
    /// It sends code somewhere other people read from.
    PushesToShared,
    /// It writes to something that is not this machine.
    WritesOffMachine,
    /// Its shape could not be read. **The one reason about this module rather
    /// than about the call**: `eval`, a backtick and a command substitution can
    /// each carry anything, and those are the lines worth getting right.
    Unreadable,
}

impl Because {
    /// The reason in a person's words, for the event and for a card.
    pub fn said(&self) -> &'static str {
        match self {
            Because::Destructive => "it removes or overwrites something",
            Because::PushesToShared => "it sends code where other people read it",
            Because::WritesOffMachine => "it writes to something off this machine",
            Because::Unreadable => "what this line does cannot be read from it",
        }
    }
}

/// The built-in tools that write a file. **Not `adapters::CHANGES_THE_CHECKOUT`**,
/// which recognises a write for the event it publishes rather than to decide
/// whether to ask about one. The two would not move together.
const WRITES_A_FILE: &[&str] = &["Write", "Edit", "NotebookEdit"];

/// Whether one call is put to a person, and why. `None` runs it.
///
/// **The tool decides which question is asked.** A door tool is one of Armada's
/// own operations, a shell line is read as one, a write is read against the
/// path it names, and anything else by its name alone.
pub fn because(asking: &AskingToRun) -> Option<Because> {
    let tool = asking.tool_name.as_str();
    if let Some(operation) = tool.strip_prefix(&format!("mcp__{}__", ipc::door::SERVER)) {
        return door(operation);
    }
    if tool == "Bash" {
        return shell(asking.detail().unwrap_or_default());
    }
    if WRITES_A_FILE.contains(&tool) {
        return writing(asking);
    }
    named(tool)
}

/// One of Armada's own operations.
///
/// **A query never asks**, since a read changes nothing anywhere. A command
/// asks only where it is one of the three: pausing a Job, adding a task,
/// writing a Studio note and starting a local run are none of them.
fn door(operation: &str) -> Option<Because> {
    const SHARED: &[&str] = &[
        "approve_dispatch",
        "approve_review",
        "merge_pull_request",
        "redispatch_job",
        "dispatch_studio_draft",
        "request_changes",
    ];
    const GONE: &[&str] = &[
        "kill_job",
        "kill_drone",
        "forget_job",
        "delete_branch",
        "delete_studio",
        "remove_studio_nodes",
        "reclaim_worktree",
        "undo_run",
        "undo_checkout_run",
        "restart_fleet",
        "save_manifest_file",
        "edit_manifest",
    ];
    const OFF_MACHINE: &[&str] = &[
        "file_finding_issue",
        "rerun_checks",
        "rerun_failed_checks",
        "rerun_gate",
        "investigate_failed_checks",
        "add_repository",
        "clone_repository",
    ];
    if SHARED.contains(&operation) {
        return Some(Because::PushesToShared);
    }
    if GONE.contains(&operation) {
        return Some(Because::Destructive);
    }
    if OFF_MACHINE.contains(&operation) {
        return Some(Because::WritesOffMachine);
    }
    None
}

/// A built-in, or a tool from a server Armada knows nothing about.
///
/// **By the verb in its name, which is all there is.** `delete_page` says what
/// it does whoever wrote it, and under *auto unless* a name carrying none of
/// these runs.
fn named(tool: &str) -> Option<Because> {
    let lowered = tool.to_ascii_lowercase();
    const GONE: &[&str] = &["delete", "remove", "destroy", "purge", "revoke"];
    const SHARED: &[&str] = &["publish", "merge", "push"];
    const OFF_MACHINE: &[&str] = &["send", "upload", "deploy", "invite"];
    if GONE.iter().any(|word| lowered.contains(word)) {
        return Some(Because::Destructive);
    }
    if SHARED.iter().any(|word| lowered.contains(word)) {
        return Some(Because::PushesToShared);
    }
    if OFF_MACHINE.iter().any(|word| lowered.contains(word)) {
        return Some(Because::WritesOffMachine);
    }
    None
}

/// A write to the checkout, read against the path it names.
///
/// **An overwrite keeps its card and a new file does not.** The owner's 17 Sep
/// decision — Helm asks before it edits, because there is no undo — and his
/// later rule agree on an overwrite and differ only on a file that did not
/// exist, which takes nothing away. A path Fleet cannot read is an overwrite:
/// being unable to tell is not a reason to assume the harmless one.
fn writing(asking: &AskingToRun) -> Option<Because> {
    let path = asking
        .input
        .get("file_path")
        .or_else(|| asking.input.get("path"))
        .or_else(|| asking.input.get("notebook_path"))
        .and_then(|named| named.as_str());
    match path {
        Some(path) if !std::path::Path::new(path).exists() => None,
        _ => Some(Because::Destructive),
    }
}

/// A shell line.
///
/// **Every segment, never the first word.** `cd crates && rm -rf target` has
/// `cd` at the front of it, and a classifier reading one word would run it.
fn shell(line: &str) -> Option<Because> {
    let line = line.trim();
    if line.is_empty() {
        return Some(Because::Unreadable);
    }
    if line.contains('`') || line.contains("$(") {
        return Some(Because::Unreadable);
    }
    // Every class at once, and never worth reading further.
    if word_in(line, "sudo") || word_in(line, "doas") {
        return Some(Because::Destructive);
    }
    // **`&&` and `||` become separators and a lone `&` does not.** Splitting on
    // every `&` cuts `2>&1` in half and leaves a segment ending in a bare `>`,
    // which then reads as a truncating redirect — `cargo build 2>&1 | tail -5`
    // asked, as a destructive line, until a case caught it.
    let chained = line.replace("&&", ";").replace("||", ";");
    chained
        .split(|c| c == ';' || c == '|' || c == '\n')
        .map(str::trim)
        .filter(|said| !said.is_empty())
        .find_map(segment)
}

/// Whether `word` appears in `line` as a word rather than inside a longer one,
/// so `evaluate.sh` is not an `eval`.
fn word_in(line: &str, word: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        .any(|said| said == word)
}

/// One segment of a shell line, already split from the rest.
fn segment(said: &str) -> Option<Because> {
    if truncating_redirect(said) {
        return Some(Because::Destructive);
    }
    // The command is the first word that is not an assignment or a leading
    // `env`, since `FOO=1 rm -rf x` is an `rm`.
    let mut rest = said
        .split_whitespace()
        .skip_while(|word| word.contains('=') || *word == "env" || *word == "command")
        .peekable();
    let program = rest.next()?;
    let program = program.rsplit('/').next().unwrap_or(program);
    // **The shell builtins, and only in the position that makes them one.**
    // `pnpm --dir x exec vitest run` carries `exec` as a subcommand and is the
    // ordinary way this repository runs its tests; reading the word anywhere on
    // the line made every one of them ask.
    if program == "eval" || program == "exec" {
        return Some(Because::Unreadable);
    }
    let arguments: Vec<&str> = rest.collect();
    match program {
        "git" => return git(&arguments),
        "gh" => return forge(&arguments),
        "curl" | "wget" | "http" => return fetching(&arguments),
        _ => {}
    }
    const GONE: &[&str] = &[
        "rm",
        "rmdir",
        "shred",
        "unlink",
        "dd",
        "mkfs",
        "truncate",
        "kill",
        "killall",
        "pkill",
        "chown",
        "chmod",
        "mv",
        "launchctl",
        "diskutil",
        "systemctl",
    ];
    const SHARED: &[&str] = &["scp", "rsync", "ssh", "sftp"];
    const OFF_MACHINE: &[&str] = &["aws", "gcloud", "az", "kubectl", "terraform", "op"];
    if GONE.contains(&program) {
        return Some(Because::Destructive);
    }
    if SHARED.contains(&program) {
        return Some(Because::PushesToShared);
    }
    if OFF_MACHINE.contains(&program) {
        return Some(Because::WritesOffMachine);
    }
    // **A `push` subcommand, whatever pushed it.** Written as a subcommand test
    // rather than a list of programs because the list would be a list of
    // vendors, which is `adapters`' to hold and not this module's.
    if arguments.iter().find(|word| !word.starts_with('-')) == Some(&"push") {
        return Some(Because::PushesToShared);
    }
    match publishing(program, &arguments) {
        true => Some(Because::PushesToShared),
        false => None,
    }
}

/// A `>` that is not `>>` and not `>&`. **It destroys what the file held**,
/// however ordinary the command in front of it.
fn truncating_redirect(said: &str) -> bool {
    let written: Vec<char> = said.chars().collect();
    written.iter().enumerate().any(|(at, c)| {
        *c == '>'
            && at.checked_sub(1).and_then(|back| written.get(back)) != Some(&'>')
            && written.get(at + 1) != Some(&'>')
            && written.get(at + 1) != Some(&'&')
    })
}

/// `git`, by its subcommand. **Reading history is not writing it**: `log`,
/// `status`, `diff`, `show` and `worktree list` run.
fn git(arguments: &[&str]) -> Option<Because> {
    let subcommand = arguments.iter().find(|word| !word.starts_with('-'))?;
    const GONE: &[&str] = &[
        "reset",
        "clean",
        "checkout",
        "restore",
        "rebase",
        "stash",
        "filter-branch",
        "gc",
        "prune",
        "am",
    ];
    match *subcommand {
        "push" => Some(Because::PushesToShared),
        // Each of these takes a ref or a tree away, and the working copy does
        // not bring one back.
        "branch" | "tag" | "worktree" | "remote" | "submodule"
            if arguments.iter().any(|word| takes_something_away(word)) =>
        {
            Some(Because::Destructive)
        }
        word if GONE.contains(&word) => Some(Because::Destructive),
        _ => None,
    }
}

fn takes_something_away(word: &str) -> bool {
    matches!(
        word,
        "-D" | "-d" | "--delete" | "-f" | "--force" | "remove" | "prune" | "rm"
    )
}

/// The code host's own CLI, by its subcommand pair. **A read of somebody
/// else's repository is still a read**: `view`, `list`, `diff`, `status` and
/// `checks` run.
fn forge(arguments: &[&str]) -> Option<Because> {
    let words: Vec<&str> = arguments
        .iter()
        .copied()
        .filter(|word| !word.starts_with('-'))
        .collect();
    let verb = words.get(1).copied().unwrap_or_default();
    match words.first().copied().unwrap_or_default() {
        "pr" | "release" if matches!(verb, "merge" | "create" | "ready" | "upload") => {
            Some(Because::PushesToShared)
        }
        "pr" | "issue" if matches!(verb, "close" | "delete" | "reopen" | "edit") => {
            Some(Because::Destructive)
        }
        "issue" if matches!(verb, "create" | "comment") => Some(Because::WritesOffMachine),
        "api"
            if arguments
                .iter()
                .any(|word| *word == "-X" || *word == "--method") =>
        {
            Some(Because::WritesOffMachine)
        }
        "repo" | "secret" | "auth" | "workflow" => Some(Because::WritesOffMachine),
        _ => None,
    }
}

/// `curl` and its kin. **A GET is a read.** Anything carrying a body, or naming
/// a method other than GET or HEAD, writes to somebody else's machine.
fn fetching(arguments: &[&str]) -> Option<Because> {
    let mut words = arguments.iter().copied().peekable();
    while let Some(word) = words.next() {
        let writes = match word {
            "-d" | "--data" | "--data-raw" | "--data-binary" | "-F" | "--form" | "-T"
            | "--upload-file" | "--json" => true,
            "-X" | "--request" => !matches!(
                words
                    .peek()
                    .copied()
                    .unwrap_or_default()
                    .to_ascii_uppercase()
                    .as_str(),
                "GET" | "HEAD"
            ),
            _ => false,
        };
        if writes {
            return Some(Because::WritesOffMachine);
        }
    }
    None
}

/// A package manager sending a build where other people install from.
/// **`publish` and nothing else** — `cargo build` and `pnpm test` are the
/// ordinary case, which is why this reads the subcommand and not the program.
fn publishing(program: &str, arguments: &[&str]) -> bool {
    const MANAGERS: &[&str] = &[
        "cargo", "npm", "pnpm", "yarn", "gem", "pip", "pip3", "twine",
    ];
    MANAGERS.contains(&program) && arguments.iter().any(|word| *word == "publish")
}
