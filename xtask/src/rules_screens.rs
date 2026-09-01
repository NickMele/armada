//! The Bridge half of the visual gate: every arrangement the app can draw has
//! a screen somebody can look at, and every screen has a snapshot in the tree.
//!
//! `pnpm shoot --bridge` renders the app's own compositions and writes PNGs
//! under `.shots/`, which is ignored. That makes it a tool an agent has to
//! choose to run, and choosing not to look is the failure it was built to end —
//! one level down, a screen shipped about thirty differences from its drawing
//! with every gate green because nobody had turned a render into an image.
//!
//! This rule is the two halves of "kept up to date" that a tool cannot be:
//!
//! - **A render with no screen is a state nobody has looked at.** `Render` in
//!   `render.ts` is the app's own union of arrangements, so the count is the
//!   app's rather than a list somebody maintains beside it. Two of the five
//!   had no shot when the rule was written, and neither was noticed.
//! - **A screen with no snapshot leaves nothing in a diff.** The markup is the
//!   cheap half of a shot and it is text, so a rebuilt header shows up in
//!   review whether or not anybody ran the tool. Whether the snapshot is
//!   *current* is `pnpm shoot --bridge --check`, which has to render to know;
//!   this rule checks the file is there at all.
//!
//! **No TS parser, and the gate keeps no dependencies** — so this reads the two
//! shapes these files have: a `Render` union of quoted strings, and a
//! `render: "..."` line inside a screen entry.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// The union that says how many arrangements job detail has.
const RENDER: &str = "apps/desktop/src/renderer/src/render.ts";

/// Where a screen is declared, and where its snapshot lands.
const SRC: &str = "apps/desktop/src/renderer/src";
const SNAPSHOTS: &str = "apps/desktop/screens/snapshots";

/// The command that writes what is missing, named wherever this rule fails.
const WRITE: &str = "pnpm shoot --bridge";

pub fn every_render_has_a_screen(root: &Path) -> Report {
    let mut report = Report::new("every render the app draws has a screen, and a snapshot");

    let renders = renders_of(root, &mut report);
    let screens = screens_in(root, &mut report);

    if screens.is_empty() {
        report.fail(format!(
            "{SRC} — no `*.screens.tsx` declares a screen. Nothing in Bridge can be looked at \
             without taking the owner's machine"
        ));
        return report;
    }

    for render in &renders {
        if !screens.iter().any(|(_, r)| r == render) {
            report.fail(format!(
                "{RENDER} — `{render}` is an arrangement job detail takes and no screen draws \
                 it. Add one to a `*.screens.tsx` and run `{WRITE}`"
            ));
        }
    }

    // The other direction. A screen naming a render the union does not hold is
    // a screen of nothing — usually a variant that was renamed, and the shot
    // goes on being captured under the old name.
    for (mark, render) in &screens {
        if !renders.contains(render) {
            report.fail(format!(
                "{SRC} — `{mark}` says `render: \"{render}\"`, which {RENDER} does not hold. \
                 A screen of an arrangement that does not exist is captured and read by nobody"
            ));
        }
    }

    let held: BTreeSet<String> = files_with_ext(root, &root.join(SNAPSHOTS), &["html"])
        .iter()
        .filter_map(|p| p.rsplit('/').next().map(|f| f.trim_end_matches(".html").to_string()))
        .collect();

    for (mark, _) in &screens {
        if !held.contains(mark) {
            report.fail(format!(
                "{SNAPSHOTS} — `{mark}` has no snapshot, so a change to it lands in no diff. \
                 Run `{WRITE}`"
            ));
        }
    }
    for mark in &held {
        if !screens.iter().any(|(m, _)| m == mark) {
            report.fail(format!(
                "{SNAPSHOTS}/{mark}.html — a snapshot for a screen nothing declares. \
                 Run `{WRITE}`, which removes it"
            ));
        }
    }

    report
}

/// The arrangements, read off the union rather than listed here.
fn renders_of(root: &Path, report: &mut Report) -> BTreeSet<String> {
    let Ok(text) = fs::read_to_string(root.join(RENDER)) else {
        report.fail(format!("{RENDER} — not readable, so no render can be counted"));
        return BTreeSet::new();
    };
    // `export type Render = "working" | "reviewing" | ...`, which may wrap.
    let Some(at) = text.find("export type Render =") else {
        report.fail(format!(
            "{RENDER} — no `export type Render =`. This rule reads that union and nothing else"
        ));
        return BTreeSet::new();
    };
    let rest = &text[at..];
    let end = rest.find(';').unwrap_or(rest.len());
    let found = quoted(&rest[..end]);
    if found.is_empty() {
        report.fail(format!("{RENDER} — `Render` names no arrangement this rule could read"));
    }
    found
}

/// Every screen declared under `SRC`, as its mark and the render it draws.
///
/// Read as a pair rather than as two lists, because the failure this catches is
/// one entry missing a field — and two independent counts would agree while the
/// rows they came from did not.
fn screens_in(root: &Path, report: &mut Report) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in files_with_ext(root, &root.join(SRC), &["tsx"]) {
        if !path.ends_with(".screens.tsx") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            report.fail(format!("{path} — not readable"));
            continue;
        };
        let mut mark: Option<String> = None;
        for line in text.lines() {
            let line = line.trim();
            if let Some(value) = field(line, "mark:") {
                if let Some(orphan) = mark.replace(value) {
                    report.fail(format!(
                        "{path} — `{orphan}` states a mark and no `render:` before the next one. \
                         Every screen says which arrangement it is"
                    ));
                }
            } else if let Some(value) = field(line, "render:") {
                match mark.take() {
                    Some(m) => out.push((m, value)),
                    // The type's own `render: Render` field, and any other
                    // `render:` outside an entry. Not a screen and not a fault.
                    None => {}
                }
            }
        }
        if let Some(orphan) = mark {
            report.fail(format!(
                "{path} — `{orphan}` states a mark and no `render:`. Every screen says which \
                 arrangement it is"
            ));
        }
    }
    out
}

/// `key: "value",` — the one shape a screen entry's fields have.
fn field(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim();
    let inner = rest.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

/// Every double-quoted string in a fragment, in order of appearance.
fn quoted(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        found.insert(after[..close].to_string());
        rest = &after[close + 1..];
    }
    found
}
