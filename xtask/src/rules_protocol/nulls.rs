//! An optional field on the wire is left out when empty, not sent as `null`.
//!
//! Bridge spells an `Option` as `field?: T` and compares against `undefined`,
//! which `null` passes. serde writes `None` as `null` unless the field says
//! `skip_serializing_if`, and neither compiler can see the mismatch: #827 was
//! the board's Clear crashing on `null.slice`, and the same scan found two more
//! fields drawing `file:null` and the wrong button.
//!
//! **`deserialize_with = "stated"` is the one way to mean `null`** — a key that
//! must be present and whose `null` is a value a person chose, with a
//! TypeScript type of `T | null` to match.
//!
//! A text reading rather than a parse, like every rule beside it: `rustfmt`
//! keeps a field and its attributes on lines of their own.

use std::fs;
use std::path::Path;

use crate::{walk, Report};

const IPC: &str = "crates/ipc/src";

pub fn no_optional_field_is_sent_as_null(root: &Path) -> Report {
    let mut report = Report::new("an optional field on the wire is left out, not sent as null");
    let mut sources = Vec::new();
    walk(&root.join(IPC), &mut |path| {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let is_test = relative.contains("/tests/") || relative.ends_with("tests.rs");
        if relative.ends_with(".rs") && !is_test {
            sources.push((relative, path.to_path_buf()));
        }
    });
    if sources.is_empty() {
        report.fail(format!("{IPC} — no Rust source found to read"));
        return report;
    }
    sources.sort();
    for (relative, path) in sources {
        match fs::read_to_string(&path) {
            Ok(text) => check(&relative, &text, &mut report),
            Err(_) => report.fail(format!("{relative} — could not be read")),
        }
    }
    report
}

/// Every bare `Option` field in a `Serialize` struct in one file.
pub fn check(path: &str, source: &str, report: &mut Report) {
    let mut serialize = false;
    // The struct being read, and how deep inside its braces this line is.
    let mut inside: Option<String> = None;
    let mut depth = 0usize;
    let mut attrs = String::new();
    // How many `[` an attribute spread over several lines still has open.
    let mut open = 0usize;

    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if open > 0 || line.starts_with("#[") {
            attrs.push_str(line);
            open = (open + line.matches('[').count()).saturating_sub(line.matches(']').count());
            if line.starts_with("#[derive(") && inside.is_none() {
                serialize = line.contains("Serialize");
            }
            continue;
        }
        if line.starts_with("//") || line.is_empty() {
            continue;
        }

        match &inside {
            None => {
                if let Some(name) = struct_name(line) {
                    if serialize && line.ends_with('{') {
                        inside = Some(name.to_string());
                        depth = 1;
                    }
                }
                if item_keyword(line) {
                    serialize = false;
                }
                attrs.clear();
            }
            Some(name) => {
                if depth == 1 {
                    if let Some(field) = optional_field(line) {
                        let said = attrs.contains("skip_serializing_if")
                            || attrs.contains("deserialize_with = \"stated\"");
                        if !said {
                            report.fail(format!(
                                "{path}:{} — `{name}::{field}` is sent as `null` when it is empty. \
                                 Add `#[serde(default, skip_serializing_if = \"Option::is_none\")]`, \
                                 or `deserialize_with = \"stated\"` where `null` is a value",
                                index + 1
                            ));
                        }
                    }
                }
                depth =
                    (depth + line.matches('{').count()).saturating_sub(line.matches('}').count());
                if depth == 0 {
                    inside = None;
                    serialize = false;
                }
                attrs.clear();
            }
        }
    }
}

/// `pub struct Name {` → `Name`.
fn struct_name(line: &str) -> Option<&str> {
    let rest = line.split_once("struct ")?.1;
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

/// Whether a line opens an item, so the derive above it is spent.
fn item_keyword(line: &str) -> bool {
    [
        "struct ", "enum ", "fn ", "impl ", "impl<", "type ", "const ", "static ", "mod ", "trait ",
    ]
    .iter()
    .any(|keyword| line.contains(keyword))
}

/// `pub line: Option<u32>,` → `line`.
fn optional_field(line: &str) -> Option<&str> {
    let (name, ty) = line.split_once(':')?;
    let name = name
        .trim()
        .rsplit(|c: char| c.is_whitespace() || c == ')')
        .next()?;
    let valid = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
    (valid && ty.trim_start().starts_with("Option<")).then_some(name)
}

#[cfg(test)]
mod tests;
