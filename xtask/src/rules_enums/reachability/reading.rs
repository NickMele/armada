//! Reading a registry, and saying a set back.
//!
//! Split from [`super`] for size, and shared with [`super::machine`]. Nothing
//! here compares anything — it is the parsing the gate has to do by hand,
//! because it carries no `toml` and no `syn` for the reason
//! [`crate::rules_enums`] gives.
//!
//! **What cannot be read is reported, never skipped.** A field this rule drops
//! is a claim it does not check, and a rule that quietly checks less than it
//! says is the failure the whole of `reachability` exists to end.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::Report;

/// One `[prefix<key>]` table, and the arrays it declares.
pub(super) struct Row {
    pub(super) key: String,
    pub(super) line: usize,
    pub(super) arrays: BTreeMap<String, Array>,
}

/// One `key = ["a", "b"]`, and the line it is on.
pub(super) struct Array {
    pub(super) values: BTreeSet<String>,
    pub(super) line: usize,
}

/// Every `[prefix<key>]` table in a registry, with the arrays it declares.
///
/// `"""` blocks are skipped for the reason `rules_enums`'s reader skips them: a
/// `notes` field quoting a bracketed line is prose, not a table. An array that
/// does not close on its line is reported rather than dropped.
pub(super) fn read_rows(text: &str, prefix: &str, path: &str, report: &mut Report) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
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
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if let Some(key) = inner.strip_prefix(prefix) {
                if !key.is_empty() && !key.contains('.') {
                    rows.push(Row {
                        key: key.to_string(),
                        line: n + 1,
                        arrays: BTreeMap::new(),
                    });
                }
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(open) = value.trim().strip_prefix('[') else {
            continue;
        };
        let Some(row) = rows.last_mut() else { continue };
        let key = key.trim().to_string();
        match open.strip_suffix(']') {
            Some(body) => {
                let values = body.split('"').skip(1).step_by(2).map(str::to_string);
                row.arrays.insert(
                    key,
                    Array {
                        values: values.collect(),
                        line: n + 1,
                    },
                );
            }
            None => report.fail(format!(
                "{path}:{} — `{key}` opens an array that does not close on its line. A field \
                 this rule cannot read is a field it does not compare",
                n + 1
            )),
        }
    }
    rows
}

/// A file, or a finding naming what could not be read.
pub(super) fn read(root: &Path, path: &str, what: &str, report: &mut Report) -> Option<String> {
    match fs::read_to_string(root.join(path)) {
        Ok(text) => Some(text),
        Err(_) => {
            report.fail(format!("{path} — {what}"));
            None
        }
    }
}

/// A variant identifier as the wire spelling it carries. The qualifier is
/// optional, for the reason `rules_enums::edges`'s reader makes it optional.
pub(super) fn spelled(arg: &str, variants: &BTreeMap<String, String>) -> Option<String> {
    variants
        .get(arg.rsplit("::").next().unwrap_or(arg).trim())
        .cloned()
}

/// Variant names as wire spellings, naming any that has none.
pub(super) fn resolve(
    variants: &[String],
    spellings: &BTreeMap<String, String>,
    enum_name: &str,
    path: &str,
    report: &mut Report,
) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for variant in variants {
        match spellings.get(variant) {
            Some(wire) => {
                found.insert(wire.clone());
            }
            None => report.fail(format!(
                "{path} — `{enum_name}::{variant}` has no wire spelling. The machine cannot be \
                 walked in the registry's own spellings"
            )),
        }
    }
    found
}

/// What is in the first and not the second.
pub(super) fn difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> BTreeSet<String> {
    left.difference(right).cloned().collect()
}

/// A set as a finding should say it. `nothing` rather than `[]`, because an
/// empty bracket in a sentence reads as a formatting slip.
pub(super) fn list<'a>(values: impl IntoIterator<Item = &'a String>) -> String {
    let values: Vec<&str> = values.into_iter().map(String::as_str).collect();
    strs(&values.into_iter().collect())
}

pub(super) fn strs(values: &BTreeSet<&str>) -> String {
    if values.is_empty() {
        return "nothing".to_string();
    }
    format!(
        "[{}]",
        values.iter().copied().collect::<Vec<_>>().join(", ")
    )
}
