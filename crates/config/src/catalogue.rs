//! Where a workflow comes from, and which one a repository runs.
//!
//! **Three sources, merged by `workflow_id`, the most specific winning**: the
//! set Armada carries, then Kit's Workflows, then the repository's own. So a
//! repository nobody set up dispatches on the carried set, and one file in
//! either later place replaces a carried definition by id. #425.
//!
//! **Text in, not directories.** Reading a directory is the composition
//! root's; this is a function over definitions in hand, which is what lets an
//! acceptance test drive the merge without touching a file.
//!
//! **Every definition is parsed, and only the winners are resolved.** A file
//! that will not parse is wrong wherever it sits. One another source replaced
//! never runs, so whether its Checks resolve here has no Job behind it.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use core_model::WorkflowId;

use crate::error::{LoadError, ResolveError};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::roster::Roster;
use crate::workflow::WorkflowDef;

/// Which of the three places a workflow was read from.
///
/// **Declared least specific first, and the order is the rule**: `Ord` is
/// what [`Catalogue::of`] compares, so a fourth source is placed by where its
/// variant is written rather than by a precedence table beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkflowSource {
    /// Compiled into the binary. What a repository with no workflows of its
    /// own runs on.
    Armada,
    /// Kit's Workflows, which a person keeps for every repository they work in.
    Kit,
    /// The repository's own `.armada/workflows/`.
    Repository,
}

impl WorkflowSource {
    /// One word, for a machine to read.
    pub fn as_str(self) -> &'static str {
        match self {
            WorkflowSource::Armada => "armada",
            WorkflowSource::Kit => "kit",
            WorkflowSource::Repository => "repository",
        }
    }
}

impl fmt::Display for WorkflowSource {
    /// Where it came from, as a person would say it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WorkflowSource::Armada => "carried by Armada",
            WorkflowSource::Kit => "from Kit",
            WorkflowSource::Repository => "from the repository",
        })
    }
}

/// Where a carried definition says it was read from. **Not a file** — a name
/// for a refusal to cite, bracketed so it cannot be mistaken for a path
/// relative to a repository.
pub const CARRIED_AT: &str = "<carried by Armada>";

/// The set Armada carries: this repository's own eight, compiled in.
///
/// **The files and not copies of them**, so an edit to one is an edit to what
/// every repository runs, and `tests/carried.rs` fails if a ninth is added
/// under `.armada/workflows/` without being listed here.
const CARRIED: [(&str, &str); 8] = [
    (
        "bug.json",
        include_str!("../../../.armada/workflows/bug.json"),
    ),
    (
        "code-review.json",
        include_str!("../../../.armada/workflows/code-review.json"),
    ),
    (
        "design-plan.json",
        include_str!("../../../.armada/workflows/design-plan.json"),
    ),
    (
        "epic.json",
        include_str!("../../../.armada/workflows/epic.json"),
    ),
    (
        "feature.json",
        include_str!("../../../.armada/workflows/feature.json"),
    ),
    (
        "prototype.json",
        include_str!("../../../.armada/workflows/prototype.json"),
    ),
    (
        "refactor.json",
        include_str!("../../../.armada/workflows/refactor.json"),
    ),
    (
        "revert.json",
        include_str!("../../../.armada/workflows/revert.json"),
    ),
];

/// One definition, as text, and the place it was read from.
///
/// **No public way to say a definition is Armada's.** [`carried`] is the only
/// source of those, so a file on a machine cannot pass itself off as the set
/// the binary carries.
#[derive(Debug, Clone)]
pub struct Written {
    source: WorkflowSource,
    path: PathBuf,
    text: String,
}

impl Written {
    /// A definition read from Kit's Workflows.
    pub fn in_kit(path: PathBuf, text: String) -> Written {
        Written {
            source: WorkflowSource::Kit,
            path,
            text,
        }
    }

    /// A definition read from the repository's own `.armada/workflows/`.
    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written {
            source: WorkflowSource::Repository,
            path,
            text,
        }
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Every definition Armada carries, in file-name order.
pub fn carried() -> Vec<Written> {
    CARRIED
        .iter()
        .map(|(file, text)| Written {
            source: WorkflowSource::Armada,
            path: Path::new(CARRIED_AT).join(file),
            text: (*text).to_string(),
        })
        .collect()
}

/// The definitions a repository runs, one per id, each knowing its source.
#[derive(Debug, Clone)]
pub struct Catalogue {
    held: BTreeMap<WorkflowId, (WorkflowDef, WorkflowSource)>,
}

impl Catalogue {
    /// Parse every definition and keep the most specific one for each id.
    ///
    /// **The order they arrive in decides nothing**, and the source decides
    /// everything. Two definitions sharing an id *and* a source are refused,
    /// naming both: within one place, a catalogue that picked would be choosing
    /// on behalf of whoever wrote the second file. Across places, it is the
    /// ordinary case once a person overrides anything.
    pub fn of(
        written: impl IntoIterator<Item = Written>,
        roster: &Roster,
    ) -> Result<Catalogue, CatalogueRefused> {
        let mut held: BTreeMap<WorkflowId, (WorkflowDef, WorkflowSource)> = BTreeMap::new();
        let mut seen: BTreeMap<(WorkflowSource, WorkflowId), PathBuf> = BTreeMap::new();
        for one in written {
            let def = WorkflowDef::parse(&one.path, &one.text, roster)
                .map_err(CatalogueRefused::Refused)?;
            let key = (one.source, def.id().clone());
            if let Some(first) = seen.get(&key) {
                return Err(CatalogueRefused::DuplicateWorkflowId {
                    id: def.id().as_str().to_string(),
                    first: first.clone(),
                    second: one.path,
                });
            }
            seen.insert(key, one.path);
            let replaces = match held.get(def.id()) {
                Some((_, standing)) => one.source > *standing,
                None => true,
            };
            if replaces {
                held.insert(def.id().clone(), (def, one.source));
            }
        }
        Ok(Catalogue { held })
    }

    /// Which source each id's definition came from.
    pub fn sources(&self) -> BTreeMap<&WorkflowId, WorkflowSource> {
        self.held
            .iter()
            .map(|(id, (_, source))| (id, *source))
            .collect()
    }

    /// Resolve every held definition against the repository's Manifest.
    ///
    /// The first definition that does not resolve refuses the whole set, as a
    /// repository's own always has: a Fleet serving the workflows that happened
    /// to resolve would be a picker quietly shorter than the files.
    pub fn resolve(
        &self,
        manifest: &Manifest,
    ) -> Result<BTreeMap<WorkflowId, ResolvedWorkflow>, ResolveError> {
        self.held
            .iter()
            .map(|(id, (def, source))| {
                let resolved = ResolvedWorkflow::resolve(def, manifest)?.read_from(*source);
                Ok((id.clone(), resolved))
            })
            .collect()
    }
}

/// Why a set of definitions could not become a catalogue.
#[derive(Debug)]
pub enum CatalogueRefused {
    /// One definition will not parse. Its path says which of the three places
    /// it is in.
    Refused(LoadError),
    /// Two definitions in one place name the same `workflow_id`.
    DuplicateWorkflowId {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
}
