//! The layers contract, checked mechanically.
//!
//! `ARCHITECTURE.md` §1.5 states it: **dependencies point inward.** `core` is
//! pure and imports nothing of charkit's; `adapters` depends on core traits
//! only; `cli` is the only crate that depends on both and wires them together.
//! §2.4 lists "crate boundaries" as one of the six things the merge gate runs,
//! and §1.5 says it should be there "from phase 1 — before there is anything to
//! untangle."
//!
//! **Why a check at all, when the crate graph already enforces it.** It
//! enforces the direction — a cycle will not compile — but nothing stops
//! someone adding `charkit-adapters` to `core`'s manifest, which compiles fine
//! and quietly inverts the design. The failure is a one-line diff in a file
//! nobody reads twice, and the consequence is that the pure core acquires I/O.
//!
//! **Why `cargo metadata` rather than reading the manifests.** The manifests
//! have three spellings for a dependency (`[dependencies]`, `[dependencies.x]`,
//! and the workspace-inherited form), and a regex that misses one reports a
//! clean repository. `cargo metadata` has already resolved all three, and it
//! reports dev- and build-dependencies too — which matter here: a *test* in
//! core that reaches for adapters is the same leak arriving through a door
//! marked `[dev-dependencies]`.

use crate::docs::Finding;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// Who may depend on whom. Anything not listed here may depend on nothing of
/// charkit's, and any charkit crate not listed at all is a finding: a new crate
/// has to state its place in the layering deliberately.
const ALLOWED: &[(&str, &[&str])] = &[
    ("charkit-core", &[]),
    ("charkit-adapters", &["charkit-core"]),
    ("charkit", &["charkit-core", "charkit-adapters"]),
    ("xtask", &[]),
];

/// The prefix that marks a package as one of ours.
const OURS: &str = "charkit";

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
}

pub fn check(root: &Path) -> Result<Vec<Finding>, String> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not run `cargo metadata`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("could not read `cargo metadata` output: {e}"))?;

    Ok(findings(&metadata))
}

fn findings(metadata: &Metadata) -> Vec<Finding> {
    let mut out = Vec::new();

    for package in &metadata.packages {
        if !metadata.workspace_members.contains(&package.id) {
            continue;
        }
        let Some((_, allowed)) = ALLOWED.iter().find(|(name, _)| *name == package.name) else {
            out.push(Finding {
                file: format!("{}/Cargo.toml", package.name),
                line: 0,
                message: format!(
                    "`{}` is a workspace member with no entry in the layers contract — \
                     add it to ARCHITECTURE.md §1.5 and to this check, deliberately",
                    package.name
                ),
            });
            continue;
        };

        for dependency in &package.dependencies {
            if !dependency.name.starts_with(OURS) || allowed.contains(&dependency.name.as_str()) {
                continue;
            }
            let kind = dependency.kind.as_deref().unwrap_or("normal");
            out.push(Finding {
                file: format!("{}/Cargo.toml", package.name),
                line: 0,
                message: format!(
                    "`{}` depends on `{}` ({kind}) — dependencies point inward \
                     (ARCHITECTURE.md §1.5); {} may depend on [{}]",
                    package.name,
                    dependency.name,
                    package.name,
                    if allowed.is_empty() {
                        "nothing of charkit's".to_string()
                    } else {
                        allowed.join(", ")
                    }
                ),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, deps: &[(&str, &str)]) -> Package {
        Package {
            id: format!("id {name}"),
            name: name.to_string(),
            dependencies: deps
                .iter()
                .map(|(dep, kind)| Dependency {
                    name: dep.to_string(),
                    kind: Some(kind.to_string()),
                })
                .collect(),
        }
    }

    fn metadata(packages: Vec<Package>) -> Metadata {
        let workspace_members = packages.iter().map(|p| p.id.clone()).collect();
        Metadata {
            packages,
            workspace_members,
        }
    }

    #[test]
    fn the_layering_this_repo_has_is_accepted() {
        let m = metadata(vec![
            package("charkit-core", &[("serde", "normal"), ("boon", "dev")]),
            package("charkit-adapters", &[("charkit-core", "normal")]),
            package(
                "charkit",
                &[("charkit-core", "normal"), ("charkit-adapters", "normal")],
            ),
            package("xtask", &[("regex", "normal")]),
        ]);
        assert!(findings(&m).is_empty());
    }

    #[test]
    fn the_core_may_not_reach_outward() {
        let m = metadata(vec![package(
            "charkit-core",
            &[("charkit-adapters", "normal")],
        )]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("dependencies point inward"));
    }

    /// The same leak, arriving through a door marked `[dev-dependencies]`.
    #[test]
    fn a_dev_dependency_is_the_same_leak() {
        let m = metadata(vec![package(
            "charkit-core",
            &[("charkit-adapters", "dev")],
        )]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("(dev)"), "{}", found[0].message);
    }

    #[test]
    fn adapters_may_not_depend_on_the_cli() {
        let m = metadata(vec![package("charkit-adapters", &[("charkit", "normal")])]);
        assert_eq!(findings(&m).len(), 1);
    }

    /// A new crate has to state its place in the layering rather than inherit
    /// one by being unlisted.
    #[test]
    fn a_workspace_member_nobody_placed_is_a_finding() {
        let m = metadata(vec![package("charkit-mcp", &[])]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("layers contract"));
    }
}
