//! The three places a Studio's sets are spelled, read back as sets.
//!
//! The enums in `crates/core-model/src/studio.rs`, the `CHECK` constraints in
//! `crates/store/src/studio.rs`, and the mirror in
//! `packages/protocol/src/studio.ts`. Each reader answers a `BTreeSet` of wire
//! spellings, and [`super`] does every comparison — so a reader here never
//! decides whether something is wrong.
//!
//! **An empty answer is a finding, never agreement.** A reader that stops
//! matching the shape it reads returns nothing, and nothing compares clean
//! against nothing; [`super::same_set`] refuses an empty side for that reason.

use std::collections::{BTreeMap, BTreeSet};

use super::registry::quoted;

// ------------------------------------------------------------------- Rust

/// Every variant of one `spelled!` enum, as `wire -> variant`.
///
/// The macro writes `ALL`, `as_wire` and `from_wire` from one list, which is
/// why this reads the list rather than the three bodies
/// [`crate::rules_enums`]'s reader has to cross-check: there is only one
/// statement here to disagree with.
pub fn spelled(text: &str, name: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let header = format!("\n    {name} {{\n");
    let Some(start) = text.find(&header) else {
        return found;
    };
    let body = &text[start + header.len()..];
    let body = body.split("\n    }").next().unwrap_or_default();
    for line in body.lines().map(str::trim) {
        let Some((variant, value)) = line.split_once(" => ") else {
            continue;
        };
        if !variant.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let Some(wire) = value.strip_prefix('"').and_then(|v| v.split('"').next()) else {
            continue;
        };
        found.insert(wire.to_string(), variant.to_string());
    }
    found
}

/// The states each node kind declares in `StudioNodeKind::states`, as variant
/// names on both sides.
///
/// The arms group several kinds onto one `&[]`, and one spreads over four
/// lines, so the body is collapsed to a single line before it is split. Read
/// from that function alone: `admits`, `starts_proposed` and `added_by_hand`
/// name the same variants and mean something else.
pub fn states_by_kind(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut found = BTreeMap::new();
    let Some(body) = body_of(text, "fn states") else {
        return found;
    };
    let mut rest = collapsed(&body);
    while let Some(at) = rest.find("=> &[") {
        let (lhs, after) = rest.split_at(at);
        let after = &after["=> &[".len()..];
        let Some(end) = after.find(']') else { break };
        let states: Vec<String> = names(&after[..end], "S::");
        for kind in names(lhs, "StudioNodeKind::") {
            found.insert(kind, states.clone());
        }
        rest = after[end + 1..].to_string();
    }
    found
}

/// The kinds `StudioNodeKind::added_by_hand` answers for, as variant names.
pub fn added_by_hand(text: &str) -> BTreeSet<String> {
    let Some(body) = body_of(text, "fn added_by_hand") else {
        return BTreeSet::new();
    };
    names(&body, "StudioNodeKind::").into_iter().collect()
}

/// One function's body, from its `fn` line to the `}` that closes it at method
/// indentation.
fn body_of(text: &str, header: &str) -> Option<String> {
    let body = text.split(header).nth(1)?;
    Some(body.split("\n    }").next().unwrap_or_default().to_string())
}

/// Whitespace collapsed to single spaces, with `//` lines dropped — an arm
/// spread over four lines reads as one.
fn collapsed(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Every `Prefix::Name` in a chunk of Rust, in order.
fn names(text: &str, prefix: &str) -> Vec<String> {
    text.split(prefix)
        .skip(1)
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

// ------------------------------------------------------------------ store

/// One column's `CHECK (… IN (…))` set, off the **last** `CREATE TABLE` for
/// that table in the migrations file.
///
/// **The last one, because a `CHECK` is a migration.** SQLite cannot widen
/// one, so adding a kind rebuilds the table in a new version — `V79` did, for
/// the three forge kinds. Reading the last statement is what makes the gate
/// compare the set a fresh database actually gets, and what makes an edit to a
/// version that has already run invisible to it, which it should be.
pub fn check_set(text: &str, table: &str, column: &str) -> BTreeSet<String> {
    let header = format!("CREATE TABLE {table} (");
    let Some(start) = text.rfind(&header) else {
        return BTreeSet::new();
    };
    let body = &text[start..];
    let body = body.split(") STRICT").next().unwrap_or_default();
    let Some(at) = body.find(&format!("{column} IN (")) else {
        return BTreeSet::new();
    };
    let list = &body[at..];
    let Some(end) = list.find("))") else {
        return BTreeSet::new();
    };
    list[..end]
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The kinds the edges table's `CHECK (kind <> 'x' OR standing = 'accepted')`
/// names — the edges nobody may propose, which is the Studio's own.
pub fn accepted_on_sight(text: &str) -> BTreeSet<String> {
    let header = "CREATE TABLE studio_edges (";
    let Some(start) = text.rfind(header) else {
        return BTreeSet::new();
    };
    let body = &text[start..];
    let body = body.split(") STRICT").next().unwrap_or_default();
    body.split("CHECK (kind <> ")
        .skip(1)
        .filter(|arm| arm.starts_with('\''))
        .filter_map(|arm| arm.split('\'').nth(1).map(str::to_string))
        .collect()
}

// ------------------------------------------------------------------- wire

/// One `export type Name = …` declaration's body, up to the `;` that ends it.
///
/// **Braces are counted and comment lines do not end it**, because a field ends
/// in `;` too and so does a sentence of prose. Stopping at either read
/// `ProposeStudioEdge` as its `from` field, and `StudioNodeContent` as its first
/// seven kinds — both compared against a set that was short by everything after
/// the next doc comment.
fn declaration(text: &str, name: &str) -> Option<String> {
    let header = format!("export type {name} =");
    let start = text.find(&header)? + header.len();
    let mut body = String::new();
    for line in text[start..].lines() {
        body.push_str(line);
        body.push('\n');
        let head = line.trim_start();
        if head.starts_with('*') || head.starts_with("/*") || head.starts_with("//") {
            continue;
        }
        let end = line.trim_end();
        if end.ends_with("};") {
            return Some(body);
        }
        let closed = body.matches('{').count() == body.matches('}').count();
        if closed && end.ends_with(';') {
            return Some(body);
        }
    }
    None
}

/// The `kind: "…"` tags of a discriminated union.
pub fn tags(text: &str, name: &str) -> BTreeSet<String> {
    let Some(body) = declaration(text, name) else {
        return BTreeSet::new();
    };
    body.split("kind: \"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next().map(str::to_string))
        .collect()
}

/// Every string literal in a union of them — `"open" | "closed" | "merged"`.
pub fn literals(text: &str, name: &str) -> BTreeSet<String> {
    let Some(body) = declaration(text, name) else {
        return BTreeSet::new();
    };
    quoted(&body).into_iter().collect()
}

// ---------------------------------------------------------------- lexicon

/// Every word the Design System's lexicon says never to use, lowercased.
///
/// One entry per `- **Term**`, and the ban is the clause after `Never` up to
/// the first sentence end or em dash. A leading `the` is stripped, so `Never
/// the dashboard, the UI` bans `dashboard` and `ui` — which is what a node
/// name would collide with.
pub fn never_words(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Some(section) = text.split("### Lexicon").nth(1) else {
        return found;
    };
    let section = section.split("\n### ").next().unwrap_or_default();
    let joined = section
        .lines()
        .map(str::trim)
        .collect::<Vec<&str>>()
        .join(" ");
    for entry in joined.split("- **").skip(1) {
        let Some(after) = entry.split(" Never ").nth(1) else {
            continue;
        };
        let clause = after
            .split(" — ")
            .next()
            .unwrap_or(after)
            .split(". ")
            .next()
            .unwrap_or(after);
        let clause = clause.trim_end_matches('.');
        for word in clause.split(',') {
            let word = word.trim().trim_start_matches("the ").trim().to_lowercase();
            if !word.is_empty() && word.len() < 40 {
                found.insert(word);
            }
        }
    }
    found
}
