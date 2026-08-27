//! Which branch a Job's work merges into.
//!
//! # One answer, two callers
//!
//! A clean asks it to decide whether a branch holds work nobody has taken; a
//! finished Job asks it to decide what to rebase onto and what to open a pull
//! request against. Two copies of this would drift, and the drift would show up
//! as a clean keeping a branch that had just been merged into something else.
//!
//! # Declared beats inferred, and a declared name that is not there is refused
//!
//! `base:` in `armada.yml` is the repository stating its own answer. Falling
//! back to a guess when that name does not resolve would make the key look
//! honoured while something else was used.

use adapter_traits::{Base, NotDelivered};
use git2::{BranchType, Repository};

/// The branch names a base is looked for under when nothing declares one, in
/// order. `main` and `master` are what a repository with no remote has.
const FALLBACK_BASES: &[&str] = &["main", "master"];

/// The local branch names to look under, best first.
///
/// A declared name is the only candidate: it is an answer, and a list with a
/// fallback after it would quietly use the fallback.
pub(crate) fn candidates(repo: &Repository, declared: Option<&str>) -> Vec<String> {
    if let Some(declared) = declared {
        return vec![declared.to_string()];
    }
    let mut names: Vec<String> = default_branch_of_the_remote(repo).into_iter().collect();
    for fallback in FALLBACK_BASES {
        if !names.iter().any(|name| name == fallback) {
            names.push((*fallback).to_string());
        }
    }
    names
}

/// The base a repository resolves to, or `None` where it names none.
///
/// A declared name that no branch answers to is a [`NotDelivered`] rather than
/// a `None`: the file said something and the repository disagrees, which is a
/// thing to fix rather than a thing to work around.
pub(crate) fn resolve(
    repo: &Repository,
    declared: Option<&str>,
) -> Result<Option<Base>, NotDelivered> {
    let looked_for = candidates(repo, declared);
    let found = looked_for.iter().find(|name| exists(repo, name));
    match (found, declared) {
        (Some(name), Some(_)) => Ok(Some(Base::Declared(name.clone()))),
        (Some(name), None) => Ok(Some(Base::Inferred(name.clone()))),
        (None, Some(declared)) => Err(NotDelivered::of(
            "reading the base branch",
            format!(
                "`armada.yml` declares `base: {declared}` and this repository has no branch \
                 called that. Create it, or change the key"
            ),
        )),
        (None, None) => Ok(None),
    }
}

fn exists(repo: &Repository, name: &str) -> bool {
    repo.find_branch(name, BranchType::Local).is_ok()
}

/// What a clone recorded as the remote's default, which is the only place a
/// repository states its own answer without being told.
fn default_branch_of_the_remote(repo: &Repository) -> Option<String> {
    let head = repo.find_reference("refs/remotes/origin/HEAD").ok()?;
    head.symbolic_target()?
        .strip_prefix("refs/remotes/origin/")
        .map(str::to_string)
}
