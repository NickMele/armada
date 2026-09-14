//! A new worktree's build directories, cloned from a warm seed in the base
//! checkout `crate::basing` keeps. #1064.
//!
//! **The seed's state is on the disk.** [`BaseSpec::seed_marker`] is written
//! once every `setup.seed.warm` command has succeeded, so a Fleet killed halfway
//! leaves nothing a Job would clone. Which commit is warming is held in memory
//! only, because a restart ends the build it would name.
//!
//! **The warm-up runs off the turn loop**, which awaits every Job in turn and
//! would hold them all for the length of a workspace build. One runs at a time
//! on the machine until #1063 gives it a place beside the Checks.
//!
//! **What each worktree got is written beside the Job's log**, under
//! `.armada/seeds/`, because the run sheet asks after a restart too and no
//! column holds it.

mod cloning;
mod reading;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, BaseSpec, Delivery, Vcs, WorkProduct, Worktree};
use config::{Manifest, Preparation, Seed};
use core_model::{Component, Envelope, FieldValue, Job, Level};
use tokio::task::JoinHandle;

use crate::basing::NoBase;
use crate::daemon::Fleet;
use crate::preparing::prepare_one;

pub use cloning::{CopyOnWrite, NotCloned, TheVolume};

/// Which seed is warming, and which warm-up failed at which commit.
#[derive(Debug, Default)]
pub(crate) struct Seeds {
    /// Repository root and commit.
    warming: Option<(String, String)>,
    /// By repository root: the commit, and why its warm-up did not finish.
    failed: BTreeMap<String, (String, String)>,
    /// The warm-up the sweep started, kept so a test can await it.
    #[cfg(test)]
    task: Option<JoinHandle<()>>,
}

impl Seeds {
    /// The commit a seed is warming at in this repository, if one is.
    pub(crate) fn warming_in(&self, root: &str) -> Option<&str> {
        self.warming
            .as_ref()
            .filter(|(at, _)| at == root)
            .map(|(_, commit)| commit.as_str())
    }

    fn failed_at(&self, root: &str, commit: &str) -> Option<&str> {
        self.failed
            .get(root)
            .filter(|(at, _)| at == commit)
            .map(|(_, why)| why.as_str())
    }
}

/// What a new worktree's build directories started from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seeding {
    /// Cloned from the seed warmed at this base commit.
    Seeded { commit: String, paths: Vec<String> },
    /// Nothing was cloned.
    Cold(Cold),
}

/// Why a worktree starts cold. **Each sends a person somewhere different**, for
/// [`NoBase`]'s reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cold {
    NoBase(NoBase),
    /// The seed at this commit is warming. A seed is never cloned mid-warm-up.
    Warming {
        commit: String,
    },
    /// No warm-up has finished at this commit, and why the last one failed.
    NotWarm {
        commit: String,
        failed: Option<String>,
    },
    /// The warm seed holds none of the declared directories.
    Empty {
        commit: String,
    },
    /// This volume cannot clone, and a seed is never copied in full.
    NoCopyOnWrite {
        why: String,
    },
    NotCloned {
        path: String,
        why: String,
    },
}

impl Cold {
    /// One line, for the Job's log and its run sheet.
    pub fn said(&self) -> String {
        match self {
            Cold::NoBase(why) => format!("there is no base checkout to seed from: {}", why.said()),
            Cold::Warming { commit } => format!(
                "the seed at base {} is still warming, and a seed is never cloned mid-warm-up",
                short(commit)
            ),
            Cold::NotWarm {
                commit,
                failed: None,
            } => format!("the seed at base {} has not been warmed yet", short(commit)),
            Cold::NotWarm {
                commit,
                failed: Some(why),
            } => format!(
                "the seed's warm-up at base {} did not finish: {why}",
                short(commit)
            ),
            Cold::Empty { commit } => format!(
                "the seed at base {} holds none of the directories `setup.seed.paths` names",
                short(commit)
            ),
            Cold::NoCopyOnWrite { why } => format!(
                "this volume cannot clone copy-on-write, and a seed is never copied in full: {why}"
            ),
            Cold::NotCloned { path, why } => {
                format!("`{path}` could not be cloned from the seed: {why}")
            }
        }
    }
}

fn short(commit: &str) -> &str {
    &commit[..commit.len().min(12)]
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Clone the warm seed into a worktree just cut, and say what it got.
    ///
    /// **A repository that declares no seed returns here** and writes nothing,
    /// logs nothing and clones nothing. `crate::preparing` calls this before
    /// `setup.requires`, so an install writes into a tree already holding the
    /// build.
    ///
    /// The source is never swept mid-clone: dispatch is awaited on the turn
    /// loop, the sweep runs on that same loop, and it never takes the current
    /// commit's checkout.
    pub(crate) async fn seeded(&self, job: &Job, worktree: &Worktree, manifest: &Manifest) {
        let Some(seed) = manifest.seed() else {
            return;
        };
        let seeding = self.seeding(job, worktree, manifest, seed).await;
        match &seeding {
            Seeding::Seeded { commit, paths } => self.noted_seeding(
                job,
                "the worktree was seeded from the base checkout's warm build",
                &[
                    ("commit", FieldValue::Str(commit.clone())),
                    ("paths", FieldValue::Str(paths.join(", "))),
                ],
            ),
            Seeding::Cold(why) => self.noted_seeding(
                job,
                "the worktree starts cold, with no seed",
                &[("because", FieldValue::Str(why.said()))],
            ),
        }
        let Ok(served) = self.served_by(job) else {
            return;
        };
        if let Err(why) = record(served.records_root(), &job.handle(), &seeding) {
            self.noted_seeding(
                job,
                "what the worktree was seeded with could not be written down",
                &[("because", FieldValue::Str(why.to_string()))],
            );
        }
    }

    async fn seeding(
        &self,
        job: &Job,
        worktree: &Worktree,
        manifest: &Manifest,
        seed: &Seed,
    ) -> Seeding {
        let cold = |why: NoBase| Seeding::Cold(Cold::NoBase(why));
        let served = match self.served_by(job) {
            Ok(served) => served,
            Err(why) => {
                return cold(NoBase::NotCheckedOut {
                    why: why.to_string(),
                })
            }
        };
        let at = match self.vcs().base_commit(served.root(), manifest.base()) {
            Ok(Some(at)) => at,
            Ok(None) => return cold(NoBase::Unnamed),
            Err(cause) => {
                return cold(NoBase::NotCheckedOut {
                    why: cause.to_string(),
                })
            }
        };
        let spec = match BaseSpec::at(served.root(), &at) {
            Ok(spec) => spec,
            Err(why) => return cold(NoBase::NotCheckedOut { why: why.said() }),
        };
        let commit = spec.commit().to_string();
        {
            let seeds = self.seeds().lock().expect("the seeds lock is not poisoned");
            if seeds.warming_in(served.root()) == Some(commit.as_str()) {
                return Seeding::Cold(Cold::Warming { commit });
            }
            if !Path::new(&spec.seed_marker()).exists() {
                let failed = seeds.failed_at(served.root(), &commit).map(str::to_string);
                return Seeding::Cold(Cold::NotWarm { commit, failed });
            }
        }
        let copying = Arc::clone(self.copy_on_write());
        let (from, into) = (PathBuf::from(spec.path()), PathBuf::from(worktree.path()));
        let paths = seed.paths().to_vec();
        let cloned = tokio::task::spawn_blocking(move || {
            cloned_into(copying.as_ref(), &from, &into, &paths)
        })
        .await;
        match cloned {
            Ok(Ok(paths)) if paths.is_empty() => Seeding::Cold(Cold::Empty { commit }),
            Ok(Ok(paths)) => Seeding::Seeded { commit, paths },
            Ok(Err(cold)) => Seeding::Cold(cold),
            Err(joined) => Seeding::Cold(Cold::NotCloned {
                path: seed.paths().join(", "),
                why: joined.to_string(),
            }),
        }
    }

    /// Start warming the seed at a base commit that has none, where one is due.
    ///
    /// **At most one at a time on the machine.** A warm-up is a workspace build.
    /// One that failed is not tried again at the same commit: it would fail the
    /// same way on every sweep, and the base moving is what changes the answer.
    ///
    /// The handle is the task's, for a test to await. The sweep drops it.
    pub(crate) fn warm_seeds(&self) -> Option<JoinHandle<()>> {
        let mut seeds = self.seeds().lock().expect("the seeds lock is not poisoned");
        if seeds.warming.is_some() {
            return None;
        }
        for served in self.repositories().served() {
            let manifest = served.manifest();
            let Some(seed) = manifest.seed() else {
                continue;
            };
            let Ok(Some(at)) = self.vcs().base_commit(served.root(), manifest.base()) else {
                continue;
            };
            let Ok(spec) = BaseSpec::at(served.root(), &at) else {
                continue;
            };
            if Path::new(&spec.seed_marker()).exists()
                || seeds.failed_at(served.root(), spec.commit()).is_some()
            {
                continue;
            }
            seeds.warming = Some((served.root().to_string(), spec.commit().to_string()));
            let warm = Warm {
                vcs: Arc::clone(self.vcs()),
                spec,
                required: manifest.prepared_by().to_vec(),
                warm: seed.warmed_by().to_vec(),
                paths: seed.paths().to_vec(),
                budget: self.budget().duration(),
                seeds: Arc::clone(self.seeds()),
                preparing: Arc::clone(self.base_preparing()),
                copying: Arc::clone(self.copy_on_write()),
            };
            return Some(tokio::spawn(warm.run()));
        }
        None
    }

    /// [`warm_seeds`](Fleet::warm_seeds), from the sweep. The task runs on
    /// detached; a test build keeps its handle.
    pub(crate) fn warm_seeds_on_the_sweep(&self) {
        let _started = self.warm_seeds();
        #[cfg(test)]
        if let Some(task) = _started {
            self.seeds()
                .lock()
                .expect("the seeds lock is not poisoned")
                .task = Some(task);
        }
    }

    /// The warm-up the last sweep started, if one did.
    #[cfg(test)]
    pub(crate) fn warm_up_the_sweep_started(&self) -> Option<JoinHandle<()>> {
        self.seeds()
            .lock()
            .expect("the seeds lock is not poisoned")
            .task
            .take()
    }

    /// A line in the Job's own log, for `crate::preparing`'s reason.
    fn noted_seeding(&self, job: &Job, said: &str, fields: &[(&'static str, FieldValue)]) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone());
        for (key, value) in fields {
            envelope = envelope.with_field(*key, value.clone());
        }
        self.noted_in_the_log(job.id(), &envelope);
    }
}

/// One warm-up, owning everything it needs so it can be spawned.
struct Warm<V> {
    vcs: Arc<V>,
    spec: BaseSpec,
    required: Vec<Preparation>,
    warm: Vec<Preparation>,
    paths: Vec<String>,
    budget: Duration,
    seeds: Arc<std::sync::Mutex<Seeds>>,
    preparing: Arc<tokio::sync::Mutex<()>>,
    copying: Arc<dyn CopyOnWrite>,
}

impl<V> Warm<V>
where
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
{
    async fn run(self) {
        let warmed = self.warmed().await;
        let root = self.spec.repo_root().to_string();
        let mut seeds = self.seeds.lock().expect("the seeds lock is not poisoned");
        seeds.warming = None;
        match warmed {
            Ok(()) => {
                seeds.failed.remove(&root);
            }
            Err(why) => {
                seeds
                    .failed
                    .insert(root, (self.spec.commit().to_string(), why));
            }
        }
    }

    async fn warmed(&self) -> Result<(), String> {
        let (vcs, spec) = (Arc::clone(&self.vcs), self.spec.clone());
        let checkout = tokio::task::spawn_blocking(move || {
            vcs.base_checkout(&spec).map_err(|cause| cause.to_string())
        })
        .await
        .map_err(|joined| joined.to_string())??;
        {
            // A before run may be preparing this same checkout.
            let _held = self.preparing.lock().await;
            if !Path::new(&self.spec.ready_marker()).exists() {
                crate::basing::prepare_checkout(&self.required, &self.spec, self.budget)
                    .await
                    .map_err(|why| why.said())?;
            }
        }
        let (copying, spec, paths) = (
            Arc::clone(&self.copying),
            self.spec.clone(),
            self.paths.clone(),
        );
        let _ =
            tokio::task::spawn_blocking(move || carried_forward(copying.as_ref(), &spec, &paths))
                .await;
        for command in &self.warm {
            prepare_one(
                command,
                Path::new(checkout.path()),
                self.budget,
                &BTreeMap::new(),
                &[],
            )
            .await
            .map_err(|cause| format!("`{}` {}", cause.command, verification::how(&cause.exit)))?;
        }
        // Written last: a checkout carrying this file ran every warm-up command.
        std::fs::write(self.spec.seed_marker(), self.spec.commit())
            .map_err(|cause| format!("the seed could not be marked warm: {cause}"))
    }
}

/// Start from the last warm seed, so a base that moved builds on it rather
/// than from nothing. Best effort: a warm-up with nothing carried is only slower.
fn carried_forward(copying: &dyn CopyOnWrite, spec: &BaseSpec, paths: &[String]) {
    if let Some(previous) = newest_warm_seed(spec) {
        let _ = cloned_into(
            copying,
            Path::new(&previous.path()),
            Path::new(&spec.path()),
            paths,
        );
    }
}

/// The most recently marked warm seed at any commit but `spec`'s, by the
/// marker's modified time: `read_dir` order is arbitrary, and a commit id says
/// nothing about age. The sweep keeps this one until `spec`'s seed is warm.
pub(crate) fn newest_warm_seed(spec: &BaseSpec) -> Option<BaseSpec> {
    std::fs::read_dir(spec.parent())
        .ok()?
        .flatten()
        .filter_map(|entry| {
            BaseSpec::at(spec.repo_root(), &entry.file_name().to_string_lossy()).ok()
        })
        .filter(|other| other.commit() != spec.commit())
        .filter_map(|other| {
            let marked = std::fs::metadata(other.seed_marker())
                .ok()?
                .modified()
                .ok()?;
            Some((marked, other))
        })
        .max_by_key(|(marked, _)| *marked)
        .map(|(_, newest)| newest)
}

/// Clone each declared directory that the seed holds and the tree does not.
fn cloned_into(
    copying: &dyn CopyOnWrite,
    from: &Path,
    into: &Path,
    paths: &[String],
) -> Result<Vec<String>, Cold> {
    let mut cloned = Vec::new();
    for path in paths {
        let (source, target) = (from.join(path), into.join(path));
        // Never over what the tree already holds, and never a directory the
        // warm-up did not write.
        if !source.is_dir() || target.exists() {
            continue;
        }
        let not_cloned = |why: String| Cold::NotCloned {
            path: path.clone(),
            why,
        };
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|why| not_cloned(why.to_string()))?;
        }
        match copying.clone_tree(&source, &target) {
            Ok(()) => cloned.push(path.clone()),
            Err(NotCloned::Unsupported { why }) => return Err(Cold::NoCopyOnWrite { why }),
            Err(NotCloned::Failed { why }) => return Err(not_cloned(why)),
        }
    }
    Ok(cloned)
}

/// What one Job's worktree was seeded with, as [`record`] wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded {
    Seeded { commit: String, paths: Vec<String> },
    Cold { why: String },
}

/// Beside the Job's log, outside the worktree, so it outlives both a restart
/// and `armada clean`.
pub(crate) fn record_of(records_root: &str, handle: &str) -> PathBuf {
    Path::new(records_root)
        .join(".armada")
        .join("seeds")
        .join(handle)
}

fn record(records_root: &str, handle: &str, seeding: &Seeding) -> std::io::Result<()> {
    let at = record_of(records_root, handle);
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = match seeding {
        Seeding::Seeded { commit, paths } => format!("seeded\n{commit}\n{}\n", paths.join("\n")),
        Seeding::Cold(why) => format!("cold\n{}\n", why.said().replace('\n', " ")),
    };
    std::fs::write(at, text)
}

/// What [`record`] wrote, read back. `None` where nothing was, which is a
/// worktree cut before this existed or one whose repository declares no seed.
pub(crate) fn recorded(records_root: &str, handle: &str) -> Option<Recorded> {
    let text = std::fs::read_to_string(record_of(records_root, handle)).ok()?;
    let mut lines = text.lines();
    match lines.next()? {
        "seeded" => Some(Recorded::Seeded {
            commit: lines.next()?.to_string(),
            paths: lines.map(str::to_string).collect(),
        }),
        "cold" => Some(Recorded::Cold {
            why: lines.next().unwrap_or_default().to_string(),
        }),
        _ => None,
    }
}
