//! The transition registry and the edge table name the same edges.
//!
//! `domain/job-transitions.toml` is the authority on where a Job may move next;
//! `transition.rs`'s `EDGES` is it transcribed by hand into a `no_std` crate
//! that cannot read TOML. A dropped edge is a legal move the machine refuses,
//! an invented one a move nothing sanctions, and neither is a compile error.
//!
//! The registry's header settles identity: "an edge has no name of its own that
//! a Rust identifier could carry — `from` and `to` are the whole identity". So
//! the match is on pairs, both ways, and the trigger is a value *inside* a
//! matched edge; as identity it would report a changed trigger as one edge
//! added and one removed, reading as a redesign rather than a one-word slip.
//!
//! `from` and `to` are keys in `job-statuses.toml`, `escalation_trigger` one
//! in `escalation-triggers.toml`; all are checked against the *enums'* wire
//! spellings, because [`super::every_registry_key_is_a_variant`] holds those
//! sets equal and a trigger the enum cannot spell is an edge that cannot be
//! transcribed at all.
//!
//! `EDGES.len() == 34` proved nothing — thirty-four rows with one `from` wrong
//! is still thirty-four. No `toml` and no `syn`, for the reason [`super`] has
//! neither; this refuses what it cannot read rather than skipping it, because a
//! comparison that silently drops what it does not understand is the failure it
//! replaces.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::Report;

const REGISTRY: &str = "crates/core-model/domain/job-transitions.toml";
const TABLE: &str = "crates/core-model/src/job/transition.rs";

/// One edge, in wire spellings, from whichever side declared it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct EdgeRow {
    from: String,
    to: String,
    trigger: Option<String>,
    line: usize,
}

impl EdgeRow {
    fn pair(&self) -> (String, String) {
        (self.from.clone(), self.to.clone())
    }

    /// The trigger as a finding should say it, including the absence — which is
    /// not nothing but the default edge, and worth naming as such.
    fn trigger_says(&self) -> String {
        match &self.trigger {
            Some(t) => format!("`{t}`"),
            None => "no trigger, the default edge".to_string(),
        }
    }
}

/// Every registry edge is in `EDGES`, every `EDGES` entry is in the registry,
/// and the values inside a matched edge agree.
pub fn the_registry_and_the_edge_table_hold_the_same_edges(root: &Path) -> Report {
    let mut report = Report::new("the transition registry and the edge table name the same edges");

    // The enums are read with a report of their own. Their internal failures
    // belong to the rule above and would be said twice here; what this rule
    // needs to know is only whether the spellings arrived.
    let mut reading = Report::new("reading the enums");
    let spellings = super::wire_spellings(root, &mut reading);
    let (Some(statuses), Some(triggers)) = (
        spellings.get("JobStatus"),
        spellings.get("EscalationTrigger"),
    ) else {
        report.fail(
            "`JobStatus` and `EscalationTrigger` did not read — see the rule above. \
             Nothing can be compared against spellings that were not found",
        );
        return report;
    };
    let Ok(registry_text) = fs::read_to_string(root.join(REGISTRY)) else {
        report.fail(format!("{REGISTRY} — the registry `EDGES` transcribes"));
        return report;
    };
    let Ok(table_text) = fs::read_to_string(root.join(TABLE)) else {
        report.fail(format!("{TABLE} — the source `EDGES` is declared in"));
        return report;
    };

    let registry = read_registry(&registry_text, &mut report);
    let table = read_table(&table_text, statuses, triggers, &mut report);
    if registry.is_empty() {
        report.fail(format!(
            "{REGISTRY} has no `[[transitions]]` row at all — nothing to compare `EDGES` against"
        ));
        return report;
    }
    if table.is_empty() {
        report.fail(format!(
            "{TABLE} has no `EDGES` entry at all — nothing to compare {REGISTRY} against"
        ));
        return report;
    }

    check_values(&registry, statuses, triggers, &mut report);
    check_shape(&registry, REGISTRY, &mut report);
    check_shape(&table, TABLE, &mut report);
    compare(&registry, &table, &mut report);

    report
}

/// The values inside a registry row, against the spellings an `Edge` can hold.
///
/// Only the registry side needs this: an `EDGES` entry names a variant, and a
/// variant with no wire spelling is caught when it is resolved into one.
fn check_values(
    rows: &[EdgeRow],
    statuses: &BTreeMap<String, String>,
    triggers: &BTreeMap<String, String>,
    report: &mut Report,
) {
    let spelled = |variants: &BTreeMap<String, String>, value: &str| {
        variants.values().any(|wire| wire == value)
    };
    for row in rows {
        for (field, value) in [("from", &row.from), ("to", &row.to)] {
            if !spelled(statuses, value) {
                report.fail(format!(
                    "{REGISTRY}:{} — `{field} = \"{value}\"` is a spelling no `JobStatus` \
                     variant carries. No `Edge` can hold it",
                    row.line
                ));
            }
        }
        if let Some(trigger) = &row.trigger {
            if !spelled(triggers, trigger) {
                report.fail(format!(
                    "{REGISTRY}:{} — `escalation_trigger = \"{trigger}\"` is a spelling no \
                     `EscalationTrigger` variant carries. The edge cannot be transcribed",
                    row.line
                ));
            }
        }
    }
}

/// What each side must be true of on its own: one row per pair, and no self-edge.
///
/// Both, because a set comparison sees neither. Two rows for one pair match the
/// other side's one row and the duplicate is simply never read; a self-edge on
/// both sides agrees with itself while `admits` returns `Ok` for a move that
/// changes nothing.
fn check_shape(rows: &[EdgeRow], path: &str, report: &mut Report) {
    let mut seen: BTreeMap<(String, String), usize> = BTreeMap::new();
    for row in rows {
        if let Some(first) = seen.insert(row.pair(), row.line) {
            report.fail(format!(
                "{path}:{} — `{} -> {}` is already declared at line {first}. \
                 The second is read by nobody",
                row.line, row.from, row.to
            ));
        }
        if row.from == row.to {
            report.fail(format!(
                "{path}:{} — `{} -> {}` is a self-edge, and the registry names none",
                row.line, row.from, row.to
            ));
        }
    }
}

/// The set both ways, and the trigger inside every pair present on both sides.
fn compare(registry: &[EdgeRow], table: &[EdgeRow], report: &mut Report) {
    let wired: BTreeMap<(String, String), &EdgeRow> = table.iter().map(|e| (e.pair(), e)).collect();
    let sanctioned: BTreeMap<(String, String), &EdgeRow> =
        registry.iter().map(|e| (e.pair(), e)).collect();

    for row in registry {
        match wired.get(&row.pair()) {
            None => report.fail(format!(
                "{REGISTRY}:{} — `{} -> {}` is an edge `EDGES` has no entry for. \
                 The machine refuses a move the registry sanctions",
                row.line, row.from, row.to
            )),
            Some(entry) if entry.trigger != row.trigger => report.fail(format!(
                "{REGISTRY}:{} — `{} -> {}` declares {}, and {TABLE}:{} gives it {}. \
                 An edge accepts the trigger it declares and no other",
                row.line,
                row.from,
                row.to,
                row.trigger_says(),
                entry.line,
                entry.trigger_says()
            )),
            Some(_) => {}
        }
    }
    for entry in table {
        if !sanctioned.contains_key(&entry.pair()) {
            report.fail(format!(
                "{TABLE}:{} — `EDGES` names `{} -> {}`, which {REGISTRY} has no row for. \
                 The machine admits a move nothing sanctions",
                entry.line, entry.from, entry.to
            ));
        }
    }
}

/// Every `[[transitions]]` row, as wire spellings.
///
/// Everything inside a `"""` block is skipped: the `notes` prose quotes edges,
/// and a quoted `from = "running"` inside one is a sentence, not a row.
fn read_registry(text: &str, report: &mut Report) -> Vec<EdgeRow> {
    let mut rows: Vec<EdgeRow> = Vec::new();
    let mut in_string = false;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let fences = line.matches("\"\"\"").count();
        if in_string {
            in_string = fences % 2 == 0;
            continue;
        }
        in_string = fences % 2 == 1;
        if line.starts_with('#') {
            continue;
        }
        if line == "[[transitions]]" {
            rows.push(EdgeRow {
                from: String::new(),
                to: String::new(),
                trigger: None,
                line: n + 1,
            });
            continue;
        }
        let Some((key, value)) = assignment(line) else {
            continue;
        };
        let Some(row) = rows.last_mut() else { continue };
        match key {
            "from" => row.from = value,
            "to" => row.to = value,
            "escalation_trigger" => row.trigger = Some(value),
            _ => {}
        }
    }
    rows.retain(|row| {
        let complete = !row.from.is_empty() && !row.to.is_empty();
        if !complete {
            report.fail(format!(
                "{REGISTRY}:{} — a `[[transitions]]` row is missing `from` or `to`. \
                 The pair is the whole identity of an edge",
                row.line
            ));
        }
        complete
    });
    rows
}

/// A `key = "value"` line, where the value is a bare string.
fn assignment(line: &str) -> Option<(&str, String)> {
    let (key, value) = line.split_once('=')?;
    let value = value.trim().strip_prefix('"')?.split('"').next()?;
    Some((key.trim(), value.to_string()))
}

/// Every `EDGES` entry, its identifiers resolved to wire spellings.
///
/// Entries are accumulated until their parentheses balance, so a line
/// `rustfmt` chose to wrap is one entry rather than two unreadable halves. The
/// line reported is the one an entry starts on.
fn read_table(
    text: &str,
    statuses: &BTreeMap<String, String>,
    triggers: &BTreeMap<String, String>,
    report: &mut Report,
) -> Vec<EdgeRow> {
    let header = "pub static EDGES: &[Edge] =";
    let Some(start) = text.find(header) else {
        report.fail(format!(
            "{TABLE} — `EDGES` is missing or is not declared as `{header}`. \
             It is what a comparison rule reads"
        ));
        return Vec::new();
    };
    let offset = text[..start].lines().count();
    let body = text[start + header.len()..]
        .split("];")
        .next()
        .unwrap_or_default();

    let mut rows = Vec::new();
    let (mut pending, mut began) = (String::new(), 0usize);
    for (n, raw) in body.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || line == "&[" {
            continue;
        }
        if pending.is_empty() {
            began = offset + n + 1;
        }
        pending.push_str(line);
        if pending.matches('(').count() != pending.matches(')').count() {
            continue;
        }
        if let Some(row) = entry(&pending, began, statuses, triggers, report) {
            rows.push(row);
        }
        pending.clear();
    }
    if !pending.is_empty() {
        report.fail(format!(
            "{TABLE}:{began} — `{pending}` does not close. `EDGES` was read no further"
        ));
    }
    rows
}

/// One `edge(From, To)` or `triggered(From, To, EscalationTrigger::Trigger)`.
fn entry(
    text: &str,
    line: usize,
    statuses: &BTreeMap<String, String>,
    triggers: &BTreeMap<String, String>,
    report: &mut Report,
) -> Option<EdgeRow> {
    let text = text.trim().trim_end_matches(',');
    let unreadable = |report: &mut Report| {
        report.fail(format!(
            "{TABLE}:{line} — `{text}` is not an `edge(…)` or `triggered(…)` entry. \
             An entry this rule cannot read is an edge it does not compare"
        ));
    };
    let (constructor, args) = match text.split_once('(') {
        Some((constructor, rest)) => (constructor.trim(), rest.trim_end_matches(')')),
        None => {
            unreadable(report);
            return None;
        }
    };
    // The trailing comma `rustfmt` leaves on a wrapped entry is punctuation, not
    // an argument. Dropping empties here is what keeps a reformatting from
    // reading as a malformed entry.
    let args: Vec<&str> = args
        .split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .collect();
    let expected = match constructor {
        "edge" => 2,
        "triggered" => 3,
        _ => {
            unreadable(report);
            return None;
        }
    };
    if args.len() != expected {
        unreadable(report);
        return None;
    }

    let from = spelled(args[0], "JobStatus", statuses, line, report)?;
    let to = spelled(args[1], "JobStatus", statuses, line, report)?;
    let trigger = match args.get(2) {
        Some(arg) => Some(spelled(arg, "EscalationTrigger", triggers, line, report)?),
        None => None,
    };
    Some(EdgeRow {
        from,
        to,
        trigger,
        line,
    })
}

/// A variant identifier as the wire spelling it carries.
///
/// The qualifier is optional because `EDGES` opens with `use JobStatus::*` and
/// spells the trigger in full. Neither form says anything the comparison uses:
/// what a variant is *called* is not what a stored row holds.
fn spelled(
    arg: &str,
    enum_name: &str,
    variants: &BTreeMap<String, String>,
    line: usize,
    report: &mut Report,
) -> Option<String> {
    let variant = arg.rsplit("::").next().unwrap_or(arg).trim();
    match variants.get(variant) {
        Some(wire) => Some(wire.clone()),
        None => {
            report.fail(format!(
                "{TABLE}:{line} — `{arg}` is not a `{enum_name}` variant with a wire spelling. \
                 The registry names its edges in spellings, so it cannot be compared"
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests;
