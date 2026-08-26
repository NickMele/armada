//! Where `unsafe` may be spoken, and which crate may relax the lint that
//! forbids it.
//!
//! # Why this rule exists, and what happened without it
//!
//! `unsafe_code = "forbid"` is a workspace lint, and `forbid` is absolute: it
//! cannot be lifted by an `allow` at a call site, which is exactly what makes
//! it worth having. But it also means the only way to write one legitimate
//! `unsafe` line is to stop inheriting the workspace lint for a whole crate —
//! and once a crate has done that, **nothing watches the second block, or the
//! twentieth.** The relaxation is one line in a `Cargo.toml` that no diff
//! reader looks at twice.
//!
//! `fleet` did exactly that, for a real reason: detaching a Drone into its own
//! session is `libc::setsid()` in `Command::pre_exec`, and `pre_exec` is unsafe
//! by construction. So the crate takes `deny` instead of `forbid`, and a single
//! site carries an `#[allow]`. This rule is what keeps that single.
//!
//! The shape is the manifest's: an explicit list, checked both ways. A file
//! that speaks `unsafe` and is not listed fails; a listed file that no longer
//! speaks it fails too, so the list cannot quietly outlive its reason.

use std::fs;
use std::path::Path;

use crate::{crate_dirs, files_with_ext, Report};

/// The crates permitted not to inherit the workspace lints, and why.
///
/// A crate here must still set `unsafe_code` to `deny` — never `allow`, and
/// never leave it out. `deny` keeps every site refused except the ones that say
/// out loud that they are exceptions.
const RELAXED: &[(&str, &str)] = &[(
    "fleet",
    "libc::setsid() in pre_exec — a Drone left in Fleet's process group dies at every restart",
)];

/// The files permitted to speak `unsafe`, and what they say it for.
const UNSAFE_SITES: &[(&str, &str)] = &[(
    "crates/fleet/src/detach.rs",
    "the fork-to-exec closure that puts a Drone in its own session",
)];

/// What counts as speaking it. `unsafe_code` in a comment or an attribute name
/// is not a block, so the tokens are the ones that actually open one.
const SPOKEN: &[&str] = &["unsafe {", "unsafe fn", "unsafe impl", "unsafe trait"];

pub fn unsafe_is_spoken_in_one_place(root: &Path) -> Report {
    let mut report = Report::new("unsafe is spoken only where the gate names, and only there");

    lints_are_inherited_or_named(root, &mut report);
    only_listed_files_speak_it(root, &mut report);
    every_listed_file_still_does(root, &mut report);

    report
}

/// Every crate either inherits the workspace `forbid` or is named here with a
/// `deny` of its own.
fn lints_are_inherited_or_named(root: &Path, report: &mut Report) {
    for dir in crate_dirs(root) {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let Ok(text) = fs::read_to_string(dir.join("Cargo.toml")) else {
            continue;
        };
        // Read the manifest, not the prose around it. The very comment
        // explaining a relaxation quotes the line it is not using, so a
        // substring search over the whole file reads it as the opposite of
        // what it says.
        let inherits = section(&text, "[lints]").contains(&"workspace = true".to_string());
        let denies = section(&text, "[lints.rust]").contains(&"unsafe_code = \"deny\"".to_string());
        let named = RELAXED.iter().find(|(crate_name, _)| *crate_name == name);

        match (inherits, named) {
            (true, None) => {}
            (true, Some(_)) => report.fail(format!(
                "crates/{name}/Cargo.toml inherits the workspace lints, so its \
                 entry in the gate's relaxed list is stale — remove one or the other"
            )),
            (false, None) => report.fail(format!(
                "crates/{name}/Cargo.toml does not inherit the workspace lints, \
                 and nothing says why — `unsafe_code = \"forbid\"` is off for the whole crate"
            )),
            (false, Some((_, why))) => {
                if !denies {
                    report.fail(format!(
                        "crates/{name}/Cargo.toml is relaxed for {why}, and must \
                         still set `unsafe_code = \"deny\"` — anything weaker \
                         leaves every other site in the crate unwatched"
                    ));
                }
            }
        }
    }
}

/// The declarations under one table header, comments dropped.
///
/// Read the manifest, not the prose around it: the comment explaining why a
/// crate does *not* inherit the workspace lints quotes the very line it is not
/// using, and a substring search over the whole file reads that as the opposite
/// of what it says.
fn section(text: &str, header: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .map(str::to_string)
        .collect()
}

fn only_listed_files_speak_it(root: &Path, report: &mut Report) {
    for source_root in ["crates", "apps", "packages"] {
        for path in files_with_ext(root, &root.join(source_root), &["rs"]) {
            if UNSAFE_SITES.iter().any(|(listed, _)| *listed == path) {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            for (n, line) in text.lines().enumerate() {
                if SPOKEN.iter().any(|token| line.contains(token))
                    || line.contains("#[allow(unsafe_code)]")
                {
                    report.fail(format!(
                        "{path}:{} speaks unsafe, and the gate does not list it",
                        n + 1
                    ));
                }
            }
        }
    }
}

/// A listed file that no longer speaks it means the exemption outlived its
/// reason, and the next person to need one finds a door already open.
fn every_listed_file_still_does(root: &Path, report: &mut Report) {
    for (path, why) in UNSAFE_SITES {
        let Ok(text) = fs::read_to_string(root.join(path)) else {
            report.fail(format!(
                "{path} is listed as the home of {why}, and is not on disk"
            ));
            continue;
        };
        if !SPOKEN.iter().any(|token| text.contains(token)) {
            report.fail(format!(
                "{path} no longer speaks unsafe — drop it from the gate's list \
                 rather than leaving the exemption standing"
            ));
        }
        if !text.contains("SAFETY:") {
            report.fail(format!(
                "{path} speaks unsafe with no `SAFETY:` note saying what makes it sound"
            ));
        }
    }
}
