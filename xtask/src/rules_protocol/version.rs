//! `protocol-version.toml` and the constant Bridge reads say the same thing.
//!
//! The Rust half cannot drift: `crates/ipc/build.rs` reads the file on every
//! build. The TypeScript half is generated once and committed, so it drifts as
//! far as the last person forgot — #270 moved the major to 6, the constant
//! stayed at 5.7, and a Fleet and a Bridge built from that commit refused each
//! other into the v0 lifeboat with every other check green.
//!
//! **The rule refuses; it does not rewrite.** A gate that runs the codegen
//! means nobody learns the step exists, and the next registry to grow a
//! generated half ships the same defect.
//!
//! **No TOML parser and no TypeScript parser**, for the reason the rules beside
//! it have none: the gate keeps no dependencies. Each file has one shape, and a
//! line that does not hold it is reported rather than skipped — a version half
//! read is the drift this rule exists to catch.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::Report;

const SOURCE: &str = "protocol-version.toml";
const MIRROR: &str = "apps/desktop/src/shared/generated/protocol-version.ts";
const CODEGEN: &str = "pnpm --filter @armada/desktop codegen";

/// The keys the source file defines. It carries these two under no table, and
/// a third would be a number one of the three readers does not read.
const KEYS: &[&str] = &["major", "minor"];

/// A protocol version, from either side. Compared as a pair, because which of
/// the two moved decides what a mismatch does and neither alone is the version.
#[derive(PartialEq, Eq)]
pub struct Version {
    major: u32,
    minor: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Rule: the protocol version and its generated constant agree.
pub fn the_version_and_its_generated_constant_agree(root: &Path) -> Report {
    let mut report = Report::new("the protocol version and its generated constant agree");

    let source = fs::read_to_string(root.join(SOURCE));
    let mirror = fs::read_to_string(root.join(MIRROR));
    if source.is_err() {
        report.fail(format!(
            "{SOURCE} — the source of truth for the protocol version"
        ));
    }
    if mirror.is_err() {
        report.fail(format!(
            "{MIRROR} — the constant Bridge reads. `{CODEGEN}` writes it"
        ));
    }
    if let (Ok(source), Ok(mirror)) = (source, mirror) {
        check(&source, &mirror, &mut report);
    }
    report
}

/// Every check the rule makes, over the two files as text.
pub fn check(source: &str, mirror: &str, report: &mut Report) {
    let (want, have) = (read_source(source, report), read_mirror(mirror, report));
    let (Some(want), Some(have)) = (want, have) else {
        return;
    };
    if want != have {
        report.fail(format!(
            "{SOURCE} is {want} and {MIRROR} is {have}. Bridge reads the generated constant, so \
             a Fleet and a Bridge built from this commit refuse each other and Bridge drops to \
             the v0 lifeboat. `{CODEGEN}` writes the file — run it and commit what it emits"
        ));
    }
}

/// The two numbers in the source file, or nothing if either is unreadable.
///
/// A line that is neither a comment nor `key = value` is reported rather than
/// ignored, because the other two readers of this file match a regular
/// expression against it and a line they skip is a number nobody set.
fn read_source(text: &str, report: &mut Report) -> Option<Version> {
    let mut found: BTreeMap<&str, (usize, u32)> = BTreeMap::new();

    for (n, raw) in text.lines().enumerate() {
        let (line, ln) = (raw.trim(), n + 1);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            report.fail(format!(
                "{SOURCE}:{ln} — `{line}` is neither a comment nor `key = value`. This file \
                 carries two keys and no tables"
            ));
            continue;
        };
        let key = key.trim();
        if !KEYS.contains(&key) {
            report.fail(format!(
                "{SOURCE}:{ln} — `{key}` is not a key this file defines: {}",
                KEYS.join(", ")
            ));
            continue;
        }
        let value = value.split('#').next().unwrap_or_default().trim();
        let Ok(number) = value.parse::<u32>() else {
            report.fail(format!(
                "{SOURCE}:{ln} — `{key} = {value}` is not a number. A version is two of them"
            ));
            continue;
        };
        if let Some((first, _)) = found.insert(key, (ln, number)) {
            report.fail(format!(
                "{SOURCE}:{ln} — `{key}` is already set at line {first}. Which of the two is the \
                 version depends on which reader reads it"
            ));
        }
    }

    for key in KEYS {
        if !found.contains_key(key) {
            report.fail(format!(
                "{SOURCE} — no `{key} = <n>` line. Half a version cannot be compared to anything"
            ));
        }
    }
    Some(Version {
        major: found.get("major")?.1,
        minor: found.get("minor")?.1,
    })
}

/// The two numbers in the generated constant, or nothing if either is
/// unreadable. Comment lines are passed over, so the header's prose about the
/// file cannot be read as the file's value.
fn read_mirror(text: &str, report: &mut Report) -> Option<Version> {
    if !text.contains("GENERATED") {
        report.fail(format!(
            "{MIRROR} — does not say it is generated. A file that does not is one somebody \
             hand-edits, which is the second source of truth this pair exists to prevent"
        ));
    }
    let Some(line) = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("export const PROTOCOL_VERSION"))
    else {
        report.fail(format!(
            "{MIRROR} — exports no `PROTOCOL_VERSION`. It is what Bridge imports, and \
             `{CODEGEN}` writes it"
        ));
        return None;
    };
    let (major, minor) = (field(line, "major", report), field(line, "minor", report));
    Some(Version {
        major: major?,
        minor: minor?,
    })
}

/// One field of the exported object, read off the line whatever the spacing.
/// The constant is an object rather than a number so that neither side can
/// spell the comparison `!==`, and a bare number here is that defect returning.
fn field(line: &str, key: &str, report: &mut Report) -> Option<u32> {
    let Some(rest) = line.split(key).nth(1) else {
        report.fail(format!(
            "{MIRROR} — `{line}` names no `{key}`. The constant is \
             `{{ major: <n>, minor: <n> }}`, and a bare number is a version half stated"
        ));
        return None;
    };
    let digits: String = rest
        .trim_start_matches([':', ' '])
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    match digits.parse::<u32>() {
        Ok(number) => Some(number),
        Err(_) => {
            report.fail(format!(
                "{MIRROR} — `{key}` is not a number in `{line}`. A version is two of them"
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests;
