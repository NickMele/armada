//! The action half of the gate: every act carries a verb, a glyph and a
//! binding, and the contract's key map says the same thing this registry does.
//!
//! `docs/contracts/design-system.md` has promised the closure since the key map
//! was settled — "generated from one source with a test asserting no entry is
//! missing any of the three". The source is
//! `crates/core-model/domain/actions.toml` and this is the test. Its value is
//! that a new action cannot ship incomplete; without it, every action added has
//! to be back-filled by hand, which is what happened to Copy debug info.
//!
//! **The glyph column is checked against the icon registry, not restated.**
//! An `icon` is a key in `packages/icons/icons.toml`, which stays the authority
//! on what a silhouette may mean.
//!
//! **A row Bridge cannot answer yet says which issue answers it.** The
//! registry is ahead of the app on purpose — `p` and `s` were bound when the
//! map was settled and neither act exists — and `unbuilt` is what stops that
//! reading as the file claiming an act that is there. See [`unbuilt`].
//!
//! **No `toml` crate**, for the reason [`crate::rules_icons`] has none: the
//! gate keeps no dependencies. This is a line parser for the one shape the
//! registry has, and a line it cannot read is reported as one.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::Report;

use self::contract::MapLine;

const REGISTRY: &str = "crates/core-model/domain/actions.toml";
const ICONS: &str = "packages/icons/icons.toml";
const CONTRACT: &str = "docs/contracts/design-system.md";

/// The keys a row may carry. An unrecognised one is a typo, and the value it
/// holds is read by nobody.
const KEYS: &[&str] = &[
    "kind",
    "tier",
    "verb",
    "icon",
    "icon_absent",
    "shortcut",
    "scope",
    "destructive",
    "confirms",
    "unbuilt",
    "notes",
];

const KINDS: &[&str] = &["Action", "Motion"];
const TIERS: &[&str] = &["Global", "Contextual"];
const ABSENCES: &[&str] = &["undecided", "by design"];
const SCOPES: &[&str] = &[
    "anywhere",
    "list",
    "list and detail",
    "detail",
    "job board",
    "piloted job",
    "dispatch card",
];

/// The QWERTY rows, for the one safety rule that is a fact about the layout:
/// a destructive key is never adjacent to a navigation key.
const ROWS: &[&str] = &["qwertyuiop", "asdfghjkl", "zxcvbnm"];

/// One `[actions.<id>]` table, as read.
pub struct Entry {
    pub line: usize,
    pub fields: BTreeMap<String, String>,
}

impl Entry {
    fn get(&self, key: &str) -> &str {
        self.fields.get(key).map(String::as_str).unwrap_or_default()
    }
    fn is(&self, key: &str) -> bool {
        self.get(key) == "true"
    }
    /// Whether the row carries the key at all, which `get` cannot say: an
    /// absent column and one set to `""` read the same through it.
    fn declares(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }
}

/// Rule twenty-five: every action carries a verb, an icon and a shortcut, and
/// the contract's map names exactly what the registry holds.
pub fn every_action_carries_three_columns(root: &Path) -> Report {
    let mut report = Report::new("every action carries a verb, an icon and a shortcut");

    let Ok(text) = fs::read_to_string(root.join(REGISTRY)) else {
        report.fail(format!(
            "{REGISTRY} — the action registry the design contract's key map is generated from"
        ));
        return report;
    };
    let entries = read_registry(&text, &mut report);

    let glyphs = match fs::read_to_string(root.join(ICONS)) {
        Ok(icons) => glyph_statuses(&icons),
        Err(_) => {
            report.fail(format!(
                "{ICONS} — the icon registry `icon` is checked against"
            ));
            BTreeMap::new()
        }
    };

    let map = match fs::read_to_string(root.join(CONTRACT)) {
        Ok(contract) => contract::read_map(&contract, &mut report),
        Err(_) => {
            report.fail(format!(
                "{CONTRACT} — the contract whose key map this registry is the source of"
            ));
            Vec::new()
        }
    };

    check(&entries, &glyphs, &map, &mut report);
    report
}

/// Every glyph the icon registry declares, and its status.
///
/// The registry's own well-formedness findings are dropped: `rules_icons`
/// already reports them, and a second rule repeating them reads as two defects
/// where there is one. Parsing the file a second way would be worse — a second
/// reader of one shape is the drift a registry exists to prevent.
fn glyph_statuses(text: &str) -> BTreeMap<String, Option<String>> {
    let mut aside = Report::new("the icon registry, read for another rule");
    crate::rules_icons::read_registry(text, &mut aside)
        .into_iter()
        .map(|(name, entry)| (name, entry.status))
        .collect()
}

/// Every `[actions.<id>]` table, reporting anything malformed as it goes.
pub fn read_registry(text: &str, report: &mut Report) -> BTreeMap<String, Entry> {
    let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
    let mut current: Option<String> = None;

    for (n, raw) in text.lines().enumerate() {
        let (line, ln) = (raw.trim(), n + 1);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = None;
            let Some(id) = inner.strip_prefix("actions.").filter(|id| !id.is_empty()) else {
                report.fail(format!(
                    "{REGISTRY}:{ln} — `{line}` is not a table this registry defines. \
                     It has `[actions.<id>]` and nothing else"
                ));
                continue;
            };
            if let Some(first) = entries.get(id) {
                report.fail(format!(
                    "{REGISTRY}:{ln} — `{id}` already has a table at line {}. \
                     The second overwrites the first",
                    first.line
                ));
                continue;
            }
            entries.insert(
                id.to_string(),
                Entry {
                    line: ln,
                    fields: BTreeMap::new(),
                },
            );
            current = Some(id.to_string());
            continue;
        }

        let Some((key, value)) = line.split_once(" = ") else {
            report.fail(format!(
                "{REGISTRY}:{ln} — `{line}` is neither a table header nor `key = value`"
            ));
            continue;
        };
        let key = key.trim();
        if !KEYS.contains(&key) {
            report.fail(format!(
                "{REGISTRY}:{ln} — `{key}` is not a key this registry defines"
            ));
            continue;
        }
        let Some(entry) = current.as_ref().and_then(|c| entries.get_mut(c)) else {
            report.fail(format!(
                "{REGISTRY}:{ln} — `{key}` sits under no `[actions.<id>]` table"
            ));
            continue;
        };
        if entry.fields.insert(key.into(), unquote(value)).is_some() {
            report.fail(format!(
                "{REGISTRY}:{ln} — `{key}` is set twice on this row"
            ));
        }
    }
    entries
}

/// Every check the rule makes, over what was read.
pub fn check(
    entries: &BTreeMap<String, Entry>,
    glyphs: &BTreeMap<String, Option<String>>,
    map: &[MapLine],
    report: &mut Report,
) {
    let mut undecided: Vec<String> = Vec::new();
    let mut registered: Vec<String> = Vec::new();
    let mut bindings: BTreeMap<(&str, &str, &str), &str> = BTreeMap::new();

    for (id, entry) in entries {
        let at = format!("{REGISTRY}:{}", entry.line);
        one_of(&at, id, entry, "kind", KINDS, report);
        one_of(&at, id, entry, "tier", TIERS, report);
        one_of(&at, id, entry, "scope", SCOPES, report);
        columns(&at, id, entry, glyphs, &mut undecided, report);
        unbuilt(&at, id, entry, &mut registered, report);

        if entry.is("destructive") && !entry.is("confirms") {
            report.fail(format!(
                "{at} — `{id}` is destructive and does not confirm. Every destructive action \
                 confirms, even from the keyboard"
            ));
        }
        // Scope is part of the key, not decoration. `Enter` is `open_focused`
        // on a list and `open_log` on detail, and those are one binding each on
        // a surface a person is looking at — not two acts fighting over a key.
        // Keying on tier and shortcut alone refused a registry that was right.
        let (tier, shortcut, scope) =
            (entry.get("tier"), entry.get("shortcut"), entry.get("scope"));
        if let Some(first) = bindings.insert((tier, shortcut, scope), id) {
            report.fail(format!(
                "{at} — `{id}` and `{first}` are both bound to `{shortcut}` in the {tier} tier, \
                 both scoped `{scope}`. One binding, one act"
            ));
        }
    }

    adjacency(entries, report);
    contract::against_the_contract(entries, map, report);

    if !undecided.is_empty() {
        let count = undecided.len();
        let awaits = if count == 1 {
            "action awaits"
        } else {
            "actions await"
        };
        report.warn(format!(
            "{REGISTRY} — {count} {awaits} a glyph: {}. Each is a decision for {ICONS}, \
             which is the authority on what a silhouette may mean",
            undecided.join(", ")
        ));
    }

    if !registered.is_empty() {
        let count = registered.len();
        let binds = if count == 1 { "binding" } else { "bindings" };
        report.warn(format!(
            "{REGISTRY} — {count} {binds} registered and not built: {}. The registry is ahead \
             of Bridge deliberately; each names the issue that closes the gap",
            registered.join(", ")
        ));
    }
}

/// A binding whose act Bridge does not have names the issue that builds it.
///
/// The palette draws a binding beside every entry, so a row nothing answers is
/// a key a person will press. Without a column saying which rows those are,
/// the alternative is a list of them living in `apps/`, which is the second
/// registry this file exists to prevent — and the gate would keep reading as
/// though the app were wrong rather than behind. `icon_absent` is the same
/// idiom one column over: a field that exists only where something is missing
/// and carries why.
///
/// The value is an issue reference and nothing else. A prose excuse here would
/// be unfollowable, and the point of the column is that the gap is tracked.
fn unbuilt(at: &str, id: &str, entry: &Entry, registered: &mut Vec<String>, report: &mut Report) {
    if !entry.declares("unbuilt") {
        return;
    }
    let issue = entry.get("unbuilt");
    let numbered = issue
        .strip_prefix('#')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if !numbered {
        report.fail(format!(
            "{at} — `unbuilt = \"{issue}\"` on `{id}` is not an issue reference. \
             A binding with no act names the issue that gives it one, as `#265`"
        ));
        return;
    }
    registered.push(format!("{id} ({issue})"));
}

/// The three columns, and what a blank one has to say for itself.
fn columns(
    at: &str,
    id: &str,
    entry: &Entry,
    glyphs: &BTreeMap<String, Option<String>>,
    undecided: &mut Vec<String>,
    report: &mut Report,
) {
    for column in ["verb", "shortcut"] {
        if entry.get(column).is_empty() {
            report.fail(format!("{at} — `{id}` has no `{column}`"));
        }
    }

    let (icon, absent) = (entry.get("icon"), entry.get("icon_absent"));
    if entry.get("kind") == "Motion" {
        if !icon.is_empty() || !absent.is_empty() {
            report.fail(format!(
                "{at} — `{id}` is a Motion and carries a glyph column. A motion moves the \
                 cursor and appears in no palette, so it has no third column to fill"
            ));
        }
        return;
    }

    match (icon.is_empty(), absent.is_empty()) {
        (true, true) => report.fail(format!(
            "{at} — `{id}` has no `icon` and no `icon_absent` saying why. An action carries a \
             verb, an icon and a shortcut, and a blank one says which of those it is missing"
        )),
        (false, false) => report.fail(format!(
            "{at} — `{id}` carries both `icon = \"{icon}\"` and `icon_absent = \"{absent}\"`. \
             A glyph that is there is not absent"
        )),
        (true, false) => {
            if !ABSENCES.contains(&absent) {
                report.fail(format!(
                    "{at} — `icon_absent = \"{absent}\"` on `{id}` is outside the declared set: {}",
                    ABSENCES.join(", ")
                ));
            }
            if absent == "undecided" {
                undecided.push(id.to_string());
            }
            if entry.get("notes").is_empty() {
                report.fail(format!(
                    "{at} — `{id}` has a blank glyph column and no `notes`. A gap carried \
                     without its reason is one nobody can close"
                ));
            }
        }
        (false, true) => match glyphs.get(icon).map(|s| s.as_deref()) {
            None => report.fail(format!(
                "{at} — `{id}` names the glyph `{icon}`, which has no entry in {ICONS}. \
                 A glyph is decided in the icon registry before it is used"
            )),
            Some(Some("Banned")) => report.fail(format!(
                "{at} — `{id}` names `{icon}`, which is banned in {ICONS}"
            )),
            Some(_) => {}
        },
    }
}

/// A destructive single key is never adjacent to a navigation key.
///
/// Same-row neighbours only, which is the reading the contract's own example
/// carries: kill is `x` and never `k`, because `k` sits against `j`.
fn adjacency(entries: &BTreeMap<String, Entry>, report: &mut Report) {
    let motion: BTreeSet<char> = entries
        .values()
        .filter(|e| e.get("kind") == "Motion")
        .flat_map(|e| {
            e.get("shortcut")
                .split('/')
                .filter_map(|k| one_char(k.trim()))
                .collect::<Vec<char>>()
        })
        .collect();

    for (id, entry) in entries {
        if !entry.is("destructive") {
            continue;
        }
        let Some(key) = one_char(entry.get("shortcut")) else {
            continue;
        };
        for next in neighbours(key) {
            if motion.contains(&next) {
                report.fail(format!(
                    "{REGISTRY}:{} — `{id}` is destructive and bound to `{key}`, which sits \
                     against the navigation key `{next}`. A mistyped navigation keystroke \
                     must not be able to reach it",
                    entry.line
                ));
            }
        }
    }
}

/// A field whose value comes from a closed set.
fn one_of(at: &str, id: &str, entry: &Entry, key: &str, set: &[&str], report: &mut Report) {
    let value = entry.get(key);
    if !set.contains(&value) {
        report.fail(format!(
            "{at} — `{key} = \"{value}\"` on `{id}` is outside the declared set: {}",
            set.join(", ")
        ));
    }
}

/// The letters either side of `key` on its own row of the keyboard.
fn neighbours(key: char) -> Vec<char> {
    for row in ROWS {
        let chars: Vec<char> = row.chars().collect();
        let Some(at) = chars.iter().position(|c| *c == key) else {
            continue;
        };
        let mut out = Vec::new();
        if at > 0 {
            out.push(chars[at - 1]);
        }
        if at + 1 < chars.len() {
            out.push(chars[at + 1]);
        }
        return out;
    }
    Vec::new()
}

/// A binding that is one key, or nothing. `⌘K` and `1–5` are not.
fn one_char(binding: &str) -> Option<char> {
    let mut chars = binding.chars();
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
}

/// A quoted value, in either of TOML's two quotes. `'⌘\'` is a literal string
/// because the contract spells the binding with one backslash and an escape
/// would put a second one in the comparison.
fn unquote(value: &str) -> String {
    let value = value.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|v| v.strip_suffix(quote))
        {
            return inner.to_string();
        }
    }
    value.to_string()
}

pub mod contract;

#[cfg(test)]
mod tests;
