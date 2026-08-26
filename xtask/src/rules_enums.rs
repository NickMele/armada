//! The domain registries and the enums hold the same set.
//!
//! `crates/core-model/domain/` is the authority on where a Job can be, where
//! one of its steps can be, and why it stopped and asked. `core-model` holds
//! the enums those files describe. **The registry key is the wire value** —
//! that is why the files key their tables on the spelling a stored row carries
//! rather than on a slug with a `name` beside it, and it is what lets this rule
//! compare the two with a set lookup and no mapping in between.
//!
//! So the comparison is on wire spellings, never on Rust identifiers.
//! `HatchUnbidden` and `hatch_unbidden` are the same variant to a reader and
//! nothing to a `from_wire`, and it is `from_wire` this protects: a key with no
//! variant is a stored row that reads back as `None`, and a variant with no key
//! is a value written to the column that the registry never sanctioned.
//!
//! # It runs both ways, and that is the difference from `rules_protocol`
//!
//! [`crate::rules_protocol`] runs one way on purpose: the operation inventory
//! names every operation the seam will ever carry and M1 serves a subset, so an
//! operation with no route is *not yet built* rather than wrong. Nothing here
//! is staged like that. `domain/README.md` says it outright — "a value added
//! here is a value some enum must gain, so add it in the change that adds the
//! variant, not before and not after" — and both halves of a mismatch are a
//! live defect rather than a milestone that has not arrived: one breaks
//! reading, the other breaks writing.
//!
//! **`in_code` does not soften either direction, and cannot.** Every row of all
//! three registries carries `in_code = "Not yet"` — thirty-one of thirty-one,
//! which the escalation file's own header states as a fact about the whole
//! table. Reading it as licence for a missing variant would licence every
//! absence at once and leave this rule checking nothing. It is provenance, as
//! the README says, not a staging flag.
//!
//! **No `toml` crate**, for the reason [`crate::rules_icons`] has none: the gate
//! keeps no dependencies. This reads table headers only, and skips `"""` blocks
//! so a bracket inside a `notes` string is not mistaken for one.
//!
//! # The edges are the same comparison one layer up
//!
//! [`edges`] holds the half that reads `domain/job-transitions.toml`, whose
//! rows are not keys but `from`/`to` pairs of them. It shares this module's
//! spellings and its parsing, and is split out only because both halves in one
//! file is more than one file's worth.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::Report;

const DOMAIN: &str = "crates/core-model/domain";

/// An enum, and the file its variants are spelled in.
struct EnumSource {
    name: &'static str,
    path: &'static str,
}

const ENUMS: &[EnumSource] = &[
    EnumSource { name: "JobStatus", path: "crates/core-model/src/job/status.rs" },
    EnumSource { name: "StepState", path: "crates/core-model/src/job/status.rs" },
    EnumSource { name: "EscalationTrigger", path: "crates/core-model/src/job/escalation.rs" },
];

/// A registry table, and the enum whose wire spellings its keys must be.
///
/// `enum-verbs.toml` appears twice because it nests one level: the outer key
/// names which enum an inner table belongs to, which is what lets
/// `evidence_suspect` be a variant of two enums at once.
struct Pairing {
    registry: &'static str,
    prefix: &'static str,
    enum_name: &'static str,
}

const PAIRINGS: &[Pairing] = &[
    Pairing { registry: "job-statuses.toml", prefix: "statuses.", enum_name: "JobStatus" },
    Pairing { registry: "step-states.toml", prefix: "states.", enum_name: "StepState" },
    Pairing {
        registry: "escalation-triggers.toml",
        prefix: "triggers.",
        enum_name: "EscalationTrigger",
    },
    Pairing { registry: "enum-verbs.toml", prefix: "verbs.job_status.", enum_name: "JobStatus" },
    Pairing {
        registry: "enum-verbs.toml",
        prefix: "verbs.escalation_reason.",
        enum_name: "EscalationTrigger",
    },
];

/// The seven vocabularies `enum-verbs.toml` declares in its own header.
///
/// Checked because a typo in an outer key — `job_statuses` for `job_status` —
/// does not fail the pairing above, it empties it. A comparison that silently
/// has nothing to compare is the failure this whole rule exists to end.
const VOCABULARIES: &[&str] = &[
    "job_status",
    "queued_reason",
    "escalation_reason",
    "step_verdict",
    "criterion_verdict_check",
    "criterion_verdict_judge",
    "criterion_verdict_attested",
];

/// Every enum's `variant -> wire` map, read from the source that declares it.
///
/// Shared with [`edges`], which compares the transition registry against
/// spellings rather than against identifiers for the reason this module's
/// header gives.
fn wire_spellings(
    root: &Path,
    report: &mut Report,
) -> BTreeMap<&'static str, BTreeMap<String, String>> {
    let mut spellings = BTreeMap::new();
    for source in ENUMS {
        let Ok(text) = fs::read_to_string(root.join(source.path)) else {
            report.fail(format!("{} — the source `{}` is declared in", source.path, source.name));
            continue;
        };
        spellings.insert(source.name, read_enum(&text, source, report));
    }
    spellings
}

/// Every registry key is a variant, and every variant is a registry key.
pub fn every_registry_key_is_a_variant(root: &Path) -> Report {
    let mut report = Report::new("the domain registries and the enums hold the same set");

    let spellings = wire_spellings(root, &mut report);

    for pairing in PAIRINGS {
        let path = format!("{DOMAIN}/{}", pairing.registry);
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            report.fail(format!("{path} — the registry `{}` is checked against", pairing.enum_name));
            continue;
        };
        let keys = keys_under(&text, pairing.prefix, &path, &mut report);
        let Some(variants) = spellings.get(pairing.enum_name) else { continue };
        if keys.is_empty() {
            report.fail(format!(
                "{path} has no `[{}<key>]` table at all — nothing to compare `{}` against",
                pairing.prefix, pairing.enum_name
            ));
            continue;
        }
        compare(&path, pairing, &keys, variants, &mut report);
    }

    let verbs = format!("{DOMAIN}/enum-verbs.toml");
    if let Ok(text) = fs::read_to_string(root.join(&verbs)) {
        for (table, line) in tables(&text) {
            let Some(vocabulary) = table.strip_prefix("verbs.").and_then(|r| r.split('.').next())
            else {
                continue;
            };
            if !VOCABULARIES.contains(&vocabulary) {
                report.fail(format!(
                    "{verbs}:{line} — `{vocabulary}` is not one of the seven vocabularies this \
                     file's header declares. Every variant under it is compared against nothing"
                ));
            }
        }
    }

    report
}

/// One pairing, both ways. Each finding names the key, the enum and the side
/// the spelling is missing from.
fn compare(
    path: &str,
    pairing: &Pairing,
    keys: &BTreeMap<String, usize>,
    variants: &BTreeMap<String, String>,
    report: &mut Report,
) {
    let (prefix, name) = (pairing.prefix, pairing.enum_name);
    let spelled: BTreeMap<&str, &str> =
        variants.iter().map(|(v, w)| (w.as_str(), v.as_str())).collect();

    for (key, line) in keys {
        if !spelled.contains_key(key.as_str()) {
            report.fail(format!(
                "{path}:{line} — `[{prefix}{key}]` is a key no `{name}` variant spells. \
                 A row storing `{key}` reads back as `None`"
            ));
        }
    }
    for (variant, wire) in variants {
        if !keys.contains_key(wire) {
            report.fail(format!(
                "`{name}::{variant}` spells `{wire}`, which {path} has no `[{prefix}{wire}]` \
                 table for. The registry is the authority on the set"
            ));
        }
    }
}

/// Every variant of one enum, as `variant -> wire spelling`.
///
/// The set comes from `ALL` and the spellings from the `as_wire` arms, because
/// those are the two things a comparison rule was promised. A variant in one
/// and not the other is reported here rather than passed on: it makes the
/// enum's own two statements about itself disagree, which no registry can
/// arbitrate.
fn read_enum(text: &str, source: &EnumSource, report: &mut Report) -> BTreeMap<String, String> {
    let name = source.name;
    let (path, mut found) = (source.path, BTreeMap::new());

    let all = all_variants(text, name);
    if all.is_empty() {
        report.fail(format!(
            "{path} — `{name}::ALL` is missing or empty. It is what a set-comparison rule reads"
        ));
        return found;
    }

    let arms = wire_arms(text, name);
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for (variant, wire) in &arms {
        if !all.contains(variant) {
            report.fail(format!(
                "{path} — `{name}::as_wire` spells `{variant}`, which is not in `{name}::ALL`. \
                 A rule scanning `ALL` cannot see it"
            ));
        }
        if let Some(first) = seen.insert(wire.as_str(), variant.as_str()) {
            report.fail(format!(
                "{path} — `{name}::{first}` and `{name}::{variant}` both spell `{wire}`. \
                 Two variants on one wire value cannot both be read back"
            ));
        }
    }
    for variant in &all {
        match arms.iter().find(|(v, _)| v == variant) {
            Some((_, wire)) => {
                found.insert(variant.clone(), wire.clone());
            }
            None => report.fail(format!(
                "{path} — `{name}::{variant}` is in `ALL` and has no `as_wire` arm. \
                 It has no wire spelling to compare"
            )),
        }
    }
    found
}

/// The variants listed in `pub const ALL: &'static [Enum] = &[ … ];`.
fn all_variants(text: &str, name: &str) -> Vec<String> {
    let header = format!("const ALL: &'static [{name}] =");
    let Some(start) = text.find(&header) else { return Vec::new() };
    let body = &text[start + header.len()..];
    // `rustfmt` puts the `&[` on this line or the next depending on width, so
    // the opening is stripped rather than assumed — leaving it attached hides
    // the first variant, which is the one an eyeball check is least likely to
    // miss and a rule most easily does.
    let body = body.split("];").next().unwrap_or_default();
    let body = body.trim_start().strip_prefix("&[").unwrap_or(body);
    let qualified = format!("{name}::");
    body.split(',')
        .filter_map(|part| Some(part.trim().strip_prefix(&qualified)?.trim().to_string()))
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .collect()
}

/// The `Enum::Variant => "wire",` arms, read only from inside an `as_wire`
/// body.
///
/// Scoping to that function is what keeps the rule from reading `level()` or
/// `seen_under()`, whose arms are qualified the same way and mean something
/// else entirely. `status.rs` holds two `as_wire` bodies, so every one is
/// scanned and the qualifier decides which enum an arm belongs to.
fn wire_arms(text: &str, name: &str) -> Vec<(String, String)> {
    let qualified = format!("{name}::");
    let mut found = Vec::new();
    for body in text.split("fn as_wire").skip(1) {
        let body = body.split("\n    }").next().unwrap_or_default();
        for line in body.lines().map(str::trim) {
            let Some(rest) = line.strip_prefix(&qualified) else { continue };
            let Some((variant, value)) = rest.split_once(" => ") else { continue };
            let Some(wire) = value.strip_prefix('"').and_then(|v| v.split('"').next()) else {
                continue;
            };
            found.push((variant.trim().to_string(), wire.to_string()));
        }
    }
    found
}

/// Every table key under `prefix`, with the line it is on. A key declared twice
/// is a failure: the second table overwrites the first, so one of the two rows
/// is read by nobody.
fn keys_under(
    text: &str,
    prefix: &str,
    path: &str,
    report: &mut Report,
) -> BTreeMap<String, usize> {
    let mut found: BTreeMap<String, usize> = BTreeMap::new();
    for (table, line) in tables(text) {
        let Some(key) = table.strip_prefix(prefix) else { continue };
        if key.is_empty() || key.contains('.') {
            continue;
        }
        if let Some(first) = found.insert(key.to_string(), line) {
            report.fail(format!(
                "{path}:{line} — `[{prefix}{key}]` already has a table at line {first}. \
                 The second overwrites the first"
            ));
        }
    }
    found
}

/// Every `[table.header]` in a file, as (path, line). Comments are skipped, and
/// so is everything inside a `"""` block — a `notes` field carrying a bracketed
/// line is prose, not a table.
fn tables(text: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut in_string = false;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let fences = line.matches("\"\"\"").count();
        if in_string {
            in_string = fences % 2 == 0;
            continue;
        }
        in_string = fences % 2 == 1;
        if line.starts_with('#') || !line.starts_with('[') {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            found.push((inner.to_string(), n + 1));
        }
    }
    found
}

pub mod edges;

#[cfg(test)]
mod tests;
