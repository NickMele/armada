//! No workspace package survives into a built bundle as a `require`.
//!
//! `pnpm dev` died on `Cannot find module …/packages/protocol/src/artifacts`.
//! `electron-vite` externalises a dependency by default, which is right for
//! something published as JavaScript and wrong for every `@armada/*`: they are
//! TypeScript source behind a workspace symlink, so Node reaches `src/index.ts`
//! and fails on the first extensionless import.
//!
//! **`bridge_build` was green the whole time, and could not have been
//! otherwise.** Building the main bundle never resolves what it externalised —
//! only running it does. The Check that compiles Bridge and the thing that
//! proves Bridge starts are not the same Check, and the gap between them is
//! exactly one `require` wide.
//!
//! So this reads the built output rather than the source: if a bundle names a
//! workspace package at runtime, that package was left for Node and Node cannot
//! find it. The config's own exclusion list is derived from `package.json`, so
//! the ordinary case needs nothing added here — this catches the day a new
//! entry point is added and nobody thinks about externalisation.
//!
//! It reports rather than fails where the bundles are absent: a tree nobody has
//! built yet is not a tree with a broken bundle, and the gate does not build.

use std::fs;
use std::path::Path;

use crate::Report;

/// The two Node bundles. The renderer is a browser bundle and externalises
/// nothing, so it is not in the list.
const BUNDLES: &[&str] = &[
    "apps/desktop/out/main/index.js",
    "apps/desktop/out/preload/index.js",
];

/// What a build must be run with for this rule to have anything to read.
const BUILD: &str = "pnpm -C apps/desktop build";

pub fn no_workspace_package_is_left_for_node(root: &Path) -> Report {
    let mut report = Report::new("no workspace package is left for node to resolve");

    let mut read = 0;
    for bundle in BUNDLES {
        let Ok(text) = fs::read_to_string(root.join(bundle)) else {
            continue;
        };
        read += 1;
        for name in required(&text) {
            report.fail(format!(
                "{bundle} asks Node for `{name}` at runtime. A workspace package is TypeScript \
                 source behind a symlink, so Node resolves `src/index.ts` and dies on its first \
                 extensionless import — bundle it instead, in `electron.vite.config.ts`. \
                 **This reads output, so it is only as current as the last build**: if the \
                 config already excludes it, run `{BUILD}` before believing this line"
            ));
        }
    }

    if read == 0 {
        report.warn(format!(
            "no built bundle to read. This rule checks output, so it says nothing until \
             `{BUILD}` has run — the Check that builds Bridge is where that happens"
        ));
    }

    report
}

/// Every `require("@armada/…")` a bundle carries.
///
/// The built output is minified to one shape — a `require` with a double-quoted
/// specifier — so this reads that shape rather than parsing JavaScript, which
/// is the same trade every other rule here makes.
fn required(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("require(\"@armada/") {
        let after = &rest[at + "require(\"".len()..];
        if let Some(close) = after.find('"') {
            let name = after[..close].to_string();
            if !found.contains(&name) {
                found.push(name);
            }
        }
        rest = &rest[at + 1..];
    }
    found
}
