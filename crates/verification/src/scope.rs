//! Binding evidence to what the step actually touched. **A path comparison,
//! and nothing else.**
//!
//! # No model call, deliberately
//!
//! Every question here is answered by a list of changed files and a list of
//! declared ones. The Judge is cold because it costs money; a scope check that
//! spent a call would fire on every step and buy nothing `git diff --name-only`
//! answers for free.
//!
//! # The declaration is a claim, and this is what checks it
//!
//! `context_paths` is Drone-declared, so the resolved object cannot be built by
//! taking the Drone's word: [`InScope`] has one constructor, it takes the real
//! footprint alongside the declaration, and it returns a `Result`. Holding one
//! is the fact that the two agreed.
//!
//! # The direction is one-way, and that is not an oversight
//!
//! A **changed** file that is not declared is drift. A **declared** path that
//! did not change is not: `context_paths` is also the read allowlist — a
//! sibling module, the interface being conformed to — and a Drone naming
//! reading context it did not need has done nothing wrong.

use core_model::{under, DeclaredPaths, EvidenceScope, RepoPath};

/// A footprint that agreed with what was declared.
///
/// **The resolved evidence scope**: the policy the step carried, plus the
/// `context_paths` the Drone supplied, plus the proof that nothing changed
/// outside them. There is no constructor that skips the comparison and no
/// field that can be set afterwards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InScope {
    scope: EvidenceScope,
    context_paths: DeclaredPaths,
}

/// What the declaration and the worktree disagreed about.
///
/// **Never empty**, in either list, in the variant that carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutsideScope {
    /// Files changed that no declared path covers. **This is the recycling the
    /// title names**: work whose footprint is outside the step's scope is work
    /// that belongs to another step.
    Undeclared { changed: Vec<RepoPath> },
    /// Declared paths that the step's own `exclude_paths` denies. The denylist
    /// resolves last, so it wins over anything the Drone declared.
    Excluded { declared: Vec<RepoPath> },
    /// The step wants a declaration and none arrived.
    ///
    /// Its own variant because nothing was compared: the Drone did not claim
    /// too much, it claimed nothing, and the two need different things said to
    /// it.
    NothingDeclared,
}

impl core::fmt::Display for OutsideScope {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OutsideScope::Undeclared { changed } => write!(
                f,
                "the step changed {} outside what it declared",
                Listed(changed)
            ),
            OutsideScope::Excluded { declared } => write!(
                f,
                "the step declared {}, which its evidence scope excludes",
                Listed(declared)
            ),
            OutsideScope::NothingDeclared => f.write_str(
                "the step asks the Drone which paths its work is in and none were declared",
            ),
        }
    }
}

impl std::error::Error for OutsideScope {}

/// The paths, comma-separated, so no message ends in a dangling list.
struct Listed<'a>(&'a [RepoPath]);

impl core::fmt::Display for Listed<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (n, path) in self.0.iter().enumerate() {
            if n > 0 {
                f.write_str(", ")?;
            }
            write!(f, "`{}`", path.as_str())?;
        }
        Ok(())
    }
}

impl InScope {
    /// Resolve the step's policy against what the Drone declared and what the
    /// worktree holds.
    ///
    /// `changed` is **Fleet's own reading** of the worktree, never a list the
    /// Drone reported — the same rule `diff_nonempty` follows, and for the same
    /// reason: a gating fact that arrives from the thing being gated is not a
    /// fact.
    pub fn resolved(
        scope: &EvidenceScope,
        declared: Option<&DeclaredPaths>,
        changed: &[String],
    ) -> Result<InScope, OutsideScope> {
        let Some(declared) = declared else {
            return Err(OutsideScope::NothingDeclared);
        };
        let excluded: Vec<RepoPath> = declared
            .paths()
            .iter()
            .filter(|path| {
                scope
                    .exclude_paths()
                    .iter()
                    .any(|denied| under(denied.as_str(), path.as_str()))
            })
            .cloned()
            .collect();
        if !excluded.is_empty() {
            return Err(OutsideScope::Excluded { declared: excluded });
        }
        // The footprint check is the step's own switch. A step that declares
        // paths and does not ask for them to be checked has said where to look
        // and not claimed it stayed there.
        if scope.scope_diff_check() {
            let drifted = drifted(declared, changed);
            if !drifted.is_empty() {
                return Err(OutsideScope::Undeclared { changed: drifted });
            }
        }
        Ok(InScope {
            scope: scope.clone(),
            context_paths: declared.clone(),
        })
    }

    /// The policy this resolved against.
    pub fn policy(&self) -> &EvidenceScope {
        &self.scope
    }

    /// What else may be opened beyond the diff. **Required on the resolved
    /// object**, which is why this returns a slice rather than an `Option`.
    pub fn context_paths(&self) -> &[RepoPath] {
        self.context_paths.paths()
    }
}

/// Every changed file no declared path covers, in the order they were read.
///
/// The whole of the live drift check as well as the gate's: the two differ in
/// **when** they call this and what they do with the answer, never in what
/// counts as drift.
pub fn drifted(declared: &DeclaredPaths, changed: &[String]) -> Vec<RepoPath> {
    changed
        .iter()
        .filter(|path| !declared.covers(path))
        .map(RepoPath::new)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_model::ContextSource;

    fn watching(exclude: Vec<&str>, check: bool) -> EvidenceScope {
        EvidenceScope::declared(
            ContextSource::DroneDeclared,
            exclude.into_iter().map(RepoPath::new).collect(),
            check,
            None,
        )
    }

    fn declared(paths: &[&str]) -> DeclaredPaths {
        DeclaredPaths::of(paths.iter().copied().map(RepoPath::new).collect())
    }

    #[test]
    fn a_footprint_inside_the_declaration_resolves() {
        let scope = watching(Vec::new(), true);
        let resolved = InScope::resolved(
            &scope,
            Some(&declared(&["docs/", "crates/config"])),
            &["docs/a.md".into(), "crates/config/src/lib.rs".into()],
        )
        .unwrap();
        assert_eq!(resolved.context_paths().len(), 2);
    }

    #[test]
    fn a_file_changed_outside_the_declaration_is_named() {
        let scope = watching(Vec::new(), true);
        let outside = InScope::resolved(
            &scope,
            Some(&declared(&["docs/"])),
            &["docs/a.md".into(), "crates/fleet/src/gate.rs".into()],
        )
        .unwrap_err();
        assert_eq!(
            outside,
            OutsideScope::Undeclared {
                changed: vec![RepoPath::new("crates/fleet/src/gate.rs")]
            }
        );
    }

    #[test]
    fn a_declared_path_that_did_not_change_is_read_context_and_not_drift() {
        let scope = watching(Vec::new(), true);
        assert!(InScope::resolved(
            &scope,
            Some(&declared(&["docs/", "crates/config"])),
            &["docs/a.md".into()],
        )
        .is_ok());
    }

    #[test]
    fn the_denylist_wins_over_the_declaration() {
        let scope = watching(vec!["secrets"], true);
        let outside = InScope::resolved(
            &scope,
            Some(&declared(&["secrets/keys.toml"])),
            &["secrets/keys.toml".into()],
        )
        .unwrap_err();
        assert_eq!(
            outside,
            OutsideScope::Excluded {
                declared: vec![RepoPath::new("secrets/keys.toml")]
            }
        );
    }

    #[test]
    fn a_step_that_does_not_ask_for_the_check_does_not_get_one() {
        let scope = watching(Vec::new(), false);
        assert!(InScope::resolved(
            &scope,
            Some(&declared(&["docs/"])),
            &["crates/fleet/src/gate.rs".into()],
        )
        .is_ok());
    }

    #[test]
    fn a_missing_declaration_is_not_an_empty_one() {
        let scope = watching(Vec::new(), true);
        assert_eq!(
            InScope::resolved(&scope, None, &[]).unwrap_err(),
            OutsideScope::NothingDeclared
        );
        assert!(InScope::resolved(&scope, Some(&DeclaredPaths::nothing()), &[]).is_ok());
    }

    #[test]
    fn declaring_nothing_and_changing_something_is_drift() {
        let scope = watching(Vec::new(), true);
        assert!(matches!(
            InScope::resolved(&scope, Some(&DeclaredPaths::nothing()), &["a.rs".into()]),
            Err(OutsideScope::Undeclared { .. })
        ));
    }
}
