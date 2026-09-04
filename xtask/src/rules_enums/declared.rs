//! A status row names the edges the transition table carries, both ways.
//!
//! `job-statuses.toml`'s header says `transitions_in` and `transitions_out`
//! are "declared intent checked against what was actually wired. A rule reads
//! both and fails where they disagree." **No rule read either field until this
//! one**, and that sentence was all that held the two files together.
//!
//! It cost three rows in one night, each found by an agent reading next to it
//! and none of them looking: `escalated.transitions_out` missing an edge wired
//! since #50, `running.transitions_out` missing one #215 wired, and
//! `queued.transitions_in` missing both. Issue #394.
//!
//! **The comparison runs both ways**, which is what [`super::edges`] does and
//! [`crate::rules_protocol`]'s operation half deliberately does not. Neither
//! direction is "not yet built" here: an edge is a row in a registry rather
//! than an implementation, so a row naming an edge the table does not carry is
//! a promise nothing keeps, and an edge no row names is the three above.
//!
//! It compares against the registry rather than against `EDGES`, because
//! [`super::edges`] already holds those equal — checking one claim against two
//! sides of a settled equality reports one slip as two findings.
//!
//! Which side is stale it does not decide, for [`super::reachability`]'s
//! reason: a comparison does not know, and a rule that guessed would be
//! unusable in the case it was built for.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::Report;

use super::reachability::reading::{list, read, read_rows};

const STATUSES: &str = "crates/core-model/domain/job-statuses.toml";
const EDGES: &str = "crates/core-model/domain/job-transitions.toml";

/// The two fields, and which end of an edge each one claims.
const FIELDS: &[(&str, Direction)] = &[
    ("transitions_in", Direction::In),
    ("transitions_out", Direction::Out),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    In,
    Out,
}

impl Direction {
    /// The end of an edge the declaring row has to be.
    fn own_end<'a>(&self, from: &'a str, to: &'a str) -> &'a str {
        match self {
            Direction::In => to,
            Direction::Out => from,
        }
    }

    /// What a finding says about the end that did not match.
    fn wrong_end(&self) -> &'static str {
        match self {
            Direction::In => "arrives at",
            Direction::Out => "leaves",
        }
    }

    fn carried(&self) -> &'static str {
        match self {
            Direction::In => "arriving at",
            Direction::Out => "leaving",
        }
    }
}

/// Every declared edge is one the table carries, and every edge the table
/// carries is declared by the rows at both its ends.
pub fn every_status_row_names_the_edges_it_carries(root: &Path) -> Report {
    let mut report = Report::new("a status row names the edges the transition table carries");

    let what = "the rows whose `transitions_in` and `transitions_out` this rule reads";
    let Some(text) = read(root, STATUSES, what, &mut report) else {
        return report;
    };
    // Its own malformed rows are [`super::edges`]'s to report; an unreadable
    // file is not, and comes back here.
    let edges = super::edges::sanctioned_edges(root, &mut report);

    check(&text, &edges, &mut report);
    report
}

/// The whole comparison, over the status file as text and the edges as pairs.
fn check(text: &str, edges: &[(String, String, usize)], report: &mut Report) {
    let rows = read_rows(text, "statuses.", STATUSES, report);
    if rows.is_empty() {
        report.fail(format!(
            "{STATUSES} — no `[statuses.<key>]` row found at all. Either this file declares no \
             status, which has never been true here, or the parser no longer matches its shape \
             — either way this check compared nothing and reads as agreement"
        ));
        return;
    }
    if !rows.iter().any(|row| {
        FIELDS
            .iter()
            .any(|(field, _)| row.arrays.contains_key(*field))
    }) {
        report.fail(format!(
            "{STATUSES} — not one row declares `transitions_in` or `transitions_out`. Either \
             both fields are gone from the registry, or they were renamed past what this rule \
             reads — either way the two arrays #394 exists to check were compared against \
             nothing"
        ));
        return;
    }
    if edges.is_empty() {
        report.fail(format!(
            "{EDGES} — no `[[transitions]]` row this rule could read. Every status row was \
             compared against an empty table, which agrees with a row that names nothing and \
             catches none of what #394 found"
        ));
        return;
    }

    declared_against_the_table(&rows, edges, report);
    the_table_against_the_rows(&rows, edges, report);
}

/// Every entry a row declares, against the edges the table carries.
///
/// The entry is read as far as it goes: unparseable is reported rather than
/// skipped, a well-formed edge at the wrong end of the row is reported and not
/// then also reported as absent from the table, and only a `from -> to` that
/// survives both is looked up.
fn declared_against_the_table(
    rows: &[super::reachability::reading::Row],
    edges: &[(String, String, usize)],
    report: &mut Report,
) {
    let carried: BTreeSet<(&str, &str)> = edges
        .iter()
        .map(|(from, to, _)| (from.as_str(), to.as_str()))
        .collect();

    for row in rows {
        for (field, direction) in FIELDS {
            let Some(array) = row.arrays.get(*field) else {
                continue;
            };
            for entry in &array.values {
                let at = format!("{STATUSES}:{} — `{}.{field}`", array.line, row.key);
                let Some((from, to)) = split(entry) else {
                    report.fail(format!(
                        "{at} names `{entry}`, which is not a `from -> to` edge. An entry this \
                         rule cannot read is one it does not compare"
                    ));
                    continue;
                };
                if direction.own_end(from, to) != row.key {
                    report.fail(format!(
                        "{at} names `{entry}`, which {} `{}` and not `{}`. A row declares the \
                         edges at its own end, and one at another status's is read by nobody",
                        direction.wrong_end(),
                        direction.own_end(from, to),
                        row.key
                    ));
                    continue;
                }
                if !carried.contains(&(from, to)) {
                    report.fail(format!(
                        "{at} names `{entry}`, which {EDGES} has no `[[transitions]]` row for. \
                         The row promises a move nothing sanctions"
                    ));
                }
            }
        }
    }
}

/// Every edge the table carries, against the rows at its two ends.
///
/// This is the direction that went wrong three times, and an absent array is
/// the shape it went wrong in twice — so an absent array is one finding naming
/// the whole set it should have held, rather than one per edge pointing at a
/// line that says nothing about them.
fn the_table_against_the_rows(
    rows: &[super::reachability::reading::Row],
    edges: &[(String, String, usize)],
    report: &mut Report,
) {
    for row in rows {
        for (field, direction) in FIELDS {
            let owed = owed(edges, &row.key, *direction);
            match row.arrays.get(*field) {
                // A terminal status declares no `transitions_out` and the
                // table carries none out of it. Absent and empty say the same
                // thing, and the rows have always been written that way.
                None if owed.is_empty() => {}
                None => report.fail(format!(
                    "{STATUSES}:{} — `{}` declares no `{field}`, and {EDGES} carries {} {} it. \
                     A row with the field absent claims the empty set",
                    row.line,
                    row.key,
                    list(owed.keys()),
                    direction.carried()
                )),
                Some(array) => {
                    for (edge, line) in &owed {
                        if !array.values.contains(edge) {
                            report.fail(format!(
                                "{STATUSES}:{} — `{}.{field}` does not name `{edge}`, which \
                                 {EDGES}:{line} carries. The table wires a move the row does not \
                                 admit to",
                                array.line, row.key
                            ));
                        }
                    }
                }
            }
        }
    }

    // An edge whose end has no row at all is not caught by the walk above,
    // which is over rows: there is nothing to iterate. It is a fourth way the
    // two files disagree and the reverse rule cannot see it.
    let keys: BTreeSet<&str> = rows.iter().map(|row| row.key.as_str()).collect();
    for (from, to, line) in edges {
        for key in [from, to] {
            if !keys.contains(key.as_str()) {
                report.fail(format!(
                    "{EDGES}:{line} — `{from} -> {to}` names `{key}`, which {STATUSES} has no \
                     `[statuses.{key}]` row for. There is no row that could declare it"
                ));
            }
        }
    }
}

/// The edges at one end of a status, as `from -> to` against the line the
/// table declares each on.
fn owed(
    edges: &[(String, String, usize)],
    key: &str,
    direction: Direction,
) -> BTreeMap<String, usize> {
    edges
        .iter()
        .filter(|(from, to, _)| direction.own_end(from, to) == key)
        .map(|(from, to, line)| (format!("{from} -> {to}"), *line))
        .collect()
}

/// One `"from -> to"` entry, split on the arrow.
///
/// Spacing is not load-bearing and `rustfmt` never sees this file, but a
/// registry written by hand gets a double space in it eventually and that is
/// not a disagreement worth a finding.
fn split(entry: &str) -> Option<(&str, &str)> {
    let (from, to) = entry.split_once("->")?;
    let (from, to) = (from.trim(), to.trim());
    if from.is_empty() || to.is_empty() || to.contains("->") {
        return None;
    }
    Some((from, to))
}

#[cfg(test)]
mod tests;
