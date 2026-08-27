//! A step's evidence scope: the policy a definition carries, and what the
//! Drone declares against it.
//!
//! # The policy cannot carry the paths, and that is a missing field
//!
//! [`EvidenceScope`] has no `context_paths` field and no method returning one.
//! At definition time nobody knows them — that is what `context_source:
//! drone_declared` means — so a definition that tried to author them has no
//! field to put them in, rather than a validator that rejects them afterwards.
//! The paths arrive as [`DeclaredPaths`], from the Drone, and the two are
//! joined by `verification`, which is the one place that compares them against
//! what actually changed.
//!
//! # Why these are not `WriteTargets`
//!
//! `WriteTargets` is the **Job's** declared write scope and binds nothing —
//! its own comment says so. This is one **step's**, and where
//! `scope_diff_check` is on it binds that step's footprint. The step's is the
//! Job's narrowed to one step, which is the granularity at which "did step two's
//! work during step one" is even a question. Two newtypes so neither can be
//! passed where the other is expected.

use alloc::vec::Vec;

use crate::job::ids::RepoPath;

/// Where a step's `context_paths` came from.
///
/// **An audit trail for trust level.** Paths a Manifest supplied and paths a
/// Drone chose for itself are not equally trustworthy, and the resolved object
/// must not lose which it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextSource {
    ManifestDefault,
    DroneDeclared,
    Hybrid,
}

impl ContextSource {
    pub const ALL: &'static [ContextSource] = &[
        ContextSource::ManifestDefault,
        ContextSource::DroneDeclared,
        ContextSource::Hybrid,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            ContextSource::ManifestDefault => "manifest_default",
            ContextSource::DroneDeclared => "drone_declared",
            ContextSource::Hybrid => "hybrid",
        }
    }

    pub fn from_wire(value: &str) -> Option<ContextSource> {
        ContextSource::ALL
            .iter()
            .copied()
            .find(|source| source.as_wire() == value)
    }

    /// Whether the Drone is the one who supplies the paths. False only for
    /// `manifest_default`, which no Manifest key sets yet — see this module's
    /// note in `docs/concepts/workflow.md`.
    pub fn asks_the_drone(&self) -> bool {
        matches!(self, ContextSource::DroneDeclared | ContextSource::Hybrid)
    }
}

/// When the Drone declares its `context_paths`. **One variant, and the
/// registry names one value.**
///
/// Absence is the other case and is spelled `None` rather than a second
/// variant: evidence-submission time is the default behaviour, described in
/// prose, and the enum member naming it is written down nowhere. Inventing a
/// spelling here would put a word in the schema that no file uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarePlanAt {
    /// Declared before the work, which is what makes a **live** drift check
    /// possible: there is a plan to compare against while the step runs.
    StepStart,
}

impl DeclarePlanAt {
    pub fn as_wire(&self) -> &'static str {
        match self {
            DeclarePlanAt::StepStart => "step_start",
        }
    }

    pub fn from_wire(value: &str) -> Option<DeclarePlanAt> {
        match value {
            "step_start" => Some(DeclarePlanAt::StepStart),
            _ => None,
        }
    }
}

/// The policy half of a step's evidence scope, as the definition declares it.
///
/// **No `context_paths`.** See this module's comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceScope {
    context_source: ContextSource,
    exclude_paths: Vec<RepoPath>,
    scope_diff_check: bool,
    declare_plan_at: Option<DeclarePlanAt>,
}

impl EvidenceScope {
    /// Build one from a definition already validated, or from a stored row.
    pub fn declared(
        context_source: ContextSource,
        exclude_paths: Vec<RepoPath>,
        scope_diff_check: bool,
        declare_plan_at: Option<DeclarePlanAt>,
    ) -> EvidenceScope {
        EvidenceScope {
            context_source,
            exclude_paths,
            scope_diff_check,
            declare_plan_at,
        }
    }

    pub fn context_source(&self) -> ContextSource {
        self.context_source
    }

    /// The denylist. **Applied after `context_paths` resolves**, so it wins
    /// over anything the Drone declared.
    pub fn exclude_paths(&self) -> &[RepoPath] {
        &self.exclude_paths
    }

    /// Whether the declaration is checked against the real diff footprint
    /// before the Judge runs.
    pub fn scope_diff_check(&self) -> bool {
        self.scope_diff_check
    }

    pub fn declare_plan_at(&self) -> Option<DeclarePlanAt> {
        self.declare_plan_at
    }

    /// Whether Fleet compares live edits against the plan **throughout** the
    /// step rather than only at the end.
    ///
    /// Both halves are required: a plan declared at step start with no
    /// footprint check is a plan nothing is measured against, and a footprint
    /// check with no early plan has nothing to compare until the end.
    pub fn watches_live_edits(&self) -> bool {
        self.scope_diff_check && self.declare_plan_at == Some(DeclarePlanAt::StepStart)
    }

    /// Whether a declaration is expected on this step at all. The cold switch:
    /// a step with no evidence scope behaves exactly as it did before one
    /// existed.
    pub fn wants_a_declaration(&self) -> bool {
        self.context_source.asks_the_drone()
    }
}

/// The paths a Drone declared for one step. **A claim, like any other.**
///
/// Legitimately empty: a step that will change nothing has declared that, the
/// same way `WriteTargets::nothing` does, and the two zero answers are told
/// apart by whether a declaration arrived rather than by its length.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeclaredPaths(Vec<RepoPath>);

impl DeclaredPaths {
    pub fn of(paths: Vec<RepoPath>) -> DeclaredPaths {
        DeclaredPaths(paths)
    }

    /// Declared to touch nothing, which is not the same as having declared
    /// nothing at all.
    pub fn nothing() -> DeclaredPaths {
        DeclaredPaths(Vec::new())
    }

    pub fn paths(&self) -> &[RepoPath] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `path` falls under any declared path.
    ///
    /// Prefix at a segment boundary, so `src/lib` does not cover `src/library`
    /// — the failure a bare `starts_with` produces, and the one that would let
    /// a neighbouring module read as in scope.
    pub fn covers(&self, path: &str) -> bool {
        self.0.iter().any(|declared| under(declared.as_str(), path))
    }
}

/// Whether `path` is `parent`, or lies beneath it.
///
/// **A segment boundary, never a bare prefix.** Repository-relative and
/// separator-normalised by the caller; this compares text and reaches no
/// filesystem, which is what keeps the whole check free.
pub fn under(parent: &str, path: &str) -> bool {
    let parent = parent.trim_end_matches('/');
    if parent.is_empty() {
        return true;
    }
    match path.strip_prefix(parent) {
        Some("") => true,
        Some(rest) => rest.starts_with('/'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sibling_with_a_shared_prefix_is_not_under_it() {
        assert!(under("src/lib", "src/lib/mod.rs"));
        assert!(under("src/lib", "src/lib"));
        assert!(!under("src/lib", "src/library/mod.rs"));
    }

    #[test]
    fn the_repository_root_covers_everything() {
        assert!(under("", "anything/at/all.rs"));
        assert!(under("/", "anything/at/all.rs"));
    }

    #[test]
    fn a_declaration_of_nothing_covers_nothing() {
        let nothing = DeclaredPaths::nothing();
        assert!(nothing.is_empty());
        assert!(!nothing.covers("src/lib.rs"));
    }

    #[test]
    fn a_live_watch_needs_both_halves() {
        let both = EvidenceScope::declared(
            ContextSource::DroneDeclared,
            Vec::new(),
            true,
            Some(DeclarePlanAt::StepStart),
        );
        let late = EvidenceScope::declared(ContextSource::DroneDeclared, Vec::new(), true, None);
        let unchecked = EvidenceScope::declared(
            ContextSource::DroneDeclared,
            Vec::new(),
            false,
            Some(DeclarePlanAt::StepStart),
        );
        assert!(both.watches_live_edits());
        assert!(!late.watches_live_edits());
        assert!(!unchecked.watches_live_edits());
    }
}
