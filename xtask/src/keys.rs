//! Diff the config keys that appear in examples against the keys described in
//! prose, both directions.
//!
//! Two failure shapes, and neither is visible by reading:
//!
//! - **prose only** — a key specified in detail and shown in no example. An
//!   implementer builds it from the description alone and nothing corroborates
//!   the shape. `stdio:` was found this way, governed by a whole section of
//!   §4.5 and absent from §4.5's own YAML block.
//! - **example only** — a key an example relies on that nothing defines. The
//!   example becomes the specification by accident.
//!
//! This is a report, not a gate: some asymmetry is legitimate (`version:`
//! needs no prose, a reserved key needs no example). It fails only on the
//! `deny` list, which names keys that must appear in both.

use crate::docs::{blocks, Doc, Finding};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

/// Keys that must appear in an example *and* in prose.
///
/// Add a key here once it is load-bearing enough that a wrong shape would cost
/// a phase. Everything else is reported and not enforced.
const MUST_BE_BOTH: &[&str] = &[
    "components",
    "checks",
    "run",
    "cmd",
    "driver",
    "setup",
    "owns",
    "commands",
    "version",
];

pub fn check(docs: &[Doc], verbose: bool) -> Vec<Finding> {
    // `key:` at the head of a line or inside a flow mapping.
    let in_example = Regex::new(r"([A-Za-z_][A-Za-z0-9_.\-]*)\s*:").unwrap();
    // `key = value` for the TOML blocks (~/.char/config.toml).
    let toml_key = Regex::new(r"^\s*([a-z_]+)\s*=").unwrap();
    // Prose names a key three ways: `key:` alone, `key: value` — much the most
    // common form, and missing it reported `driver:` and `version:` as
    // undescribed when both are described at length — or a bare snake_case
    // identifier in backticks, which is how the config.toml keys are written.
    let in_prose =
        Regex::new(r"`([A-Za-z_][A-Za-z0-9_.\[\]\-]*):(?:\s[^`]*)?`|`([a-z]+_[a-z_]+)`").unwrap();

    let mut example: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut prose: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for doc in docs {
        for b in blocks(&doc.text) {
            if !matches!(b.lang.as_str(), "yaml" | "yml" | "toml") {
                continue;
            }
            for (off, line) in b.body.lines().enumerate() {
                if line.trim_start().starts_with('#') {
                    continue;
                }
                let code = line.split('#').next().unwrap_or("");
                let cite = format!("{}:{}", doc.rel, b.start + off);
                for m in in_example.captures_iter(code) {
                    example
                        .entry(m[1].to_string())
                        .or_default()
                        .push(cite.clone());
                }
                if let Some(m) = toml_key.captures(code) {
                    example
                        .entry(m[1].to_string())
                        .or_default()
                        .push(cite.clone());
                }
            }
        }
        for (n, line) in doc.lines().enumerate() {
            for m in in_prose.captures_iter(line) {
                let key = m.get(1).or_else(|| m.get(2)).unwrap().as_str();
                prose
                    .entry(key.to_string())
                    .or_default()
                    .push(format!("{}:{}", doc.rel, n + 1));
            }
        }
    }

    let all: BTreeSet<&String> = example.keys().chain(prose.keys()).collect();
    let mut findings = Vec::new();

    for key in all {
        let e = example.get(key).map(Vec::len).unwrap_or(0);
        let p = prose.get(key).map(Vec::len).unwrap_or(0);
        let one_sided = e == 0 || p == 0;

        if one_sided && MUST_BE_BOTH.contains(&key.as_str()) {
            let (side, cites) = if e == 0 {
                ("never appears in an example", &prose[key])
            } else {
                ("never described in prose", &example[key])
            };
            let first = cites.first().cloned().unwrap_or_default();
            let (file, line) = split_cite(&first);
            findings.push(Finding {
                file,
                line,
                message: format!("`{key}:` {side}, and it is on the must-be-both list"),
            });
        } else if one_sided && verbose {
            let cites = if e == 0 { &prose[key] } else { &example[key] };
            let side = if e == 0 {
                "PROSE ONLY "
            } else {
                "EXAMPLE ONLY"
            };
            let shown: Vec<&str> = cites.iter().take(4).map(String::as_str).collect();
            println!("  {side}  {key:26}  {}", shown.join(", "));
        }
    }
    findings
}

fn split_cite(cite: &str) -> (String, usize) {
    match cite.rsplit_once(':') {
        Some((f, n)) => (f.to_string(), n.parse().unwrap_or(0)),
        None => (cite.to_string(), 0),
    }
}
