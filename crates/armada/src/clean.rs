//! Giving a repository's worktrees, branches and Jobs back.
//!
//! # It deletes what a record derives, never what a pattern matches
//!
//! A hand-run `git branch -D` over the `armada/*` glob destroyed nine unmerged
//! branches belonging to no Job. So the branches here are not a list: each is
//! derived from a record being forgotten, through the `WorktreeSpec` that
//! derived it in the first place. **No record, no delete.** A worktree with
//! nothing behind it is reported and left: evidence, not litter.
//!
//! **A row that will not rebuild is still a record.** It carries its id and its
//! Manifest and is cleared through that same `WorktreeSpec` — the id came out
//! of this store, not off a glob. Confusing the two left `--all` as the only
//! recovery from a migration that orphaned four rows.
//!
//! # Work nothing has taken is kept, committed or not
//!
//! Fleet commits a finished Job's work, so its branch is the only copy. A
//! branch the base cannot reach is named and left, on the same grounds as an
//! unclaimed worktree. A checkout holding *uncommitted* work is the same claim
//! one step earlier — see [`WorkKept`]. `--force` takes both.

use std::path::{Path, PathBuf};

use adapter_traits::WorktreeSpec;
use adapters::{BranchGone, Reclaimed, UnmergedWork, WorktreeStanding};
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

/// How far one clean reaches. `--all` is a different question from `--force`
/// and is kept a different type, so neither can be passed for the other.
///
/// Both scopes refuse while a Fleet is running, because it holds the records
/// being forgotten in memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// This repository's worktrees, branches and Jobs.
    Repository,
    /// And the machine's store and the files beside it.
    AndTheMachine,
}

/// What one Job's clean did.
#[derive(Debug)]
pub struct JobCleaned {
    pub job_id: String,
    pub title: String,
    pub reclaimed: Reclaimed,
    pub forgotten: Forgotten,
}

/// What clearing one unreadable row did.
///
/// **Not a [`JobCleaned`].** Nothing here folded into a Job, so there is no
/// title and no history to report — a row cleared and a Job forgotten are
/// different events and are said differently.
#[derive(Debug)]
pub struct RowCleared {
    pub job_id: String,
    /// Why it would not rebuild. Said as it goes, because after this the row is
    /// gone and the reason with it.
    pub why: String,
    pub reclaimed: Reclaimed,
    pub forgotten: Forgotten,
}

/// A Job whose checkout holds work nothing has taken.
///
/// **Left where it is, and its record with it.** An unmerged branch is kept
/// because the commit is the only copy; uncommitted work is that with nothing
/// even holding it, and it is the case that reads as no work at all from
/// outside — the branch is level with its base, so every merged-ness reading
/// says the checkout is disposable. The record stays because the record is what
/// derives the checkout: forgetting it would leave a directory nothing can
/// name, which is the `unclaimed` state this file already declines to remove.
///
/// `--force` takes it, which is the same escape a kept branch has.
#[derive(Debug)]
pub struct WorkKept {
    pub job_id: String,
    /// The Job's title, or — for a row that would not rebuild and so has none
    /// — why it would not.
    pub title: String,
    pub path: String,
    /// What `git status --porcelain` reported, so a person can look before
    /// they decide rather than read a byte count afterwards.
    pub files: Vec<String>,
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
    /// Rows this Manifest owns that would not rebuild, cleared by id.
    pub unreadable: Vec<RowCleared>,
    /// Checkouts holding uncommitted work. **Left alone**, records and all.
    pub uncommitted: Vec<WorkKept>,
    /// Rows that would not rebuild and belong to some other Manifest. Left for
    /// the repository that owns them, and counted so a person is not told
    /// nothing about rows they can see in Fleet's boot line.
    pub unreadable_elsewhere: usize,
    /// Things that went wrong and did not stop the rest.
    pub faults: Vec<String>,
}

impl Cleaned {
    /// The branches left standing because nothing has taken their commits.
    ///
    /// Derived from the Jobs rather than stored beside them: two copies of one
    /// list is how a summary comes to disagree with the lines above it.
    pub fn branches_left(&self) -> Vec<&BranchGone> {
        self.jobs
            .iter()
            .map(|job| &job.reclaimed.branch)
            .chain(self.unreadable.iter().map(|row| &row.reclaimed.branch))
            .filter(|branch| {
                matches!(
                    branch,
                    BranchGone::Kept { .. } | BranchGone::KeptUnanswered { .. }
                )
            })
            .collect()
    }

    /// Whether anything was removed at all. A clean that did nothing says so
    /// rather than printing an empty list.
    pub fn touched_nothing(&self) -> bool {
        self.jobs.is_empty()
            && self.unreadable.is_empty()
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

/// Clean `root`, as far as `scope` reaches.
///
/// **It removes worktrees, branches and store rows, and nothing beside them.**
/// A Job's log, its Drones' transcripts, its Checks' output and the
/// deliverables its Judge read all sit under the repository root rather than
/// inside the checkout — `fleet::transcript`, `fleet::check_output` and
/// `fleet::keeping` derive their paths from it — so they survive with no
/// exemption written here. `#223` was the one record that did not: a step's
/// deliverable is written inside the worktree, reaches no commit because
/// `.armada/*` is ignored, and went with the checkout an hour after its Job
/// was merged. Anything added here that reaches outside `.armada/worktrees/`
/// is that defect arriving again. `#69` bounds the four together.
pub fn clean(
    root: &Path,
    machine: &Path,
    scope: Scope,
    unmerged: UnmergedWork,
) -> Result<Cleaned, CleanRefused> {
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
        forget_this_manifests_jobs(&mut store, &manifest, &root, unmerged, &mut cleaned);
    }
    report_what_no_job_claims(&root, &mut cleaned);

    if scope == Scope::AndTheMachine {
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
    unmerged: UnmergedWork,
    cleaned: &mut Cleaned,
) {
    let (loaded, unreadable) = match store.load_all_jobs() {
        Ok(loaded) => (loaded, Vec::new()),
        // A row that will not rebuild does not hide the rows that will, and it
        // is not dropped either — it is cleared below, by the id it still has.
        Err(store::LoadAllError::SomeJobsUnreadable { loaded, failed }) => (loaded, failed),
        Err(why) => {
            cleaned
                .faults
                .push(format!("the store would not be read: {why}"));
            return;
        }
    };

    // Read once, before the loop: `base:` is the repository's own answer to
    // what merged means, and a clean that guessed would keep a branch that had
    // just been merged into the branch the file names.
    let declared_base = manifest.base().map(str::to_string);
    let base = declared_base.as_deref();

    let mine: Vec<(JobId, String)> = loaded
        .jobs
        .iter()
        .filter(|job| job.owner_manifest_id().as_str() == manifest.id().as_str())
        .map(|job| (job.id().clone(), job.title().as_str().to_string()))
        .collect();

    for (job_id, title) in mine {
        match give_back(store, root, &job_id, base, unmerged, cleaned) {
            GaveBack::Done {
                reclaimed,
                forgotten,
            } => cleaned.jobs.push(JobCleaned {
                job_id: job_id.as_str().to_string(),
                title,
                reclaimed,
                forgotten,
            }),
            GaveBack::HeldUncommitted { path, files } => cleaned.uncommitted.push(WorkKept {
                job_id: job_id.as_str().to_string(),
                title,
                path,
                files,
            }),
            GaveBack::Skipped => continue,
            GaveBack::RepositoryClosed => return,
        }
    }

    clear_this_manifests_unreadable_rows(
        store, manifest, root, base, unmerged, unreadable, cleaned,
    );
}

/// The rows this store holds and could not fold, removed by the id they carry.
///
/// **The Manifest on the row is what selects them**, exactly as the owner on a
/// rebuilt Job selects it above — a clean in one repository never reaches
/// another's rows, which is the whole reason `--all` was the wrong recovery.
fn clear_this_manifests_unreadable_rows(
    store: &mut Store,
    manifest: &Manifest,
    root: &Path,
    base: Option<&str>,
    unmerged: UnmergedWork,
    unreadable: Vec<store::UnreadableRow>,
    cleaned: &mut Cleaned,
) {
    for row in unreadable {
        let why = row.why.to_string();
        let Some(named) = row.row else {
            // Nothing to derive from and nothing to select on. Reported, never
            // guessed at.
            cleaned.faults.push(format!(
                "a row would not rebuild and does not name a Job: {why}"
            ));
            continue;
        };
        if named.owner_manifest_id.as_str() != manifest.id().as_str() {
            cleaned.unreadable_elsewhere += 1;
            continue;
        }
        match give_back(store, root, &named.job_id, base, unmerged, cleaned) {
            GaveBack::Done {
                reclaimed,
                forgotten,
            } => cleaned.unreadable.push(RowCleared {
                job_id: named.job_id.as_str().to_string(),
                why,
                reclaimed,
                forgotten,
            }),
            GaveBack::HeldUncommitted { path, files } => cleaned.uncommitted.push(WorkKept {
                job_id: named.job_id.as_str().to_string(),
                // A row that would not rebuild has no title to carry, and the
                // reason it would not is what a person needs instead.
                title: why,
                path,
                files,
            }),
            GaveBack::Skipped => continue,
            GaveBack::RepositoryClosed => return,
        }
    }
}

/// What one id's clean did, and whether the next one is worth attempting.
enum GaveBack {
    Done {
        reclaimed: Reclaimed,
        forgotten: Forgotten,
    },
    /// Its checkout holds work nobody has committed, so neither the checkout
    /// nor the record went. Reported by the caller, which is the one that knows
    /// the Job's title.
    HeldUncommitted { path: String, files: Vec<String> },
    /// This id did not come back; the next may.
    Skipped,
    /// The repository itself would not open, so no id will do better.
    RepositoryClosed,
}

/// One id's worktree and branch given back, and then its row.
///
/// Both callers reach here holding an id this store handed out — a Job that
/// folded, or a row that would not. Neither holds a name it matched.
fn give_back(
    store: &mut Store,
    root: &Path,
    job_id: &JobId,
    base: Option<&str>,
    unmerged: UnmergedWork,
    cleaned: &mut Cleaned,
) -> GaveBack {
    let spec = match WorktreeSpec::for_job(&root.to_string_lossy(), job_id.as_str()) {
        Ok(spec) => spec,
        Err(refused) => {
            cleaned
                .faults
                .push(format!("{}: {}", job_id.as_str(), refused.said()));
            return GaveBack::Skipped;
        }
    };
    // **Before the removal, because after it there is nothing left to ask.**
    // The same reading `adapters::reclaim` takes on its way to the branch, and
    // the same one Fleet's own sweep is gated on — one derivation of what is
    // safe, so a clean and a sweep cannot come to different answers.
    if unmerged == UnmergedWork::Keep {
        if let Ok(WorktreeStanding::Dirty { files }) =
            adapters::standing(&spec, base).map(|stands| stands.worktree)
        {
            return GaveBack::HeldUncommitted {
                path: spec.worktree_path(),
                files,
            };
        }
    }
    let reclaimed = match adapters::reclaim(&spec, base, unmerged) {
        Ok(reclaimed) => reclaimed,
        Err(why) => {
            cleaned
                .faults
                .push(format!("{} could not be opened: {}", why.repo, why.why));
            return GaveBack::RepositoryClosed;
        }
    };
    // The worktree first, the record second. A record forgotten before its
    // worktree failed to go is a worktree nothing can derive a branch for. A
    // *kept* branch is not a fault: the worktree went, the record goes, and the
    // branch is a git branch a person merges with git.
    if reclaimed.faulted() {
        return GaveBack::Done {
            reclaimed,
            forgotten: Forgotten::default(),
        };
    }
    match store.forget_job(job_id) {
        Ok(forgotten) => GaveBack::Done {
            reclaimed,
            forgotten,
        },
        Err(why) => {
            cleaned
                .faults
                .push(format!("{} was not forgotten: {why}", job_id.as_str()));
            GaveBack::Skipped
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
    let accounted: Vec<&str> = cleaned
        .jobs
        .iter()
        .map(|job| job.job_id.as_str())
        .chain(cleaned.unreadable.iter().map(|row| row.job_id.as_str()))
        .collect();
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
