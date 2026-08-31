//! Where two Jobs claim to be writing the same place.
//!
//! **A list, never a `bool`.** There is no answer here a caller could gate a
//! dispatch on, because binding the declaration is what
//! `domain/job-fields.toml`'s `write_targets[]` row rejects by name.
//!
//! It compares two *claims*. The Drone that writes where it never said is not
//! caught here and cannot be — `verification::scope` is the check that reads a
//! real diff, and it measures one step against its own plan.
//!
//! `docs/concepts/fleet.md`, Write-scope overlap, holds the rest.

use alloc::vec::Vec;

use crate::job::ids::{RepoPath, StepId};
use crate::job::scope::under;

/// One statement about where a Job's writes will land.
///
/// **Empty is a claim.** A Job whose `write_targets` is `Some` with no paths
/// has said it writes nothing, and it collides with nobody — which is not the
/// same as the Job that has said nothing at all, and that one has no
/// `ScopeClaim` here rather than an empty one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeClaim {
    /// The step whose Drone declared it, or `None` where the claim is the
    /// Job's own `write_targets`.
    pub step: Option<StepId>,
    pub paths: Vec<RepoPath>,
}

impl ScopeClaim {
    /// The Job's own declaration, made before anything ran.
    pub fn by_the_job(paths: Vec<RepoPath>) -> ScopeClaim {
        ScopeClaim { step: None, paths }
    }

    /// One step's plan, declared by the Drone working it.
    pub fn by_a_step(step: StepId, paths: Vec<RepoPath>) -> ScopeClaim {
        ScopeClaim {
            step: Some(step),
            paths,
        }
    }
}

/// One place two Jobs both claim.
///
/// **The narrower path, never both.** Where one Job claims `crates/` and
/// another claims `crates/fleet/src`, a person is shown `crates/fleet/src`:
/// the wider path is what contains the collision and the narrower one is
/// where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collision {
    pub path: RepoPath,
    /// Which of this Job's claims reached it. `None` is the Job's own
    /// `write_targets`.
    pub mine: Option<StepId>,
    /// Which of the other Job's claims reached it, on the same reading.
    pub theirs: Option<StepId>,
}

/// Every place `mine` and `theirs` both reach, narrowest path first seen.
///
/// **A list, and never a `bool`.** See this module's first note: an answer
/// shaped `is_blocked` is the lease, and there is nowhere to put one.
///
/// A path is reported once. Where several pairs of claims reach it, the first
/// pair in the order given is the one named — the alternative is one screen row
/// per pair for one collision, which reads as several collisions.
pub fn collisions(mine: &[ScopeClaim], theirs: &[ScopeClaim]) -> Vec<Collision> {
    let mut found: Vec<Collision> = Vec::new();
    for ours in mine {
        for other in theirs {
            for narrow in narrower(&ours.paths, &other.paths) {
                if found.iter().any(|seen| seen.path == narrow) {
                    continue;
                }
                found.push(Collision {
                    path: narrow,
                    mine: ours.step.clone(),
                    theirs: other.step.clone(),
                });
            }
        }
    }
    found
}

/// For every pair of paths where one contains the other, the contained one.
///
/// Segment-boundary containment through [`under`], so `crates/api` and
/// `crates/apiserver` are two places rather than one — the bare-prefix mistake
/// that would name a collision between neighbours.
fn narrower(mine: &[RepoPath], theirs: &[RepoPath]) -> Vec<RepoPath> {
    let mut narrow = Vec::new();
    for ours in mine {
        for other in theirs {
            if under(ours.as_str(), other.as_str()) {
                narrow.push(other.clone());
            } else if under(other.as_str(), ours.as_str()) {
                narrow.push(ours.clone());
            }
        }
    }
    narrow
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn paths(each: &[&str]) -> Vec<RepoPath> {
        each.iter().map(|path| RepoPath::new(*path)).collect()
    }

    #[test]
    fn the_narrower_path_is_what_is_named() {
        let mine = vec![ScopeClaim::by_the_job(paths(&["crates"]))];
        let theirs = vec![ScopeClaim::by_the_job(paths(&["crates/fleet/src"]))];
        let found = collisions(&mine, &theirs);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path.as_str(), "crates/fleet/src");
    }

    #[test]
    fn a_shared_prefix_that_is_not_a_segment_is_not_a_collision() {
        let mine = vec![ScopeClaim::by_the_job(paths(&["crates/api"]))];
        let theirs = vec![ScopeClaim::by_the_job(paths(&["crates/apiserver"]))];
        assert!(collisions(&mine, &theirs).is_empty());
    }

    #[test]
    fn a_job_that_claims_to_write_nothing_collides_with_nobody() {
        let mine = vec![ScopeClaim::by_the_job(Vec::new())];
        let theirs = vec![ScopeClaim::by_the_job(paths(&["crates/fleet"]))];
        assert!(collisions(&mine, &theirs).is_empty());
    }

    #[test]
    fn each_side_carries_which_claim_reached_the_path() {
        let mine = vec![ScopeClaim::by_a_step(
            StepId::new("implement"),
            paths(&["crates/store"]),
        )];
        let theirs = vec![ScopeClaim::by_the_job(paths(&["crates/store/src/read.rs"]))];
        let found = collisions(&mine, &theirs);
        assert_eq!(
            found[0].mine.as_ref().map(StepId::as_str),
            Some("implement")
        );
        assert_eq!(found[0].theirs, None);
    }

    #[test]
    fn one_path_reached_by_two_pairs_is_named_once() {
        let mine = vec![
            ScopeClaim::by_the_job(paths(&["crates/fleet"])),
            ScopeClaim::by_a_step(StepId::new("implement"), paths(&["crates/fleet/src"])),
        ];
        let theirs = vec![ScopeClaim::by_the_job(paths(&["crates/fleet/src"]))];
        let found = collisions(&mine, &theirs);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].mine, None);
    }

    #[test]
    fn the_repository_root_collides_with_everything_claimed() {
        let mine = vec![ScopeClaim::by_the_job(paths(&[""]))];
        let theirs = vec![ScopeClaim::by_the_job(paths(&["crates/fleet", "docs"]))];
        let found = collisions(&mine, &theirs);
        assert_eq!(found.len(), 2);
    }
}
