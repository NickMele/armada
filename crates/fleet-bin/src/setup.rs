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
//! The [`ResolvedWorkflow`] inside it can only have been built against the
//! [`Manifest`] beside it, so every Check the workflow's steps name was
//! declared by that Manifest at the moment the daemon started — checked once,
//! before a worktree exists and before a Drone is spawned.
//!
//! # Every fault, and never the first one
//!
//! `config` collects each refusal in a document rather than stopping, and
//! [`SetupRefused`] carries the whole set through to the terminal. The person
//! reading the output is the person who wrote the file, and a parser that
//! reported one fault per run would turn one edit into three.

use std::fmt;
use std::path::{Path, PathBuf};

use config::{LoadError, Manifest, ResolveError, ResolvedWorkflow, WorkflowDef};

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

/// One repository's Manifest and the workflow its steps resolved against.
#[derive(Debug)]
pub struct Setup {
    root: PathBuf,
    manifest: Manifest,
    workflow: ResolvedWorkflow,
}

impl Setup {
    /// Read `root`'s Manifest, read its one workflow, and resolve the second
    /// against the first.
    ///
    /// **`root` is the repository, not a search hint.** Nothing walks upward
    /// looking for an `armada.yml` in a parent: a daemon that quietly adopted
    /// an ancestor's Manifest would run a Job against a repository nobody
    /// pointed it at, and the failure would read as a workflow problem.
    pub fn at(root: &Path) -> Result<Setup, SetupRefused> {
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

        let workflow_path = one_definition(root)?;
        let def = WorkflowDef::load(&workflow_path).map_err(SetupRefused::WorkflowRefused)?;
        let workflow =
            ResolvedWorkflow::resolve(&def, &manifest).map_err(SetupRefused::ChecksNotDeclared)?;

        Ok(Setup {
            root: root.to_path_buf(),
            manifest,
            workflow,
        })
    }

    /// The repository this was read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn workflow(&self) -> &ResolvedWorkflow {
        &self.workflow
    }

    /// The two halves, for a `Fittings` that wants both by value.
    pub fn into_parts(self) -> (Manifest, ResolvedWorkflow) {
        (self.manifest, self.workflow)
    }
}

/// The one definition in `.armada/workflows/`, or why there is not one.
///
/// **Exactly one, refused otherwise.** `Fittings` holds a single workflow at
/// M1 — `ResolvedWorkflow` carries a name and a version and no id, and a
/// proposal names a `workflow_id`, so there is nothing to look a second one up
/// by. A daemon that picked the alphabetically first of three would be making
/// that choice silently and on no stated rule.
fn one_definition(root: &Path) -> Result<PathBuf, SetupRefused> {
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
    // so the refusal below names them in an order somebody can compare.
    found.sort();

    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(SetupRefused::NoWorkflow { path: dir }),
        _ => Err(SetupRefused::MoreThanOneWorkflow { path: dir, found }),
    }
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
    /// It holds more than one, and M1 dispatches one.
    MoreThanOneWorkflow { path: PathBuf, found: Vec<PathBuf> },
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
            | SetupRefused::NoWorkflow { path }
            | SetupRefused::MoreThanOneWorkflow { path, .. } => path,
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
            SetupRefused::MoreThanOneWorkflow { path, found } => {
                write!(
                    f,
                    "{} holds {} workflow definitions and M1 dispatches one",
                    path.display(),
                    found.len()
                )?;
                for one in found {
                    write!(f, "\n  {}", one.display())?;
                }
                Ok(())
            }
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
