//! A repository's own setup, found from its root.
//!
//! **A repository carries its setup, and Fleet is pointed at the repository.**
//! `armada.yml` at the root; workflows from what Armada carries, Kit's, and
//! `.armada/workflows/` beside it — see `config::Catalogue` for which wins.
//! There is no `--manifest` flag and no `--workflow` flag, and that
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
//! **Holding one of these is proof the files agree.** [`Setup`] has no
//! constructor but [`Setup::at`] and its fields are private, so every
//! [`ResolvedWorkflow`] inside it can only have been built against the
//! [`Manifest`] beside it. Every Check any workflow's steps name was declared
//! by that Manifest at the moment the daemon started — checked once, before a
//! worktree exists and before a Drone is spawned.
//! **It stays proof while the file is re-read** — `#430`: a reload moves the
//! Manifest's `lifetime = "Live"` keys and nothing a workflow resolved
//! against. See `config::live`; [`Setup`] holds the one handle that can.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use config::{
    Catalogue, CatalogueRefused, LoadError, Manifest, Reloads, ResolveError, ResolvedWorkflow,
    Roster, Written,
};

/// The Manifest's name at a repository root. Not configurable: a repository
/// that could name its own Manifest is one where finding the Manifest requires
/// already having read it.
pub const MANIFEST: &str = "armada.yml";

/// Where a repository's workflow definitions live, relative to its root.
pub const WORKFLOWS: &str = ".armada/workflows";

/// Kit's home, relative to the operator's. Workflows are the first thing read
/// from it; the rest of Kit arrives in the same place under #41.
pub const KIT_HOME: &str = ".armada";

/// Where Kit's Workflows live inside Kit's home — the name a repository uses,
/// so a definition moves between the two without an edit.
pub const KIT_WORKFLOWS: &str = "workflows";

/// Kit's home under `home`, with its Workflows directory made if it was not.
///
/// **`home` is handed in**, because the composition root is the one place that
/// reads the environment. Made rather than required: a person looking for
/// where Kit's Workflows go should find the folder, not a sentence about it.
pub fn kit(home: &Path) -> std::io::Result<PathBuf> {
    let kit = home.join(KIT_HOME);
    std::fs::create_dir_all(kit.join(KIT_WORKFLOWS))?;
    Ok(kit)
}

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
    /// The handle that reads `armada.yml` again. **Read here, so the thing
    /// that can re-read the file is the thing that opened it** — see
    /// `config::live` for what a re-read may and may not move.
    reloads: Reloads,
}

impl Setup {
    /// Read `root`'s Manifest, merge what Armada carries with Kit's Workflows
    /// under `kit` and the repository's own, and resolve each against it.
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
    pub fn at(root: &Path, kit: &Path, roster: &Roster) -> Result<Setup, SetupRefused> {
        let manifest_path = root.join(MANIFEST);
        let (manifest, reloads) =
            Manifest::reloadable(&manifest_path).map_err(|why| match &why {
                // Absent is its own answer. The fix is "this directory is not a
                // repository Armada has been set up for", which is a different act
                // from correcting a file that is there and wrong.
                LoadError::Unreadable { cause, .. }
                    if cause.kind() == std::io::ErrorKind::NotFound =>
                {
                    SetupRefused::NoManifest {
                        path: manifest_path.clone(),
                    }
                }
                _ => SetupRefused::ManifestRefused(why),
            })?;

        let mut written = config::carried();
        written.extend(definitions(&kit.join(KIT_WORKFLOWS), Written::in_kit)?);
        written.extend(definitions(&root.join(WORKFLOWS), Written::in_repository)?);
        let catalogue = Catalogue::of(written, roster).map_err(|why| match why {
            CatalogueRefused::Refused(why) => SetupRefused::WorkflowRefused(why),
            CatalogueRefused::DuplicateWorkflowId { id, first, second } => {
                SetupRefused::DuplicateWorkflowId { id, first, second }
            }
        })?;
        let workflows = catalogue
            .resolve(&manifest)
            .map_err(SetupRefused::ChecksNotDeclared)?;

        Ok(Setup {
            root: root.to_path_buf(),
            manifest,
            workflows,
            reloads,
        })
    }

    /// The repository this was read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Every workflow this repository runs, keyed by its `workflow_id`, each
    /// saying which of the three places it came from.
    ///
    /// Two files in one place naming the same id is refused at start: a Fleet
    /// that picked one silently would be choosing on behalf of whoever wrote
    /// the second file. The same id in two places is an override.
    pub fn workflows(&self) -> &BTreeMap<core_model::WorkflowId, ResolvedWorkflow> {
        &self.workflows
    }

    /// The two halves a `Fittings` wants by value, and the handle that reads
    /// the Manifest again.
    ///
    /// **The reload handle comes out here and goes nowhere near Fleet.** The
    /// composition root hands the first two down and keeps the third; Fleet
    /// holds a Manifest it cannot move, which is `config::live`'s point.
    pub fn into_parts(
        self,
    ) -> (
        Manifest,
        BTreeMap<core_model::WorkflowId, ResolvedWorkflow>,
        Reloads,
    ) {
        (self.manifest, self.workflows, self.reloads)
    }
}

/// Every definition in one directory, as text and marked with its place.
///
/// **Absent or empty adds nothing, and is not refused.** Until #425 an empty
/// `.armada/workflows/` was a repository not yet set up; now a repository with
/// none of its own runs on what Armada carries, and a machine with no Kit
/// Workflows adds none.
fn definitions(
    dir: &Path,
    place: fn(PathBuf, String) -> Written,
) -> Result<Vec<Written>, SetupRefused> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(cause) => {
            return Err(SetupRefused::WorkflowsUnreadable {
                path: dir.to_path_buf(),
                cause,
            })
        }
    };

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

    found
        .into_iter()
        .map(|path| match std::fs::read_to_string(&path) {
            Ok(text) => Ok(place(path, text)),
            Err(cause) => Err(SetupRefused::WorkflowRefused(LoadError::Unreadable {
                path,
                cause,
            })),
        })
        .collect()
}

/// Why a repository's setup could not be read.
///
/// **Every fault, and never the first one.** `config` collects each refusal in
/// a document rather than stopping, and this carries the whole set through to
/// the terminal: the person reading the output is the person who wrote the
/// file, and a parser reporting one fault per run turns one edit into three.
///
/// Six variants because a person has six different things to do about them,
/// and each names the file it is about. `source` is deliberately absent: every
/// variant renders its own detail below, and returning the inner error as a
/// cause would print the same faults a second time in a different shape.
#[derive(Debug)]
pub enum SetupRefused {
    /// There is no `armada.yml` here at all.
    NoManifest { path: PathBuf },
    /// There is one and Armada will not have it.
    ManifestRefused(LoadError),
    /// A directory of workflows is there and could not be listed.
    WorkflowsUnreadable {
        path: PathBuf,
        cause: std::io::Error,
    },
    /// Two definitions in one place name the same `workflow_id`. Naming both paths rather
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
    /// declared, or names one it declares and says something else about it.
    /// **The cross-file fault**, and the reason it is answered at start rather
    /// than at the step that needed the name.
    ///
    /// The name is the first of the two shapes and is kept, because it is the
    /// one every caller and every test already says. What it carries is
    /// [`ResolveError`], which is where the shapes are told apart.
    ChecksNotDeclared(ResolveError),
}

impl SetupRefused {
    /// The file or directory the refusal is about.
    pub fn path(&self) -> &Path {
        match self {
            SetupRefused::NoManifest { path } | SetupRefused::WorkflowsUnreadable { path, .. } => {
                path
            }
            // The first occurrence, by the sorted order `definitions` reads
            // them in — the file a person would fix, since it was already
            // there when the second one was added.
            SetupRefused::DuplicateWorkflowId { first, .. } => first,
            SetupRefused::ManifestRefused(why) | SetupRefused::WorkflowRefused(why) => why.path(),
            SetupRefused::ChecksNotDeclared(
                ResolveError::ChecksNotDeclared { workflow, .. }
                | ResolveError::StepsDisagreeWithTheManifest { workflow, .. },
            ) => workflow,
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
            SetupRefused::WorkflowsUnreadable { path, cause } => {
                write!(f, "{} could not be listed: {cause}", path.display())
            }
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
            SetupRefused::ChecksNotDeclared(ResolveError::StepsDisagreeWithTheManifest {
                workflow,
                manifest,
                disagreements,
            }) => {
                write!(
                    f,
                    "{} and {} disagree in {} place(s)",
                    workflow.display(),
                    manifest.display(),
                    disagreements.len()
                )?;
                for said in disagreements {
                    write!(f, "\n  {said}")?;
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
