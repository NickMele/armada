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

use core_model::{under, DeclaredPaths, EvidenceScope, Job, RepoPath};

use crate::forbidden::{forbidden_among, reaches, Forbidden};

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
///
/// # One variant is the Judge's and three are mechanical
///
/// `docs/concepts/judge.md` gives declared plan drift to the Judge and says it
/// does not fail the step. That is [`Undeclared`](OutsideScope::Undeclared)
/// alone — the other three are not drift, and the split is made where the gate
/// matches on this enum.
///
/// # Two of the three are the same shape and not the same thing
///
/// [`Excluded`](OutsideScope::Excluded) is a boundary a Judge may lift and
/// [`Forbidden`](OutsideScope::Forbidden) is one nothing lifts. They are
/// separate variants rather than one carrying a flag, so a caller answering
/// only the first has a match arm missing rather than a boolean it forgot to
/// read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutsideScope {
    /// Files changed that no declared path covers. **Declared plan drift**, and
    /// the Judge's: the step said where its work would be and the work went
    /// elsewhere, which is a question about whether the task required it.
    Undeclared { changed: Vec<RepoPath> },
    /// Declared paths that the step's own `exclude_paths` denies, and that no
    /// Judge has cleared. The denylist resolves last, so it wins over anything
    /// the Drone declared.
    ///
    /// **Liftable, and that is the whole of what separates it from
    /// [`Forbidden`](OutsideScope::Forbidden).** A Drone that meets this has a
    /// route left — `request_scope`, where a Judge is asked whether the paths
    /// belong to the step — and `fleet::scope` says so in the refusal rather
    /// than leaving the step to fail for want of a file.
    Excluded { declared: Vec<RepoPath> },
    /// Paths under a boundary nothing lifts: secrets, what decides which checks
    /// run, what decides how the work is judged. `crate::forbidden` is the list
    /// and carries the reason for each entry.
    ///
    /// **Answered over the footprint as well as the declaration**, which
    /// `Excluded` is not: an ordinary boundary is a statement about the plan and
    /// this one is a statement about the files, so a step that never declared
    /// `.env` and wrote to it anyway meets this rather than drift.
    ///
    /// **No argument reaches it.** There is no verdict, no widening and no
    /// setting that empties this variant, which is why the tier is a boundary
    /// rather than a default.
    Forbidden { paths: Vec<Forbidden> },
    /// The step wants a declaration and none arrived.
    ///
    /// Nothing drifted, because there was no plan to drift from, and a Judge
    /// asked about it would have no declaration to compare the diff against.
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
            OutsideScope::Forbidden { paths } => f.write_str(&reaches(paths)),
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

/// The excluded paths a Judge has already cleared for this Job.
///
/// **There is no constructor taking paths.** One way in — [`Lifted::of`], off
/// the Job's own scope revisions, and only the entries that took. A boundary is
/// lifted by a recorded decision, which is `fleet::widening`'s to write, and by
/// nothing a caller can assemble.
///
/// **It reaches only the liftable tier.** `crate::forbidden` takes no argument
/// at all, so there is no signature through which one of these could reach it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lifted(Vec<RepoPath>);

impl Lifted {
    /// What this Job's recorded widenings cleared.
    ///
    /// **Entry zero is not one**, and skipping it is the whole correctness of
    /// this: the first revision carries the scope the Job was created with, and
    /// counting it would make the Job's own `write_targets` lift the step's
    /// denylist — which is the ordering `exclude_paths` exists to invert. It is
    /// told apart by carrying no step, the way `fleet::widening` already tells
    /// it apart when it counts a step's one ask.
    ///
    /// **Every remaining taken revision's `paths_added`**, not only the ones a
    /// denylist named: one it did not name was never blocked, so carrying it
    /// changes nothing and dropping it would need the step's list here too.
    pub fn of(job: &Job) -> Lifted {
        Lifted(
            job.scope_revisions()
                .iter()
                .filter(|revision| revision.at_step.is_some() && revision.outcome.took_effect())
                .flat_map(|revision| revision.paths_added.iter().cloned())
                .collect(),
        )
    }

    /// Whether a cleared path covers this one.
    fn covers(&self, path: &RepoPath) -> bool {
        self.0
            .iter()
            .any(|cleared| under(cleared.as_str(), path.as_str()))
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
    ///
    /// **Two tiers, and `#417` is the split.** [`forbidden`](crate::forbidden)
    /// answers first and over both lists: a path nothing lifts is not made
    /// ordinary by having been declared, and not made drift by having been left
    /// undeclared. `exclude_paths` answers over the declaration alone and is
    /// subject to `lifted`.
    ///
    /// **A plan Fleet took must not fail at the gate for being the plan Fleet
    /// took**, which is why the lift reaches this function rather than stopping
    /// at the tool: `declare_scope` and the gate both resolve here.
    ///
    /// `lifted` is what a Judge already cleared for this Job. It is required
    /// rather than optional so that a call site with a Job in hand cannot pass
    /// nothing by omission, and `Lifted::default()` is a Job that has asked for
    /// nothing rather than an override.
    pub fn resolved(
        scope: &EvidenceScope,
        declared: Option<&DeclaredPaths>,
        lifted: &Lifted,
        changed: &[String],
    ) -> Result<InScope, OutsideScope> {
        let Some(declared) = declared else {
            return Err(OutsideScope::NothingDeclared);
        };
        // The plan and the footprint, once each. A path in both lists is one
        // path, and a refusal naming it twice reads as two boundaries.
        let mut reached: Vec<RepoPath> = declared.paths().to_vec();
        for path in changed {
            let path = RepoPath::new(path);
            if !reached.contains(&path) {
                reached.push(path);
            }
        }
        let absolute = forbidden_among(reached.iter());
        if !absolute.is_empty() {
            return Err(OutsideScope::Forbidden { paths: absolute });
        }
        let excluded: Vec<RepoPath> = declared
            .paths()
            .iter()
            .filter(|path| {
                !lifted.covers(path)
                    && scope
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
            Vec::new(),
            check,
            None,
        )
    }

    fn declared(paths: &[&str]) -> DeclaredPaths {
        DeclaredPaths::of(paths.iter().copied().map(RepoPath::new).collect())
    }

    /// A Job that has never asked for more scope. Named rather than inlined so
    /// that every case below reads as "nothing was lifted here" rather than as
    /// a default somebody took.
    fn nothing_lifted() -> Lifted {
        Lifted::default()
    }

    /// One a Judge cleared, built the way `fleet::widening` writes it. `Lifted`
    /// has no constructor taking paths, so this goes through a `Job` and a
    /// recorded revision — which is the property under test as much as the
    /// resolution is.
    fn cleared(paths: &[&str]) -> Lifted {
        let job = testkit::asked_for().scope_revised(core_model::ScopeRevision {
            at_step: Some(core_model::StepId::new("the-step")),
            paths_added: paths.iter().copied().map(RepoPath::new).collect(),
            paths_removed: Vec::new(),
            atomic_before: false,
            atomic_after: false,
            rationale: "the fix needs it".to_string(),
            outcome: core_model::ScopeRevisionOutcome::took(),
            approved_by: core_model::Actor::Fleet,
            at: core_model::Timestamp::from_rfc3339("2026-09-03T00:00:00Z"),
        });
        Lifted::of(&job)
    }

    #[test]
    fn a_footprint_inside_the_declaration_resolves() {
        let scope = watching(Vec::new(), true);
        let resolved = InScope::resolved(
            &scope,
            Some(&declared(&["docs/", "crates/config"])),
            &nothing_lifted(),
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
            &nothing_lifted(),
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
            &nothing_lifted(),
            &["docs/a.md".into()],
        )
        .is_ok());
    }

    #[test]
    fn an_ordinary_denied_path_wins_over_the_declaration_until_somebody_lifts_it() {
        let scope = watching(vec!["secrets"], true);
        let outside = InScope::resolved(
            &scope,
            Some(&declared(&["secrets/keys.toml"])),
            &nothing_lifted(),
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

    /// **The ordinary boundary, lifted.** The step still excludes `secrets`,
    /// the Drone still declares a path under it, and the resolution passes
    /// because a Judge cleared exactly that path and the decision was recorded.
    #[test]
    fn a_denied_path_a_judge_cleared_resolves() {
        let scope = watching(vec!["secrets"], true);
        assert!(InScope::resolved(
            &scope,
            Some(&declared(&["secrets/keys.toml"])),
            &cleared(&["secrets/keys.toml"]),
            &["secrets/keys.toml".into()],
        )
        .is_ok());
    }

    /// **The one nothing lifts.** The same clearance, against a path in the
    /// absolute tier, and the answer does not move: `Lifted` reaches the
    /// denylist and reaches nothing here.
    #[test]
    fn a_cleared_widening_does_not_reach_the_absolute_tier() {
        let scope = watching(Vec::new(), true);
        let outside = InScope::resolved(
            &scope,
            Some(&declared(&[".env"])),
            &cleared(&[".env"]),
            &[".env".into()],
        )
        .unwrap_err();
        assert_eq!(
            outside,
            OutsideScope::Forbidden {
                paths: vec![crate::forbidden::forbidden(&RepoPath::new(".env")).unwrap()]
            }
        );
    }

    /// A step that wrote to an absolute path without declaring it meets the
    /// boundary rather than the drift check — which would have handed a secrets
    /// file to a Judge and let it be excused.
    #[test]
    fn an_absolute_path_reached_and_never_declared_is_not_drift() {
        let scope = watching(Vec::new(), true);
        assert!(matches!(
            InScope::resolved(
                &scope,
                Some(&declared(&["docs/"])),
                &nothing_lifted(),
                &["docs/a.md".into(), ".env".into()],
            ),
            Err(OutsideScope::Forbidden { .. })
        ));
    }

    #[test]
    fn a_step_that_does_not_ask_for_the_check_does_not_get_one() {
        let scope = watching(Vec::new(), false);
        assert!(InScope::resolved(
            &scope,
            Some(&declared(&["docs/"])),
            &nothing_lifted(),
            &["crates/fleet/src/gate.rs".into()],
        )
        .is_ok());
    }

    #[test]
    fn a_missing_declaration_is_not_an_empty_one() {
        let scope = watching(Vec::new(), true);
        assert_eq!(
            InScope::resolved(&scope, None, &nothing_lifted(), &[]).unwrap_err(),
            OutsideScope::NothingDeclared
        );
        assert!(InScope::resolved(
            &scope,
            Some(&DeclaredPaths::nothing()),
            &nothing_lifted(),
            &[]
        )
        .is_ok());
    }

    #[test]
    fn declaring_nothing_and_changing_something_is_drift() {
        let scope = watching(Vec::new(), true);
        assert!(matches!(
            InScope::resolved(
                &scope,
                Some(&DeclaredPaths::nothing()),
                &nothing_lifted(),
                &["a.rs".into()]
            ),
            Err(OutsideScope::Undeclared { .. })
        ));
    }
}
