//! A `Vcs` that touches no disk.
//!
//! # Why this exists rather than a temporary repository per test
//!
//! Because a suite that shells out to git for every case is a suite people stop
//! running, and because the acceptance test is hermetic by definition — no
//! process spawned, no repository touched, no network opened. Fleet's own tests
//! ask *did a worktree get created before the Drone, and what did Fleet do when
//! one could not be*; neither question needs git to answer it.
//!
//! The cases that genuinely need git's opinion — an existing branch, an
//! administrative record that outlives its directory — are tested against a
//! real repository in `adapters`, where they are five tests rather than every
//! test.
//!
//! # What it is faithful about
//!
//! The two refusals a caller has to handle, and the derivation. Paths and
//! branch names come from the same [`WorktreeSpec`] the real implementation
//! uses, so a test asserting on a path is asserting on the derivation that
//! ships rather than on a second copy of it — the second-vocabulary defect.
//!
//! It is **not** faithful about the filesystem: nothing is created, so a test
//! that wants to read a file out of a worktree wants the real one.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use adapter_traits::{Vcs, Worktree, WorktreeSpec};

/// Why the fake refused.
///
/// Two variants, matching the split the real error draws: a name already taken,
/// and the machine not cooperating. A caller that handles both handles the real
/// implementation's whole surface as far as its own logic is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeVcsError {
    /// A branch of that name is already there and was refused, never reused.
    BranchExists { branch: String },
    /// A scripted failure standing in for a disk, a permission or a repository
    /// that would not answer.
    Refused { standing_in_for: &'static str },
}

impl fmt::Display for FakeVcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FakeVcsError::BranchExists { branch } => {
                write!(f, "the branch `{branch}` is already there")
            }
            FakeVcsError::Refused { standing_in_for } => {
                write!(f, "refused, standing in for {standing_in_for}")
            }
        }
    }
}

impl Error for FakeVcsError {}

/// Version control that remembers what it was asked for and creates nothing.
#[derive(Debug, Default)]
pub struct FakeVcs {
    branches: RefCell<BTreeSet<String>>,
    created: RefCell<Vec<Worktree>>,
    refuse_next: RefCell<Option<&'static str>>,
}

impl FakeVcs {
    pub fn new() -> FakeVcs {
        FakeVcs::default()
    }

    /// Seed a branch somebody else already made, so a collision can be
    /// exercised without a repository.
    pub fn with_existing_branch(self, branch: impl Into<String>) -> FakeVcs {
        self.branches.borrow_mut().insert(branch.into());
        self
    }

    /// Make the next creation fail as a machine would.
    ///
    /// The argument is what it stands in for, and it is a fixed string chosen
    /// at the call site rather than a message assembled from data — a fake's
    /// failure is a script, not a diagnosis.
    pub fn refuse_next(&self, standing_in_for: &'static str) {
        *self.refuse_next.borrow_mut() = Some(standing_in_for);
    }

    /// Every worktree this fake said it made, in order. **Nothing removes an
    /// entry**, for the same reason nothing removes a worktree.
    pub fn created(&self) -> Vec<Worktree> {
        self.created.borrow().clone()
    }
}

impl Vcs for FakeVcs {
    type Error = FakeVcsError;

    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, Self::Error> {
        if let Some(standing_in_for) = self.refuse_next.borrow_mut().take() {
            return Err(FakeVcsError::Refused { standing_in_for });
        }
        let branch = spec.branch();
        if !self.branches.borrow_mut().insert(branch.clone()) {
            return Err(FakeVcsError::BranchExists { branch });
        }
        let made = Worktree::at(spec.worktree_path(), branch);
        self.created.borrow_mut().push(made.clone());
        Ok(made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";

    fn spec(job: &str) -> WorktreeSpec {
        WorktreeSpec::for_job("/repos/armada", job).expect("a legal spec")
    }

    #[test]
    fn it_derives_the_same_path_and_branch_the_real_one_would() {
        let made = FakeVcs::new().create_worktree(&spec(JOB)).unwrap();
        assert_eq!(made.path(), spec(JOB).worktree_path());
        assert_eq!(made.branch(), spec(JOB).branch());
    }

    #[test]
    fn a_seeded_branch_is_refused_rather_than_reused() {
        let vcs = FakeVcs::new().with_existing_branch(format!("armada/{JOB}"));
        assert_eq!(
            vcs.create_worktree(&spec(JOB)),
            Err(FakeVcsError::BranchExists {
                branch: format!("armada/{JOB}")
            })
        );
        assert!(vcs.created().is_empty());
    }

    #[test]
    fn the_same_job_twice_collides_with_itself() {
        let vcs = FakeVcs::new();
        vcs.create_worktree(&spec(JOB)).expect("the first");
        assert!(vcs.create_worktree(&spec(JOB)).is_err());
    }

    #[test]
    fn a_scripted_refusal_applies_once() {
        let vcs = FakeVcs::new();
        vcs.refuse_next("a full disk");
        assert_eq!(
            vcs.create_worktree(&spec(JOB)),
            Err(FakeVcsError::Refused {
                standing_in_for: "a full disk"
            })
        );
        assert!(vcs.created().is_empty());
        vcs.create_worktree(&spec(JOB)).expect("the retry");
    }

    #[test]
    fn nothing_it_recorded_ever_goes_away() {
        let vcs = FakeVcs::new();
        vcs.create_worktree(&spec("01AAA")).unwrap();
        vcs.create_worktree(&spec("01BBB")).unwrap();
        assert!(vcs.create_worktree(&spec("01AAA")).is_err());
        assert_eq!(vcs.created().len(), 2);
    }

    #[test]
    fn it_creates_nothing_on_disk() {
        let vcs = FakeVcs::new();
        let made = vcs.create_worktree(&spec(JOB)).unwrap();
        assert!(!std::path::Path::new(made.path()).exists());
    }
}
