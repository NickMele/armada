//! Scan: one read-only pass over a repository nobody set up for Armada, the
//! first thing Setup does — `docs/concepts/manifest.md`, *Setup*.
//!
//! **It writes nothing, because it cannot.** It is handed a [`Tree`], which
//! reads a file and lists a directory, so there is no write to call by
//! mistake. It runs nothing either, for drift's reason.
//!
//! **A Scan is not a Job**: no worktree, no toolset, no `owner_manifest_id`.
//! Fleet serves it against the checkout it was started in until Locate (#821)
//! names another.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use adapter_traits::CiConfiguration;
use ipc::{
    EvidenceStrength, MissingName, NotRead, RepositoryScan, ScannedWorkspace, WorkspaceGlob,
};

mod ci;
mod compose;
mod findings;
mod workspaces;

#[cfg(test)]
mod tests;

/// A repository's files, read and never written — the seam's own type, so the
/// CI reader is handed exactly what Scan is and neither can be handed a write.
pub use adapter_traits::{FileEntry as Entry, FileRead as Read, RepositoryFiles as Tree};

/// A directory on disk, as a [`Tree`].
pub struct Checkout {
    root: PathBuf,
}

impl Checkout {
    pub fn at(root: impl Into<PathBuf>) -> Checkout {
        Checkout { root: root.into() }
    }
}

impl Tree for Checkout {
    fn read(&self, path: &str) -> Read {
        match std::fs::read(self.root.join(path)) {
            Ok(bytes) => Read::Bytes(bytes),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Read::Absent,
            Err(why) => Read::Unreadable(why.to_string()),
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<Entry>, String> {
        let listed = std::fs::read_dir(self.root.join(dir)).map_err(|why| why.to_string())?;
        let mut entries = Vec::new();
        for entry in listed {
            let entry = entry.map_err(|why| why.to_string())?;
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if !kind.is_symlink() {
                entries.push(Entry {
                    name,
                    is_dir: kind.is_dir(),
                });
            }
        }
        Ok(entries)
    }
}

/// Every workspace below the root holding its own `armada.yml`, off Scan's own
/// walk and patterns rather than a second one.
pub(crate) fn manifested(tree: &impl Tree) -> Vec<String> {
    workspaces::discover(tree)
        .dirs
        .into_iter()
        .filter(|(dir, (names, _))| !dir.is_empty() && names.contains("armada.yml"))
        .map(|(dir, _)| dir)
        .collect()
}

/// One workspace as it is being read, before it is put on the wire.
pub(crate) struct Reading {
    pub(crate) scanned: ScannedWorkspace,
    /// Whether every file that could name a runnable here was read. An unread
    /// name is not an absent one, so such a workspace is neither marked nor
    /// compared against.
    pub(crate) names_known: bool,
}

/// Read every workspace in `tree`, and what its CI jobs run through `ci`,
/// writing nothing. `checkout` is only what the answer says was read.
pub fn scan(checkout: &str, tree: &impl Tree, ci: &dyn CiConfiguration) -> RepositoryScan {
    let found = workspaces::discover(tree);
    let mut readings: Vec<Reading> = found
        .dirs
        .iter()
        .map(|(dir, (names, declared_by))| read_one(tree, dir, names, declared_by))
        .collect();
    mark_missing(&mut readings);

    let mut scanned = RepositoryScan {
        checkout: checkout.to_string(),
        workspaces: readings.into_iter().map(|one| one.scanned).collect(),
        ci_commands: Vec::new(),
        not_read: found.not_read,
    };
    ci::join(&mut scanned, ci.read_jobs(tree));
    scanned
}

fn read_one(
    tree: &impl Tree,
    dir: &str,
    names: &BTreeSet<String>,
    declared_by: &[WorkspaceGlob],
) -> Reading {
    let mut reading = findings::read(tree, dir, names);
    reading.scanned.declared_by = declared_by.to_vec();
    reading.scanned.evidence = strength(&reading.scanned);
    reading
}

/// Strong only where a file names something runnable. That `cargo test` would
/// work on a bare `Cargo.toml` is convention, and convention is Proposal's to
/// say and label.
fn strength(workspace: &ScannedWorkspace) -> EvidenceStrength {
    if !workspace.runnables.is_empty() {
        return EvidenceStrength::Strong;
    }
    let read_anything = !workspace.manifests.is_empty()
        || !workspace.lockfiles.is_empty()
        || !workspace.tools.is_empty()
        || !workspace.services.is_empty()
        || !workspace.ports.is_empty();
    match read_anything {
        true => EvidenceStrength::Thin,
        false => EvidenceStrength::NotFollowed,
    }
}

/// Mark a runnable name every sibling declares and this workspace does not —
/// narrowly, as the journey's grid is, so `typecheck` absent from some is no
/// gap.
///
/// **Over the batch the picker ticks by default**: strong workspaces only. The
/// grid is drawn over a batch, and one thin sibling declaring nothing would
/// otherwise erase every mark. **The root is not a sibling** — it holds its own
/// commands, never hoisted from a workspace's.
pub(crate) fn mark_missing(readings: &mut [Reading]) {
    let named: BTreeMap<String, BTreeSet<String>> = readings
        .iter()
        .filter(|one| one.scanned.dir != "." && one.names_known)
        .filter(|one| one.scanned.evidence == EvidenceStrength::Strong)
        .map(|one| {
            let names = one.scanned.runnables.iter().map(|r| r.name.clone());
            (one.scanned.dir.clone(), names.collect())
        })
        .collect();

    for reading in readings.iter_mut() {
        let dir = &reading.scanned.dir;
        let Some(own) = named.get(dir) else {
            continue;
        };
        let siblings: Vec<(&String, &BTreeSet<String>)> =
            named.iter().filter(|(other, _)| *other != dir).collect();
        let Some(((_, first), rest)) = siblings.split_first() else {
            continue;
        };
        let shared = first
            .iter()
            .filter(|name| rest.iter().all(|(_, names)| names.contains(*name)))
            .filter(|name| !own.contains(*name));
        reading.scanned.missing = shared
            .map(|name| MissingName {
                name: name.clone(),
                declared_in: siblings.iter().map(|(dir, _)| (*dir).clone()).collect(),
            })
            .collect();
    }
}

/// `dir` and `name` as one repository path.
pub(crate) fn at(dir: &str, name: &str) -> String {
    match dir.is_empty() {
        true => name.to_string(),
        false => format!("{dir}/{name}"),
    }
}

/// A directory as a person searches for it: `.` for the root.
pub(crate) fn shown(dir: &str) -> String {
    match dir.is_empty() {
        true => ".".to_string(),
        false => dir.to_string(),
    }
}

pub(crate) fn not_read(file: String, why: impl Into<String>) -> NotRead {
    NotRead {
        file,
        why: why.into(),
    }
}
