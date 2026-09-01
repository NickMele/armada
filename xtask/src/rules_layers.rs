//! The one rule the layer split rests on: a package imports downward, never up
//! and never sideways.
//!
//! Bridge is being taken apart into layers — the wire, the components, the
//! shell, the screens, and the app that composes them — so that a screen can be
//! rendered, storied and tested without an Electron process. That only holds
//! while the dependency runs one way. A screen that reaches into `apps/desktop`
//! for a type is a screen that cannot leave it, and the split is a naming
//! convention rather than a structure.
//!
//! **The rule arrives before the packages do.** A rule written after the move
//! is a rule written after the violations, and the violations are the thing it
//! exists to prevent. `shell` and `screens` do not exist yet; a layer with no
//! package on it is not a fault, and the day one lands it is already governed.
//!
//! Two ways to import, and both are checked. A package name says which layer it
//! is on. A relative path that climbs out of its own package says nothing, so
//! it is refused outright — there is no legitimate reason for a file in one
//! package to reach another through `../../`, and it is the one form that would
//! slip past a rule reading names.
//!
//! **No TS parser, and the gate keeps no dependencies.** This reads the two
//! shapes an import has: `from "…"` and `import("…")`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// The layers, ground up. A package may import a package strictly below it.
///
/// **Strictly below, not below-or-equal.** Two packages on one layer importing
/// each other is a cycle waiting for a third, and the ground layer is the one
/// that has to import nothing at all — `@armada/protocol` is what every other
/// package depends on, so anything it reaches for is reached by all of them.
const LAYERS: &[(&str, &str)] = &[
    ("@armada/tokens", "packages/tokens"),
    ("@armada/brand", "packages/brand"),
    ("@armada/icons", "packages/icons"),
    ("@armada/protocol", "packages/protocol"),
    ("@armada/components", "packages/components"),
    ("@armada/shell", "packages/shell"),
    ("@armada/screens", "packages/screens"),
    ("@armada/desktop", "apps/desktop"),
];

/// Which layer each package sits on. Everything in the first group is ground.
fn layer_of(name: &str) -> Option<usize> {
    match name {
        "@armada/tokens" | "@armada/brand" | "@armada/icons" | "@armada/protocol" => Some(0),
        "@armada/components" => Some(1),
        "@armada/shell" => Some(2),
        "@armada/screens" => Some(3),
        "@armada/desktop" => Some(4),
        _ => None,
    }
}

pub fn every_package_imports_downward(root: &Path) -> Report {
    let mut report = Report::new("every package imports downward, and never out of itself");

    let mut present: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, dir) in LAYERS {
        if root.join(dir).is_dir() {
            present.insert(name, dir);
        }
    }
    if present.is_empty() {
        report.fail("no package this rule governs is on disk. The layer list is wrong");
        return report;
    }

    for (name, dir) in &present {
        let Some(mine) = layer_of(name) else { continue };
        for path in files_with_ext(root, &root.join(dir), &["ts", "tsx", "mjs"]) {
            if path.contains("/node_modules/") || path.contains("/dist/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            for spec in specifiers(&text) {
                if let Some(theirs) = layer_of(package_of(&spec)) {
                    if theirs >= mine {
                        let (them, us) = (package_of(&spec), *name);
                        report.fail(format!(
                            "{path} imports `{them}`, which is on layer {theirs}, and {us} is on \
                             layer {mine}. A package imports one strictly below it — otherwise \
                             the layer it is in is a name rather than a boundary"
                        ));
                    }
                    continue;
                }
                if escapes(&path, &spec, dir) {
                    report.fail(format!(
                        "{path} reaches out of its own package with `{spec}`. A package is \
                         reached by its name or not at all: a relative path that climbs out \
                         states no layer and so can be checked against none"
                    ));
                }
            }
        }
    }

    let absent: Vec<&str> = LAYERS
        .iter()
        .filter(|(name, _)| !present.contains_key(name))
        .map(|(name, _)| *name)
        .collect();
    if !absent.is_empty() {
        report.warn(format!(
            "governed and not built yet: {}. A layer with no package is not a fault — the rule \
             is here first so the day one lands it is already bound",
            absent.join(", ")
        ));
    }

    report
}

/// The package a specifier names, which is the first two segments of a scope.
fn package_of(spec: &str) -> &str {
    if !spec.starts_with('@') {
        return spec.split('/').next().unwrap_or(spec);
    }
    let mut parts = spec.splitn(3, '/');
    match (parts.next(), parts.next()) {
        (Some(scope), Some(name)) => &spec[..scope.len() + 1 + name.len()],
        _ => spec,
    }
}

/// Whether a relative specifier resolves outside the package it was written in.
///
/// Counted rather than resolved: the gate has no filesystem view of what a
/// bundler would pick, and `..` past the package root is the whole of what is
/// being refused.
fn escapes(path: &str, spec: &str, dir: &str) -> bool {
    if !spec.starts_with('.') {
        return false;
    }
    let depth = path.strip_prefix(dir).unwrap_or(path).matches('/').count();
    let up = spec.split('/').filter(|seg| *seg == "..").count();
    up >= depth
}

/// Every module specifier in a file: `from "…"`, and a dynamic `import("…")`.
fn specifiers(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for marker in [" from ", "import("] {
        let mut rest = text;
        while let Some(at) = rest.find(marker) {
            rest = &rest[at + marker.len()..];
            let rest_trimmed = rest.trim_start();
            let Some(quote) = rest_trimmed
                .chars()
                .next()
                .filter(|c| *c == '"' || *c == '\'')
            else {
                continue;
            };
            let after = &rest_trimmed[1..];
            let Some(close) = after.find(quote) else {
                continue;
            };
            found.push(after[..close].to_string());
        }
    }
    found
}
