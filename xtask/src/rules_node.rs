//! `.nvmrc` names a Node the floor in `package.json` permits.
//!
//! Two files declare which Node this repository runs and neither reads the
//! other. `engineStrict` catches a **person** on the wrong Node, naming both
//! versions. Nothing caught the **files** disagreeing, and the last time they
//! did, `engines.node` said `>=22` while the pinned pnpm needed 22.13 — the
//! symptom read as a registry outage and cost a session to trace.
//!
//! A pin below the floor refuses everybody who follows it, which is loud. A pin
//! above it is the quiet one, and **it fails too**. It was a warning first, and
//! a warning is invisible among seventy others — while the state it describes
//! is that nobody runs the Node the floor claims to permit. Failing it makes
//! whoever bumps `.nvmrc` past the floor move the floor in the same commit,
//! which is the discipline the rule exists for.
//!
//! **The comparison is satisfaction, not equality**, because `.nvmrc` holds an
//! exact version and `engines.node` a range. `24.20.0` against `>=24` is the
//! correct state, and a rule matching the two strings would go red on a
//! repository that is right.
//!
//! **No JSON parser**, for the reason the rules beside it have none: the gate
//! keeps no dependencies, and `verify-foundations` refuses `serde_json::from_*`
//! outside `store` and `ipc`. Each file has one shape, and a line that does not
//! hold it is reported rather than skipped.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::Report;

const PIN: &str = ".nvmrc";
const MANIFEST: &str = "package.json";
const KEY: &str = "engines.node";
/// The one range form this rule reads, named in every refusal it writes.
const FORM: &str = ">=<major>";

const ENGINES: &str = "\"engines\"";
const NODE: &str = "\"node\"";

/// The exact version `.nvmrc` names. Three parts, because an exact pin is what
/// that file is for — a range or an alias there is a second floor, not a pin.
struct Pin {
    major: u32,
    minor: u32,
    patch: u32,
}

impl fmt::Display for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Rule: the pinned Node satisfies the floor the manifest declares.
///
/// **`>=<major>` is the only range read.** `engines.node` may hold anything npm
/// accepts and this repository has only ever written that form; general range
/// satisfaction in `xtask` is the version of this rule that does not land.
///
/// **It refuses; it does not rewrite.** A gate that quietly corrects `.nvmrc`
/// teaches nobody that two files have to move together, which is the defect.
pub fn the_pinned_node_satisfies_the_declared_floor(root: &Path) -> Report {
    let mut report = Report::new("the pinned Node satisfies the floor package.json declares");

    let pin = fs::read_to_string(root.join(PIN));
    let manifest = fs::read_to_string(root.join(MANIFEST));
    if pin.is_err() {
        report.fail(format!(
            "{PIN} — the exact Node a developer switches to. Without it every machine picks its \
             own and the floor is the only thing they share"
        ));
    }
    if manifest.is_err() {
        report.fail(format!(
            "{MANIFEST} — the manifest declaring `{KEY}`, the floor a wrong Node is measured \
             against"
        ));
    }
    if let (Ok(pin), Ok(manifest)) = (pin, manifest) {
        check(&pin, &manifest, &mut report);
    }
    report
}

/// Every check the rule makes, over the two files as text.
pub fn check(pin: &str, manifest: &str, report: &mut Report) {
    let (pin, floor) = (read_pin(pin, report), read_floor(manifest, report));
    let (Some(pin), Some(floor)) = (pin, floor) else {
        return;
    };
    if pin.major < floor {
        report.fail(format!(
            "{PIN} pins {pin} and {MANIFEST} `{KEY}` is `>={floor}`, so the pin names a Node the \
             floor refuses — `engineStrict` exits 1 for everybody who follows it. Move one of the \
             two: lower the floor to `>={major}`, or pin a {floor}.x",
            major = pin.major
        ));
    } else if pin.major > floor {
        report.fail(format!(
            "{PIN} pins {pin} and {MANIFEST} `{KEY}` is `>={floor}`, so nobody runs a Node {floor} \
             and the floor permits one that is never tested. Either raise the floor to \
             `>={major}` or pin a {floor}.x",
            major = pin.major
        ));
    }
}

/// The exact version `.nvmrc` names, or nothing if it is unreadable.
///
/// A line that is not a version is reported rather than passed over: `nvm`
/// reads the first line of this file whatever it holds, so a line this rule
/// skips is a version somebody is switching to unchecked.
fn read_pin(text: &str, report: &mut Report) -> Option<Pin> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(n, raw)| (n + 1, raw.trim()))
        .filter(|(_, line)| !line.is_empty());

    let Some((ln, line)) = lines.next() else {
        report.fail(format!(
            "{PIN} — names no version. An empty pin is read by `nvm` as no pin at all"
        ));
        return None;
    };

    for (extra, line) in lines {
        report.fail(format!(
            "{PIN}:{extra} — `{line}` is a second line, and the version is already set at line \
             {ln}. `nvm` reads the first and nothing reads the rest"
        ));
    }

    // A leading `v` is how `nvm` prints a version and names the same one, so
    // refusing it would make this a rule about spelling.
    let parts: Vec<&str> = line.trim_start_matches('v').split('.').collect();
    let numbers: Vec<u32> = parts.iter().filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 3 || numbers.len() != parts.len() {
        report.fail(format!(
            "{PIN}:{ln} — `{line}` is not an exact `<major>.<minor>.<patch>` version. This file \
             names the one Node a developer switches to, and an alias or a range here is a pin \
             nothing can compare to {MANIFEST} `{KEY}`"
        ));
        return None;
    }
    Some(Pin {
        major: numbers[0],
        minor: numbers[1],
        patch: numbers[2],
    })
}

/// The major `engines.node` sets as its floor, or nothing if it is unreadable.
///
/// The key is read inside the `engines` object rather than wherever a `node`
/// key appears, so a dependency spelled that way cannot be read as the floor.
fn read_floor(text: &str, report: &mut Report) -> Option<u32> {
    let mut inside = false;
    let mut engines = false;
    // Separate from `found`, so a floor that was written and could not be read
    // is reported once for what is wrong with it and not twice.
    let mut node = false;
    let mut found: Option<(usize, u32)> = None;

    for (n, raw) in text.lines().enumerate() {
        let (line, ln) = (raw.trim(), n + 1);
        // The value may be on the `"engines"` line itself or under it, and
        // which of the two is a formatter's decision rather than a fact.
        let rest = if let Some(rest) = line.strip_prefix(ENGINES) {
            (inside, engines) = (true, true);
            rest
        } else if inside {
            if line.starts_with('}') {
                inside = false;
                continue;
            }
            line
        } else {
            continue;
        };
        let Some(at) = rest.find(NODE) else {
            continue;
        };
        node = true;
        let Some(spec) = quoted(&rest[at + NODE.len()..]) else {
            report.fail(format!(
                "{MANIFEST}:{ln} — `{KEY}` is set to something other than a quoted range on \
                 `{line}`. The floor is a string, and `{FORM}` is the form this rule reads"
            ));
            continue;
        };
        if let Some((first, _)) = found {
            report.fail(format!(
                "{MANIFEST}:{ln} — `{KEY}` is already set at line {first}. Which of the two is \
                 the floor depends on which reader reads it"
            ));
            continue;
        }
        found = major_of(spec, ln, report).map(|major| (ln, major));
    }

    if !engines {
        report.fail(format!(
            "{MANIFEST} — declares no `engines` object, so `{PIN}` is measured against nothing"
        ));
    } else if !node {
        report.fail(format!(
            "{MANIFEST} — the `engines` object sets no `node` floor, so `{PIN}` is measured \
             against nothing"
        ));
    }
    found.map(|(_, major)| major)
}

/// The major a `>=<major>` range floors at.
///
/// Any other form is reported and read no further. That is the honest outcome
/// rather than the lazy one: an unread floor is exactly the state where the two
/// files can drift apart with the gate green, so it fails and says which form
/// it reads.
fn major_of(spec: &str, ln: usize, report: &mut Report) -> Option<u32> {
    let major = spec
        .strip_prefix(">=")
        .and_then(|rest| rest.trim().parse::<u32>().ok());
    if major.is_none() {
        report.fail(format!(
            "{MANIFEST}:{ln} — `{KEY}` is `{spec}`, which this rule cannot read. It reads \
             `{FORM}` and no other range, because that is the only form this repository has \
             written — an unread floor is one `{PIN}` can drift past with the gate green. Write \
             `{FORM}`, or teach this rule the form you need"
        ));
    }
    major
}

/// The quoted string a JSON key is set to, read off the text after the key.
fn quoted(after_key: &str) -> Option<&str> {
    after_key
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?
        .split('"')
        .next()
}

#[cfg(test)]
mod tests;
