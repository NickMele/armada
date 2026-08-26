//! Where a Job's checkout lives and what its branch is called — derived, in a
//! crate that cannot open a file.
//!
//! # Why the derivation is here and not in the implementation
//!
//! The System Architecture fixes the layout: `<repo>/.armada/worktrees/<job-id>`,
//! **not configurable, and derived rather than stored**. v1 stored the path as a
//! string field on the Job record and then needed machinery to shorten it for
//! display and expand it again at every use — a field that could disagree with
//! the record holding it.
//!
//! Putting the derivation beside the trait rather than beside the
//! implementation is what stops a second implementation inventing a second
//! layout. An implementation is handed a [`WorktreeSpec`] and asks it where to
//! go; it has no say in the answer.
//!
//! # Why a spec is refused at construction
//!
//! A job id carrying `/` or `..` would put the worktree somewhere else in the
//! filesystem, and no amount of checking inside the implementation would catch
//! every path that reaches it. [`WorktreeSpec::for_job`] is the only
//! constructor and it refuses, so a spec that exists is a spec whose derived
//! path is inside the repo — the wrong value is not representable rather than
//! rejected later.

use alloc::string::String;
use alloc::vec::Vec;

/// The branch namespace every Armada worktree lands in.
///
/// v1's reason, kept: namespacing is what would make deleting one of these
/// branches safe to contemplate at all. Without it a Job could derive the name
/// of a branch a person is using.
const BRANCH_NAMESPACE: &str = "armada/";

/// The repo-relative directory holding every Job's checkout.
const WORKTREE_ROOT: &str = ".armada/worktrees/";

/// Why a spec was refused before anything touched a disk.
///
/// Not an I/O failure — every variant here is a value that could not have
/// produced a worktree inside the repo, caught at the only constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeSpecRefused {
    /// No repository root was given.
    RepoRootEmpty,
    /// The repository root is relative, so the derived path would depend on
    /// whichever directory the process happened to be in. A derived path that
    /// moves with the caller is the stored-path failure in another shape.
    RepoRootNotAbsolute { given: String },
    /// No job id was given.
    JobIdEmpty,
    /// The job id holds a character that is not `A-Z`, `a-z`, `0-9`, `-` or
    /// `_`. A separator or a dot segment here would put the checkout outside
    /// the repo, and a ref-illegal character would put the branch nowhere.
    JobIdNotPortable { job_id: String, found: char },
}

impl WorktreeSpecRefused {
    /// A sentence for a person, built here rather than by each caller — the
    /// same shape [`SpawnConfigRefused`](crate::SpawnConfigRefused) takes, and
    /// for the same reason: the words belong beside the variant that raised
    /// them, not in whichever crate happened to catch it.
    pub fn said(&self) -> String {
        match self {
            WorktreeSpecRefused::RepoRootEmpty => "no repository root was given".into(),
            WorktreeSpecRefused::RepoRootNotAbsolute { given } => {
                let mut said = String::from("the repository root `");
                said.push_str(given);
                said.push_str("` is relative, so the derived path would move with the caller");
                said
            }
            WorktreeSpecRefused::JobIdEmpty => "no job id was given".into(),
            WorktreeSpecRefused::JobIdNotPortable { job_id, found } => {
                let mut said = String::from("the job id `");
                said.push_str(job_id);
                said.push_str("` holds `");
                said.push(*found);
                said.push_str("`, which names no directory and no branch");
                said
            }
        }
    }
}

/// What a worktree is to be made from: one repository, one Job.
///
/// Everything else about it — the directory, the branch, the name git
/// registers it under — is derived from these two and cannot be overridden.
/// There is no setter and no second constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeSpec {
    repo_root: String,
    job_id: String,
}

impl WorktreeSpec {
    /// The only way to make one.
    ///
    /// `repo_root` is an absolute path to the repository's working directory.
    /// `job_id` is the Job's id exactly as the record holds it.
    pub fn for_job(repo_root: &str, job_id: &str) -> Result<WorktreeSpec, WorktreeSpecRefused> {
        if repo_root.is_empty() {
            return Err(WorktreeSpecRefused::RepoRootEmpty);
        }
        if !repo_root.starts_with('/') {
            return Err(WorktreeSpecRefused::RepoRootNotAbsolute {
                given: String::from(repo_root),
            });
        }
        if job_id.is_empty() {
            return Err(WorktreeSpecRefused::JobIdEmpty);
        }
        if let Some(found) = job_id
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        {
            return Err(WorktreeSpecRefused::JobIdNotPortable {
                job_id: String::from(job_id),
                found,
            });
        }

        // One trailing separator is the ordinary shape of a path handed in by a
        // shell, and keeping it would derive `//.armada/...`. Trimmed here so
        // every caller derives the same string for the same repository.
        let trimmed = repo_root.trim_end_matches('/');
        let repo_root = if trimmed.is_empty() { "/" } else { trimmed };

        Ok(WorktreeSpec {
            repo_root: String::from(repo_root),
            job_id: String::from(job_id),
        })
    }

    /// The repository the worktree is added to.
    pub fn repo_root(&self) -> &str {
        &self.repo_root
    }

    /// The Job this worktree belongs to. The worktree outlives every Drone that
    /// works it, so this is the only thing that names it.
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// `<repo>/.armada/worktrees/<job-id>` — where the checkout goes.
    pub fn worktree_path(&self) -> String {
        let mut path = String::from(&self.repo_root);
        if !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(WORKTREE_ROOT);
        path.push_str(&self.job_id);
        path
    }

    /// `<repo>/.armada/worktrees` — the directory the checkout goes inside,
    /// which an implementation may have to create first.
    pub fn worktree_parent(&self) -> String {
        let full = self.worktree_path();
        let cut = full.len() - self.job_id.len() - 1;
        let mut parent = full;
        parent.truncate(cut);
        parent
    }

    /// `armada/<job-id>` — the branch, in Armada's own namespace.
    pub fn branch(&self) -> String {
        let mut branch = String::from(BRANCH_NAMESPACE);
        branch.push_str(&self.job_id);
        branch
    }

    /// The name git files the worktree's administrative record under.
    ///
    /// The job id, matching the directory's own name, so a record left behind
    /// under it is findable from the path a person is looking at.
    pub fn registration_name(&self) -> &str {
        &self.job_id
    }
}

/// A worktree that exists.
///
/// **The receipt, not the handle.** It carries where the checkout is and what
/// branch is checked out in it, and it offers no operation — nothing here can
/// remove, reset or push anything, because in M1 nothing may.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    path: String,
    branch: String,
}

impl Worktree {
    /// Record a worktree that was created. Called by the implementation that
    /// created it, and by a fake standing in for one.
    pub fn at(path: impl Into<String>, branch: impl Into<String>) -> Worktree {
        Worktree {
            path: path.into(),
            branch: branch.into(),
        }
    }

    /// The absolute path to the checkout. This is what a Drone is given as its
    /// working directory, and what a person opens a terminal in.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The branch checked out in it.
    pub fn branch(&self) -> &str {
        &self.branch
    }
}

/// Everything derived from one spec, in one value.
///
/// A convenience for a caller that wants to log or display all three without
/// deriving each separately.
pub fn derived(spec: &WorktreeSpec) -> Vec<(&'static str, String)> {
    alloc::vec![
        ("path", spec.worktree_path()),
        ("branch", spec.branch()),
        ("registration", String::from(spec.registration_name())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: &str = "01J8ZQ8K7N0000000000000000";

    fn spec() -> WorktreeSpec {
        WorktreeSpec::for_job("/repos/armada", JOB).expect("a legal spec")
    }

    #[test]
    fn the_path_is_the_architecture_s_layout() {
        assert_eq!(
            spec().worktree_path(),
            alloc::format!("/repos/armada/.armada/worktrees/{JOB}")
        );
    }

    #[test]
    fn the_parent_is_the_directory_holding_every_job_s_checkout() {
        assert_eq!(spec().worktree_parent(), "/repos/armada/.armada/worktrees");
    }

    #[test]
    fn the_branch_is_namespaced_under_armada() {
        assert_eq!(spec().branch(), alloc::format!("armada/{JOB}"));
    }

    #[test]
    fn the_registration_name_is_the_directory_s_own_name() {
        let spec = spec();
        assert!(spec.worktree_path().ends_with(spec.registration_name()));
    }

    #[test]
    fn a_trailing_separator_does_not_change_the_derived_path() {
        let with = WorktreeSpec::for_job("/repos/armada/", JOB).expect("a legal spec");
        assert_eq!(with.worktree_path(), spec().worktree_path());
    }

    #[test]
    fn a_job_id_that_would_escape_the_repo_is_refused() {
        for escape in ["../elsewhere", "a/b", "..", "./x"] {
            let refused = WorktreeSpec::for_job("/repos/armada", escape);
            assert!(
                matches!(refused, Err(WorktreeSpecRefused::JobIdNotPortable { .. })),
                "`{escape}` was not refused"
            );
        }
    }

    #[test]
    fn a_relative_repo_root_is_refused() {
        assert_eq!(
            WorktreeSpec::for_job("repos/armada", JOB),
            Err(WorktreeSpecRefused::RepoRootNotAbsolute {
                given: String::from("repos/armada")
            })
        );
    }

    #[test]
    fn an_empty_half_is_refused_by_name() {
        assert_eq!(
            WorktreeSpec::for_job("", JOB),
            Err(WorktreeSpecRefused::RepoRootEmpty)
        );
        assert_eq!(
            WorktreeSpec::for_job("/repos/armada", ""),
            Err(WorktreeSpecRefused::JobIdEmpty)
        );
    }

    #[test]
    fn two_jobs_in_one_repo_derive_two_paths_and_two_branches() {
        let one = WorktreeSpec::for_job("/repos/armada", "01AAA").expect("a legal spec");
        let two = WorktreeSpec::for_job("/repos/armada", "01BBB").expect("a legal spec");
        assert_ne!(one.worktree_path(), two.worktree_path());
        assert_ne!(one.branch(), two.branch());
        assert_eq!(one.worktree_parent(), two.worktree_parent());
    }

    #[test]
    fn deriving_twice_gives_the_same_answer() {
        assert_eq!(derived(&spec()), derived(&spec()));
    }

    #[test]
    fn a_worktree_carries_both_halves() {
        let made = Worktree::at("/repos/armada/.armada/worktrees/01AAA", "armada/01AAA");
        assert_eq!(made.path(), "/repos/armada/.armada/worktrees/01AAA");
        assert_eq!(made.branch(), "armada/01AAA");
    }
}
