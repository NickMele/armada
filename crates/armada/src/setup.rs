//! A repository's own setup, found from its root.
//!
//! # A repository carries its setup, and Fleet is pointed at the repository
//!
//! `armada.yml` at the root, workflow definitions in `.armada/workflows/`
//! beside it. There is no `--manifest` flag and no `--workflow` flag, and that
//! is the decision this module exists to hold: a pair of paths on a command
//! line puts the answer in a second place, so two Fleets started differently
//! can disagree about one repository, and a scratch copy of a Manifest becomes
//! indistinguishable from the real one.
//!
//! It also keeps the machine out of it. A list of projects held somewhere
//! central is where per-repository configuration ends up, and the Job Board is
//! per Manifest — which makes that list a Reach-milestone question rather than
//! a shortcut available here.
//!
//! # Holding one of these is proof the files agree
//!
//! [`Setup`] has no constructor but [`Setup::at`], and its fields are private.
//! Every [`ResolvedWorkflow`] inside it can only have been built against the
//! [`Manifest`] beside it, so every Check any workflow's steps name was
//! declared by that Manifest at the moment the daemon started — checked once,
//! before a worktree exists and before a Drone is spawned.
//!
//! # Every file in `.armada/workflows/`, keyed by its own id
//!
//! A repository may declare more than one workflow — Bug and Feature are not
//! the same shape of work — so [`Setup::workflows`] is a map rather than a
//! single value, keyed by the `workflow_id` each definition carries. Two files
//! naming the same id is refused at start: a Fleet that picked one silently
//! would be choosing on behalf of whoever wrote the second file.
//!
//! # Every fault, and never the first one
//!
//! `config` collects each refusal in a document rather than stopping, and
//! [`SetupRefused`] carries the whole set through to the terminal. The person
//! reading the output is the person who wrote the file, and a parser that
//! reported one fault per run would turn one edit into three.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use config::{LoadError, Manifest, ResolveError, ResolvedWorkflow, Roster, WorkflowDef};

/// The Manifest's name at a repository root. Not configurable: a repository
/// that could name its own Manifest is one where finding the Manifest requires
/// already having read it.
pub const MANIFEST: &str = "armada.yml";

/// Where a repository's workflow definitions live, relative to its root.
pub const WORKFLOWS: &str = ".armada/workflows";

/// The extensions a definition may carry.
///
/// All three, because `config` reads a definition with a YAML parser and JSON
/// is a subset of YAML — so which of these a repository wrote is a choice about
/// tooling rather than about meaning, and refusing two of them would be this
/// module inventing a rule the parser does not have.
const DEFINITION_EXTS: &[&str] = &["json", "yml", "yaml"];

/// One repository's Manifest and every workflow its steps resolved against.
#[derive(Debug)]
pub struct Setup {
    root: PathBuf,
    manifest: Manifest,
    workflows: BTreeMap<core_model::WorkflowId, ResolvedWorkflow>,
}

impl Setup {
    /// Read `root`'s Manifest, read every workflow beside it, and resolve each
    /// against the Manifest.
    ///
    /// **`root` is the repository, not a search hint.** Nothing walks upward
    /// looking for an `armada.yml` in a parent: a daemon that quietly adopted
    /// an ancestor's Manifest would run a Job against a repository nobody
    /// pointed it at, and the failure would read as a workflow problem.
    ///
    /// `roster` is what this machine can run a Drone as, resolved by
    /// [`crate::model_choices`] before this is called. A workflow step naming
    /// something outside it is refused here — before the port and before the
    /// runtime file — for the reason every other refusal in this function is:
    /// the alternative is finding out with a Drone already on a worktree.
    pub fn at(root: &Path, roster: &Roster) -> Result<Setup, SetupRefused> {
        let manifest_path = root.join(MANIFEST);
        let manifest = Manifest::load(&manifest_path).map_err(|why| match &why {
            // Absent is its own answer. The fix is "this directory is not a
            // repository Armada has been set up for", which is a different act
            // from correcting a file that is there and wrong.
            LoadError::Unreadable { cause, .. } if cause.kind() == std::io::ErrorKind::NotFound => {
                SetupRefused::NoManifest {
                    path: manifest_path.clone(),
                }
            }
            _ => SetupRefused::ManifestRefused(why),
        })?;

        let mut workflows: BTreeMap<core_model::WorkflowId, ResolvedWorkflow> = BTreeMap::new();
        let mut paths: BTreeMap<core_model::WorkflowId, PathBuf> = BTreeMap::new();
        for workflow_path in definitions(root)? {
            let def =
                WorkflowDef::load(&workflow_path, roster).map_err(SetupRefused::WorkflowRefused)?;
            let resolved = ResolvedWorkflow::resolve(&def, &manifest)
                .map_err(SetupRefused::ChecksNotDeclared)?;
            let id = resolved.id().clone();
            if let Some(first) = paths.get(&id) {
                return Err(SetupRefused::DuplicateWorkflowId {
                    id: id.as_str().to_string(),
                    first: first.clone(),
                    second: workflow_path,
                });
            }
            paths.insert(id.clone(), workflow_path);
            workflows.insert(id, resolved);
        }

        Ok(Setup {
            root: root.to_path_buf(),
            manifest,
            workflows,
        })
    }

    /// The repository this was read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Every workflow this repository declares, keyed by its `workflow_id`.
    pub fn workflows(&self) -> &BTreeMap<core_model::WorkflowId, ResolvedWorkflow> {
        &self.workflows
    }

    /// The two halves, for a `Fittings` that wants both by value.
    pub fn into_parts(self) -> (Manifest, BTreeMap<core_model::WorkflowId, ResolvedWorkflow>) {
        (self.manifest, self.workflows)
    }
}

/// Every definition in `.armada/workflows/`, or why there is not one.
///
/// **At least one, refused otherwise.** A directory is a rule until it holds
/// nothing, and an empty one is a repository not yet set up rather than a
/// repository with no workflows.
fn definitions(root: &Path) -> Result<Vec<PathBuf>, SetupRefused> {
    let dir = root.join(WORKFLOWS);
    let entries = std::fs::read_dir(&dir).map_err(|cause| {
        if cause.kind() == std::io::ErrorKind::NotFound {
            SetupRefused::NoWorkflowDirectory { path: dir.clone() }
        } else {
            SetupRefused::WorkflowsUnreadable {
                path: dir.clone(),
                cause,
            }
        }
    })?;

    let mut found: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let carries = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| DEFINITION_EXTS.contains(&ext));
        if carries && path.is_file() {
            found.push(path);
        }
    }
    // Read order is the filesystem's and is not stable across machines. Sorted,
    // so a refusal that names one of these does so in an order somebody can
    // compare, and so the duplicate-id refusal above always names the first
    // occurrence by that same order.
    found.sort();

    if found.is_empty() {
        return Err(SetupRefused::NoWorkflow { path: dir });
    }
    Ok(found)
}

/// Why a repository's setup could not be read.
///
/// Eight variants because a person has eight different things to do about them,
/// and each names the file it is about. `source` is deliberately absent: every
/// variant renders its own detail below, and returning the inner error as a
/// cause would print the same faults a second time in a different shape.
#[derive(Debug)]
pub enum SetupRefused {
    /// There is no `armada.yml` here at all.
    NoManifest { path: PathBuf },
    /// There is one and Armada will not have it.
    ManifestRefused(LoadError),
    /// No `.armada/workflows/` beside the Manifest.
    NoWorkflowDirectory { path: PathBuf },
    /// It is there and could not be listed.
    WorkflowsUnreadable {
        path: PathBuf,
        cause: std::io::Error,
    },
    /// It is there and holds no definition.
    NoWorkflow { path: PathBuf },
    /// Two definitions name the same `workflow_id`. Naming both paths rather
    /// than picking one — a Fleet that chose silently would be deciding on
    /// behalf of whoever wrote the second file.
    DuplicateWorkflowId {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
    /// The definition is there and Armada will not have it.
    WorkflowRefused(LoadError),
    /// The two files disagree: a step names a Check the Manifest has not
    /// declared. **The one cross-file fault**, and the reason it is answered at
    /// start rather than at the step that needed the name.
    ChecksNotDeclared(ResolveError),
}

impl SetupRefused {
    /// The file or directory the refusal is about.
    pub fn path(&self) -> &Path {
        match self {
            SetupRefused::NoManifest { path }
            | SetupRefused::NoWorkflowDirectory { path }
            | SetupRefused::WorkflowsUnreadable { path, .. }
            | SetupRefused::NoWorkflow { path } => path,
            // The first occurrence, by the sorted order `definitions` reads
            // them in — the file a person would fix, since it was already
            // there when the second one was added.
            SetupRefused::DuplicateWorkflowId { first, .. } => first,
            SetupRefused::ManifestRefused(why) | SetupRefused::WorkflowRefused(why) => why.path(),
            SetupRefused::ChecksNotDeclared(ResolveError::ChecksNotDeclared {
                workflow, ..
            }) => workflow,
        }
    }
}

impl fmt::Display for SetupRefused {
    /// **Multi-line, deliberately.** This is what a person starting Fleet by
    /// hand reads, and a set of refusals folded onto one line with semicolons
    /// sends them back to the file to work out which key each one meant.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetupRefused::NoManifest { path } => write!(
                f,
                "there is no {} at {} — a repository carries its own setup, \
                 and Fleet is pointed at a repository",
                MANIFEST,
                path.parent().unwrap_or(path).display()
            ),
            SetupRefused::NoWorkflowDirectory { path } => write!(
                f,
                "there is no {} — a repository's workflows live beside its {}",
                path.display(),
                MANIFEST
            ),
            SetupRefused::WorkflowsUnreadable { path, cause } => {
                write!(f, "{} could not be listed: {cause}", path.display())
            }
            SetupRefused::NoWorkflow { path } => write!(
                f,
                "{} holds no workflow definition — one is expected, ending {}",
                path.display(),
                Listed(DEFINITION_EXTS)
            ),
            SetupRefused::DuplicateWorkflowId { id, first, second } => write!(
                f,
                "workflow_id `{id}` is declared twice, and Fleet does not pick between them:\n  \
                 {}\n  {}",
                first.display(),
                second.display()
            ),
            SetupRefused::ManifestRefused(why) | SetupRefused::WorkflowRefused(why) => {
                loudly(f, why)
            }
            SetupRefused::ChecksNotDeclared(ResolveError::ChecksNotDeclared {
                workflow,
                manifest,
                unknown,
            }) => {
                write!(
                    f,
                    "{} names {} Check(s) {} does not declare",
                    workflow.display(),
                    unknown.len(),
                    manifest.display()
                )?;
                for miss in unknown {
                    write!(
                        f,
                        "\n  step `{}` needs `{}`",
                        miss.step.as_str(),
                        miss.check
                    )?;
                    if miss.is_a_command {
                        write!(f, ", which is declared as a Command, not a Check")?;
                    } else {
                        let names: Vec<&str> = miss.declared.iter().map(String::as_str).collect();
                        write!(f, ", and the declared Checks are {}", Listed(&names))?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for SetupRefused {}

/// A load failure, with one line per fault.
///
/// `LoadError`'s own `Display` joins its refusals with semicolons, which is
/// right for a wire message and wrong for a terminal. The key and the fault are
/// both public fields, so this reads them rather than reformatting a sentence.
fn loudly(f: &mut fmt::Formatter<'_>, why: &LoadError) -> fmt::Result {
    let faults = why.refusals();
    if faults.is_empty() {
        // Unreadable, or not YAML at all. `LoadError` already names the file
        // and carries the parser's line and column.
        return write!(f, "{why}");
    }
    write!(
        f,
        "{} was refused, {} fault(s)",
        why.path().display(),
        faults.len()
    )?;
    for fault in faults {
        write!(f, "\n  `{}` {}", fault.key, fault.fault)?;
    }
    Ok(())
}

/// A comma-separated list, for a message that names what was expected.
struct Listed<'a, T>(&'a [T]);

impl<T: fmt::Display> fmt::Display for Listed<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (n, item) in self.0.iter().enumerate() {
            if n > 0 {
                write!(f, ", ")?;
            }
            write!(f, "`{item}`")?;
        }
        Ok(())
    }
}
