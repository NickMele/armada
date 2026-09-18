//! Reading `studio-kinds.toml`, and nothing else.
//!
//! **No `toml` crate**, for the reason [`crate::rules_enums`] has none: the
//! gate carries no dependencies and must run on a checkout with nothing built.
//! This reads what that file is written in — table headers, `[[promotions]]`
//! rows, single-line strings, bools and arrays — and reports what it cannot
//! read rather than dropping it, because a field this skips is a field the
//! rule does not compare.
//!
//! **One reader for one file.** [`crate::rules_enums::reachability::reading`]
//! reads the Job registries; a second parser over the same rows is where two
//! readers would start disagreeing about what a file says. This one reads a
//! file that one does not.

use std::collections::BTreeMap;

use crate::Report;

/// One `[table.key]` or `[[table]]` row, and the fields under it.
pub struct Row {
    /// The table it is under — `nodes`, `states`, `promotions`.
    pub table: String,
    /// The key, or empty for a `[[table]]` row, which has none.
    pub key: String,
    pub line: usize,
    pub strings: BTreeMap<String, String>,
    pub bools: BTreeMap<String, bool>,
    pub arrays: BTreeMap<String, Vec<String>>,
}

impl Row {
    /// One string field, or a finding naming the row it is missing from.
    pub fn string(&self, field: &str, path: &str, report: &mut Report) -> Option<&str> {
        match self.strings.get(field) {
            Some(value) => Some(value.as_str()),
            None => {
                report.fail(format!(
                    "{path}:{} — `{}` declares no `{field}`. A field this rule cannot read \
                     is a claim it does not check",
                    self.line,
                    self.header()
                ));
                None
            }
        }
    }

    /// One bool field, or a finding. Same reasoning as [`Row::string`].
    pub fn bool(&self, field: &str, path: &str, report: &mut Report) -> Option<bool> {
        match self.bools.get(field) {
            Some(value) => Some(*value),
            None => {
                report.fail(format!(
                    "{path}:{} — `{}` declares no `{field}`. A field this rule cannot read \
                     is a claim it does not check",
                    self.line,
                    self.header()
                ));
                None
            }
        }
    }

    /// One array field, or a finding. An empty array is a declared answer and
    /// an absent one is not, which is why this does not fall back to empty.
    pub fn array(&self, field: &str, path: &str, report: &mut Report) -> Option<&[String]> {
        match self.arrays.get(field) {
            Some(value) => Some(value.as_slice()),
            None => {
                report.fail(format!(
                    "{path}:{} — `{}` declares no `{field}`. An absent array and an empty one \
                     are not the same answer, so neither is guessed",
                    self.line,
                    self.header()
                ));
                None
            }
        }
    }

    /// How the row is written, for a finding that can be searched for.
    pub fn header(&self) -> String {
        match self.key.is_empty() {
            true => format!("[[{}]]", self.table),
            false => format!("[{}.{}]", self.table, self.key),
        }
    }
}

/// Every row in the file, in the order written.
///
/// A `#` opens a comment only where it starts the line, so a field's prose may
/// carry one. A key declared twice is reported: the second table overwrites the
/// first, so one of the two rows is read by nobody.
pub fn rows(text: &str, path: &str, report: &mut Report) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let (line, n) = (line, n + 1);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(inner) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
            rows.push(row(inner.to_string(), String::new(), n));
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let Some((table, key)) = inner.split_once('.') else {
                report.fail(format!(
                    "{path}:{n} — `[{inner}]` is a table with no key. Every set here is keyed \
                     on its wire value, and a bare table has no value to compare"
                ));
                continue;
            };
            if let Some(first) = seen.insert(inner.to_string(), n) {
                report.fail(format!(
                    "{path}:{n} — `[{inner}]` already has a table at line {first}. The second \
                     overwrites the first, so one of the two rows is read by nobody"
                ));
            }
            rows.push(row(table.to_string(), key.to_string(), n));
            continue;
        }
        let Some((field, value)) = line.split_once('=') else {
            report.fail(format!(
                "{path}:{n} — `{line}` is neither a table header nor a `field = value`. A line \
                 this rule cannot read is a line it does not check"
            ));
            continue;
        };
        let (field, value) = (field.trim().to_string(), value.trim());
        let Some(row) = rows.last_mut() else {
            report.fail(format!(
                "{path}:{n} — `{field}` sits above every table header, so it belongs to no row"
            ));
            continue;
        };
        if let Some(open) = value.strip_prefix('[') {
            match open.strip_suffix(']') {
                Some(body) => {
                    row.arrays.insert(field, quoted(body));
                }
                None => report.fail(format!(
                    "{path}:{n} — `{field}` opens an array that does not close on its line. A \
                     field this rule cannot read is a field it does not compare"
                )),
            }
            continue;
        }
        match value {
            "true" => {
                row.bools.insert(field, true);
            }
            "false" => {
                row.bools.insert(field, false);
            }
            _ => match value.strip_prefix('"').and_then(|v| v.split('"').next()) {
                Some(text) => {
                    row.strings.insert(field, text.to_string());
                }
                None => report.fail(format!(
                    "{path}:{n} — `{field}` is neither a quoted string, a bool nor an array. \
                     This file is read without a TOML parser and carries only those three"
                )),
            },
        }
    }
    rows
}

fn row(table: String, key: String, line: usize) -> Row {
    Row {
        table,
        key,
        line,
        strings: BTreeMap::new(),
        bools: BTreeMap::new(),
        arrays: BTreeMap::new(),
    }
}

/// Every `"…"` in a line, in order.
pub fn quoted(body: &str) -> Vec<String> {
    body.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}
