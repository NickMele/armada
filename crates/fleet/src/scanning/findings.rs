//! What one workspace's files declare, each finding citing its file.
//!
//! Only files in the workspace's own directory are read, and the root is read
//! the same way as any other. A lockfile at the root of a monorepo is cited
//! there once and not copied into every member: it is evidence common to all
//! of them, and `docs/concepts/manifest.md` says so under *Workspace mapping*.

use std::collections::BTreeSet;

use ipc::{DeclaredPort, EvidenceStrength, Runnable, ScannedWorkspace, ToolFile, ToolSection};

use super::workspaces::{is_yaml, MANIFESTS, NOT_FOLLOWED};
use super::{at, compose, not_read, shown, Read, Reading, Tree};
use crate::drifting::declared;

/// Every lockfile this read knows, with the tool that writes it. The file's
/// own name is the evidence, which is why this is a table and not a guess.
const LOCKFILES: &[(&str, &str)] = &[
    ("Cargo.lock", "cargo"),
    ("bun.lock", "bun"),
    ("bun.lockb", "bun"),
    ("package-lock.json", "npm"),
    ("pnpm-lock.yaml", "pnpm"),
    ("poetry.lock", "poetry"),
    ("uv.lock", "uv"),
    ("yarn.lock", "yarn"),
];

/// Read one workspace. `names` are the files its directory holds.
pub(super) fn read(tree: &impl Tree, dir: &str, names: &BTreeSet<String>) -> Reading {
    let mut reading = Reading {
        scanned: ScannedWorkspace {
            dir: shown(dir),
            declared_by: Vec::new(),
            evidence: EvidenceStrength::NotFollowed,
            manifests: Vec::new(),
            lockfiles: Vec::new(),
            runnables: Vec::new(),
            tools: Vec::new(),
            services: Vec::new(),
            ports: Vec::new(),
            missing: Vec::new(),
            not_read: Vec::new(),
        },
        names_known: true,
    };
    let workspace = &mut reading.scanned;

    for (name, tool) in MANIFESTS {
        if names.contains(*name) {
            workspace.manifests.push(tool_file(at(dir, name), tool));
        }
    }
    for (name, tool) in LOCKFILES {
        if names.contains(*name) {
            workspace.lockfiles.push(tool_file(at(dir, name), tool));
        }
    }
    for name in NOT_FOLLOWED {
        if names.contains(*name) {
            let why = "not a tool this read follows";
            workspace.not_read.push(not_read(at(dir, name), why));
        }
    }

    if names.contains("package.json") {
        reading.names_known &= package_scripts(tree, dir, workspace);
    }
    reading.names_known &= cargo_aliases(tree, dir, workspace);
    if names.contains("pyproject.toml") {
        pyproject(tree, dir, workspace);
    }
    for name in names {
        if is_compose(name) {
            compose_file(tree, &at(dir, name), workspace);
        } else if name == ".env.example" {
            env_example(tree, &at(dir, name), workspace);
        } else if name == "armada.yml" {
            let why = "Armada's own Manifest. Scan reads what the repository declares, \
                       not what was written for Armada";
            workspace.not_read.push(not_read(at(dir, name), why));
        } else if dir.is_empty() && is_yaml(name) && !is_known_yaml(name) {
            let why = "a YAML file at the root this read does not follow";
            workspace.not_read.push(not_read(name.clone(), why));
        }
    }
    reading
}

fn tool_file(file: String, tool: &str) -> ToolFile {
    ToolFile {
        file,
        tool: tool.to_string(),
    }
}

/// A compose file, by the names the Compose specification reads — `compose`
/// alone, or with a prefix before a hyphen.
fn is_compose(name: &str) -> bool {
    ["compose.yaml", "compose.yml"]
        .iter()
        .any(|base| name == *base || name.ends_with(&format!("-{base}")))
}

fn is_known_yaml(name: &str) -> bool {
    matches!(
        name,
        "pnpm-workspace.yaml" | "pnpm-lock.yaml" | "armada.yml"
    ) || is_compose(name)
}

/// Bytes, or the reason there are none recorded against the file. `None`
/// where the file is absent, which records nothing.
fn bytes(tree: &impl Tree, file: &str, workspace: &mut ScannedWorkspace) -> Option<Vec<u8>> {
    match tree.read(file) {
        Read::Bytes(bytes) => Some(bytes),
        Read::Absent => None,
        Read::Unreadable(why) => {
            let why = format!("would not read: {why}");
            workspace.not_read.push(not_read(file.to_string(), why));
            None
        }
    }
}

/// Scripts, through drift's decode. `false` where the file was there and its
/// scripts could not be read.
fn package_scripts(tree: &impl Tree, dir: &str, workspace: &mut ScannedWorkspace) -> bool {
    let file = at(dir, "package.json");
    let Some(bytes) = bytes(tree, &file, workspace) else {
        return false;
    };
    let Some(scripts) = declared::script_lines(&bytes) else {
        let why = "does not read as a package.json whose scripts are all text";
        workspace.not_read.push(not_read(file, why));
        return false;
    };
    for (name, run) in scripts {
        let key = format!("scripts.{name}");
        if let Some(port) = port_flag(&run) {
            workspace.ports.push(DeclaredPort {
                file: file.clone(),
                key: key.clone(),
                name: name.clone(),
                container: port,
            });
        }
        workspace.runnables.push(Runnable {
            file: file.clone(),
            key,
            name,
            run,
        });
    }
    true
}

/// The value of a `--port` flag, where a script passes one as a number.
fn port_flag(run: &str) -> Option<u16> {
    let mut words = run.split_whitespace();
    while let Some(word) = words.next() {
        if word == "--port" {
            return words.next()?.parse().ok();
        }
        if let Some(value) = word.strip_prefix("--port=") {
            return value.parse().ok();
        }
    }
    None
}

/// Cargo aliases, through drift's reader. `false` where a config was there
/// and would not read.
fn cargo_aliases(tree: &impl Tree, dir: &str, workspace: &mut ScannedWorkspace) -> bool {
    for name in [".cargo/config.toml", ".cargo/config"] {
        let file = at(dir, name);
        let Some(bytes) = bytes(tree, &file, workspace) else {
            continue;
        };
        let Some(aliases) = declared::aliases(&String::from_utf8_lossy(&bytes)) else {
            let why = "an alias in a shape this read does not follow";
            workspace.not_read.push(not_read(file, why));
            return false;
        };
        for (name, run) in aliases {
            workspace.runnables.push(Runnable {
                file: file.clone(),
                key: format!("alias.{name}"),
                name,
                run,
            });
        }
        return true;
    }
    true
}

/// Each `[tool.*]` section, once.
fn pyproject(tree: &impl Tree, dir: &str, workspace: &mut ScannedWorkspace) {
    let file = at(dir, "pyproject.toml");
    let Some(bytes) = bytes(tree, &file, workspace) else {
        return;
    };
    let mut seen = BTreeSet::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        let Some(header) = line.trim().strip_prefix("[tool.") else {
            continue;
        };
        let header = header.trim_end_matches(']');
        if header.starts_with("uv.workspace") {
            let why = "a uv workspace's members, which this read does not expand";
            workspace.not_read.push(not_read(file.clone(), why));
        }
        let tool = header.split('.').next().unwrap_or(header).to_string();
        if seen.insert(tool.clone()) {
            workspace.tools.push(ToolSection {
                file: file.clone(),
                key: format!("tool.{tool}"),
                tool,
            });
        }
    }
}

fn compose_file(tree: &impl Tree, file: &str, workspace: &mut ScannedWorkspace) {
    let Some(bytes) = bytes(tree, file, workspace) else {
        return;
    };
    let Some(services) = compose::services(&String::from_utf8_lossy(&bytes)) else {
        let why = "a compose file in a shape this read does not follow";
        return workspace.not_read.push(not_read(file.to_string(), why));
    };
    for service in services {
        workspace.services.push(ipc::ComposeService {
            file: file.to_string(),
            key: format!("services.{}", service.name),
            name: service.name.clone(),
        });
        for port in service.ports {
            match port.container {
                Ok(container) => workspace.ports.push(DeclaredPort {
                    file: file.to_string(),
                    key: port.key,
                    name: service.name.clone(),
                    container,
                }),
                Err(why) => {
                    let cited = format!("{file}: {}", port.key);
                    workspace.not_read.push(not_read(cited, why));
                }
            }
        }
    }
}

/// `PORT=` and `*_PORT=` lines holding a number.
fn env_example(tree: &impl Tree, file: &str, workspace: &mut ScannedWorkspace) {
    let Some(bytes) = bytes(tree, file, workspace) else {
        return;
    };
    for line in String::from_utf8_lossy(&bytes).lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name != "PORT" && !name.ends_with("_PORT") {
            continue;
        }
        let value = value.trim().trim_matches(['"', '\'']);
        if let Ok(container) = value.parse() {
            workspace.ports.push(DeclaredPort {
                file: file.to_string(),
                key: name.to_string(),
                name: name.to_string(),
                container,
            });
        }
    }
}
