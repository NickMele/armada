//! Scan's findings to a first proposal. A line one of these tables produced reads
//! `convention`; tables match file names, never a finding's rendered `tool`.

use std::collections::BTreeSet;
use std::path::Path;

use ipc::{
    DeclaredPort, ProposedCheck, ProposedCommand, ProposedId, ProposedPort, ProposedSetup,
    Provenance, RepositoryScan, ScannedWorkspace,
};

use super::{default_policy, Draft};

/// Script names that conventionally gate code. Every other script is proposed as a Command.
const CHECK_NAMES: &[&str] = &["test", "lint", "typecheck", "type-check", "check", "e2e"];

/// The tool that runs a `package.json` script, by its lockfile.
const SCRIPT_RUNNERS: &[(&str, &str)] = &[
    ("pnpm-lock.yaml", "pnpm"),
    ("yarn.lock", "yarn"),
    ("bun.lock", "bun"),
    ("bun.lockb", "bun"),
    ("package-lock.json", "npm"),
];

/// Setup's install, by lockfile. Frozen, so preparation writes nothing into a Drone's diff.
const INSTALLS: &[(&str, &str)] = &[
    ("pnpm-lock.yaml", "pnpm install --frozen-lockfile"),
    ("yarn.lock", "yarn install --frozen-lockfile"),
    ("bun.lock", "bun install --frozen-lockfile"),
    ("bun.lockb", "bun install --frozen-lockfile"),
    ("package-lock.json", "npm ci"),
    ("uv.lock", "uv sync --frozen"),
    ("poetry.lock", "poetry install"),
];

/// Root lockfiles a workspace pattern's file shares with its members. A member found by
/// its own manifest shares none: a Go service in a `pnpm` repository is not a `pnpm` package.
const SHARED_BY: &[(&str, &[&str])] = &[
    ("pnpm-workspace.yaml", &["pnpm-lock.yaml"]),
    (
        "package.json",
        &["yarn.lock", "bun.lock", "bun.lockb", "package-lock.json"],
    ),
];

/// A `pyproject.toml` section, the Check it conventionally means, and its run.
const PYTHON_CHECKS: &[(&str, &str, &str)] = &[
    ("tool.pytest", "test", "pytest"),
    ("tool.ruff", "lint", "ruff check ."),
    ("tool.mypy", "typecheck", "mypy ."),
];

const PYTHON_RUNNERS: &[(&str, &str)] = &[("uv.lock", "uv run "), ("poetry.lock", "poetry run ")];

pub(super) fn drafts(scan: &RepositoryScan) -> Vec<Draft> {
    let root = scan.workspaces.iter().find(|one| one.dir == ".");
    let ids = ids(scan);
    scan.workspaces
        .iter()
        .zip(ids)
        .map(|(workspace, id)| draft(&scan.checkout, root, workspace, id))
        .collect()
}

fn draft(
    checkout: &str,
    root: Option<&ScannedWorkspace>,
    workspace: &ScannedWorkspace,
    id: String,
) -> Draft {
    let lockfiles = lockfiles(root, workspace);
    let mut checks: Vec<ProposedCheck> = Vec::new();
    let mut commands: Vec<ProposedCommand> = Vec::new();
    let runner = found(&lockfiles, SCRIPT_RUNNERS).map_or("npm", |(_, run)| run);

    for runnable in &workspace.runnables {
        let run = match runnable.key.starts_with("alias.") {
            true => format!("cargo {}", runnable.name),
            false => format!("{runner} run {}", runnable.name),
        };
        let provenance = convention(&runnable.file, Some(&runnable.key));
        match is_check(&runnable.name) {
            true => checks.push(check(&runnable.name, run, provenance)),
            false => commands.push(command(&runnable.name, run, provenance)),
        }
    }
    let taken = |checks: &[ProposedCheck], commands: &[ProposedCommand], name: &str| {
        checks.iter().any(|one| one.name == name) || commands.iter().any(|one| one.name == name)
    };
    // Scan left `cargo test` on a bare `Cargo.toml` for Proposal to say, as convention.
    if let Some(cargo) = workspace
        .manifests
        .iter()
        .find(|one| named(&one.file, "Cargo.toml"))
    {
        if !taken(&checks, &commands, "test") {
            let run = "cargo test".to_string();
            checks.push(check("test", run, convention(&cargo.file, None)));
        }
    }
    let python = found(&lockfiles, PYTHON_RUNNERS).map_or("", |(_, run)| run);
    for section in &workspace.tools {
        let Some((_, name, run)) = PYTHON_CHECKS.iter().find(|(key, ..)| *key == section.key)
        else {
            continue;
        };
        if !taken(&checks, &commands, name) {
            let provenance = convention(&section.file, Some(&section.key));
            checks.push(check(name, format!("{python}{run}"), provenance));
        }
    }
    let mut setup = None;
    if let Some((lockfile, install)) = found(&lockfiles, INSTALLS) {
        if !taken(&checks, &commands, "install") {
            let provenance = convention(lockfile, None);
            commands.push(command("install", install.to_string(), provenance.clone()));
            setup = Some(ProposedSetup {
                requires: vec!["install".to_string()],
                provenance,
            });
        }
    }

    let file = match workspace.dir.as_str() {
        "." => "armada.yml".to_string(),
        dir => format!("{dir}/armada.yml"),
    };
    Draft {
        dir: workspace.dir.clone(),
        path: Path::new(checkout).join(&file),
        file,
        id: ProposedId {
            value: id,
            provenance: convention(&workspace.dir, None),
        },
        ports: ports(&workspace.ports),
        checks,
        commands,
        setup,
        policy: default_policy(),
        written: None,
    }
}

fn is_check(name: &str) -> bool {
    CHECK_NAMES.contains(&name) || name.starts_with("test:") || name.starts_with("lint:")
}

fn check(name: &str, run: String, provenance: Provenance) -> ProposedCheck {
    ProposedCheck {
        name: name.to_string(),
        run,
        requires: Vec::new(),
        provenance,
    }
}

fn command(name: &str, run: String, provenance: Provenance) -> ProposedCommand {
    ProposedCommand {
        name: name.to_string(),
        run,
        destructive: false,
        provenance,
    }
}

fn convention(file: &str, key: Option<&str>) -> Provenance {
    Provenance::Convention {
        file: file.to_string(),
        key: key.map(str::to_string),
    }
}

fn named(file: &str, name: &str) -> bool {
    file.rsplit('/').next() == Some(name)
}

/// The workspace's own lockfiles, then the root's it shares by a workspace pattern.
fn lockfiles(root: Option<&ScannedWorkspace>, workspace: &ScannedWorkspace) -> Vec<String> {
    let mut files: Vec<String> = workspace
        .lockfiles
        .iter()
        .map(|one| one.file.clone())
        .collect();
    let Some(root) = root.filter(|_| workspace.dir != ".") else {
        return files;
    };
    let shared: BTreeSet<&str> = workspace
        .declared_by
        .iter()
        .flat_map(|glob| SHARED_BY.iter().filter(|(by, _)| named(&glob.file, by)))
        .flat_map(|(_, lockfiles)| lockfiles.iter().copied())
        .collect();
    files.extend(
        root.lockfiles
            .iter()
            .filter(|one| shared.iter().any(|name| named(&one.file, name)))
            .map(|one| one.file.clone()),
    );
    files
}

/// The first of `table`'s entries a lockfile here is named for, with that file.
fn found<'a>(
    files: &'a [String],
    table: &[(&str, &'static str)],
) -> Option<(&'a str, &'static str)> {
    table.iter().find_map(|(name, value)| {
        files
            .iter()
            .find(|file| named(file, name))
            .map(|file| (file.as_str(), *value))
    })
}

/// A port is evidence, so every one reads `read`. A variable keeps its file's name, a
/// compose service needs none, and a script's flag gets `<NAME>_PORT`.
fn ports(declared: &[DeclaredPort]) -> Vec<ProposedPort> {
    let mut names = BTreeSet::new();
    declared
        .iter()
        .map(|port| {
            let variable = port.key == port.name;
            let base = match variable {
                true => port.name.strip_suffix("_PORT").unwrap_or(&port.name),
                false => &port.name,
            };
            let mut name = slug(base);
            let mut n = 1;
            while !names.insert(name.clone()) {
                n += 1;
                name = format!("{}_{n}", slug(base));
            }
            let env = match (variable, port.key.starts_with("services.")) {
                (true, _) => Some(port.name.clone()),
                (false, true) => None,
                (false, false) => Some(format!("{}_PORT", name.to_uppercase())),
            };
            ProposedPort {
                name,
                container: Some(port.container),
                env,
                provenance: Provenance::Read {
                    file: port.file.clone(),
                    key: port.key.clone(),
                },
            }
        })
        .collect()
}

fn slug(text: &str) -> String {
    text.chars()
        .map(|c| match c.is_ascii_alphanumeric() {
            true => c.to_ascii_lowercase(),
            false => '_',
        })
        .collect()
}

/// The checkout's name for the root and the directory's otherwise, or the whole path
/// where two collide, since a Job is keyed to one.
fn ids(scan: &RepositoryScan) -> Vec<String> {
    let short: Vec<String> = scan
        .workspaces
        .iter()
        .map(|one| match one.dir.as_str() {
            "." => Path::new(&scan.checkout).file_name().map_or_else(
                || "root".to_string(),
                |name| name.to_string_lossy().to_string(),
            ),
            dir => dir.rsplit('/').next().unwrap_or(dir).to_string(),
        })
        .collect();
    short
        .iter()
        .zip(&scan.workspaces)
        .map(
            |(id, one)| match short.iter().filter(|other| *other == id).count() {
                1 => id.clone(),
                _ => one.dir.replace('/', "-"),
            },
        )
        .collect()
}
