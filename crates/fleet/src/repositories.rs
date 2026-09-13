//! The repositories one Fleet serves: the one it was started in, first, and
//! every one a person added by folder since.
//!
//! **A Job reaches its repository through `owner_manifest_id`**, so two
//! repositories are never allowed to hold one Manifest id — [`Repositories::add`]
//! refuses the second rather than guessing which a Job meant.
//!
//! **A repository may be served before it has a Manifest.** Setup starts from a
//! folder nobody wrote an `armada.yml` for; Scan reads it as it is, and the
//! Manifest is set once, when one loads there. [`Served`] is the type that
//! proves one did, and only a [`Served`] reaches a Job, a Check or a worktree.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};

use config::{Manifest, ResolvedWorkflow};
use core_model::WorkflowId;

mod adding;

/// A repository's `armada.yml` and the workflows resolved against it.
#[derive(Clone, Debug)]
pub struct SetUp {
    manifest: Manifest,
    workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    /// The Kit and carried definitions that would not resolve here.
    left_out: Vec<ipc::LeftOutWorkflow>,
}

impl SetUp {
    /// Every workflow here was resolved against `manifest`: `armada::Setup` is
    /// what hands the pair over, so this does not check them again.
    pub fn of(manifest: Manifest, workflows: BTreeMap<WorkflowId, ResolvedWorkflow>) -> SetUp {
        SetUp {
            manifest,
            workflows,
            left_out: Vec::new(),
        }
    }

    /// What this repository's catalogue left out, as the wire carries it.
    pub fn leaving_out(self, left_out: Vec<ipc::LeftOutWorkflow>) -> SetUp {
        SetUp { left_out, ..self }
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn workflows(&self) -> &BTreeMap<WorkflowId, ResolvedWorkflow> {
        &self.workflows
    }
}

/// What reading a folder found, before Fleet serves it.
#[derive(Clone)]
pub struct Located {
    /// The repository's root, resolved.
    pub root: String,
    /// Where its records go — `crate::records::root`, resolved by whoever
    /// knows the machine's data directory.
    pub records_root: String,
    /// Absent where the folder has no `armada.yml` yet.
    pub set_up: Option<SetUp>,
}

/// Why a folder was not read as a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotLocated {
    /// Not a folder, or not the root of a git repository.
    NotARepository { folder: String, why: String },
    /// A repository whose `armada.yml` or workflows Armada will not have.
    Refused { root: String, why: String },
}

impl std::fmt::Display for NotLocated {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotLocated::NotARepository { folder, why } => {
                write!(out, "{folder} is not the root of a git repository: {why}")
            }
            NotLocated::Refused { root, why } => {
                write!(out, "{root} is a repository Armada will not serve: {why}")
            }
        }
    }
}

/// Reading a folder into a repository. **A seam**, because what reads one —
/// git, `armada.yml`, Kit's workflows — is the composition root's.
pub trait Locating: Send + Sync {
    /// Read `folder`. **No side effects**: Fleet may still refuse what comes back.
    fn located(&self, folder: &Path) -> Result<Located, NotLocated>;
    /// Fleet now serves the Manifest at `root`: watch it, and publish what a
    /// repository Fleet serves is given.
    fn serving(&self, root: &str);
}

/// One repository Fleet serves.
pub struct Repository {
    root: String,
    records_root: String,
    /// Set once and never unset, which is what lets [`Served`] hold it.
    set_up: OnceLock<SetUp>,
    /// What this Fleet's last read of its `armada.yml` came to. Never written
    /// down — `crate::daemon::rereading`.
    reading: Mutex<Option<ipc::ManifestReading>>,
    /// The `armada.yml` proposals Setup is iterating here, by workspace.
    proposals: crate::manifest_proposal::Held,
}

impl Repository {
    fn of(root: String, records_root: String, set_up: Option<SetUp>) -> Arc<Repository> {
        let repository = Repository {
            root,
            records_root,
            set_up: OnceLock::new(),
            reading: Mutex::new(None),
            proposals: Default::default(),
        };
        if let Some(set_up) = set_up {
            let _ = repository.set_up.set(set_up);
        }
        Arc::new(repository)
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn records_root(&self) -> &str {
        &self.records_root
    }

    pub fn manifest(&self) -> Option<&Manifest> {
        self.set_up.get().map(SetUp::manifest)
    }

    pub(crate) fn proposals(&self) -> &crate::manifest_proposal::Held {
        &self.proposals
    }

    pub(crate) fn reading(&self) -> Option<ipc::ManifestReading> {
        self.reading
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn read(&self, reading: ipc::ManifestReading) {
        *self.reading.lock().unwrap_or_else(PoisonError::into_inner) = Some(reading);
    }
}

/// A repository that has a Manifest. **The only way to one is through
/// [`Repositories`]**, which makes it only where the Manifest is set.
#[derive(Clone)]
pub struct Served(Arc<Repository>);

impl Served {
    fn of(repository: &Arc<Repository>) -> Option<Served> {
        repository
            .set_up
            .get()
            .map(|_| Served(Arc::clone(repository)))
    }

    fn set_up(&self) -> &SetUp {
        self.0
            .set_up
            .get()
            .expect("a Served is only made over a repository whose Manifest is set")
    }

    pub fn root(&self) -> &str {
        &self.0.root
    }

    pub fn records_root(&self) -> &str {
        &self.0.records_root
    }

    pub fn manifest(&self) -> &Manifest {
        self.set_up().manifest()
    }

    pub fn workflows(&self) -> &BTreeMap<WorkflowId, ResolvedWorkflow> {
        self.set_up().workflows()
    }

    pub fn left_out(&self) -> &[ipc::LeftOutWorkflow] {
        &self.set_up().left_out
    }

    pub fn repository(&self) -> &Arc<Repository> {
        &self.0
    }

    /// Whether a Job naming `manifest` as its owner is this repository's.
    pub fn owns(&self, manifest: &str) -> bool {
        self.manifest().id().as_str() == manifest
    }
}

/// Why a repository was not served.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAdded {
    /// Its root is already served.
    AlreadyServed { root: String },
    /// Another served repository's `armada.yml` declares the same id, and a
    /// Job names its repository by that id alone.
    ManifestServed { id: String, by: String },
    /// Nothing is served at this root.
    NotServed { root: String },
}

impl std::fmt::Display for NotAdded {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotAdded::AlreadyServed { root } => write!(out, "{root} is already served"),
            NotAdded::NotServed { root } => write!(out, "nothing is served at {root}"),
            NotAdded::ManifestServed { id, by } => write!(
                out,
                "{by} already declares Manifest id `{id}`, and a Job names its repository by that \
                 id — give one of the two `armada.yml` files another"
            ),
        }
    }
}

/// Every repository one Fleet serves, in the order they were added.
pub struct Repositories(RwLock<Vec<Arc<Repository>>>);

impl Repositories {
    /// The repository Fleet was started in, which always has a Manifest.
    pub fn starting_in(root: String, records_root: String, set_up: SetUp) -> Repositories {
        Repositories(RwLock::new(vec![Repository::of(
            root,
            records_root,
            Some(set_up),
        )]))
    }

    fn every_held(&self) -> std::sync::RwLockReadGuard<'_, Vec<Arc<Repository>>> {
        self.0.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Serve one more. Checked and added under one lock, so two adds of one
    /// folder cannot both pass.
    pub fn add(&self, located: Located) -> Result<Arc<Repository>, NotAdded> {
        let mut every = self.0.write().unwrap_or_else(PoisonError::into_inner);
        if every.iter().any(|one| one.root == located.root) {
            return Err(NotAdded::AlreadyServed { root: located.root });
        }
        if let Some(set_up) = &located.set_up {
            refuse_a_second_id(&every, set_up.manifest())?;
        }
        let added = Repository::of(located.root, located.records_root, located.set_up);
        every.push(Arc::clone(&added));
        Ok(added)
    }

    /// Give a repository served without a Manifest the one that now loads
    /// there. A repository that has one keeps it.
    pub fn set_up(&self, root: &str, set_up: SetUp) -> Result<Served, NotAdded> {
        let every = self.0.write().unwrap_or_else(PoisonError::into_inner);
        let Some(repository) = every.iter().find(|one| one.root == root) else {
            return Err(NotAdded::NotServed {
                root: root.to_string(),
            });
        };
        if repository.set_up.get().is_none() {
            refuse_a_second_id(&every, set_up.manifest())?;
            let _ = repository.set_up.set(set_up);
        }
        Ok(Served::of(repository).expect("set just above, or already"))
    }

    /// The repository Fleet was started in.
    pub fn first(&self) -> Served {
        Served::of(&self.every_held()[0]).expect("the first repository always has a Manifest")
    }

    /// The repository whose Manifest has this id.
    pub fn serving(&self, manifest: &str) -> Option<Served> {
        self.served().into_iter().find(|one| one.owns(manifest))
    }

    /// The repository at this root, Manifest or none.
    pub fn at(&self, root: &str) -> Option<Arc<Repository>> {
        self.every_held()
            .iter()
            .find(|one| one.root == root)
            .cloned()
    }

    pub fn every(&self) -> Vec<Arc<Repository>> {
        self.every_held().clone()
    }

    /// Every repository that has a Manifest, first first.
    pub fn served(&self) -> Vec<Served> {
        self.every_held().iter().filter_map(Served::of).collect()
    }
}

fn refuse_a_second_id(every: &[Arc<Repository>], manifest: &Manifest) -> Result<(), NotAdded> {
    let id = manifest.id().as_str();
    match every
        .iter()
        .find(|one| one.manifest().is_some_and(|held| held.id().as_str() == id))
    {
        Some(by) => Err(NotAdded::ManifestServed {
            id: id.to_string(),
            by: by.root.clone(),
        }),
        None => Ok(()),
    }
}
