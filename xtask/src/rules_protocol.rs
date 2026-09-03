//! The route table and the operation inventory say the same thing.
//!
//! `crates/ipc/operations.toml` is the authority on which operations exist and
//! what each one is; `crates/api/src/routes.rs` is Fleet's hand-written half of
//! the seam. **The route table being hand-written is an accepted cost, and this
//! is the check it was accepted against** — a typo in a path is a runtime 404
//! and not a compile error, and only a rule reading both files catches an
//! operation that is served under a name the inventory does not have.
//!
//! It runs one way on purpose, against the inventory. The inventory names
//! every operation the seam will carry and M1 serves a subset, so one with no
//! route is *not yet built* rather than wrong. What is wrong is the reverse: a
//! route serving something the inventory never named, a `SERVED` row with no
//! route under it, or a command answered on `GET`.
//!
//! # The event half runs both ways
//!
//! An event kind is never "not yet built": it is a variant of
//! `crates/ipc/src/event.rs`'s `Event` enum, which means something already
//! constructs and publishes it, or it does not exist. So every variant needs a
//! `SERVED` row — **this is what #124 found**, `drone.spawned` and
//! `drone.exited` crossing the wire and invisible to every rule reading
//! `SERVED`, tolerated because nothing compared the two.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::Report;

const INVENTORY: &str = "crates/ipc/operations.toml";
const TABLE: &str = "crates/api/src/routes.rs";
const EVENT_ENUM: &str = "crates/ipc/src/event.rs";

pub fn the_router_serves_what_the_inventory_names(root: &Path) -> Report {
    let mut report = Report::new("every route served is an operation the inventory names");

    let Ok(inventory) = fs::read_to_string(root.join(INVENTORY)) else {
        report.fail(format!("{INVENTORY} — the operation inventory itself"));
        return report;
    };
    let Ok(table) = fs::read_to_string(root.join(TABLE)) else {
        report.fail(format!("{TABLE} — the route table itself"));
        return report;
    };
    let Ok(event_source) = fs::read_to_string(root.join(EVENT_ENUM)) else {
        report.fail(format!(
            "{EVENT_ENUM} — the closed set of published event kinds"
        ));
        return report;
    };

    check(&inventory, &table, &event_source, &mut report);
    report
}

/// Every check the rule makes, over the three files as text.
fn check(inventory: &str, table: &str, event_source: &str, report: &mut Report) {
    let kinds = operations(inventory);
    let served = rows(table);
    // Whitespace-blind, because `rustfmt` decides where a `.route(` call
    // breaks and the rule must not depend on that.
    let compact: String = table.chars().filter(|c| !c.is_whitespace()).collect();
    if served.is_empty() {
        report.fail(format!("{TABLE} — a SERVED table with no rows in it"));
        return;
    }

    for (operation, method, path) in &served {
        match kinds.get(operation) {
            None => report.fail(format!(
                "{TABLE} serves `{operation}`, which {INVENTORY} does not name"
            )),
            // Who initiates is the whole rule: a query and a command are alike
            // request-response over HTTP, and only an unsolicited push needs
            // the socket — which is reached by upgrading a GET.
            Some(kind) => {
                let expected = match kind.as_str() {
                    "command" => "POST",
                    _ => "GET",
                };
                if method != expected {
                    report.fail(format!(
                        "`{operation}` is a {kind} and is served on {method}, not {expected}"
                    ));
                }
            }
        }
        if !compact.contains(&format!(".route(\"{path}\"")) {
            report.fail(format!(
                "`{operation}` is in the SERVED table at {path} and in no route — a runtime 404"
            ));
        }
    }

    let paths: Vec<&str> = served.iter().map(|(_, _, path)| path.as_str()).collect();
    for routed in routes(&compact) {
        if !paths.contains(&routed.as_str()) {
            report.fail(format!(
                "{TABLE} routes {routed}, which no SERVED row names — nothing can compare it to \
                 the inventory"
            ));
        }
    }

    let published = published_event_kinds(event_source);
    if published.is_empty() {
        // A rule that finds nothing to compare is the failure this rule
        // exists to end, one level up: `pub enum Event {` moved, was
        // reformatted past what the parser matches, or lost every
        // `#[serde(rename = ...)]` line, and a silent zero-comparison run
        // reads as green. Refuse rather than pass on an empty set.
        report.fail(format!(
            "{EVENT_ENUM} — no `#[serde(rename = \"...\")]` variant found inside `pub enum \
             Event {{ ... }}`. Either this file holds no events, which has never been true here, \
             or the parser no longer matches its shape — either way this check compared nothing \
             and could not have caught #124 again"
        ));
        return;
    }

    let operations: Vec<&str> = served.iter().map(|(op, _, _)| op.as_str()).collect();
    for kind in published {
        if !operations.contains(&kind.as_str()) {
            report.fail(format!(
                "{EVENT_ENUM} declares `{kind}`, which {TABLE} does not list — a kind already \
                 published on /events that no rule reading SERVED can see"
            ));
        }
    }
}

/// Every `[operations.<name>]` key, with its `kind`. No TOML parser: the gate
/// has no dependencies, and the file's shape is one table per operation.
fn operations(inventory: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in inventory.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("[operations.") {
            let name = rest.trim_end_matches(']').trim_matches('"');
            current = Some(name.to_string());
        } else if let (Some(name), Some(kind)) = (current.as_ref(), line.strip_prefix("kind")) {
            let kind = kind.trim_start_matches(['=', ' ']).trim_matches('"');
            found.insert(name.clone(), kind.to_string());
            current = None;
        }
    }
    found
}

/// Every row of the `SERVED` table, as (operation, method, path).
fn rows(table: &str) -> Vec<(String, String, String)> {
    let field = |line: &str, key: &str| -> Option<String> {
        line.trim()
            .strip_prefix(key)?
            .trim_start_matches([':', ' '])
            .trim_end_matches(',')
            .trim_matches('"')
            .to_string()
            .into()
    };
    let mut rows = Vec::new();
    let (mut operation, mut method) = (None, None);
    for line in table.lines() {
        if let Some(value) = field(line, "operation:") {
            operation = Some(value);
        } else if let Some(value) = field(line, "method:") {
            method = Some(value);
        } else if let Some(path) = field(line, "path:") {
            if let (Some(operation), Some(method)) = (operation.take(), method.take()) {
                rows.push((operation, method, path));
            }
        }
    }
    rows
}

/// Every path the router actually registers, read from the whitespace-stripped
/// source so a line break inside the call cannot hide one.
fn routes(compact: &str) -> Vec<String> {
    compact
        .split(".route(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .collect()
}

/// Every `#[serde(rename = "...")]` kind inside `pub enum Event { ... }`, read
/// off the block between that line and its closing brace. This is the closed
/// set: the enum is what a value has to be shaped as before anything can
/// publish it, so a variant here is a kind that already crosses the wire.
///
/// No syn, for the reason the two parsers above have none — the gate keeps no
/// dependencies, and the enum has one shape.
fn published_event_kinds(source: &str) -> Vec<String> {
    let Some(start) = source.find("pub enum Event {") else {
        return Vec::new();
    };
    let body = &source[start..];
    let end = body.find("\n}").unwrap_or(body.len());
    body[..end]
        .lines()
        .filter_map(|line| line.trim().strip_prefix("#[serde(rename = \""))
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .collect()
}

pub mod version;

#[cfg(test)]
mod tests;
