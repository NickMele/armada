//! `--help`, restructured.
//!
//! **The old page was one block of text with flags inline**, which meant the
//! answer to "what can I run" and the answer to "what does `--orphaned` do" were
//! the same twenty-five lines, and neither could be found at a glance. PHASES.md
//! §8.3.1's done-when is that it "can be read without squinting".
//!
//! Four changes, and each is a rule rather than a preference:
//!
//! 1. **Grouped under headings** — usage, the verbs, the global flags, and what
//!    is not built. A reader arrives with one of those four questions.
//! 2. **Aligned by the same table renderer every verb uses**, so a flag column
//!    lines up here for the same reason a status column lines up there, and a
//!    narrow terminal truncates the explanation rather than wrapping it.
//! 3. **A page per verb — every verb, not the ones somebody got to.** One
//!    [`PAGES`] table holds all of them, across all three modules and the two
//!    machine verbs, and [`page_for`] is what `args.rs` asks whether a `--help`
//!    it just saw belongs to Armada. A verb with no entry here has no `--help`,
//!    and [`every_verb_the_parser_accepts_has_a_page`] fails when that happens.
//! 4. **A synopsis, not a bare name, on every list.** `spawn <task>` answers
//!    "do I give it the prompt, or does it ask me" in the line the reader was
//!    already looking at. A name alone costs a second command to find out.
//!
//! **What is not built is stated, not omitted.** Half of this CLI is reserved
//! names that answer "not built yet" (`args.rs`), and a help page that lists only
//! what works leaves a reader to discover the rest by typing it. The lists come
//! from `args.rs`'s own constants rather than being retyped here, so a verb
//! cannot ship without appearing on this page — there is a test.
//!
//! [`every_verb_the_parser_accepts_has_a_page`]: tests::every_verb_the_parser_accepts_has_a_page

use crate::args::{
    BUILTIN_VERBS, MANIFEST_BUILT, RESERVED_CHECK_FLAGS, RESERVED_GUILD_VERBS, RESERVED_TOP_LEVEL,
};

use super::palette::Role;
use super::style::Style;
use super::table::{Cell, Column, Table};
use super::term::Terminal;

/// Which page to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    /// `armada`, with no arguments at all. The same page as [`Topic::Root`],
    /// under the wordmark — this is the one moment of orientation
    /// (`docs/commands/render.md`).
    Bare,
    /// `armada --help`.
    Root,
    /// `armada manifest`, or `armada manifest --help`.
    Manifest,
    /// `armada guild`, or `armada guild --help`.
    Guild,
    /// `armada fleet`, or `armada fleet --help`.
    Fleet,
    /// `armada mcp`, or `armada mcp --help`.
    Mcp,
    /// One verb's own page, keyed by what the caller types after `armada` —
    /// `"manifest check"`, `"fleet spawn"`, `"doctor"`.
    ///
    /// **The whole path rather than the bare verb**, because `init` is three
    /// different verbs (`armada init`, `armada manifest init`, `armada guild
    /// init`) and a bare name could not tell them apart.
    Verb(&'static str),
}

/// One built verb's page.
struct Page {
    /// What the caller types after `armada` — `"fleet spawn"`. The key.
    path: &'static str,
    /// The verb and its **positional arguments**, as a module page lists it:
    /// `"spawn <task>"`, `"kill [<job>]"`, `"ls"`.
    ///
    /// **Positionals only, and flags never.** The list exists to answer what a
    /// verb *takes*; the flags are a page away and are what that page is for.
    synopsis: &'static str,
    summary: &'static str,
    usage: &'static [&'static str],
    flags: &'static [(&'static str, &'static str)],
    /// Real invocations, each with what it does. Empty where the synopsis
    /// already says everything a line could.
    examples: &'static [(&'static str, &'static str)],
    notes: &'static [&'static str],
}

impl Page {
    /// `"fleet"` for `"fleet spawn"`, and `""` for `"doctor"`.
    fn module(&self) -> &'static str {
        match self.path.split_once(' ') {
            Some((module, _)) => module,
            None => "",
        }
    }
}

/// The flags every verb takes, listed on every verb's page.
///
/// **Repeated per page rather than cross-referenced.** A reader on `check`'s
/// page asking "can I have this as JSON" should not be sent to a second page to
/// find out, and the two lines cost less than the round trip.
const EVERYWHERE: [(&str, &str); 2] = [
    (
        "--json",
        "answer as one JSON object, for a script or an agent",
    ),
    (
        "--color <when>",
        "auto (default), always, never; NO_COLOR wins",
    ),
];

/// Every verb with a page, in the order someone meets them.
///
/// **Not alphabetical, and grouped by module.** Within Manifest, `config` comes
/// first because a repository has no `armada.yml` before it runs, `init` next
/// because nothing else works until *it* has run, and `clean` last because it
/// undoes the rest.
///
/// **Every flag here was read off the verb's parser, not off its document.**
/// `docs/commands/**` describes the verb Armada is going to have; this table
/// describes the one it has, and the two are allowed to differ only in that
/// direction.
const PAGES: [Page; 43] = [
    // ------------------------------------------------------------- Manifest
    Page {
        path: "manifest config",
        synopsis: "config <scan|verify>",
        summary: "propose what the repository proves, then verify it",
        usage: &[
            "armada manifest config scan [--json]",
            "armada manifest config verify [--json]",
        ],
        flags: &[],
        examples: &[
            (
                "armada manifest config scan",
                "the evidence, and the lines it can prove from it",
            ),
            (
                "armada manifest config scan --json",
                "the same, as the envelope an agent reads before it writes",
            ),
            (
                "armada manifest config verify",
                "static pass, then every check the written armada.yml declares",
            ),
        ],
        notes: &[
            "scan proposes only what it can prove: a lockfile names the package",
            "manager, a script named exactly test is the test check, test:changed",
            "is neither. Nothing is written until you tick it.",
            "scan is the one verb that runs in a repo with no armada.yml.",
            "At a terminal it offers to write the proposals or to hand the",
            "repository to an agent. Through a pipe it prints the command",
            "instead, so nothing waits on an answer.",
        ],
    },
    Page {
        path: "manifest init",
        synopsis: "init",
        summary: "claim this workspace: ports, .armada/, setup",
        usage: &["armada manifest init [--json] [--dry-run]"],
        flags: &[("--dry-run", "report what would happen; change nothing")],
        examples: &[],
        notes: &["Reaps what is no longer owned before it claims anything."],
    },
    Page {
        path: "manifest up",
        synopsis: "up [<component>]",
        // **Shorter by three columns than it was**, because the list it sits in
        // now carries `skills [show <name>]` and the widest name decides the
        // description column for every row. A truncated line on a help page is
        // the failure the page exists to prevent.
        summary: "start this workspace's services; wait until they answer",
        usage: &["armada manifest up [<component>] [--json] [--dry-run]"],
        flags: &[(
            "--dry-run",
            "report the argv and the ready-checks; start nothing",
        )],
        examples: &[],
        notes: &[
            "Started is not ready: a service is up when its ready-check passes.",
            "Everything it starts is recorded before it is confirmed working.",
        ],
    },
    Page {
        path: "manifest down",
        synopsis: "down [<component>]",
        summary: "stop this workspace's services; keep the port block",
        usage: &["armada manifest down [<component>] [--json]"],
        flags: &[],
        examples: &[],
        notes: &[
            "down is pause and clean is release — the port block is kept.",
            "Dependents stop before their dependencies.",
        ],
    },
    Page {
        path: "manifest status",
        synopsis: "status",
        summary: "what is running, what is mine, what is stale",
        usage: &["armada manifest status [--json] [--project|--all]"],
        flags: &[
            ("--project", "every workspace of this repository"),
            ("--all", "every workspace on this machine"),
        ],
        examples: &[],
        notes: &[
            "A read verb: its exit code describes the query, not what it found.",
            "A gate uses `armada manifest check`, never a query's exit code.",
        ],
    },
    Page {
        path: "manifest check",
        synopsis: "check [<selector>]",
        summary: "lint, format and test — the verb a gate calls",
        usage: &[
            "armada manifest check [flags] [<selector>]",
            "armada manifest check --files <path>…",
            "armada manifest check --status [<run-id>]",
        ],
        flags: &[
            ("--dry-run", "report what would run; run nothing"),
            (
                "--all-files",
                "scope from each component's match: globs, not the diff",
            ),
            (
                "--fix",
                "run fix: instead of cmd:; skips checks with no fix:",
            ),
            ("--wait", "queue for the run lease instead of failing fast"),
            ("--detach", "start the run in its own session and return"),
            (
                "--status [<run-id>]",
                "what a run decided, or is still deciding",
            ),
            (
                "--files <path>…",
                "run only the checks these paths belong to",
            ),
            ("--component <name>", "every check on one component"),
            (
                "--concurrency <n>",
                "this run's CPU budget, overriding the machine's",
            ),
        ],
        examples: &[
            (
                "armada manifest check",
                "every check whose component the working diff touched",
            ),
            (
                "armada manifest check api:lint",
                "one check on one component, by <component>:<check>",
            ),
            (
                "armada manifest check --files src/main.rs src/args.rs",
                "only the checks those two paths belong to",
            ),
            (
                "armada manifest check --component api --all-files --fix",
                "every check on api, over its match: globs, applying fix:",
            ),
        ],
        notes: &[
            "A selector is <component>:<check>, a component, or a check name.",
            "One selector, or several paths — never both, so nothing is guessed.",
            "--detach prints a run id; --status with no id reads the most recent run.",
            "--status reads the run directory; it never asks a process how it is doing.",
        ],
    },
    Page {
        path: "manifest skills",
        synopsis: "skills [show <name>]",
        summary: "what this repository knows about itself",
        usage: &[
            "armada manifest skills [--json]",
            "armada manifest skills show <name> [--json]",
        ],
        flags: &[],
        examples: &[],
        notes: &[
            "A skill tells an agent what this repository lets it do, and where the",
            "instructions live. Armada shows you one; running it is yours to do.",
        ],
    },
    Page {
        path: "manifest components",
        synopsis: "components",
        summary: "what this repository can be filtered by",
        usage: &["armada manifest components [--json]"],
        flags: &[],
        examples: &[
            (
                "armada manifest components",
                "the names --component takes, and the checks each one has",
            ),
            (
                "armada manifest check --component api",
                "then narrow a run to one of them",
            ),
        ],
        notes: &[
            "RUNS means the component has a run:, so up and down act on it.",
            "A check is reached as <component>:<check>.",
        ],
    },
    Page {
        path: "manifest commands",
        synopsis: "commands",
        summary: "the verbs this repository declares",
        usage: &["armada manifest commands [--json]"],
        flags: &[],
        examples: &[
            (
                "armada manifest commands",
                "the names `armada manifest <name>` takes here",
            ),
            (
                "armada manifest seed-db -- --force",
                "then run one; everything after -- is the command's own",
            ),
        ],
        notes: &[
            "Every row is this repository's own, declared in armada.yml.",
            "Armada provides none of them; armada manifest --help lists Armada's.",
            "An entry with no help: shows what it runs instead.",
        ],
    },
    Page {
        path: "manifest clean",
        synopsis: "clean",
        summary: "release what this workspace owns",
        usage: &[
            "armada manifest clean [--json] [--dry-run] [--project|--all]",
            "armada manifest clean --orphaned --force-rebuild",
        ],
        flags: &[
            (
                "--dry-run",
                "report what would be released; release nothing",
            ),
            ("--artifacts", "also remove declared owns.files"),
            ("--orphaned", "only workspaces whose directory is gone"),
            ("--force", "override the liveness guard"),
            (
                "--force-rebuild",
                "rebuild an unreadable ~/.armada/manifest.db from labels",
            ),
            ("--project", "every workspace of this repository"),
            ("--all", "every workspace on this machine"),
        ],
        examples: &[],
        notes: &[
            "A declared owns.release: command is reported, never executed.",
            "--force-rebuild needs --orphaned, and takes nothing else.",
        ],
    },
    // ---------------------------------------------------------------- Guild
    Page {
        path: "guild init",
        synopsis: "init",
        summary: "build your guild by interviewing you",
        usage: &[
            "armada guild init [--from <path>] [--remote <url>] [--defaults]",
            "armada guild init --no-import [--force]",
        ],
        flags: &[
            ("--from <path>", "import an existing setup from here"),
            ("--no-import", "start empty; import nothing"),
            ("--remote <url>", "set the sync remote without being asked"),
            ("--defaults", "take the default answer to every question"),
            ("--force", "overwrite an existing guild"),
        ],
        examples: &[
            (
                "armada guild init",
                "interview you, then write ~/.armada/guild/",
            ),
            (
                "armada guild init --from ~/.claude --remote git@host:you/guild",
                "import what you already have, and say where it syncs",
            ),
            (
                "armada guild init --no-import --defaults",
                "an empty guild on every default answer; asks nothing",
            ),
        ],
        notes: &["Your guild is yours, not this repository's. It never lands in a project."],
    },
    Page {
        path: "guild project",
        synopsis: "project",
        summary: "put your guild where Claude Code will read it",
        usage: &["armada guild project [--remove] [--json]"],
        flags: &[(
            "--remove",
            "take back exactly what was placed, and nothing else",
        )],
        examples: &[],
        notes: &[
            "guild init and guild pull both end here, so this is for a guild",
            "you edited by hand. A file you changed under ~/.claude is never",
            "overwritten: it is left alone and reported.",
        ],
    },
    Page {
        path: "guild pull",
        synopsis: "pull",
        summary: "bring this machine's guild up to date",
        usage: &["armada guild pull [--json]"],
        flags: &[],
        examples: &[],
        notes: &[
            "Takes nothing else on purpose: what to do when it will not",
            "fast-forward is reported for you to decide, not chosen by a flag.",
        ],
    },
    Page {
        path: "guild push",
        synopsis: "push",
        summary: "send local guild changes to the remote",
        usage: &["armada guild push [--force] [--json]"],
        flags: &[(
            "--force",
            "force-push; refused unless the remote is strictly behind",
        )],
        examples: &[],
        notes: &[],
    },
    Page {
        path: "guild export",
        synopsis: "export",
        summary: "write the whole guild to one portable file",
        usage: &["armada guild export [--out <path>] [--include-secrets] [--json]"],
        flags: &[
            ("--out <path>", "where to write the bundle"),
            (
                "--include-secrets",
                "include this machine's own settings, which normally stay",
            ),
        ],
        examples: &[],
        notes: &[
            "Left out by default, so a bundle you hand to someone else carries",
            "none of your credentials unless you asked for it to.",
        ],
    },
    Page {
        path: "guild import",
        synopsis: "import <bundle>",
        summary: "restore a guild from a bundle",
        usage: &["armada guild import <bundle> [--merge] [--force] [--json]"],
        flags: &[
            (
                "--merge",
                "merge rather than replace; conflicts are skipped and named",
            ),
            ("--force", "replace an existing guild"),
        ],
        examples: &[],
        notes: &["The bundle is a path, and it is required — there is no default source."],
    },
    Page {
        path: "guild ls",
        synopsis: "ls",
        summary: "see what is in your guild",
        usage: &["armada guild ls [--list] [--json]"],
        flags: &[(
            "--list",
            "print the listing and stop, rather than navigating it",
        )],
        examples: &[
            (
                "armada guild ls",
                "at a terminal: pick a thing, then view, edit or delete it",
            ),
            (
                "armada guild ls --json",
                "every item with its kind, path and summary",
            ),
        ],
        notes: &[
            "Every other guild verb moves the guild somewhere. This one says what",
            "is in it: your skills, workflows, subagents, hooks and memory files,",
            "each with a line about what it is. At a terminal the rows are",
            "navigable — there is no flag to ask for that — and through a pipe",
            "the same rows are printed.",
        ],
    },
    Page {
        path: "guild show",
        synopsis: "show <item>",
        summary: "print one guild file",
        usage: &["armada guild show <item> [--json]"],
        flags: &[],
        examples: &[
            (
                "armada guild show voice.md",
                "the whole file, as it is on disk",
            ),
            (
                "armada guild show skills/onboard-repo --json",
                "the same content, with the row that describes it",
            ),
        ],
        notes: &[
            "The item is a name or a guild-relative path — the same names armada",
            "guild ls prints. This is what the terminal's view action runs.",
        ],
    },
    Page {
        path: "guild edit",
        synopsis: "edit <item>",
        summary: "open one guild file, validate it, and commit it",
        usage: &[
            "armada guild edit <item> [--json]",
            "armada guild edit <item> --from <file> [--json]",
        ],
        flags: &[(
            "--from <file>",
            "replace its content from this file instead of opening a box",
        )],
        examples: &[
            (
                "armada guild edit voice.md",
                "open it here and save with ctrl-d",
            ),
            (
                "armada guild edit workflows/bug.yml --from ./bug.yml",
                "replace it from a file, with no terminal needed",
            ),
        ],
        notes: &[
            "A workflow is parsed before it is committed, and one that no longer",
            "parses is left on disk uncommitted rather than pushed to your other",
            "machine. The item is a name or a guild-relative path.",
        ],
    },
    Page {
        path: "guild delete",
        synopsis: "delete <item>",
        summary: "remove one thing from your guild, and commit the removal",
        usage: &["armada guild delete <item> [--yes] [--json]"],
        flags: &[(
            "--yes",
            "skip the confirmation; required where there is nobody to ask",
        )],
        examples: &[(
            "armada guild delete skills/onboard-repo --yes",
            "remove the skill and commit it, without asking",
        )],
        notes: &[
            "The removal is committed, so armada guild push carries it to your",
            "other machines. Anything else in the guild that names it is reported",
            "first; a project's own armada.yml is not checked.",
        ],
    },
    // ---------------------------------------------------------------- Fleet
    Page {
        path: "fleet spawn",
        synopsis: "spawn <task>",
        summary: "start an isolated agent Job on a task",
        usage: &[
            "armada fleet spawn <task> [--workflow <name>] [--name <handle>]",
            "armada fleet spawn <task> [--budget <k>=<v>]… [--confidence <0-1>]",
            "armada fleet spawn <task> [-C <path>] [--dry-run]",
        ],
        flags: &[
            (
                "--workflow <name>",
                "override classification; a workflow in your guild",
            ),
            (
                "--name <handle>",
                "the Job's handle; derived from the task when absent",
            ),
            ("--budget <k>=<v>", "override one ceiling, repeatable"),
            (
                "--confidence <0-1>",
                "below this, stop and ask which workflow; 0 never asks",
            ),
            (
                "-C <path>",
                "the repository to branch from; the cwd's when absent",
            ),
            (
                "--dry-run",
                "report the plan: workflow, worktree, ports, budget",
            ),
        ],
        examples: &[
            (
                "armada fleet spawn \"add rate limiting to the API\"",
                "the task is the argument: one quoted string, given up front",
            ),
            (
                "armada fleet spawn \"port the CLI to clap\" --workflow refactor",
                "skip classification and name the workflow yourself",
            ),
            (
                "armada fleet spawn \"audit the auth flow\" -C ~/work/api --dry-run",
                "another repository, and the plan only — nothing starts",
            ),
        ],
        notes: &[
            "The task is required. Armada never prompts for it afterwards, so a",
            "shell that ate your quotes is a refusal rather than an empty Job.",
        ],
    },
    Page {
        path: "mcp serve",
        synopsis: "serve",
        summary: "serve Armada's tools over stdio, until the client hangs up",
        usage: &["armada mcp serve [--stdio] [--json]"],
        flags: &[(
            "--stdio",
            "serve over stdio (the default, and the only one)",
        )],
        examples: &[(
            "claude mcp add armada -- armada mcp serve",
            "register it with Claude Code, which is what normally starts it",
        )],
        notes: &[
            "Which toolbelt is served is decided by ARMADA_JOB and not by a flag.",
            "Inside a Job it is report, ask and verdict; a Drone cannot spawn.",
            "Every tool answers the same `--json` envelope the CLI does.",
        ],
    },
    Page {
        path: "fleet ls",
        synopsis: "ls",
        summary: "what is running, what it has spent, and who needs you",
        usage: &["armada fleet ls [--all] [--needs-attention] [--json]"],
        flags: &[
            ("--all", "finished and killed Jobs too, not just live ones"),
            ("--needs-attention", "only the Jobs waiting on you"),
        ],
        examples: &[],
        notes: &["The handle in the first column is what every other verb takes."],
    },
    Page {
        path: "fleet show",
        synopsis: "show <job>",
        summary: "one Job in full, and why it needs you",
        usage: &["armada fleet show <job> [--json]"],
        flags: &[],
        examples: &[],
        notes: &[
            "`ls` and the Bridge draw a row; this is what the row was cut from —",
            "the question that raised NEEDS YOU, the whole task, and what is held.",
        ],
    },
    Page {
        path: "fleet board",
        synopsis: "board <job>",
        summary: "take a Job over yourself: worktree and resume command",
        usage: &["armada fleet board <job> [--print|--exec] [--json]"],
        flags: &[
            (
                "--print",
                "print the worktree and the resume command (default)",
            ),
            (
                "--exec",
                "cd there and exec the resume, replacing this shell",
            ),
        ],
        examples: &[],
        notes: &[
            "Board does not attach. It hands you the conversation to drive",
            "yourself, in the Job's own worktree — Armada owns no terminal.",
        ],
    },
    Page {
        path: "fleet answer",
        synopsis: "answer <job> <answer>",
        summary: "give a waiting Job your decision; let it continue",
        usage: &["armada fleet answer <job> <answer> [--json]"],
        flags: &[],
        examples: &[
            (
                "armada fleet inbox",
                "what is being asked, and which Job is asking it",
            ),
            (
                "armada fleet answer nightly-flake \"raise the timeout to 90s\"",
                "the Job, then the answer: two arguments, in that order",
            ),
        ],
        notes: &[
            "Two arguments, not a sentence. Quote the answer, or the shell splits",
            "it into words and Armada refuses rather than guessing which is which.",
        ],
    },
    Page {
        path: "fleet inbox",
        synopsis: "inbox",
        summary: "what the fleet needs from you",
        usage: &["armada fleet inbox [--job <job>] [--all] [--json]"],
        flags: &[
            ("--job <job>", "only this Job's entries"),
            ("--all", "include entries already answered"),
        ],
        examples: &[],
        notes: &["`armada fleet answer <job> <answer>` is how an entry is closed."],
    },
    Page {
        path: "fleet pause",
        synopsis: "pause <job>",
        summary: "stop a Job's Drone and keep everything else",
        usage: &["armada fleet pause <job> [--json]"],
        flags: &[],
        examples: &[],
        notes: &[
            "The worktree, the branch and the port block all stay. A Job is",
            "durable and a Drone is not, which is what makes this reversible.",
        ],
    },
    Page {
        path: "fleet resume",
        synopsis: "resume <job>",
        summary: "start a paused Job's session again",
        usage: &["armada fleet resume <job> [--json]"],
        flags: &[],
        examples: &[],
        notes: &[
            "A Job with an open question is answered, not resumed, and one past",
            "its ceiling is neither: a person decides what happens to that.",
        ],
    },
    Page {
        path: "fleet tick",
        synopsis: "tick [<job>]",
        summary: "gate the step it rests on, then advance, retry or stop",
        usage: &[
            "armada fleet tick [<job>] [--json]",
            "armada fleet tick [<job>] --watch [--json]",
        ],
        flags: &[(
            "--watch",
            "keep going until nothing in scope could move again",
        )],
        examples: &[],
        notes: &[
            "A Drone runs one exchange and exits. This is what notices, runs the",
            "step's verify: predicate, and writes the verdict it earns.",
            "A step advances only on evidence an external command produced.",
            "review_clean and subjob_passed stop and ask: nothing can decide them.",
        ],
    },
    Page {
        path: "fleet reap",
        synopsis: "reap",
        summary: "end every finished Job and release what they hold",
        usage: &[
            "armada fleet reap [--dry-run] [--yes] [--json]",
            "armada fleet reap --job <job> [--job <job>]… [--yes]",
        ],
        flags: &[
            ("--dry-run", "the plan, and nothing reaped"),
            ("--yes", "reap without asking; what a pipe passes"),
            (
                "--job <job>",
                "reap exactly these, instead of the default set",
            ),
        ],
        examples: &[],
        notes: &[
            "DONE and ABORTED are taken; PAUSED, STALLED and BLOCKED are listed",
            "and left, because a state you might still act on is not garbage.",
            "Without --yes and without a terminal it refuses rather than reaps.",
        ],
    },
    Page {
        path: "fleet kill",
        synopsis: "kill [<job>]",
        summary: "end a Job and release everything it owns",
        usage: &[
            "armada fleet kill <job> [--keep-branch] [--keep-worktree] [--json]",
            "armada fleet kill --all-finished [--json]",
        ],
        flags: &[
            ("--keep-branch", "leave the branch; remove the worktree"),
            (
                "--keep-worktree",
                "leave the directory too; implies --keep-branch",
            ),
            (
                "--all-finished",
                "every Job whose workflow has ended, instead of one",
            ),
        ],
        examples: &[],
        notes: &[
            "A Job or --all-finished, never both and never neither: naming one",
            "Job and asking for all of them are two different requests.",
        ],
    },
    // --------------------------------------------------------- this machine
    Page {
        path: "init",
        synopsis: "init",
        summary: "set up this machine: checks, ~/.armada/, and your guild",
        usage: &[
            "armada init [--guild <remote>] [--defaults] [--force]",
            "armada init --bundle <path> [--defaults] [--force]",
        ],
        flags: &[
            (
                "--guild <remote>",
                "clone your guild from here, without asking",
            ),
            ("--bundle <path>", "unpack this bundle instead of asking"),
            ("--defaults", "take the default answer to every question"),
            ("--force", "re-run against an existing ~/.armada/"),
        ],
        examples: &[],
        notes: &[
            "--guild and --bundle are two sources for one guild; pass one.",
            "`armada init` sets up you, here. `armada manifest init` claims a workspace.",
        ],
    },
    Page {
        path: "doctor",
        synopsis: "doctor",
        summary: "what this machine is missing, and what fixes it",
        usage: &["armada doctor [--fix] [--json]"],
        flags: &[("--fix", "repair what is safely repairable")],
        examples: &[],
        notes: &["Every finding carries the line that fixes it, whether or not --fix ran."],
    },
    Page {
        path: "failures",
        synopsis: "failures",
        summary: "what Armada broke on, and how to put a Job on one",
        usage: &[
            "armada failures [--all] [--json]",
            "armada failures show <id> [--json]",
            "armada failures fix <id> [--dry-run] [--json]",
            "armada failures clear <id> | --all",
        ],
        flags: &[
            (
                "--all",
                "listing: include cleared; clear: clear every entry",
            ),
            ("--dry-run", "fix: report the Job, and spawn nothing"),
        ],
        examples: &[
            ("armada failures", "everything still open on this machine"),
            (
                "armada failures show a1b2",
                "one failure, and what a Job would be told",
            ),
            (
                "armada failures fix a1b2",
                "a Job on the bug workflow, in the repo it happened in",
            ),
            ("armada failures clear a1b2", "done with it"),
        ],
        notes: &[
            "Armada records every failure it reports to you, in ~/.armada/failures.jsonl.",
            "The same failure twice is one row with a count and the time you last saw it.",
            "An id can be shortened to any prefix that names one row.",
            "Nothing here is sent anywhere: the log is this machine's and stays on it.",
        ],
    },
    Page {
        path: "report",
        synopsis: "report",
        summary: "tell Armada something went wrong that it did not notice",
        usage: &["armada report \"<what happened>\" [--json]"],
        flags: &[],
        examples: &[
            (
                "armada report \"the dry-run said CREATED and made nothing\"",
                "file it, with the diagnostics attached",
            ),
            ("armada failures", "your reports, beside what Armada caught"),
        ],
        notes: &[
            "For output that was wrong, missing, or misleading — anything that",
            "looked fine to Armada, so nothing was recorded.",
            "It attaches your last runs and what each answered, armada doctor's",
            "findings, versions, and this workspace. You paste nothing.",
            "Credential-shaped values and your home path go before it is written.",
            "A report joins the same list: armada failures show, fix and clear",
            "each take it, and it lists beside what Armada caught itself.",
            "Nothing is sent anywhere. The record stays on this machine.",
        ],
    },
    Page {
        path: "task",
        synopsis: "task",
        summary: "write down a thing to do, and spend nothing on it",
        usage: &["armada task \"<what to do>\" [--json]"],
        flags: &[],
        examples: &[
            (
                "armada task \"rename the port allocator\"",
                "written down, with an id, in under a second",
            ),
            ("armada tasks", "everything you have written down"),
        ],
        notes: &[
            "Capture is free and execution is deliberate: nothing runs, nothing",
            "is classified, and no tokens are spent until you start it.",
            "It records the repository you were in, so armada tasks start opens",
            "the Job there — from a worktree, that is the checkout it belongs to.",
            "A task joins the same store your failures and reports are in, with",
            "one id space: armada failures show takes a task id and the reverse.",
            "Nothing is sent anywhere and nothing syncs. This machine only.",
        ],
    },
    Page {
        path: "tasks",
        synopsis: "tasks",
        summary: "what you wrote down, and how to put a Job on one",
        usage: &[
            "armada tasks [--all] [--json]",
            "armada tasks show <id> [--json]",
            "armada tasks start <id> [--workflow <name>] [--dry-run] [--json]",
            "armada tasks clear <id> | --all",
        ],
        flags: &[
            ("--all", "listing: include cleared; clear: clear every task"),
            ("--workflow <name>", "start: design|plan|bug|feature"),
            ("--dry-run", "start: report the Job, and spawn nothing"),
        ],
        examples: &[
            ("armada tasks", "everything still to do; enter picks one"),
            (
                "armada tasks show a1b2",
                "one task, and what a Job would be told",
            ),
            (
                "armada tasks start a1b2",
                "a Job on it, in the repo it was written in",
            ),
            ("armada tasks clear a1b2", "done, or not doing it"),
        ],
        notes: &[
            "armada task \"<sentence>\" is how one gets here.",
            "Without --workflow, start classifies the task the way fleet spawn",
            "does, and asks you to confirm a guess it is not sure of.",
            "A started task keeps the Job's handle and stays open: a Drone",
            "reaching the end is not the same claim as the thing being done.",
            "An id can be shortened to any prefix that names one row.",
            "Tasks and failures share one store and one id space, and list",
            "separately: this asks what to do, armada failures asks what broke.",
        ],
    },
    Page {
        path: "untried",
        synopsis: "untried",
        summary: "which of Armada's verbs you have never run here",
        usage: &["armada untried [--all] [--json]"],
        flags: &[("--all", "every verb, not only the ones never run")],
        examples: &[
            ("armada untried", "what you have not got to yet"),
            ("armada untried --all", "the whole roster, stalest first"),
        ],
        notes: &[
            "Counted as you use it: every run of an Armada verb is tallied at",
            "the end, in ~/.armada/untried.jsonl. Nothing is sent anywhere.",
            "A sub-verb counts as its parent: armada tasks start counts as tasks.",
            "At a terminal, picking a row writes a task for it — nothing is",
            "written down unless you pick it.",
            "Not `coverage`: this repository's own CI job already owns that",
            "word, for code coverage (AGENTS.md, docs/glossary.md).",
        ],
    },
    Page {
        path: "bridge",
        synopsis: "bridge",
        summary: "the live screen: every Job, its state, its spend, who needs you",
        usage: &[
            "armada bridge [--filter <expr>] [--interval <s>]",
            "armada bridge --once [--filter <expr>]",
            "armada bridge --json",
        ],
        flags: &[
            ("--filter <expr>", "show only matching Jobs"),
            ("--interval <s>", "redraw cadence in seconds; 2 by default"),
            ("--once", "render one frame and exit"),
        ],
        examples: &[
            ("armada bridge", "watch the fleet; q leaves"),
            (
                "armada bridge --filter needs=you",
                "only the Jobs waiting on an answer",
            ),
            (
                "armada bridge --once",
                "one frame, for a pipe or a terminal with no alt-screen",
            ),
        ],
        notes: &[
            "Read-only. It never resumes, interrupts or probes a Job to draw a frame.",
            "It holds nothing: every key calls a fleet verb you could type yourself.",
            "--filter takes a word, or job=, workflow=, state=, task=, needs=you.",
        ],
    },
    Page {
        path: "helm",
        synopsis: "helm",
        summary: "the one agent you talk to: wire the toolbelt, report the launch",
        usage: &["armada helm [--agent <name>] [--new]", "armada helm --json"],
        // **The flag is listed, and listed as gated.** Leaving it off the page
        // would make `armada helm --exec` read as a typo worth retrying, which
        // is the reading the refusal itself is written against.
        flags: &[
            (
                crate::verbs::helm::ENTER,
                "become the session — off until `armada helm enable`; see NOTES",
            ),
            ("--new", "start a fresh conversation instead of resuming"),
            (
                "--agent <name>",
                "a persona from ~/.armada/guild/subagents/",
            ),
        ],
        examples: &[
            (
                "armada helm",
                "wire the inbox and the toolbelt; print the command; start nothing",
            ),
            (
                "armada helm --json",
                "the same, with argv as a vector a script can run",
            ),
            (
                "armada helm --new",
                "put yesterday's conversation down and mint another",
            ),
        ],
        notes: &[
            "It starts nothing by itself. The launch is assembled and verified;",
            "--exec is what would enter it, and it is gated by a machine switch —",
            "off on a fresh install. `armada helm enable` turns it on here;",
            "`armada helm disable` puts it back. See their own --help.",
            "It is the same conversation each day; the id lives in ~/.armada/helm/.",
            "Its toolbelt is fleet.* and manifest.*; classification is Fleet's job.",
            "There is no `helm` binary. Kubernetes owns that name on PATH.",
        ],
    },
    Page {
        path: "helm enable",
        synopsis: "enable",
        summary: "let --exec become a session, on this machine",
        usage: &["armada helm enable [--json]"],
        flags: &[],
        examples: &[
            (
                "armada helm enable",
                "then armada helm --exec becomes claude, on this machine",
            ),
            (
                "armada helm enable && armada helm --exec",
                "the whole sequence a script would run",
            ),
        ],
        notes: &[
            "Writes one boolean to ~/.armada/machine.yml, under helm: — the file",
            "that never syncs, because whether a session may open here is a fact",
            "about this machine and not a preference that travels with your",
            "guild to every machine it has ever been pulled onto.",
            "Off is the default; a fresh install cannot exec until this runs.",
            "It does not touch the guild, the persona, or wire the inbox — that",
            "is still armada helm. armada helm disable puts the switch back.",
        ],
    },
    Page {
        path: "helm disable",
        synopsis: "disable",
        summary: "the opposite of enable — the default on a fresh install",
        usage: &["armada helm disable [--json]"],
        flags: &[],
        examples: &[(
            "armada helm disable",
            "armada helm --exec goes back to being refused, on this machine",
        )],
        notes: &[
            "Writes the same boolean armada helm enable does, false. A machine",
            "that has never run enable is already in this state.",
        ],
    },
];

/// The page for what a caller typed after `armada`, or `None`.
///
/// **This is the whole of `args.rs`'s question.** A `--help` that lands on a
/// name with no entry here is not Armada's `--help` — it belongs to a
/// `commands:` child, or to a verb that is not built — and the parser is
/// answered `None` rather than being handed a second list to keep in step.
pub fn page_for(path: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|page| page.path == path)
        .map(|page| page.path)
}

/// Draw a page.
pub fn render(topic: Topic, style: Style, terminal: Terminal) -> String {
    match topic {
        // **The wordmark's one place on this page, and only on this topic.**
        // Bare `armada` is the moment of orientation; `--help` is the page you
        // reached for in a hurry (`docs/commands/render.md`). `banner` decides
        // for itself whether the reader is a person.
        Topic::Bare => format!(
            "{}{}",
            super::banner::banner(style, terminal),
            root(style, terminal)
        ),
        Topic::Root => root(style, terminal),
        Topic::Manifest => manifest(style, terminal),
        Topic::Guild => guild(style, terminal),
        Topic::Fleet => fleet(style, terminal),
        Topic::Mcp => mcp(style, terminal),
        Topic::Verb(path) => match PAGES.iter().find(|page| page.path == path) {
            Some(page) => verb(page, style, terminal),
            // Unreachable through `args::parse`, which only produces a `Verb`
            // for a path `page_for` just returned. Answering with the root page
            // rather than panicking keeps a wrong call harmless.
            None => root(style, terminal),
        },
    }
}

/// The verbs of one module, as `(synopsis, summary)` — the shape every list of
/// verbs on every page takes.
fn verbs_of(module: &str) -> Vec<(&'static str, &'static str)> {
    // **Helm and the Bridge are top-level and are not machine verbs.** `armada
    // init` and `armada doctor` describe this laptop; these two describe the
    // fleet running on it, and filing them under `THIS MACHINE` would put them
    // beside two verbs they have nothing in common with. They share a heading,
    // so the one page a reader meets Armada on says what each is for — and says
    // it in the pair the design keeps insisting on: Helm is where you talk, the
    // Bridge is what you watch.
    //
    // **`enable`/`disable` sit beside `helm` rather than under a `THIS
    // MACHINE` entry of their own**, because the switch they flip has no
    // meaning apart from the verb it gates — a reader who has never seen
    // `armada helm` has no reason to be looking for `armada helm enable`.
    const ORCHESTRATION: [&str; 4] = ["helm", "helm enable", "helm disable", "bridge"];
    if module == "orchestration" {
        return ORCHESTRATION
            .iter()
            .filter_map(|path| PAGES.iter().find(|page| page.path == *path))
            .map(|page| {
                // **`enable` and `disable` keep the module they belong to.**
                // The heading above this list is not one uniform prefix the
                // way `FLEET`'s is — `helm` and `bridge` really are top-level
                // verbs, so they read bare, but `enable`/`disable` are
                // `helm`'s own sub-verbs and a bare row here reads as
                // `armada enable`, which the parser refuses. The individual
                // `--help` page still opens with the bare word — that page
                // already spells the module out in USAGE — so only the
                // shared listing needs the fold-back-in.
                let synopsis = match page.path {
                    "helm enable" => "helm enable",
                    "helm disable" => "helm disable",
                    _ => page.synopsis,
                };
                (synopsis, page.summary)
            })
            .collect();
    }
    PAGES
        .iter()
        .filter(|page| page.module() == module && !ORCHESTRATION.contains(&page.path))
        .map(|page| (page.synopsis, page.summary))
        .collect()
}

fn root(style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    let mut out = format!(
        "{}\n\n",
        style.strong(
            Role::SignalAmber,
            "armada — one vocabulary for a repo's stack, and the agents working in it"
        )
    );

    out.push_str(&heading(style, "USAGE"));
    out.push_str(
        // **`armada helm`, spelled out.** PLAN.md §15.1 gives the bare word to
        // Helm eventually; until it does, a USAGE line promising that `armada`
        // alone enters the orchestrator would be this page describing a build
        // that does not exist — which is the one thing a help page may never do.
        &two_column(&[
            ("armada <module> <verb> [flags]", "the verbs Armada owns"),
            ("armada helm", "enter Helm, the agent you talk to"),
        ])
        .render(style, width),
    );

    out.push('\n');
    out.push_str(&heading(
        style,
        "MANIFEST — what a workspace is and how to operate it",
    ));
    let mut verbs = verbs_of("manifest");
    // **`<name>` is a placeholder, and a placeholder with nowhere to look it up
    // is worse than no row at all.** The line used to end at "this repo's
    // armada.yml", which told a reader that names exist and left opening the
    // file as the only way to learn them. `armada manifest commands` is that
    // way now, so the row names it.
    verbs.push((
        "<name>",
        "a verb this repo declares; manifest commands lists them",
    ));
    out.push_str(&two_column(&verbs).render(style, width));

    out.push('\n');
    out.push_str(&heading(
        style,
        "GUILD — you, portable: voice, skills, hooks, workflows",
    ));
    out.push_str(&two_column(&verbs_of("guild")).render(style, width));

    out.push('\n');
    out.push_str(&heading(
        style,
        "FLEET — the agents you do not talk to: Jobs, worktrees, budgets",
    ));
    out.push_str(&two_column(&verbs_of("fleet")).render(style, width));

    out.push('\n');
    out.push_str(&heading(
        style,
        "HELM AND THE BRIDGE \u{2014} where you talk, and what you watch",
    ));
    out.push_str(&two_column(&verbs_of("orchestration")).render(style, width));

    out.push('\n');
    out.push_str(&heading(style, "THIS MACHINE"));
    // **`armada init` and `armada manifest init` are two verbs**, and the one
    // place a reader is most likely to confuse them is this page — so the line
    // says which is which rather than leaving the module level to imply it.
    out.push_str(&two_column(&verbs_of("")).render(style, width));

    out.push('\n');
    out.push_str(&heading(style, "GLOBAL FLAGS"));
    out.push_str(
        &two_column(&[
            EVERYWHERE[0],
            EVERYWHERE[1],
            ("-h, --help", "this page, or a verb's own flags"),
            ("-V, --version", "the version"),
        ])
        .render(style, width),
    );

    out.push('\n');
    out.push_str(&heading(style, "NOT BUILT YET"));
    out.push_str(&not_built(style, width));

    out.push('\n');
    out.push_str("Run `armada <module> <verb> --help` for a verb's own flags.\n\n");
    out.push_str(
        "Global flags come before the module. Everything after a commands: name is the\n\
         child's, including flags Armada itself defines.\n",
    );
    out
}

/// `armada guild`, the module page.
fn guild(style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    let mut out = format!(
        "{}\n\n",
        style.strong(
            Role::SignalAmber,
            "armada guild \u{2014} you, portable: machine-global, and synced between your machines"
        )
    );
    out.push_str(&heading(style, "VERBS"));
    out.push_str(&two_column(&verbs_of("guild")).render(style, width));
    out.push('\n');
    out.push_str(&heading(style, "NOT BUILT YET"));
    out.push_str(
        &two_column(
            &RESERVED_GUILD_VERBS
                .iter()
                .map(|(name, summary)| (*name, summary.trim_start_matches("M2 \u{2014} ")))
                .collect::<Vec<_>>(),
        )
        .render(style, width),
    );
    out.push('\n');
    out.push_str(&trailer("armada guild"));
    // **What the guild lets you do, not what is in it.** The old paragraph
    // listed four filenames and cited a section number, which told a reader
    // holding only the binary nothing they could act on.
    out.push_str(
        "\nYour guild is how you work: your voice, your skills, your workflows. Set it up\n\
         once and it follows you \u{2014} a new laptop is a `pull` rather than an afternoon.\n\n\
         It never lands in a project's history, and what is only true of this machine\n\
         stays on it: its settings, its database, and the Jobs running here.\n",
    );
    out
}

/// `armada fleet`, the module page.
fn fleet(style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    let mut out = format!(
        "{}\n\n",
        style.strong(
            Role::SignalAmber,
            "armada fleet \u{2014} the agents you do not talk to: Jobs, Drones, worktrees"
        )
    );
    out.push_str(&heading(style, "VERBS"));
    out.push_str(&two_column(&verbs_of("fleet")).render(style, width));
    out.push('\n');
    out.push_str(&trailer("armada fleet"));
    // **The one thing a reader has to know before the verbs make sense**
    // (`glossary.md`): the durable thing and the running thing are not the same
    // thing, and every verb here acts on the durable one.
    //
    // **Said by what it buys the reader, not by what the record holds.** The
    // paragraph this replaced opened by listing a Job's five fields, which is
    // what a Job is internally and not what it means to someone deciding
    // whether it is safe to close their laptop.
    out.push_str(
        "\nYou ask for work; that is a Job. Something runs it; that is a Drone.\n\n\
         The two are separate so that a crash, a reboot or a killed process\n\
         does not lose the work \u{2014} the Job keeps its branch, its notes and what\n\
         it spent, and you can pick it up later with `board`.\n\n\
         Board does not attach. It hands you the conversation to drive yourself, in the\n\
         Job's own worktree \u{2014} Armada owns no terminal.\n",
    );
    out
}

/// **The module page for the one module nobody types.** It exists because
/// `armada mcp` has to answer something, and because the person reading it is
/// almost certainly debugging a registration that is not working — so the page
/// says what registers it and which toolbelt it will serve, rather than
/// explaining MCP.
fn mcp(style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    let mut out = format!(
        "{}\n\n",
        style.strong(
            Role::SignalAmber,
            "armada mcp \u{2014} Armada's toolbelt, for Helm and for a Drone"
        )
    );
    out.push_str(&heading(style, "VERBS"));
    out.push_str(&two_column(&verbs_of("mcp")).render(style, width));
    out.push('\n');
    out.push_str(&trailer("armada mcp"));
    out.push_str(
        "\nYou do not normally run this. Whatever starts an agent starts the server,\n\
         and it lives as long as that session.\n\n\
         Which tools it serves is decided by where it is started, not by a flag: a\n\
         process inside a Job gets report, ask and verdict, and cannot spawn. Every\n\
         other process gets the orchestrator's fleet and manifest verbs.\n",
    );
    out
}

fn manifest(style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    let mut out = format!(
        "{}\n\n",
        style.strong(
            Role::SignalAmber,
            "armada manifest — one repository's stack: ports, services, checks"
        )
    );
    out.push_str(&heading(style, "VERBS"));
    // **The root page carried this row and the module page did not**, which
    // meant `armada manifest --help` — the page a reader reaches by asking
    // about this module specifically — never said that a repository's own verbs
    // are invocable here at all.
    let mut verbs = verbs_of("manifest");
    verbs.push((
        "<name>",
        "a verb this repo declares; manifest commands lists them",
    ));
    out.push_str(&two_column(&verbs).render(style, width));
    out.push('\n');
    out.push_str(&heading(style, "NOT BUILT YET"));
    out.push_str(
        &two_column(&[(
            not_built_verbs().join(", ").as_str(),
            "claimed; each answers `not built yet`",
        )])
        .render(style, width),
    );
    out.push('\n');
    out.push_str(&trailer("armada manifest"));
    // The same rule as Fleet's paragraph: what claiming a workspace buys you,
    // rather than what the record of one contains.
    out.push_str(
        "\nA workspace is one checkout, with services and ports that are only its own.\n\
         Claim it and you can run two branches of the same repository at once without\n\
         either one taking the other's port or stopping the other's database.\n",
    );
    out
}

/// The line every module page ends its verb list with.
///
/// **The same sentence on all three**, because a reader who learned it on one
/// module has learned it on the others — and the module pages are the only
/// place that sentence has to appear.
fn trailer(module: &str) -> String {
    format!("`{module} <verb> --help` for one verb's usage and flags.\n")
}

fn verb(page: &Page, style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    let mut out = format!(
        "{}\n\n",
        style.strong(
            Role::SignalAmber,
            &format!("armada {} — {}", page.path, page.summary)
        )
    );

    out.push_str(&heading(style, "USAGE"));
    let mut usage = Table::new(vec![Column::flexible("")])
        .headerless()
        .indent(2);
    for line in page.usage {
        usage = usage.row(vec![Cell::plain(*line)]);
    }
    out.push_str(&usage.render(style, width));

    let mut flags: Vec<(&str, &str)> = page.flags.to_vec();
    flags.extend_from_slice(&EVERYWHERE);

    out.push('\n');
    out.push_str(&heading(style, "FLAGS"));
    out.push_str(&two_column(&flags).render(style, width));

    if !page.examples.is_empty() {
        out.push('\n');
        out.push_str(&heading(style, "EXAMPLES"));
        out.push_str(&examples(page.examples, style));
    }

    if !page.notes.is_empty() {
        out.push('\n');
        for note in page.notes {
            out.push_str(note);
            out.push('\n');
        }
    }
    out
}

/// The invocation, then what it does under it.
///
/// **Not a two-column table.** A real invocation is sixty columns wide before
/// its explanation starts, and a table would either truncate the explanation to
/// nothing or shear the page — so the pair stacks, and the indent is what ties
/// the two lines together.
fn examples(rows: &[(&str, &str)], style: Style) -> String {
    let mut out = String::new();
    for (index, (call, what)) in rows.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!("  {call}\n"));
        out.push_str(&format!("    {}\n", style.paint(Role::SteelGrey, what)));
    }
    out
}

/// A section heading: signal amber, which is what the palette reserves it for.
fn heading(style: Style, text: &str) -> String {
    format!("{}\n", style.strong(Role::SignalAmber, text))
}

/// A name and what it does, aligned. The one shape every list on these pages
/// takes, so no two of them align differently.
fn two_column(rows: &[(&str, &str)]) -> Table {
    let mut table = Table::new(vec![Column::fixed(""), Column::flexible("")])
        .headerless()
        .indent(2);
    for (name, description) in rows {
        table = table.row(vec![Cell::plain(*name), Cell::muted(*description)]);
    }
    table
}

/// The claimed Manifest verbs that answer "not built yet".
///
/// Read off `args.rs`'s own tables rather than retyped, so a verb that ships
/// leaves this list by shipping.
fn not_built_verbs() -> Vec<&'static str> {
    BUILTIN_VERBS
        .iter()
        .copied()
        .filter(|verb| !MANIFEST_BUILT.contains(verb))
        .collect()
}

/// Everything a caller can type that answers "not built yet".
///
/// Generated from `args.rs`'s own tables, so a name claimed there cannot be
/// missing here.
fn not_built(style: Style, width: usize) -> String {
    let modules: Vec<&str> = RESERVED_TOP_LEVEL.iter().map(|(name, _)| *name).collect();
    let manifest_verbs = not_built_verbs();
    let guild_verbs: Vec<&str> = RESERVED_GUILD_VERBS.iter().map(|(name, _)| *name).collect();

    let mut table = Table::new(vec![Column::fixed(""), Column::flexible("")])
        .headerless()
        .indent(2);
    // **The row is skipped when nothing is in it**, rather than printing
    // `armada ` with an empty list. Every top-level name is built as of M3, and
    // the table still exists for the next one that is not.
    if !modules.is_empty() {
        table = table.row(vec![
            Cell::plain(format!("armada {}", modules.join(", "))),
            Cell::muted("M3"),
        ]);
    }
    table = table
        .row(vec![
            Cell::plain(format!("armada guild {}", guild_verbs.join(", "))),
            Cell::muted("M2"),
        ])
        .row(vec![
            Cell::plain(format!("armada manifest {}", manifest_verbs.join(", "))),
            Cell::muted("reserved"),
        ]);
    // **Skipped when the list is empty**, for the same reason the module row
    // above is: `--detach` and `--status` were its only two entries and both
    // shipped, and a row reading `armada manifest check ` with nothing after it
    // claims a flag that does not exist. The row returns for free the next time
    // a `check` flag is claimed and not built.
    if !RESERVED_CHECK_FLAGS.is_empty() {
        table = table.row(vec![
            Cell::plain(format!(
                "armada manifest check {}",
                RESERVED_CHECK_FLAGS.join(", ")
            )),
            Cell::muted("reserved"),
        ]);
    }
    let mut out = table.render(style, width);
    out.push_str(
        "\n  Each is claimed and answers `not built yet`, so that a name which is\n  \
         going to mean one thing never means something else first.\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::every_verb;

    fn root_page() -> String {
        render(Topic::Root, Style::plain(), Terminal::at(100))
    }

    fn page_text(path: &'static str) -> String {
        render(Topic::Verb(path), Style::plain(), Terminal::at(100))
    }

    /// **The gap this milestone closed, stated so it cannot reopen.**
    ///
    /// `args.rs` enumerates every verb it accepts; every one of them has a page
    /// here, and that page has the four things a page is. A verb that ships
    /// without one fails this test rather than being discovered by a reader
    /// typing `--help` and being told `unknown flag`.
    #[test]
    fn every_verb_the_parser_accepts_has_a_page() {
        for path in every_verb() {
            let key =
                page_for(&path).unwrap_or_else(|| panic!("`armada {path}` has no --help page"));
            let text = page_text(key);
            assert!(
                text.starts_with(&format!("armada {path} \u{2014} ")),
                "`armada {path} --help` does not open by naming itself"
            );
            assert!(text.contains("USAGE"), "`armada {path}` has no USAGE");
            assert!(text.contains("FLAGS"), "`armada {path}` has no FLAGS");
        }
    }

    /// And the other direction: a page for something the parser would refuse is
    /// a promise the CLI does not keep.
    #[test]
    fn every_page_is_a_verb_the_parser_accepts() {
        let accepted = every_verb();
        for page in &PAGES {
            assert!(
                accepted.iter().any(|path| path == page.path),
                "`armada {}` has a page and is not a verb",
                page.path
            );
        }
    }

    /// **USAGE shows the positional arguments, and shows which are required.**
    /// The report that opened this was a reader asking whether `spawn` takes the
    /// prompt or asks for it afterwards, which is exactly what a synopsis
    /// answers and what prose never did.
    #[test]
    fn a_verb_that_takes_an_argument_says_so_in_its_synopsis_and_its_usage() {
        for page in &PAGES {
            let verb = page.path.rsplit(' ').next().expect("a last word");
            assert!(
                page.synopsis == verb || page.synopsis.starts_with(&format!("{verb} ")),
                "`{}`'s synopsis does not open with the verb: {}",
                page.path,
                page.synopsis
            );
            // Every argument the module page advertises is spelled out in
            // USAGE. `<scan|verify>` counts as two, because a reader who saw it
            // on the module page will type one of them.
            let usage = page.usage.join("\n");
            for argument in page
                .synopsis
                .split_whitespace()
                .skip(1)
                .flat_map(|word| word.trim_matches(['[', ']', '<', '>']).split('|'))
            {
                assert!(
                    usage.contains(argument),
                    "`{}` advertises `{argument}` and its USAGE never mentions it",
                    page.path
                );
            }
            for line in page.usage {
                assert!(
                    line.starts_with(&format!("armada {}", page.path)),
                    "`{}`'s usage line does not start with the verb: {line}",
                    page.path
                );
            }
        }
    }

    /// **A module page shows what each verb takes.** `spawn` alone sends the
    /// reader for a second command to learn there is a task argument; `spawn
    /// <task>` does not.
    #[test]
    fn the_module_pages_carry_a_synopsis_rather_than_a_bare_name() {
        for (topic, module) in [
            (Topic::Manifest, "manifest"),
            (Topic::Guild, "guild"),
            (Topic::Fleet, "fleet"),
        ] {
            let text = render(topic, Style::plain(), Terminal::at(100));
            for (synopsis, _) in verbs_of(module) {
                assert!(
                    text.contains(synopsis),
                    "the {module} page does not carry `{synopsis}`"
                );
            }
        }
        let fleet = render(Topic::Fleet, Style::plain(), Terminal::at(100));
        assert!(fleet.contains("spawn <task>"), "spawn hides its argument");
    }

    /// **Nothing a caller can type is missing from the help.** A verb that ships
    /// without a line here is a verb discovered by typing it, which is the
    /// failure this page exists to prevent.
    #[test]
    fn every_claimed_name_appears_on_the_root_page() {
        let page = root_page();
        for verb in BUILTIN_VERBS {
            assert!(page.contains(verb), "`manifest {verb}` is not on the page");
        }
        for (module, _) in RESERVED_TOP_LEVEL {
            assert!(
                page.contains(module),
                "`armada {module}` is not on the page"
            );
        }
        for path in every_verb() {
            let verb = path.rsplit(' ').next().expect("a last word");
            assert!(page.contains(verb), "`armada {path}` is not listed");
        }
    }

    /// **The NOT BUILT YET row for `check` is drawn from `RESERVED_CHECK_FLAGS`,
    /// not retyped.** This was the drift `docs/reserved/009` item 5 named by
    /// example: the row and `check`'s own parser said the same two flags in two
    /// places, so a flag added to one and not the other shipped silently. A
    /// flag added here now reaches the page for free; this test is what fails
    /// if that ever stops being true.
    ///
    /// **The list is empty as of M4** — `--detach` and `--status` shipped — so
    /// what is asserted is the other half of the same property: nothing is
    /// claimed as unbuilt that is built. The loop is kept rather than deleted
    /// because it is the assertion that has to be here the moment the list
    /// gains an entry again.
    #[test]
    fn the_reserved_check_flags_row_names_every_flag_in_the_list() {
        let page = root_page();
        for flag in RESERVED_CHECK_FLAGS {
            assert!(
                page.contains(flag),
                "`{flag}` is reserved and missing from the NOT BUILT YET row"
            );
        }
        if RESERVED_CHECK_FLAGS.is_empty() {
            assert!(
                !page.contains("armada manifest check\n")
                    && !page.contains("armada manifest check "),
                "an empty reserved list still drew a NOT BUILT YET row for `check`"
            );
        }
    }

    /// **Both flags are on `check`'s page, and neither is on the unbuilt list.**
    /// The pair that blocked M4 is the case that would most obviously go wrong
    /// in one direction only: a parser that accepts a flag the help still calls
    /// reserved teaches every reader the opposite of the truth.
    #[test]
    fn detach_and_status_are_documented_as_built() {
        let page = page_text("manifest check");
        for flag in ["--detach", "--status"] {
            assert!(page.contains(flag), "`{flag}` is not on `check`'s page");
        }
        let root = root_page();
        for flag in ["--detach", "--status"] {
            assert!(
                !root.contains(&format!("armada manifest check {flag}")),
                "`{flag}` is built and still listed as NOT BUILT YET"
            );
        }
    }

    /// **The module page names the six and says what a Job is.** A reader who
    /// has not met the Job/Drone distinction cannot make sense of `board` or
    /// `kill`, and this page is where they meet it (`glossary.md`).
    #[test]
    fn the_fleet_page_lists_its_verbs_and_draws_the_job_drone_line() {
        let page = render(Topic::Fleet, Style::plain(), Terminal::at(100));
        for (synopsis, _) in verbs_of("fleet") {
            assert!(page.contains(synopsis), "`{synopsis}` is not on the page");
        }
        assert!(page.contains("Drone"), "the Drone is not explained");
        assert!(page.contains("does not attach"), "board is not qualified");
    }

    /// Every verb's page names its own flags, and the two everything takes.
    #[test]
    fn every_page_lists_its_flags_and_the_shared_two() {
        for page in &PAGES {
            let text = page_text(page.path);
            for (flag, _) in page.flags {
                assert!(text.contains(flag), "{} lost {flag}", page.path);
            }
            for (flag, _) in EVERYWHERE {
                assert!(text.contains(flag), "{} lost {flag}", page.path);
            }
        }
    }

    /// **The five verbs whose shape prose cannot carry have worked examples.**
    /// Each was named in the report that opened this: a synopsis alone leaves
    /// the reader guessing at the *shape* of a real call.
    #[test]
    fn the_verbs_whose_usage_is_not_obvious_carry_examples() {
        for path in [
            "fleet spawn",
            "fleet answer",
            "manifest check",
            "manifest config",
            "guild init",
        ] {
            let page = PAGES
                .iter()
                .find(|page| page.path == path)
                .unwrap_or_else(|| panic!("`{path}` has no page"));
            assert!(
                page.examples.len() >= 2,
                "`{path}` has fewer than two examples"
            );
            let text = page_text(page.path);
            assert!(text.contains("EXAMPLES"), "`{path}` renders no EXAMPLES");
            for (call, _) in page.examples {
                assert!(
                    call.starts_with("armada "),
                    "`{path}`'s example is not an invocation: {call}"
                );
                assert!(text.contains(call), "`{path}` lost the example {call}");
            }
        }
    }

    /// The scope lens appears on the two verbs that **act on** it and on
    /// neither of the two that do not.
    ///
    /// `check` refusing `--project` outright is a decision (`args.rs`), and a
    /// page offering it would contradict the parser. `init` is the subtler case:
    /// its parser goes through `common`, which reads a lens for every verb that
    /// shares it, and `init` then ignores what it read. Listing it there would
    /// document a flag that changes nothing — worse than not listing a flag that
    /// is silently tolerated, which is what this asserts instead.
    #[test]
    fn the_scope_lens_is_listed_only_where_it_is_acted_on() {
        for (path, expected) in [
            ("manifest status", true),
            ("manifest clean", true),
            ("manifest check", false),
            ("manifest init", false),
        ] {
            let text = page_text(path);
            assert_eq!(
                text.contains("--project"),
                expected,
                "`{path}` --project listed: {}",
                text.contains("--project")
            );
        }
    }

    /// The one thing PHASES.md §8.3.1 asks for: it can be read. Every line fits
    /// an ordinary terminal, and no line is padded out with trailing spaces.
    #[test]
    fn no_page_overflows_an_eighty_column_terminal() {
        let pages = [Topic::Root, Topic::Manifest, Topic::Guild, Topic::Fleet]
            .into_iter()
            .chain(PAGES.iter().map(|page| Topic::Verb(page.path)));
        for topic in pages {
            let text = render(topic, Style::plain(), Terminal::piped());
            for line in text.lines() {
                assert!(
                    super::super::term::display_width(line) <= 80,
                    "{topic:?} overflows: {line:?}"
                );
                assert!(!line.ends_with(' '), "{topic:?} trailing space: {line:?}");
            }
        }
    }

    /// Every page the renderer can draw, at a width nothing truncates.
    fn every_page() -> Vec<(Topic, String)> {
        [
            Topic::Bare,
            Topic::Root,
            Topic::Manifest,
            Topic::Guild,
            Topic::Fleet,
        ]
        .into_iter()
        .chain(PAGES.iter().map(|page| Topic::Verb(page.path)))
        .map(|topic| (topic, render(topic, Style::plain(), Terminal::at(100))))
        .collect()
    }

    /// **No spec citation reaches a reader.**
    ///
    /// The person reading `--help` has the binary and not the repository, so
    /// `(PLAN.md §3.2)` reads to them the way an error code does: it looks like
    /// it means something and there is nowhere to go and find out. A doc comment
    /// may cite a section, and so may a `next_action` an agent will act on —
    /// **rendered help may not**, and a sentence that needed the citation to
    /// carry its meaning was an incomplete sentence.
    ///
    /// Kept as a test rather than a habit for the same reason the page-per-verb
    /// check is: the next page reintroduces them otherwise.
    #[test]
    fn no_rendered_help_cites_a_document_the_reader_does_not_have() {
        for (topic, text) in every_page() {
            for citation in ["§", "PLAN.md", "PHASES.md", "ARCHITECTURE.md", "traps.md"] {
                assert!(
                    !text.contains(citation),
                    "{topic:?} cites {citation}. Help is read by someone holding \
                     the binary and not the repository; say the thing the section \
                     says, or say nothing."
                );
            }
        }
    }

    /// **Fitting in eighty columns is not the same as being readable in them.**
    /// A table degrades by truncating its description column, which is right for
    /// a `status` listing and wrong here — the description *is* the page. The
    /// widest synopsis sets that column for every row, so adding one lengthens
    /// nothing and shortens everything, silently, until a summary loses its last
    /// three words to an ellipsis.
    #[test]
    fn no_summary_on_a_list_of_verbs_is_truncated_away() {
        for (topic, module) in [
            (Topic::Root, "manifest"),
            (Topic::Root, "guild"),
            (Topic::Root, "fleet"),
            (Topic::Root, ""),
            (Topic::Manifest, "manifest"),
            (Topic::Guild, "guild"),
            (Topic::Fleet, "fleet"),
        ] {
            let text = render(topic, Style::plain(), Terminal::piped());
            for (synopsis, summary) in verbs_of(module) {
                assert!(
                    text.contains(summary),
                    "{topic:?} truncated `{synopsis}`'s summary at eighty columns"
                );
            }
        }
    }

    /// The help is human output, so it obeys the same rule everything else does:
    /// the two audiences differ in styling and in nothing else.
    #[test]
    fn the_help_is_unpainted_for_a_pipe_and_painted_for_a_terminal() {
        assert!(!root_page().contains('\x1b'));
        assert!(render(Topic::Root, Style::painted(), Terminal::at(100)).contains('\x1b'));
        // And the section that is not drawn by the table renderer obeys it too.
        assert!(!page_text("fleet spawn").contains('\x1b'));
        assert!(render(
            Topic::Verb("fleet spawn"),
            Style::painted(),
            Terminal::at(100)
        )
        .contains('\x1b'));
    }

    /// **The wordmark is on bare `armada` and on nothing else here.** A banner
    /// above `--help` is a banner in the way of the one page someone reaches for
    /// when they are in a hurry (`docs/commands/render.md`).
    #[test]
    fn the_wordmark_is_on_bare_armada_and_on_no_other_page() {
        let wide = Terminal::at(120);
        assert!(render(Topic::Bare, Style::painted(), wide).contains("█████╗"));
        for topic in [
            Topic::Root,
            Topic::Manifest,
            Topic::Verb("manifest check"),
            Topic::Verb("fleet spawn"),
        ] {
            assert!(
                !render(topic, Style::painted(), wide).contains('█'),
                "{topic:?} drew the wordmark"
            );
        }
    }
}
