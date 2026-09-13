//! A proposal as the file Write puts down, and what `config` makes of it.
//!
//! **Built by hand rather than serialised**, because nothing in this crate
//! writes YAML and a dependency for one small file is a supply-chain line for
//! nothing. The price is quoting, which [`scalar`] pays conservatively: a value
//! is left bare only where it cannot read as anything but itself, and `tests`
//! holds the awkward ones to the parser.
//!
//! **The parse is `config::Manifest::parse`, with no disk access**, so what a
//! proposal says it would refuse is what a load of the written file refuses.

use std::fmt::Write as _;
use std::path::Path;

use ipc::{ManifestFault, ManifestRefused, PolicyKey, Provenance};

use super::Draft;

pub(super) fn text(draft: &Draft) -> String {
    let mut out = String::from("version: 1\n");
    let _ = writeln!(out, "id: {}", scalar(&draft.id.value));
    if !draft.ports.is_empty() {
        out.push_str("ports:\n");
        for port in &draft.ports {
            match (port.container, &port.env) {
                (None, None) => {
                    let _ = writeln!(out, "  {}: {{}}", scalar(&port.name));
                }
                (container, env) => {
                    let _ = writeln!(out, "  {}:", scalar(&port.name));
                    if let Some(container) = container {
                        let _ = writeln!(out, "    container: {container}");
                    }
                    if let Some(env) = env {
                        let _ = writeln!(out, "    env: {}", scalar(env));
                    }
                }
            }
        }
    }
    if !draft.checks.is_empty() {
        out.push_str("checks:\n");
        for check in &draft.checks {
            let _ = writeln!(
                out,
                "  {}:\n    run: {}",
                scalar(&check.name),
                scalar(&check.run)
            );
            list(&mut out, "    requires", &check.requires);
        }
    }
    if !draft.commands.is_empty() {
        out.push_str("commands:\n");
        for command in &draft.commands {
            let _ = writeln!(
                out,
                "  {}:\n    run: {}",
                scalar(&command.name),
                scalar(&command.run)
            );
            if command.destructive {
                out.push_str("    destructive: true\n");
            }
        }
    }
    if let Some(setup) = &draft.setup {
        out.push_str("setup:\n");
        list(&mut out, "  requires", &setup.requires);
    }
    // An inherited value is an absent key, so only a pinned one is written.
    for row in draft
        .policy
        .iter()
        .filter(|row| row.provenance != Provenance::Default)
    {
        let key = match row.key {
            PolicyKey::AutoMerge => "auto_merge",
            PolicyKey::ReviewGate => "review_gate",
        };
        let _ = writeln!(out, "{key}: {}", scalar(&row.value));
    }
    out
}

fn list(out: &mut String, key: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "{key}:");
    let indent = " ".repeat(key.len() - key.trim_start().len() + 2);
    for item in items {
        let _ = writeln!(out, "{indent}- {}", scalar(item));
    }
}

/// What `config` refuses in `text`, key by key, or nothing where it loads.
/// `summary` is the parser's own sentence, as `ManifestReading` carries it.
pub(super) fn refused(path: &Path, text: &str) -> Option<ManifestRefused> {
    let why = config::Manifest::parse(path, text).err()?;
    Some(ManifestRefused {
        summary: why.to_string(),
        faults: why
            .refusals()
            .iter()
            .map(|refusal| ManifestFault {
                key: refusal.key.clone(),
                fault: refusal.fault.to_string(),
            })
            .collect(),
    })
}

/// `text` as a YAML scalar that reads back as exactly `text`.
pub(super) fn scalar(text: &str) -> String {
    match bare(text) {
        true => text.to_string(),
        false => quoted(text),
    }
}

/// Whether `text` can stand unquoted. **Deliberately narrow**: a bare value
/// that YAML reads as a number, a boolean, a null, a mapping or a comment
/// changes meaning silently, and a quoted one never does.
fn bare(text: &str) -> bool {
    let Some(first) = text.chars().next() else {
        return false;
    };
    let allowed = |c: char| c.is_ascii_alphanumeric() || " _./-=+:,".contains(c);
    let lower = text.to_ascii_lowercase();
    let numeric = text
        .chars()
        .all(|c| c.is_ascii_digit() || "._:-+eE".contains(c));
    (first.is_ascii_alphanumeric() || first == '_' || first == '/')
        && text.chars().all(allowed)
        && !text.ends_with([' ', ':'])
        && !text.contains(": ")
        && !numeric
        && !matches!(
            lower.as_str(),
            "y" | "n" | "yes" | "no" | "true" | "false" | "on" | "off" | "null"
        )
}

fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
