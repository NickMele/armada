//! The agreed layouts for commands that **do not exist yet**, in
//! `tests/golden/render/pending/`.
//!
//! # Why these are here before the code is
//!
//! **The fixture is the specification and the code follows it.** Every file in
//! that directory is transcribed from
//! `docs/reference-output/command-output.html`, which is the reviewed and agreed
//! output for M2 and M3. None of these commands is built. **Do not delete one as
//! unused** — it is the thing the milestone implements against.
//!
//! The alternative is what would otherwise happen: M2 ships `armada doctor` with
//! whatever shape its author reached for, M3 ships `armada fleet ls` with
//! another, and both need a refactor pass afterwards to look like one tool. A
//! fixture written now means there is never a first version to replace.
//!
//! # What is checked, and what is not
//!
//! There is no renderer for any of these, so nothing here is a byte comparison.
//! What is checked is everything that must be true of **any** Armada table,
//! which is exactly the set of things a new verb's author gets wrong:
//!
//! | Rule | Why |
//! |---|---|
//! | the first column is `STATUS` | the universal shape (`render.rs`) |
//! | no cell overruns its column | a table that does not align is not a table |
//! | no status symbol anywhere | a tick only shows at a terminal; one shape is the point |
//! | no ANSI escape | this is the agent's render; colour is the other file |
//! | eighty columns, no trailing space | it has to fit and it has to diff |
//!
//! # When the verb ships
//!
//! The milestone that builds it authors the `.tty` twin — the colour of each
//! lowercase status word is a palette role that only exists once the word does —
//! moves the pair up into `tests/golden/render/`, and it becomes an ordinary
//! byte comparison in `render_golden.rs`. Nothing about the layout is renegotiated
//! at that point, which is the entire purpose of writing it down now.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn pending_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/render/pending")
}

/// The commands whose layout is agreed and unbuilt, and the milestone that owns
/// each. Named here so a fixture cannot be quietly dropped, and so the list can
/// be read against `PHASES.md` §8.
const PENDING: [(&str, &str); 7] = [
    (
        "init-machine",
        "M2 — `armada init`, the first command on a new machine",
    ),
    ("doctor", "M2 — `armada doctor`"),
    ("config-scan", "not built — `armada manifest config scan`"),
    (
        "config-verify",
        "not built — `armada manifest config verify`",
    ),
    ("fleet-ls", "M3 — `armada fleet ls`"),
    ("fleet-spawn", "M3 — `armada fleet spawn`"),
    ("guild-pull", "M2 — `armada guild pull`"),
];

fn fixture(name: &str) -> String {
    let path = pending_dir().join(format!("{name}.plain"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is missing ({e}). It is a specification, not an artefact — see this file's header.",
            path.display()
        )
    })
}

/// Every agreed layout has a file, and every file is an agreed layout. A fixture
/// with no entry here is one nobody will notice has gone stale.
#[test]
fn the_pending_set_is_exactly_what_is_listed() {
    let on_disk: BTreeSet<String> = std::fs::read_dir(pending_dir())
        .expect("tests/golden/render/pending exists")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()?.to_str()? == "plain")
                .then(|| path.file_stem()?.to_str().map(str::to_string))?
        })
        .collect();
    let listed: BTreeSet<String> = PENDING.iter().map(|(name, _)| name.to_string()).collect();
    assert_eq!(on_disk, listed);
}

/// **No `.tty` twin yet, and that is the marker.** A pending layout is the
/// agent's render — the shape — because that is what a milestone must implement.
/// The colour of `stale` or `conflict` is a palette role that does not exist
/// until the word does, and transcribing a derivation by hand is where errors
/// live.
#[test]
fn a_pending_layout_has_no_terminal_twin() {
    for (name, milestone) in PENDING {
        let tty = pending_dir().join(format!("{name}.tty"));
        assert!(
            !tty.exists(),
            "{} exists. If `{name}` ({milestone}) is now built, move the pair up \
             into tests/golden/render/ and add it to render_golden.rs.",
            tty.display()
        );
    }
}

/// The rules that hold for every table Armada draws, checked against a layout
/// that has no renderer to produce it.
#[test]
fn every_pending_layout_is_a_well_formed_armada_render() {
    for (name, milestone) in PENDING {
        let text = fixture(name);
        let context = |line: usize| format!("{name}.plain:{line} ({milestone})");

        assert!(
            text.ends_with('\n'),
            "{name}.plain does not end in a newline"
        );
        for (index, line) in text.lines().enumerate() {
            let at = context(index + 1);
            assert!(
                !line.contains('\x1b'),
                "{at} carries an ANSI escape. This is the render an agent reads."
            );
            assert!(
                line.chars().count() <= 80,
                "{at} is {} columns; eighty is the floor every terminal has",
                line.chars().count()
            );
            assert!(!line.ends_with(' '), "{at} ends in whitespace");
            for symbol in ['✓', '✗', '✔', '✘', '×', '●', '○', '⚠', '→', '—', '–']
            {
                assert!(
                    !line.contains(symbol),
                    "{at} contains {symbol}. The agent's render is ASCII, and a \
                     symbol that only appears at a terminal would give the two \
                     audiences different shapes."
                );
            }
        }

        let tables = tables(&text);
        assert!(
            !tables.is_empty(),
            "{name}.plain has no table. Every agreed layout has one."
        );
        for table in tables {
            check_table(name, milestone, &table);
        }
    }
}

/// A table: its header line and the body lines under it.
struct Block {
    header: String,
    body: Vec<String>,
    first_line: usize,
}

/// Every table in the text. A table starts at a `  STATUS` header and runs until
/// a line that is not a two-space-indented row.
fn tables(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if !is_row(lines[index]) || !lines[index].trim_start().starts_with("STATUS") {
            index += 1;
            continue;
        }
        let header = lines[index].to_string();
        let first_line = index + 1;
        index += 1;
        let mut body = Vec::new();
        while index < lines.len() && is_row(lines[index]) {
            body.push(lines[index].to_string());
            index += 1;
        }
        blocks.push(Block {
            header,
            body,
            first_line,
        });
    }
    blocks
}

/// A row of a table: exactly two spaces of indent, then content.
fn is_row(line: &str) -> bool {
    line.starts_with("  ") && !line.starts_with("   ") && line.len() > 2
}

fn check_table(name: &str, milestone: &str, block: &Block) {
    assert!(
        block.header.starts_with("  STATUS"),
        "{name}.plain:{} does not open with STATUS ({milestone}). Every table is \
         STATUS · NAME · DETAIL · TIME, status first and always a word.",
        block.first_line
    );
    assert!(
        !block.body.is_empty(),
        "{name}.plain:{} is a header with nothing under it, which claims \
         something was listed",
        block.first_line
    );

    // Where each column begins, read off the header.
    let starts = column_starts(&block.header);
    assert!(
        starts.len() >= 2,
        "{name}.plain:{} has one column, which is a list rather than a table",
        block.first_line
    );

    for (offset, row) in block.body.iter().enumerate() {
        let line = block.first_line + 1 + offset;
        let chars: Vec<char> = row.chars().collect();
        for start in starts.iter().skip(1) {
            // The two-space gap in front of every column, intact. A cell that
            // overran its column eats it, which is the one way a hand-written
            // table goes wrong and the one thing a reader notices immediately.
            let intact = chars
                .get(start - 1)
                .zip(chars.get(start - 2))
                .is_none_or(|(a, b)| *a == ' ' && *b == ' ');
            assert!(
                intact,
                "{name}.plain:{line} collides at column {start} ({milestone}):\n\
                 header {:?}\n   row {row:?}",
                block.header
            );
        }
    }
}

/// The index at which each column's header word begins.
fn column_starts(header: &str) -> Vec<usize> {
    let chars: Vec<char> = header.chars().collect();
    let mut starts = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != ' ' && (index == 0 || chars[index - 1] == ' ') {
            starts.push(index);
        }
        index += 1;
    }
    // Header words are single tokens (`STATUS`, `WORKFLOW`), so every run of
    // non-space is one column.
    starts
}
