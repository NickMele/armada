//! `--help`, restructured.
//!
//! **The old page was one block of text with flags inline**, which meant the
//! answer to "what can I run" and the answer to "what does `--orphaned` do" were
//! the same twenty-five lines, and neither could be found at a glance. PHASES.md
//! §8.3.1's done-when is that it "can be read without squinting".
//!
//! Three changes, and each is a rule rather than a preference:
//!
//! 1. **Grouped under headings** — usage, the verbs, the global flags, and what
//!    is not built. A reader arrives with one of those four questions.
//! 2. **Aligned by the same table renderer every verb uses**, so a flag column
//!    lines up here for the same reason a status column lines up there, and a
//!    narrow terminal truncates the explanation rather than wrapping it.
//! 3. **A page per verb.** The root page lists verbs with one line each and
//!    stops; `armada manifest check --help` is where `check`'s nine flags live.
//!    Putting them all on one page is what made the old one unreadable.
//!
//! **What is not built is stated, not omitted.** Half of this CLI is reserved
//! names that answer "not built yet" (`args.rs`), and a help page that lists only
//! what works leaves a reader to discover the rest by typing it. The lists come
//! from `args.rs`'s own constants rather than being retyped here, so a verb
//! cannot ship without appearing on this page — there is a test.

use crate::args::{BUILTIN_VERBS, GUILD_VERBS, RESERVED_TOP_LEVEL, TOP_LEVEL_VERBS};

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
    /// `armada manifest <verb> --help`.
    Verb(&'static str),
}

/// One built verb's page.
struct Page {
    name: &'static str,
    summary: &'static str,
    usage: &'static [&'static str],
    flags: &'static [(&'static str, &'static str)],
    notes: &'static [&'static str],
}

/// The flags every verb takes, listed on every verb's page.
///
/// **Repeated per page rather than cross-referenced.** A reader on `check`'s
/// page asking "can I have this as JSON" should not be sent to a second page to
/// find out, and the two lines cost less than the round trip.
const EVERYWHERE: [(&str, &str); 2] = [
    (
        "--json",
        "answer in the envelope (PLAN.md §3.1), not as text",
    ),
    (
        "--color <when>",
        "auto (default), always, never; NO_COLOR wins",
    ),
];

/// The scope lens, on the two verbs that take it.
const LENS: [(&str, &str); 2] = [
    ("--project", "every workspace of this repository"),
    ("--all", "every workspace on this machine"),
];

/// The verbs Manifest has built, in the order someone meets them.
///
/// **Not alphabetical.** `config` comes first because a repository has no
/// `armada.yml` before it runs, `init` next because nothing else works until
/// *it* has run, and `clean` last because it undoes the rest.
const MANIFEST: [Page; 8] = [
    Page {
        name: "config",
        summary: "report the evidence, then verify what was written",
        usage: &[
            "armada manifest config scan [--json]",
            "armada manifest config verify [--json]",
        ],
        flags: &[],
        notes: &[
            "scan reports facts and decides nothing; an agent authors; verify checks.",
            "scan is the one verb that runs in a repo with no armada.yml.",
        ],
    },
    Page {
        name: "init",
        summary: "claim this workspace: ports, .armada/, setup",
        usage: &["armada manifest init [--json] [--dry-run]"],
        flags: &[("--dry-run", "report what would happen; change nothing")],
        notes: &["Reaps what is no longer owned before it claims anything."],
    },
    Page {
        name: "up",
        summary: "start this workspace's services, and wait until they answer",
        usage: &["armada manifest up [<component>] [--json] [--dry-run]"],
        flags: &[(
            "--dry-run",
            "report the argv and the ready-checks; start nothing",
        )],
        notes: &[
            "Started is not ready: a service is up when its ready-check passes.",
            "Everything it starts is recorded before it is confirmed working.",
        ],
    },
    Page {
        name: "down",
        summary: "stop this workspace's services; keep the port block",
        usage: &["armada manifest down [<component>] [--json]"],
        flags: &[],
        notes: &[
            "down is pause and clean is release — the port block is kept.",
            "Dependents stop before their dependencies.",
        ],
    },
    Page {
        name: "status",
        summary: "what is running, what is mine, what is stale",
        usage: &["armada manifest status [--json] [--project|--all]"],
        flags: &[],
        notes: &[
            "A read verb: its exit code describes the query, not what it found.",
            "A gate uses `armada manifest check`, never a query's exit code.",
        ],
    },
    Page {
        name: "check",
        summary: "lint, format and test — the verb a gate calls",
        usage: &[
            "armada manifest check [flags] [<selector>]",
            "armada manifest check --files <path>…",
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
        notes: &[
            "A selector is <component>:<check>, a component, or a check name.",
            "One selector, or several paths — never both (PLAN.md §3.2).",
        ],
    },
    Page {
        name: "skills",
        summary: "what this repository knows about itself",
        usage: &[
            "armada manifest skills [--json]",
            "armada manifest skills show <name> [--json]",
        ],
        flags: &[],
        notes: &[
            "A skill is a named grant plus a pointer to prose Armada never parses.",
            "There is deliberately no way to run one (PLAN.md §4.8).",
        ],
    },
    Page {
        name: "clean",
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
        ],
        notes: &[
            "A declared owns.release: command is reported, never executed.",
            "--force-rebuild needs --orphaned, and takes nothing else.",
        ],
    },
];

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
        Topic::Verb(name) => match MANIFEST.iter().find(|page| page.name == name) {
            Some(page) => verb(page, style, terminal),
            // Unreachable through `args::parse`, which only produces a
            // `Verb` for a name in this table. Answering with the module page
            // rather than panicking keeps a wrong call harmless.
            None => manifest(style, terminal),
        },
    }
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
        &two_column(&[
            ("armada <module> <verb> [flags]", "the verbs Armada owns"),
            ("armada", "enter Helm, the agent you talk to"),
        ])
        .render(style, width),
    );

    out.push('\n');
    out.push_str(&heading(
        style,
        "MANIFEST — what a workspace is and how to operate it",
    ));
    let mut verbs: Vec<(&str, &str)> = MANIFEST
        .iter()
        .map(|page| (page.name, page.summary))
        .collect();
    verbs.push(("<name>", "a commands: entry from this repo's armada.yml"));
    out.push_str(&two_column(&verbs).render(style, width));

    out.push('\n');
    out.push_str(&heading(
        style,
        "GUILD — you, portable: voice, skills, hooks, workflows",
    ));
    out.push_str(
        &two_column(
            &GUILD_VERBS
                .iter()
                .filter(|(_, summary)| !summary.starts_with("M2 "))
                .copied()
                .collect::<Vec<_>>(),
        )
        .render(style, width),
    );

    out.push('\n');
    out.push_str(&heading(style, "THIS MACHINE"));
    // **`armada init` and `armada manifest init` are two verbs**, and the one
    // place a reader is most likely to confuse them is this page — so the line
    // says which is which rather than leaving the module level to imply it.
    out.push_str(&two_column(&TOP_LEVEL_VERBS).render(style, width));

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
    out.push_str(
        &two_column(
            &GUILD_VERBS
                .iter()
                .filter(|(_, summary)| !summary.starts_with("M2 "))
                .copied()
                .collect::<Vec<_>>(),
        )
        .render(style, width),
    );
    out.push('\n');
    out.push_str(&heading(style, "NOT BUILT YET"));
    out.push_str(
        &two_column(
            &GUILD_VERBS
                .iter()
                .filter(|(_, summary)| summary.starts_with("M2 "))
                .map(|(name, summary)| (*name, summary.trim_start_matches("M2 \u{2014} ")))
                .collect::<Vec<_>>(),
        )
        .render(style, width),
    );
    out.push('\n');
    out.push_str(
        "Nothing in ~/.armada/guild/ is repository content, and no part of it is ever\n\
         committed to a project. machine.yml, manifest.db, jobs/ and workspaces/ never\n\
         sync (PLAN.md \u{a7}13.1).\n",
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
    out.push_str(
        &two_column(
            &MANIFEST
                .iter()
                .map(|page| (page.name, page.summary))
                .collect::<Vec<_>>(),
        )
        .render(style, width),
    );
    out.push('\n');
    out.push_str(&heading(style, "NOT BUILT YET"));
    out.push_str(
        &two_column(&[(
            not_built_verbs().join(", ").as_str(),
            "reserved; see PHASES.md §8",
        )])
        .render(style, width),
    );
    out.push('\n');
    out.push_str("`armada manifest <verb> --help` for one verb's flags.\n");
    out
}

fn verb(page: &Page, style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    let mut out = format!(
        "{}\n\n",
        style.strong(
            Role::SignalAmber,
            &format!("armada manifest {} — {}", page.name, page.summary)
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

    let takes_lens = matches!(page.name, "status" | "clean");
    let mut flags: Vec<(&str, &str)> = page.flags.to_vec();
    if takes_lens {
        flags.extend_from_slice(&LENS);
    }
    flags.extend_from_slice(&EVERYWHERE);

    out.push('\n');
    out.push_str(&heading(style, "FLAGS"));
    out.push_str(&two_column(&flags).render(style, width));

    if !page.notes.is_empty() {
        out.push('\n');
        for note in page.notes {
            out.push_str(note);
            out.push('\n');
        }
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
/// Read off `args.rs`'s own table rather than retyped, so a verb that ships
/// leaves this list by shipping.
fn not_built_verbs() -> Vec<&'static str> {
    BUILTIN_VERBS
        .iter()
        .copied()
        .filter(|verb| !MANIFEST.iter().any(|page| page.name == *verb))
        .collect()
}

/// Everything a caller can type that answers "not built yet".
///
/// Generated from `args.rs`'s own tables, so a name claimed there cannot be
/// missing here.
fn not_built(style: Style, width: usize) -> String {
    let modules: Vec<&str> = RESERVED_TOP_LEVEL.iter().map(|(name, _)| *name).collect();
    let manifest_verbs = not_built_verbs();

    let mut out = Table::new(vec![Column::fixed(""), Column::flexible("")])
        .headerless()
        .indent(2)
        .row(vec![
            Cell::plain(format!("armada {}", modules.join(", "))),
            Cell::muted("M3"),
        ])
        .row(vec![
            Cell::plain("armada guild edit, verify"),
            Cell::muted("M2"),
        ])
        .row(vec![
            Cell::plain(format!("armada manifest {}", manifest_verbs.join(", "))),
            Cell::muted("reserved"),
        ])
        .row(vec![
            Cell::plain("armada manifest check --detach, --status"),
            Cell::muted("reserved"),
        ])
        .render(style, width);
    out.push_str(
        "\n  Each answers `not built yet` and names its milestone, rather than\n  \
         meaning something else for a release first (PHASES.md §8).\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_page() -> String {
        render(Topic::Root, Style::plain(), Terminal::at(100))
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
    }

    /// Every built verb has a page of its own, and it names its own flags.
    #[test]
    fn every_built_verb_has_a_page_listing_its_flags() {
        for page in &MANIFEST {
            let text = render(Topic::Verb(page.name), Style::plain(), Terminal::at(100));
            assert!(text.starts_with(&format!("armada manifest {}", page.name)));
            for (flag, _) in page.flags {
                assert!(text.contains(flag), "{} lost {flag}", page.name);
            }
            for (flag, _) in EVERYWHERE {
                assert!(text.contains(flag), "{} lost {flag}", page.name);
            }
        }
    }

    /// The scope lens appears on the two verbs that take it and on neither of
    /// the two that do not — `check` refusing `--project` is a decision
    /// (`args.rs`), and a help page that offered it would contradict the parser.
    #[test]
    fn the_scope_lens_is_listed_only_where_it_is_accepted() {
        for (name, expected) in [
            ("status", true),
            ("clean", true),
            ("check", false),
            ("init", false),
        ] {
            let text = render(Topic::Verb(name), Style::plain(), Terminal::at(100));
            assert_eq!(
                text.contains("--project"),
                expected,
                "`{name}` --project listed: {}",
                text.contains("--project")
            );
        }
    }

    /// The one thing PHASES.md §8.3.1 asks for: it can be read. Every line fits
    /// an ordinary terminal, and no line is padded out with trailing spaces.
    #[test]
    fn no_page_overflows_an_eighty_column_terminal() {
        let pages = [Topic::Root, Topic::Manifest]
            .into_iter()
            .chain(MANIFEST.iter().map(|page| Topic::Verb(page.name)));
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

    /// The help is human output, so it obeys the same rule everything else does:
    /// the two audiences differ in styling and in nothing else.
    #[test]
    fn the_help_is_unpainted_for_a_pipe_and_painted_for_a_terminal() {
        assert!(!root_page().contains('\x1b'));
        assert!(render(Topic::Root, Style::painted(), Terminal::at(100)).contains('\x1b'));
    }

    /// **The wordmark is on bare `armada` and on nothing else here.** A banner
    /// above `--help` is a banner in the way of the one page someone reaches for
    /// when they are in a hurry (`docs/commands/render.md`).
    #[test]
    fn the_wordmark_is_on_bare_armada_and_on_no_other_page() {
        let wide = Terminal::at(120);
        assert!(render(Topic::Bare, Style::painted(), wide).contains("█████╗"));
        for topic in [Topic::Root, Topic::Manifest, Topic::Verb("check")] {
            assert!(
                !render(topic, Style::painted(), wide).contains('█'),
                "{topic:?} drew the wordmark"
            );
        }
    }
}
