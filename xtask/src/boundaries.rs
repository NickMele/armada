//! The layers contract, checked mechanically — **and nothing points upward**.
//!
//! Two rules, one check, because `ARCHITECTURE.md` says they are one rule at
//! two altitudes (§1.9).
//!
//! **§1.5 — dependencies point inward.** `core` is pure and imports nothing of
//! Armada's; a module's shell depends on core traits only; `helm` is the only
//! crate that depends on both and wires them together. §2.4 lists "crate
//! boundaries" as one of the six things the merge gate runs, and §1.5 says it
//! should be there "from phase 1 — before there is anything to untangle."
//!
//! **§1.9 — a module may only depend on modules below it.** Helm may name
//! Fleet, Guild and Manifest; Fleet may name Guild and Manifest; Guild and
//! Manifest are siblings and may name neither each other nor anything above.
//! The rule that matters is the negative one: **Manifest must keep knowing
//! nothing about agents**, which is what makes it usable by hand, by a script,
//! by CI, and by four parallel agents at once.
//!
//! Until M1 this check enforced §1.5 alone, because three of the four modules
//! had no crates and the module rule had nothing to bind to. §1.9's own note
//! said extending it "is part of the milestone that first creates a second
//! module crate, not a later cleanup — a rule that spends a milestone
//! unenforced is a rule that gets broken in that milestone." That milestone is
//! this one.
//!
//! **Why a check at all, when the crate graph already enforces it.** It
//! enforces the direction — a cycle will not compile — but nothing stops
//! someone adding `armada-manifest` to `core`'s manifest, which compiles fine
//! and quietly inverts the design. The failure is a one-line diff in a file
//! nobody reads twice, and the consequence is that the pure core acquires I/O.
//! The module half fails even more quietly: `armada-manifest` taking
//! `armada-fleet` for "just this one call" compiles, passes, and is the moment
//! Manifest stops being a tool about repositories.
//!
//! **Why `cargo metadata` rather than reading the manifests.** The manifests
//! have three spellings for a dependency (`[dependencies]`, `[dependencies.x]`,
//! and the workspace-inherited form), and a regex that misses one reports a
//! clean repository. `cargo metadata` has already resolved all three, and it
//! reports dev- and build-dependencies too — which matter here: a *test* in
//! core that reaches for `manifest` is the same leak arriving through a door
//! marked `[dev-dependencies]`.

use crate::docs::Finding;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// Where a crate sits in the four-module stack of `ARCHITECTURE.md` §1.9.
///
/// `Core` and `Tooling` are not modules and deliberately have no row in §1.9's
/// table: `Core` is the pure foundation every module is allowed to stand on,
/// and `Tooling` is `xtask`, which ships in nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Module {
    /// Pure, no I/O, below everything. §1.5.
    Core,
    /// What a workspace is and how to operate it.
    Manifest,
    /// Your voice, skills, hooks, subagents, workflows.
    Guild,
    /// Jobs, Drones, worktrees, classification, budgets, workflows.
    Fleet,
    /// The orchestrator, the Bridge, the MCP toolbelt, the inbox.
    Helm,
    /// Dev tooling. Never a dependency of a shipped crate.
    Tooling,
}

impl Module {
    /// The modules this one may name, from §1.9's table.
    ///
    /// `Core` is not listed anywhere: it is the pure foundation and is granted
    /// separately, so that adding it to a row cannot be mistaken for a module
    /// dependency.
    const fn may_depend_on(self) -> &'static [Module] {
        match self {
            Module::Helm => &[Module::Fleet, Module::Guild, Module::Manifest],
            Module::Fleet => &[Module::Guild, Module::Manifest],
            // Siblings. Manifest describes a repository, Guild describes a
            // person, and a dependency either way means one of those
            // descriptions has leaked into the other.
            Module::Guild | Module::Manifest => &[],
            // §1.5: the pure core imports nothing of Armada's, and `xtask` is
            // not allowed to become the place real dependencies are chosen.
            Module::Core | Module::Tooling => &[],
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Module::Core => "core",
            Module::Manifest => "Manifest",
            Module::Guild => "Guild",
            Module::Fleet => "Fleet",
            Module::Helm => "Helm",
            Module::Tooling => "tooling",
        }
    }
}

/// Every workspace crate and the module it belongs to.
///
/// **A crate not listed here is a finding**, so a new one states its place in
/// the stack deliberately rather than inheriting one by being unlisted. A
/// module may hold more than one crate — the pure/shell split of §1.5 applied
/// inside a module is the expected shape as the other three grow — and two
/// crates of the same module may name each other freely.
///
/// **`armada-guild` and `armada-fleet` are placed before they exist.** Only
/// workspace members are audited, so an absent crate costs nothing here — and
/// placing them now is what makes the rule apply to the commit that creates
/// them rather than to a cleanup afterwards.
const CRATES: &[(&str, Module)] = &[
    ("armada-core", Module::Core),
    ("armada-manifest", Module::Manifest),
    ("armada-guild", Module::Guild),
    ("armada-fleet", Module::Fleet),
    ("armada-helm", Module::Helm),
    ("xtask", Module::Tooling),
];

/// The prefix that marks a package as one of ours.
const OURS: &str = "armada";

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

fn module_of(crate_name: &str) -> Option<Module> {
    CRATES
        .iter()
        .find(|(name, _)| *name == crate_name)
        .map(|(_, module)| *module)
}

/// Whether `from` may name `to`, by §1.5 and §1.9 together.
fn permitted(from: Module, to: Module) -> bool {
    // Core is below everything, and every module may stand on it.
    to == Module::Core
        // Two crates of one module are one module's business.
        || to == from
        || from.may_depend_on().contains(&to)
}

fn findings(metadata: &Metadata) -> Vec<Finding> {
    let mut out = Vec::new();

    for package in &metadata.packages {
        if !metadata.workspace_members.contains(&package.id) {
            continue;
        }
        let Some(module) = module_of(&package.name) else {
            out.push(Finding {
                file: format!("{}/Cargo.toml", package.name),
                line: 0,
                message: format!(
                    "`{}` is a workspace member with no entry in the layers contract — \
                     add it to ARCHITECTURE.md §1.9 and to this check, deliberately",
                    package.name
                ),
            });
            continue;
        };

        for dependency in &package.dependencies {
            if !dependency.name.starts_with(OURS) {
                continue;
            }
            let kind = dependency.kind.as_deref().unwrap_or("normal");
            let Some(target) = module_of(&dependency.name) else {
                out.push(Finding {
                    file: format!("{}/Cargo.toml", package.name),
                    line: 0,
                    message: format!(
                        "`{}` depends on `{}` ({kind}), which has no entry in the layers \
                         contract — place it in ARCHITECTURE.md §1.9 and in this check",
                        package.name, dependency.name
                    ),
                });
                continue;
            };
            if permitted(module, target) {
                continue;
            }
            out.push(Finding {
                file: format!("{}/Cargo.toml", package.name),
                line: 0,
                message: format!(
                    "`{}` ({}) depends on `{}` ({}) ({kind}) — nothing points upward \
                     (ARCHITECTURE.md §1.9); {} may depend on [{}]",
                    package.name,
                    module.as_str(),
                    dependency.name,
                    target.as_str(),
                    module.as_str(),
                    permitted_list(module),
                ),
            });
        }
    }

    out
}

/// What a module may name, for the message. `core` is always in the list
/// because it is always allowed, and saying so is what stops the finding
/// reading as though the pure core were off limits too.
fn permitted_list(module: Module) -> String {
    let mut names = vec![Module::Core.as_str()];
    names.extend(module.may_depend_on().iter().map(|m| m.as_str()));
    names.join(", ")
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
            package("armada-core", &[("serde", "normal"), ("boon", "dev")]),
            package("armada-manifest", &[("armada-core", "normal")]),
            package(
                "armada-helm",
                &[("armada-core", "normal"), ("armada-manifest", "normal")],
            ),
            package("xtask", &[("regex", "normal")]),
        ]);
        assert!(findings(&m).is_empty());
    }

    #[test]
    fn the_core_may_not_reach_outward() {
        let m = metadata(vec![package(
            "armada-core",
            &[("armada-manifest", "normal")],
        )]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("nothing points upward"));
    }

    /// The same leak, arriving through a door marked `[dev-dependencies]`.
    #[test]
    fn a_dev_dependency_is_the_same_leak() {
        let m = metadata(vec![package("armada-core", &[("armada-manifest", "dev")])]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("(dev)"), "{}", found[0].message);
    }

    #[test]
    fn the_shell_may_not_depend_on_the_wiring() {
        let m = metadata(vec![package(
            "armada-manifest",
            &[("armada-helm", "normal")],
        )]);
        assert_eq!(findings(&m).len(), 1);
    }

    /// The negative rule of §1.9, and the one that matters: **Manifest must
    /// keep knowing nothing about agents.** The first time a Job id is threaded
    /// into Manifest "just for this one call", Manifest stops being a tool
    /// about repositories and every non-agent caller is carrying an agent
    /// framework it did not want.
    #[test]
    fn manifest_may_not_name_fleet() {
        let m = metadata(vec![
            package("armada-manifest", &[("armada-fleet", "normal")]),
            package("armada-fleet", &[]),
        ]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("Manifest"),
            "{}",
            found[0].message
        );
        assert!(found[0].message.contains("Fleet"), "{}", found[0].message);
    }

    /// Siblings, in both directions. Manifest describes a repository and Guild
    /// describes a person; a dependency either way means one of those
    /// descriptions has leaked into the other.
    #[test]
    fn manifest_and_guild_may_not_name_each_other() {
        for (from, to) in [
            ("armada-manifest", "armada-guild"),
            ("armada-guild", "armada-manifest"),
        ] {
            let m = metadata(vec![package(from, &[(to, "normal")]), package(to, &[])]);
            assert_eq!(findings(&m).len(), 1, "{from} -> {to}");
        }
    }

    /// Downward is the whole point: Helm and Fleet exist to drive the modules
    /// beneath them, and a check that forbade that would forbid the system.
    #[test]
    fn the_upper_modules_may_name_the_lower_ones() {
        let m = metadata(vec![
            package(
                "armada-helm",
                &[
                    ("armada-fleet", "normal"),
                    ("armada-guild", "normal"),
                    ("armada-manifest", "normal"),
                    ("armada-core", "normal"),
                ],
            ),
            package(
                "armada-fleet",
                &[("armada-guild", "normal"), ("armada-manifest", "normal")],
            ),
            package("armada-guild", &[("armada-core", "normal")]),
            package("armada-manifest", &[("armada-core", "normal")]),
            package("armada-core", &[]),
        ]);
        let found = findings(&m);
        assert!(
            found.is_empty(),
            "{}",
            found
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// A new crate has to state its place in the stack rather than inherit one
    /// by being unlisted — as the *depending* crate and as the depended-on one,
    /// because an unplaced target would otherwise be permitted by silence.
    #[test]
    fn a_workspace_member_nobody_placed_is_a_finding() {
        let m = metadata(vec![package("armada-mcp", &[])]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("layers contract"));

        let m = metadata(vec![package("armada-helm", &[("armada-mcp", "normal")])]);
        let found = findings(&m);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("layers contract"));
    }
}
