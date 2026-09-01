//! The registry and the contract say the same thing.
//!
//! Split from the rule beside it because the subject is a different file: that
//! module reads a TOML registry for entries that are whole, this one reads the
//! Markdown contract for a map that has drifted from it.

use std::collections::{BTreeMap, BTreeSet};

use super::{Entry, CONTRACT, REGISTRY};
use crate::Report;

/// One line of the contract's key map: the keys it names, and everything after.
pub struct MapLine {
    pub line: usize,
    pub keys: String,
    pub rest: String,
}

/// The two fenced blocks under "### Two tiers", split into keys and the rest.
///
/// The map is left in the contract rather than replaced by a pointer, because
/// the document is pasted whole into a design session and a pointer resolves to
/// nothing there. What stops the two drifting is this rule, not a convention.
pub fn read_map(text: &str, report: &mut Report) -> Vec<MapLine> {
    let mut found = Vec::new();
    let mut in_section = false;
    let mut in_block = false;

    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim_end();
        if line.starts_with("### ") {
            in_section = line == "### Two tiers";
            continue;
        }
        if !in_section {
            continue;
        }
        if line.trim_start().starts_with("```") {
            in_block = !in_block;
            continue;
        }
        if !in_block || line.trim().is_empty() {
            continue;
        }
        let Some((keys, rest)) = line.split_once("  ") else {
            report.fail(format!(
                "{CONTRACT}:{} — `{}` is a key map line with no gap between the keys and \
                 what they do. The gate splits on two spaces",
                n + 1,
                line.trim()
            ));
            continue;
        };
        found.push(MapLine {
            line: n + 1,
            keys: keys.trim().to_string(),
            rest: rest.trim().to_string(),
        });
    }

    if found.is_empty() {
        report.fail(format!(
            "{CONTRACT} — no key map under `### Two tiers`. The registry is compared against \
             nothing"
        ));
    }
    found
}

/// The registry and the contract's map name the same bindings, and the map's
/// annotations say what the row says.
///
/// Matched on the binding, which is what moves — `r` went from redirect to
/// review and back. The verb is checked as a substring of the line, because the
/// map's second column is a sentence where the registry's is one word.
pub fn against_the_contract(
    entries: &BTreeMap<String, Entry>,
    map: &[MapLine],
    report: &mut Report,
) {
    // One key can be two acts on two surfaces — `Enter` opens a focused job on a
    // list and the step's log on detail — so a shortcut does not name a row on
    // its own. Rows are gathered per shortcut and the line picks between them by
    // the scope it states, which is where the map already carries it.
    let mut by_key: BTreeMap<&str, Vec<(&String, &Entry)>> = BTreeMap::new();
    for (id, e) in entries {
        by_key.entry(e.get("shortcut")).or_default().push((id, e));
    }

    for line in map {
        let Some(rows) = by_key.get(line.keys.as_str()) else {
            report.fail(format!(
                "{CONTRACT}:{} — `{}` is bound in the key map and has no row in {REGISTRY}. \
                 The registry is the source the map is transcribed from",
                line.line, line.keys
            ));
            continue;
        };
        let rest = line.rest.to_lowercase();
        // With one row the shortcut is unambiguous. With several, the line says
        // which surface it is about — and a line that does not is the real
        // fault, because a reader cannot tell either.
        let Some((id, entry)) = (match rows.as_slice() {
            [only] => Some(only),
            many => many.iter().find(|(_, e)| rest.contains(&e.get("scope").to_lowercase())),
        }) else {
            report.fail(format!(
                "{CONTRACT}:{} — `{}` is bound to {} acts in {REGISTRY} and this line names no \
                 scope, so it cannot say which. Name the scope the way the other lines do",
                line.line,
                line.keys,
                rows.len()
            ));
            continue;
        };
        if !rest.contains(&entry.get("verb").to_lowercase()) {
            report.fail(format!(
                "{CONTRACT}:{} — `{}` reads `{}` here and `{}` in {REGISTRY}. One verb per act",
                line.line,
                line.keys,
                line.rest,
                entry.get("verb")
            ));
        }
        if rest.contains("(confirms)") != entry.is("confirms") {
            report.fail(format!(
                "{CONTRACT}:{} — `{}` is annotated `(confirms)` in one of the two and not the \
                 other. `{id}` is the row",
                line.line, line.keys
            ));
        }
        // The map is what a design session pastes and the palette is drawn
        // from, so a binding no act answers has to say so there too — a
        // drawn palette row nobody can press is the defect #233 reported.
        if rest.contains("not built") != entry.declares("unbuilt") {
            report.fail(format!(
                "{CONTRACT}:{} — `{}` is annotated `not built` in one of the two and not the \
                 other. `{id}` is the row",
                line.line, line.keys
            ));
        }
        for (annotation, scope) in [
            ("detail only", "detail"),
            ("piloted job only", "piloted job"),
            ("job board only", "job board"),
        ] {
            if rest.contains(annotation) && entry.get("scope") != scope {
                report.fail(format!(
                    "{CONTRACT}:{} — `{}` is `{annotation}` here and `scope = \"{}\"` in \
                     {REGISTRY}",
                    line.line,
                    line.keys,
                    entry.get("scope")
                ));
            }
        }
    }

    let bound: BTreeSet<&str> = map.iter().map(|l| l.keys.as_str()).collect();
    for (id, entry) in entries {
        let shortcut = entry.get("shortcut");
        if !bound.contains(shortcut) {
            report.fail(format!(
                "{REGISTRY}:{} — `{id}` is bound to `{shortcut}`, which no line of the key map \
                 in {CONTRACT} names. A binding the palette cannot draw is one nobody discovers",
                entry.line
            ));
        }
    }
}
