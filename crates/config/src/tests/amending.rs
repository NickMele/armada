//! A form's edits, against this repository's own `armada.yml`.
//!
//! **The hardest file in the house, on purpose.** It is two thirds comment,
//! spaces its sections apart, quotes some list items and not others, holds a
//! `{}` port and a server with links — so a writer that holds here holds on a
//! file Setup wrote, which is simpler in every way. It is compiled in, so the
//! test reads no file.

use std::path::Path;

use core_model::{AutoMerge, ReviewGate};

use crate::amending::{
    amend, CheckEdit, CommandEdit, Edit, NewCheck, NewCommand, NewLink, NewNarrowing, NewPort,
    NotAmended, PortEdit, Unplaceable,
};

const OWN: &str = include_str!("../../../../armada.yml");

fn at() -> &'static Path {
    Path::new("armada.yml")
}

fn amended(text: &str, edits: &[Edit]) -> String {
    match amend(at(), text, edits) {
        Ok(done) => done.text().to_string(),
        Err(why) => panic!("the edits apply: {why}"),
    }
}

fn check(name: &str, edit: CheckEdit) -> Edit {
    Edit::Check {
        name: name.to_string(),
        edit,
    }
}

fn command(name: &str, edit: CommandEdit) -> Edit {
    Edit::Command {
        name: name.to_string(),
        edit,
    }
}

fn port(name: &str, edit: PortEdit) -> Edit {
    Edit::Port {
        name: name.to_string(),
        edit,
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// Whether every line of `before` is in `after`, in the same order.
fn keeps_every_line(before: &str, after: &str) -> bool {
    let mut after = after.lines();
    before.lines().all(|line| after.any(|found| found == line))
}

const WHEN: &[&str] = &[
    "apps/**",
    "packages/**",
    "crates/core-model/domain/**",
    "protocol-version.toml",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
];

fn format_narrowing(from: &[&str]) -> NewNarrowing {
    NewNarrowing {
        run: "rustfmt --check --edition 2021".to_string(),
        each: "{}".to_string(),
        from: strings(from),
        under: None,
        except: Vec::new(),
    }
}

fn storybook_link() -> NewLink {
    NewLink {
        url: "http://localhost:${port.storybook}".to_string(),
        name: Some("Storybook".to_string()),
    }
}

/// Every kind of edit a form makes, once each.
fn forward() -> Vec<Edit> {
    vec![
        check(
            "build",
            CheckEdit::Run("cargo build --workspace --locked --offline".to_string()),
        ),
        check("test", CheckEdit::Requires(strings(&["bootstrap"]))),
        check(
            "typecheck",
            CheckEdit::When(
                [WHEN, &["tsconfig.json"]]
                    .concat()
                    .iter()
                    .map(|p| p.to_string())
                    .collect(),
            ),
        ),
        check(
            "format",
            CheckEdit::Narrow(Some(format_narrowing(&["**/*.rs", "xtask/**/*.rs"]))),
        ),
        check(
            "lint",
            CheckEdit::Add(NewCheck {
                run: "cargo clippy --workspace".to_string(),
                requires: strings(&["bootstrap"]),
                when: strings(&["crates/**"]),
                narrow: None,
                runner: None,
            }),
        ),
        command(
            "seed",
            CommandEdit::Add(NewCommand {
                run: Some("pnpm tsx scripts/seed.ts".to_string()),
                destructive: true,
                serve: None,
                ready: None,
                links: Vec::new(),
            }),
        ),
        command("fmt", CommandEdit::Destructive(true)),
        command(
            "storybook_dev",
            CommandEdit::Ready(Some(
                "curl -sf http://localhost:${port.storybook}/".to_string(),
            )),
        ),
        command(
            "storybook_dev",
            CommandEdit::Links(vec![
                storybook_link(),
                NewLink {
                    url: "http://localhost:${port.storybook}/?path=/docs".to_string(),
                    name: Some("Docs".to_string()),
                },
            ]),
        ),
        port(
            "web",
            PortEdit::Add(NewPort {
                container: Some(3000),
                env: Some("PORT".to_string()),
            }),
        ),
        port("storybook", PortEdit::Container(Some(6006))),
        Edit::AutoMerge(Some(AutoMerge::ChecksPass)),
        Edit::ReviewGate(Some(ReviewGate::AutoIfJudgePasses)),
        Edit::CostCapMicrosPerJob(Some(2_000_000)),
        Edit::TurnCapPerJob(Some(40)),
    ]
}

/// Every edit in [`forward`], taken back.
fn back() -> Vec<Edit> {
    vec![
        check(
            "build",
            CheckEdit::Run("cargo build --workspace --locked".to_string()),
        ),
        check("test", CheckEdit::Requires(Vec::new())),
        check("typecheck", CheckEdit::When(strings(WHEN))),
        check(
            "format",
            CheckEdit::Narrow(Some(format_narrowing(&["**/*.rs"]))),
        ),
        check("lint", CheckEdit::Remove),
        command("seed", CommandEdit::Remove),
        command("fmt", CommandEdit::Destructive(false)),
        command(
            "storybook_dev",
            CommandEdit::Ready(Some(
                "curl -sf http://localhost:${port.storybook}".to_string(),
            )),
        ),
        command("storybook_dev", CommandEdit::Links(vec![storybook_link()])),
        port("web", PortEdit::Remove),
        port("storybook", PortEdit::Container(None)),
        Edit::AutoMerge(None),
        Edit::ReviewGate(None),
        Edit::CostCapMicrosPerJob(None),
        Edit::TurnCapPerJob(None),
    ]
}

/// **The claim.** Every kind of edit a form makes, applied to this repository's
/// own Manifest: every line it had is still there in order, comments and
/// blanks included, the result loads — and taking each edit back gives the
/// file back byte for byte.
#[test]
fn every_edit_a_form_makes_round_trips_this_repositorys_manifest_byte_for_byte() {
    let comments = OWN
        .lines()
        .filter(|line| line.trim_start().starts_with('#'));
    // A floor under the file's own count, which shrank when #849 split the
    // Bridge's tests into one Check per package.
    assert!(comments.count() > 150, "the fixture is the commented file");

    let edited = amended(OWN, &forward());
    // The three lines an edit rewrites in place; every other line stays.
    let named = [
        "    run: cargo build --workspace --locked",
        "    ready: curl -sf http://localhost:${port.storybook}",
        "  storybook: {}",
    ];
    let untouched: String = OWN
        .lines()
        .filter(|line| !named.contains(line))
        .map(|line| format!("{line}\n"))
        .collect();
    assert!(
        keeps_every_line(&untouched, &edited),
        "a line the edits did not name moved or went:\n{edited}"
    );
    let loaded = crate::Manifest::parse(at(), &edited).expect("the edited file loads");
    assert_eq!(
        loaded.checks_as_written().last().map(String::as_str),
        Some("lint")
    );
    assert_eq!(loaded.turn_cap(), Some(40));
    assert_eq!(loaded.auto_merge(), AutoMerge::ChecksPass);

    assert_eq!(amended(&edited, &back()), OWN, "taking every edit back");
}

/// Each edit on its own, taken back on its own — so a pair that only cancels
/// out when run together cannot hide in the one above. **The additions carry
/// no comment**, so taking them back gives the file back byte for byte even
/// though a removal takes the comment block directly above what it removes.
#[test]
fn each_edit_taken_back_on_its_own_leaves_the_file_as_it_was() {
    for (there, again) in forward().into_iter().zip(back()) {
        let edited = amended(OWN, std::slice::from_ref(&there));
        assert_ne!(edited, OWN, "{there:?} changed nothing");
        assert_eq!(amended(&edited, &[again]), OWN, "{there:?} taken back");
    }
}

/// Fix, as Setup draws it: one Check's command corrected. **One line differs.**
#[test]
fn correcting_a_checks_command_changes_that_line_and_no_other() {
    let edited = amended(
        OWN,
        &[check(
            "typecheck",
            CheckEdit::Run("pnpm -r typecheck".to_string()),
        )],
    );
    let changed: Vec<(&str, &str)> = OWN
        .lines()
        .zip(edited.lines())
        .filter(|(was, now)| was != now)
        .collect();
    assert_eq!(
        changed,
        [("    run: pnpm typecheck", "    run: pnpm -r typecheck")]
    );
    assert_eq!(OWN.len() + 3, edited.len());
}

/// A value sent back unchanged is not an edit, and the file is not touched.
#[test]
fn an_edit_to_the_value_already_there_changes_nothing() {
    let same = [
        check(
            "test",
            CheckEdit::Run(
                "cargo nextest run --workspace --exclude acceptance --test-threads ${width}"
                    .to_string(),
            ),
        ),
        check("typecheck", CheckEdit::When(strings(WHEN))),
        command("fmt", CommandEdit::Destructive(false)),
        port("storybook", PortEdit::Container(None)),
    ];
    assert_eq!(amended(OWN, &same), OWN);
}

/// `text` with its lines `first..=last` taken out, counted from zero.
fn without(text: &str, first: usize, last: usize) -> String {
    text.lines()
        .enumerate()
        .filter(|(at, _)| !(first..=last).contains(at))
        .map(|(_, line)| format!("{line}\n"))
        .collect()
}

/// Where `line` is in `text`, exactly once.
fn line_of(text: &str, line: &str) -> usize {
    let found: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, have)| *have == line)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(found.len(), 1, "`{line}` is in the file once");
    found[0]
}

/// **The owner's rule, on the file it was decided against.** Removing the last
/// port takes `ports:` — and the comment block written directly above that goes
/// with it. `mock` joined `storybook` under `ports:` in #1258, so it takes both
/// removals to empty the section. The blank lines and the comments set off by
/// them either side stay, and so does every other byte.
#[test]
fn removing_an_entry_takes_the_comment_block_directly_above_it_and_nothing_else() {
    let edited = amended(
        OWN,
        &[
            port("storybook", PortEdit::Remove),
            port("mock", PortEdit::Remove),
        ],
    );
    let comment = line_of(
        OWN,
        "# Armada places these, so two worktrees never bind the same number.",
    );
    let storybook = line_of(OWN, "  storybook: {}");
    let last = line_of(OWN, "  mock: {}");
    assert_eq!(
        storybook,
        comment + 2,
        "the comment is attached: no blank between"
    );
    assert_eq!(last, storybook + 1, "the ports are the section, in order");
    assert_eq!(
        edited,
        without(OWN, comment - 1, last),
        "the attached comment, the section, and the blank that spaced it — nothing else"
    );
    assert!(edited.contains(
        "        name: Storybook\n\n# How this repository shows what a change looks like."
    ));
    assert!(crate::Manifest::parse(at(), &edited).is_ok());
}

/// A comment set off by a blank line is not the entry's. The `storybook`
/// Check has one above it and none attached, so only its own lines go.
#[test]
fn removing_an_entry_with_a_blank_line_above_it_leaves_the_comments_around_it() {
    let edited = amended(OWN, &[check("storybook", CheckEdit::Remove)]);
    let key = line_of(OWN, "  storybook:");
    let last = line_of(
        OWN,
        "    # Shorter than the two above: nothing under `apps/` concerns it.",
    ) + 5;
    assert_eq!(OWN.lines().nth(key - 1), Some(""), "a blank line above it");
    assert_eq!(edited, without(OWN, key - 1, last));
}

/// A Check with a comment block written directly over it takes the block, and
/// the blank line above the block that spaced it; the comment starting the
/// next section, set off by a blank, stays.
#[test]
fn removing_a_check_takes_the_comment_written_over_it() {
    let edited = amended(OWN, &[check("format", CheckEdit::Remove)]);
    let comment = line_of(
        OWN,
        "  # `format`, not `fmt`: that name is a Command below and both registries",
    );
    let next = line_of(
        OWN,
        "# Commands gate nothing — they are what a Drone is given to run.",
    );
    assert_eq!(edited, without(OWN, comment - 1, next - 2));
}

/// Text that would read back as something else is quoted, and a list that
/// quotes its items gets a quoted item.
#[test]
fn a_value_that_would_read_as_something_else_is_written_so_it_does_not() {
    for run in [
        "true",
        "echo a #b",
        "*.rs",
        "08",
        "say: \"hi\"",
        "two\nlines",
    ] {
        let done = amend(
            at(),
            OWN,
            &[check("build", CheckEdit::Run(run.to_string()))],
        )
        .unwrap_or_else(|why| panic!("{run:?}: {why}"));
        assert_eq!(done.manifest().check("build").map(|c| c.run()), Some(run));
    }
    let edited = amended(
        OWN,
        &[check(
            "storybook",
            CheckEdit::When(strings(&[
                "packages/**",
                "package.json",
                "pnpm-lock.yaml",
                "pnpm-workspace.yaml",
                "apps/**",
            ])),
        )],
    );
    assert!(edited.contains("      - \"pnpm-workspace.yaml\"\n      - \"apps/**\"\n"));
}

/// **What a form produces always loads.** A Check requiring a Command nothing
/// declares is refused with the parser's own faults, and nothing comes back.
#[test]
fn an_edit_whose_result_would_not_load_is_refused_with_its_faults() {
    let refused = amend(
        at(),
        OWN,
        &[check(
            "test",
            CheckEdit::Requires(strings(&["nothing_declares_this"])),
        )],
    );
    let Err(NotAmended::Refused(why)) = refused else {
        panic!("refused for what it says, not placed: {refused:?}");
    };
    assert!(
        why.refusals()
            .iter()
            .any(|refusal| refusal.key.starts_with("checks.test.requires")),
        "{:?}",
        why.refusals()
    );
}

/// A form drawn from another reading names what is not there, or adds what is.
#[test]
fn an_edit_naming_what_the_file_does_not_hold_is_refused_before_anything_moves() {
    let missing = amend(at(), OWN, &[check("lint", CheckEdit::Run("x".to_string()))]);
    assert!(
        matches!(missing, Err(NotAmended::Misnamed { ref key, declared: false }) if key == "checks.lint")
    );
    let taken = amend(
        at(),
        OWN,
        &[command(
            "fmt",
            CommandEdit::Add(NewCommand {
                run: Some("cargo fmt".to_string()),
                destructive: false,
                serve: None,
                ready: None,
                links: Vec::new(),
            }),
        )],
    );
    assert!(
        matches!(taken, Err(NotAmended::Misnamed { ref key, declared: true }) if key == "commands.fmt")
    );
}

/// A shape outside the subset is refused at the key an edit touches, and the
/// rest of the file is still editable.
#[test]
fn a_value_written_across_lines_is_refused_where_it_is_touched_and_nowhere_else() {
    let text = "version: 1\nid: here\nchecks:\n  build:\n    run: |\n      cargo build\n  test:\n    run: cargo test\n";
    let refused = amend(
        at(),
        text,
        &[check("build", CheckEdit::Run("cargo b".to_string()))],
    );
    assert!(
        matches!(
            refused,
            Err(NotAmended::Unplaceable {
                why: Unplaceable::Shape(_),
                ..
            })
        ),
        "{refused:?}"
    );
    let edited = amended(
        text,
        &[check(
            "test",
            CheckEdit::Run("cargo nextest run".to_string()),
        )],
    );
    assert_eq!(edited, text.replace("cargo test", "cargo nextest run"));
}

/// A section the file never wrote is written, and goes again when its last key
/// does — a `drone:` holding nothing would not load.
#[test]
fn a_section_the_file_never_wrote_is_added_and_goes_with_its_last_key() {
    let text = "version: 1\nid: here\nchecks:\n  build:\n    run: cargo build";
    let edited = amended(text, &[Edit::TurnCapPerJob(Some(12))]);
    assert_eq!(edited, format!("{text}\ndrone:\n  turn_cap_per_job: 12"));
    assert_eq!(amended(&edited, &[Edit::TurnCapPerJob(None)]), text);
}
