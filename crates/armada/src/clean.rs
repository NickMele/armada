//! Giving a repository's worktrees, branches and Jobs back.
//!
//! # It deletes what a Job derives, and never what a pattern matches
//!
//! Cleaning by hand with `git branch -D $(git branch --list 'armada/*')`
//! destroyed nine unmerged branches that belonged to no Job. So the list of
//! branches here is not a list at all: each one is derived from a Job that is
//! being forgotten, through the same `WorktreeSpec` that derived it in the
//! first place. **No Job, no delete.** A worktree on disk with nothing behind
//! it is reported and left alone, because it is evidence rather than litter.
//!
//! # Two scopes, and why the second refuses more
//!
//! Bare, this is a repository's own state: its worktrees, their branches, its
//! Jobs. `--all` additionally removes the machine's store and the two files
//! beside it. Both refuse while a Fleet is running, and for one reason — the
//! Jobs being forgotten are the Jobs that Fleet is holding in memory.

use std::path::{Path, PathBuf};

use adapter_traits::WorktreeSpec;
use adapters::Reclaimed;
use config::Manifest;
use core_model::JobId;
use fleet::runtime::{self, Presence};
use store::{Forgotten, Store};

use crate::serve;
use crate::setup::MANIFEST;

/// The machine files `--all` removes, beside the runtime file itself.
///
/// Named individually rather than by wiping the directory: Application Support
/// is a place other things may put files, and a verb that empties a directory
/// is one nobody can predict.
const MACHINE_FILES: &[&str] = &[
    serve::STORE_FILE,
    "armada.db-wal",
    "armada.db-shm",
    runtime::FILE_NAME,
    serve::MCP_FILE,
];

/// What one Job's clean did.
#[derive(Debug)]
pub struct JobCleaned {
    pub job_id: String,
    pub title: String,
    pub reclaimed: Reclaimed,
    pub forgotten: Forgotten,
}

/// A machine file `--all` was asked about.
#[derive(Debug, PartialEq, Eq)]
pub enum FileGone {
    Removed(PathBuf),
    Absent(PathBuf),
    NotRemoved { path: PathBuf, why: String },
}

/// Everything the clean touched, and everything it deliberately did not.
#[derive(Debug, Default)]
pub struct Cleaned {
    pub repository: PathBuf,
    pub manifest_id: String,
    pub jobs: Vec<JobCleaned>,
    /// Worktree directories with no Job behind them. **Left alone.**
    pub unclaimed: Vec<PathBuf>,
    /// Only with `--all`.
    pub machine: Vec<FileGone>,
    /// Jobs the store holds and could not rebuild. Their worktrees show up as
    /// unclaimed, which is the honest reading — nothing knows they were Jobs.
    pub unreadable: usize,
    /// Things that went wrong and did not stop the rest.
    pub faults: Vec<String>,
}

impl Cleaned {
    /// Whether anything was removed at all. A clean that did nothing says so
    /// rather than printing an empty list.
    pub fn touched_nothing(&self) -> bool {
        self.jobs.is_empty()
            && self
                .machine
                .iter()
                .all(|f| matches!(f, FileGone::Absent(_)))
    }
}

/// Why a clean did not happen at all.
#[derive(Debug)]
pub enum CleanRefused {
    /// There is no Manifest here, so there is no repository to clean and no id
    /// to select Jobs by.
    NotARepository {
        path: PathBuf,
    },
    ManifestRefused(Box<config::LoadError>),
    /// A Fleet holds the store. **Named by pid**, because the next thing the
    /// caller wants is to stop it.
    FleetIsRunning {
        pid: u32,
        port: u16,
    },
    RuntimeUnreadable(Box<runtime::ReadError>),
    StoreUnreadable(Box<store::OpenError>),
    /// The path could not be made absolute, so nothing here could derive a
    /// worktree from it.
    RepositoryUnresolvable {
        path: PathBuf,
        why: std::io::Error,
    },
}

/// Clean `root`, and — with `everything` — the machine files under `machine`.
pub fn clean(root: &Path, machine: &Path, everything: bool) -> Result<Cleaned, CleanRefused> {
    let manifest = read_the_manifest(root)?;
    refuse_a_live_fleet(&machine.join(runtime::FILE_NAME))?;

    let root = root
        .canonicalize()
        .map_err(|why| CleanRefused::RepositoryUnresolvable {
            path: root.to_path_buf(),
            why,
        })?;
    let mut cleaned = Cleaned {
        repository: root.clone(),
        manifest_id: manifest.id().as_str().to_string(),
        ..Cleaned::default()
    };

    let db = machine.join(MACHINE_FILES[0]);
    if db.exists() {
        let mut store =
            Store::open(&db).map_err(|why| CleanRefused::StoreUnreadable(Box::new(why)))?;
        forget_this_manifests_jobs(&mut store, &manifest, &root, &mut cleaned);
    }
    report_what_no_job_claims(&root, &mut cleaned);

    if everything {
        // After the Jobs, never before. Removing the store first would leave
        // every worktree unclaimed and every branch undeleted.
        cleaned.machine = MACHINE_FILES
            .iter()
            .map(|name| remove(machine, name))
            .collect();
    }
    Ok(cleaned)
}

fn read_the_manifest(root: &Path) -> Result<Manifest, CleanRefused> {
    let path = root.join(MANIFEST);
    Manifest::load(&path).map_err(|why| match &why {
        config::LoadError::Unreadable { cause, .. }
            if cause.kind() == std::io::ErrorKind::NotFound =>
        {
            CleanRefused::NotARepository { path }
        }
        _ => CleanRefused::ManifestRefused(Box::new(why)),
    })
}

/// **The refusal that matters.** Forgetting Jobs out from under a live Fleet
/// leaves it holding records that no longer exist.
fn refuse_a_live_fleet(runtime_file: &Path) -> Result<(), CleanRefused> {
    match runtime::read(runtime_file) {
        Ok(Presence::Running(live)) => Err(CleanRefused::FleetIsRunning {
            pid: live.pid,
            port: live.port,
        }),
        Ok(_) => Ok(()),
        Err(why) => Err(CleanRefused::RuntimeUnreadable(Box::new(why))),
    }
}

fn forget_this_manifests_jobs(
    store: &mut Store,
    manifest: &Manifest,
    root: &Path,
    cleaned: &mut Cleaned,
) {
    let loaded = match store.load_all_jobs() {
        Ok(loaded) => loaded,
        // A row that will not rebuild does not hide the rows that will, and the
        // count is carried out rather than dropped.
        Err(store::LoadAllError::SomeJobsUnreadable { loaded, failed }) => {
            cleaned.unreadable = failed.len();
            loaded
        }
        Err(why) => {
            cleaned
                .faults
                .push(format!("the store would not be read: {why}"));
            return;
        }
    };

    let mine: Vec<(JobId, String)> = loaded
        .jobs
        .iter()
        .filter(|job| job.owner_manifest_id().as_str() == manifest.id().as_str())
        .map(|job| (job.id().clone(), job.title().as_str().to_string()))
        .collect();

    for (job_id, title) in mine {
        let spec = match WorktreeSpec::for_job(&root.to_string_lossy(), job_id.as_str()) {
            Ok(spec) => spec,
            Err(refused) => {
                cleaned
                    .faults
                    .push(format!("{}: {}", job_id.as_str(), refused.said()));
                continue;
            }
        };
        let reclaimed = match adapters::reclaim(&spec) {
            Ok(reclaimed) => reclaimed,
            Err(why) => {
                cleaned
                    .faults
                    .push(format!("{} could not be opened: {}", why.repo, why.why));
                return;
            }
        };
        // The worktree first, the record second. A Job forgotten before its
        // worktree failed to go is a worktree nothing can derive a branch for.
        if reclaimed.faulted() {
            cleaned.jobs.push(JobCleaned {
                job_id: job_id.as_str().to_string(),
                title,
                reclaimed,
                forgotten: Forgotten::default(),
            });
            continue;
        }
        match store.forget_job(&job_id) {
            Ok(forgotten) => cleaned.jobs.push(JobCleaned {
                job_id: job_id.as_str().to_string(),
                title,
                reclaimed,
                forgotten,
            }),
            Err(why) => cleaned
                .faults
                .push(format!("{} was not forgotten: {why}", job_id.as_str())),
        }
    }
}

/// Directories under `.armada/worktrees/` that no Job just accounted for.
///
/// **Reported, never removed.** A checkout with no Job behind it is the shape a
/// half-finished dispatch or a hand-made directory leaves, and both are worth
/// looking at before anything deletes them.
fn report_what_no_job_claims(root: &Path, cleaned: &mut Cleaned) {
    let parent = root.join(".armada").join("worktrees");
    let Ok(entries) = std::fs::read_dir(&parent) else {
        return;
    };
    let accounted: Vec<&str> = cleaned.jobs.iter().map(|job| job.job_id.as_str()).collect();
    for entry in entries.flatten() {
        let path = entry.path();
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() && !accounted.contains(&named) {
            cleaned.unclaimed.push(path);
        }
    }
    cleaned.unclaimed.sort();
}

fn remove(machine: &Path, name: &str) -> FileGone {
    let path = machine.join(name);
    if !path.exists() {
        return FileGone::Absent(path);
    }
    match std::fs::remove_file(&path) {
        Ok(()) => FileGone::Removed(path),
        Err(why) => FileGone::NotRemoved {
            path,
            why: why.to_string(),
        },
    }
}

impl std::fmt::Display for CleanRefused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanRefused::NotARepository { path } => write!(
                out,
                "there is no {} here — `armada clean` cleans the repository you are standing in",
                path.display()
            ),
            CleanRefused::ManifestRefused(why) => write!(out, "{why}"),
            CleanRefused::FleetIsRunning { pid, port } => write!(
                out,
                "Fleet is running as pid {pid} on port {port}, and it is holding the \
                 Jobs you are asking to forget. Stop it first — `kill {pid}` — then \
                 run this again."
            ),
            CleanRefused::RuntimeUnreadable(why) => write!(out, "{why}"),
            CleanRefused::StoreUnreadable(why) => write!(out, "{why}"),
            CleanRefused::RepositoryUnresolvable { path, why } => {
                write!(out, "{} could not be resolved: {why}", path.display())
            }
        }
    }
}

impl std::error::Error for CleanRefused {}
