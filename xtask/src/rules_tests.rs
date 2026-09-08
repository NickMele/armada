//! Rule: every test file under a `tests/` submodule is declared — or the gate
//! never runs it. Two dead files were found in `crates/ipc` by accident (#496)
//! before anyone thought to check for a third.
//!
//! Only the `src/tests/mod.rs` submodule shape is checked: a directory
//! literally named `tests` that itself compiles as a module, and needs a `mod`
//! line naming a sibling before Cargo will run anything in it. An integration
//! `tests/` directory with no `mod.rs` is Cargo's own top-level-binary
//! discovery and needs no declaration — `crates/acceptance/tests/` and
//! `crates/config/tests/` are that shape, and having no `mod.rs` is what tells
//! the two apart.
//!
//! A nested directory under `tests/` (`fleet/src/tests/daemon/`, say) is not
//! read here: its files are declared by `daemon.rs`, a sibling this rule
//! already requires `tests/mod.rs` to name, and checking it is that file's own
//! mod declarations' job, not this rule's.

use std::fs;
use std::path::Path;

use crate::{walk, Report};

#[cfg(test)]
mod tests;

/// Every module name a `mod.rs` declares — `mod x;`, `pub mod x;`,
/// `pub(crate) mod x;` and the like, one per line, comments skipped. A
/// doc-commented mod line still counts: the doc sits on the lines above it,
/// and the declaration itself is unadorned.
fn declared(mod_text: &str) -> Vec<String> {
    const PREFIXES: &[&str] = &[
        "pub(crate) mod ",
        "pub(super) mod ",
        "pub(self) mod ",
        "pub mod ",
        "mod ",
    ];
    let mut found = Vec::new();
    for raw in mod_text.lines() {
        let line = raw.trim();
        if line.starts_with("//") {
            continue;
        }
        let Some(rest) = PREFIXES.iter().find_map(|p| line.strip_prefix(p)) else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            found.push(name);
        }
    }
    found
}

/// The sibling `.rs` file stems a `mod.rs` has to account for, minus the ones
/// it does.
fn undeclared(mod_text: &str, siblings: &[&str]) -> Vec<String> {
    let declared = declared(mod_text);
    siblings
        .iter()
        .filter(|stem| !declared.iter().any(|m| m == *stem))
        .map(|s| s.to_string())
        .collect()
}

/// Every test file in the workspace is declared by some `mod.rs`, so `cargo
/// nextest` actually runs it.
///
/// **Two dead files were found in one crate by accident** — `crates/ipc`'s
/// `capacity.rs`, and before it `proposals.rs`'s byte-identical duplicates
/// (closed in #495) — which is what makes this the durable half of #496: a
/// third one should fail here, not wait for somebody moving cases between
/// modules to notice it by hand.
pub fn every_test_file_is_declared(root: &Path) -> Report {
    let mut report = Report::new("every test file under tests/ is declared");
    walk(root, &mut |path| {
        if path.file_name().and_then(|n| n.to_str()) != Some("mod.rs") {
            return;
        }
        let Some(dir) = path.parent() else { return };
        if dir.file_name().and_then(|n| n.to_str()) != Some("tests") {
            return;
        }
        let Ok(mod_text) = fs::read_to_string(path) else {
            return;
        };
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut siblings: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("rs") {
                    return None;
                }
                let stem = p.file_stem()?.to_str()?.to_string();
                (stem != "mod").then_some(stem)
            })
            .collect();
        siblings.sort();
        let stems: Vec<&str> = siblings.iter().map(String::as_str).collect();
        let rel_dir = dir
            .strip_prefix(root)
            .unwrap_or(dir)
            .to_string_lossy()
            .replace('\\', "/");
        for file in undeclared(&mod_text, &stems) {
            report.fail(format!(
                "{rel_dir}/{file}.rs — declared by no module in {rel_dir}/mod.rs"
            ));
        }
    });
    report
}
