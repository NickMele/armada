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
//! **A repository's own definitions are strict, and the rest are left out.**
//! The owner's decision: one from Kit or Armada that will not parse, or will not
//! resolve against this repository, is set aside and named, and the next place
//! down answers for its id. One a more specific place replaced is never
//! resolved, so its Checks have no Job behind them.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use core_model::{WorkflowId, WorkflowSource};

use crate::error::{LoadError, ResolveError};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::roster::Roster;
use crate::workflow::WorkflowDef;

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

/// Every definition in hand, grouped by id, and the ones set aside unparsed.
#[derive(Debug)]
pub struct Catalogue {
    held: BTreeMap<WorkflowId, Vec<(WorkflowDef, WorkflowSource)>>,
    left_out: Vec<LeftOut>,
}

impl Catalogue {
    /// Parse every definition, keeping each beside the others for its id.
    ///
    /// **The order they arrive in decides nothing**, and the source decides
    /// everything. Two definitions sharing an id *and* a place are refused,
    /// naming both: a catalogue that picked would be choosing on behalf of
    /// whoever wrote the second file. Across places, it is an override.
    pub fn of(
        written: impl IntoIterator<Item = Written>,
        roster: &Roster,
    ) -> Result<Catalogue, CatalogueRefused> {
        let mut held: BTreeMap<WorkflowId, Vec<(WorkflowDef, WorkflowSource)>> = BTreeMap::new();
        let mut seen: BTreeMap<(WorkflowSource, WorkflowId), PathBuf> = BTreeMap::new();
        let mut left_out = Vec::new();
        for one in written {
            let def = match WorkflowDef::parse(&one.path, &one.text, roster) {
                Ok(def) => def,
                Err(why) if one.source == WorkflowSource::Repository => {
                    return Err(CatalogueRefused::Refused(why))
                }
                Err(why) => {
                    left_out.push(LeftOut {
                        id: None,
                        source: one.source,
                        path: one.path,
                        why: WhyLeftOut::Unparsed(why),
                    });
                    continue;
                }
            };
            let key = (one.source, def.id().clone());
            if let Some(first) = seen.get(&key) {
                return Err(CatalogueRefused::DuplicateWorkflowId {
                    id: def.id().as_str().to_string(),
                    first: first.clone(),
                    second: one.path,
                });
            }
            seen.insert(key, one.path);
            held.entry(def.id().clone())
                .or_default()
                .push((def, one.source));
        }
        Ok(Catalogue { held, left_out })
    }

    /// Resolve the most specific definition for each id against the
    /// repository's Manifest, stepping down a place past any that will not.
    ///
    /// **A repository's own that will not resolve refuses the whole set**, as
    /// it always has: the repository declared it, and a Fleet quietly running
    /// Armada's in its place would be running something nobody there chose.
    pub fn resolve(self, manifest: &Manifest) -> Result<ResolvedCatalogue, ResolveError> {
        let mut workflows = BTreeMap::new();
        let mut left_out = self.left_out;
        for (id, mut candidates) in self.held {
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            for (def, source) in candidates {
                match ResolvedWorkflow::resolve(&def, manifest) {
                    Ok(resolved) => {
                        workflows.insert(id.clone(), resolved.read_from(source));
                        break;
                    }
                    Err(why) if source == WorkflowSource::Repository => return Err(why),
                    Err(why) => left_out.push(LeftOut {
                        id: Some(id.clone()),
                        source,
                        path: def.path().to_path_buf(),
                        why: WhyLeftOut::Unresolved(why),
                    }),
                }
            }
        }
        Ok(ResolvedCatalogue {
            workflows,
            left_out,
        })
    }
}

/// The workflows a repository runs, and every definition set aside on the way.
#[derive(Debug)]
pub struct ResolvedCatalogue {
    workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    left_out: Vec<LeftOut>,
}

impl ResolvedCatalogue {
    /// One per id, each knowing its source.
    pub fn workflows(&self) -> &BTreeMap<WorkflowId, ResolvedWorkflow> {
        &self.workflows
    }

    /// **Kept, with the reason, so a person picking a workflow can be told why
    /// one they wrote is not there** — at start today, and later where they
    /// pick.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }

    pub fn into_parts(self) -> (BTreeMap<WorkflowId, ResolvedWorkflow>, Vec<LeftOut>) {
        (self.workflows, self.left_out)
    }
}

/// A definition from Kit or Armada this repository runs without.
#[derive(Debug)]
pub struct LeftOut {
    /// `None` where the file did not parse far enough to say.
    id: Option<WorkflowId>,
    source: WorkflowSource,
    path: PathBuf,
    why: WhyLeftOut,
}

/// Why a definition was left out.
#[derive(Debug)]
pub enum WhyLeftOut {
    /// It will not parse.
    Unparsed(LoadError),
    /// It parses, and names what this repository does not declare, or says
    /// something else about what it does.
    Unresolved(ResolveError),
}

impl LeftOut {
    pub fn id(&self) -> Option<&WorkflowId> {
        self.id.as_ref()
    }

    pub fn source(&self) -> WorkflowSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn why(&self) -> &WhyLeftOut {
        &self.why
    }
}

impl fmt::Display for LeftOut {
    /// What Fleet prints at start: which definition, from where, and why.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.id {
            Some(id) => write!(f, "workflow `{}` {}", id.as_str(), self.source)?,
            None => write!(f, "a definition {}", self.source)?,
        }
        write!(f, " is left out, at {} — ", self.path.display())?;
        match &self.why {
            WhyLeftOut::Unparsed(why) => write!(f, "{why}"),
            WhyLeftOut::Unresolved(why) => write!(f, "{why}"),
        }
    }
}

/// Why a set of definitions could not become a catalogue.
#[derive(Debug)]
pub enum CatalogueRefused {
    /// One of the repository's own definitions will not parse.
    Refused(LoadError),
    /// Two definitions in one place name the same `workflow_id`.
    DuplicateWorkflowId {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
}
