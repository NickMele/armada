//! Where the checkout every Job photographs *before* against lives, derived
//! the way a Job's own worktree is.
//!
//! # One checkout per commit, shared by every Job on it
//!
//! A Job's worktree is per Job because a Job writes in it. Nothing writes in a
//! base checkout: it is served, read, and served again. So the thing that makes
//! two of them different is not the Job, it is **which commit is checked out**
//! — and paying `setup.requires` per Job for an identical tree is minutes a
//! Drone is not working, which `fleet::preparing` names as the one span in a
//! Job where nothing moves.
//!
//! # The commit is the key, and that is the whole invalidation story
//!
//! The path is derived from the commit and from nothing else, so a base branch
//! that moves derives a different path and the next Job checks out the new
//! commit. **A stale base checkout is not detected, it is unreachable** — there
//! is no freshness field to get wrong and no cache to invalidate, which is the
//! defect a keyed-by-branch-name checkout would have: it would photograph the
//! wrong *before* and say nothing.
//!
//! The ref name is deliberately not in the path. Two refs at one commit are one
//! tree, and a name in the path would make two checkouts of it — plus a branch
//! name is not a path component (`release/2.0`), so it would have to be
//! mangled, and a mangling is a second vocabulary.
//!
//! # Detached, and never a branch
//!
//! [`BaseSpec`] derives no branch, unlike [`WorktreeSpec`](crate::WorktreeSpec).
//! A base checkout is a photograph of a commit and not a line of history —
//! nothing commits in it — and a branch would be a ref `armada clean` derives
//! from no record, which is the litter that file refuses to create.

use alloc::string::String;

/// The repo-relative directory holding every base checkout.
///
/// **Beside `.armada/worktrees/` rather than inside it.** Everything under that
/// directory is one Job's and is derived from a `WorktreeSpec`; a directory
/// there that no Job id names would be read as an orphan by anything walking
/// it.
const BASE_ROOT: &str = ".armada/bases/";

/// The file a prepared base checkout carries, written last.
///
/// **The marker is what makes an interrupted preparation safe.** A Fleet killed
/// during `setup.requires` leaves a checkout that looks complete and has no
/// `node_modules`; the next Fleet would serve it and photograph a broken app.
/// The name is a fact about the directory rather than a record elsewhere, so
/// there is nothing to keep in sync with the disk.
const READY_FILE: &str = ".armada-base-ready";

/// Why a base checkout could not be derived.
///
/// The same shape as [`WorktreeSpecRefused`](crate::WorktreeSpecRefused) and
/// for its reason: every variant is a value that could not have produced a
/// checkout inside the repository, caught at the only constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseSpecRefused {
    /// No repository root was given.
    RepoRootEmpty,
    /// The repository root is relative, so the derived path would move with
    /// whichever directory the process happened to be in.
    RepoRootNotAbsolute { given: String },
    /// The commit is not a full object id. **Not "not found"** — nothing here
    /// opens a repository. A short id, a ref name or a path would each name a
    /// directory, and only one of the three would be an accident.
    NotACommit { given: String },
}

impl BaseSpecRefused {
    /// A sentence for a person, built beside the variant that raised it.
    pub fn said(&self) -> String {
        match self {
            BaseSpecRefused::RepoRootEmpty => "no repository root was given".into(),
            BaseSpecRefused::RepoRootNotAbsolute { given } => {
                let mut said = String::from("the repository root `");
                said.push_str(given);
                said.push_str("` is relative, so the derived path would move with the caller");
                said
            }
            BaseSpecRefused::NotACommit { given } => {
                let mut said = String::from("`");
                said.push_str(given);
                said.push_str("` is not a commit id, so it names no base checkout");
                said
            }
        }
    }
}

/// What a base checkout is to be made from: one repository, one commit.
///
/// Everything else — the directory, the name git registers it under, the file
/// that says it is prepared — is derived from those two and cannot be
/// overridden. There is no setter and no second constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseSpec {
    repo_root: String,
    commit: String,
}

impl BaseSpec {
    /// The only way to make one.
    ///
    /// `commit` is a full 40-character object id, which is what makes it a
    /// single path component by construction rather than by checking: every
    /// character is hex, so there is no separator and no dot segment to refuse
    /// individually.
    pub fn at(repo_root: &str, commit: &str) -> Result<BaseSpec, BaseSpecRefused> {
        if repo_root.is_empty() {
            return Err(BaseSpecRefused::RepoRootEmpty);
        }
        if !repo_root.starts_with('/') {
            return Err(BaseSpecRefused::RepoRootNotAbsolute {
                given: String::from(repo_root),
            });
        }
        // 40 for SHA-1 and 64 for SHA-256, which is a repository format git
        // already writes. Neither length is a choice made here — refusing the
        // longer one would refuse a repository rather than a bad value.
        let sized = commit.len() == 40 || commit.len() == 64;
        if !sized || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(BaseSpecRefused::NotACommit {
                given: String::from(commit),
            });
        }
        let trimmed = repo_root.trim_end_matches('/');
        let repo_root = if trimmed.is_empty() { "/" } else { trimmed };
        Ok(BaseSpec {
            repo_root: String::from(repo_root),
            // Lowercased so two spellings of one id derive one directory. git
            // answers lowercase; a caller that read an id off a person would
            // not necessarily.
            commit: commit.to_ascii_lowercase(),
        })
    }

    /// The repository the checkout is added to.
    pub fn repo_root(&self) -> &str {
        &self.repo_root
    }

    /// The commit checked out in it.
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// `<repo>/.armada/bases/<commit>` — where the checkout goes.
    pub fn path(&self) -> String {
        let mut path = String::from(&self.repo_root);
        if !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(BASE_ROOT);
        path.push_str(&self.commit);
        path
    }

    /// `<repo>/.armada/bases` — the directory the checkout goes inside, which
    /// an implementation may have to create first.
    pub fn parent(&self) -> String {
        let full = self.path();
        let cut = full.len() - self.commit.len() - 1;
        let mut parent = full;
        parent.truncate(cut);
        parent
    }

    /// The file whose presence says `setup.requires` finished in this checkout.
    ///
    /// **Asked of the disk rather than of a record**, for the reason the module
    /// header gives: a record of preparedness and a directory can disagree, and
    /// the one that decides whether the frames are worth anything is the
    /// directory.
    pub fn ready_marker(&self) -> String {
        let mut marker = self.path();
        marker.push('/');
        marker.push_str(READY_FILE);
        marker
    }

    /// The name git files the worktree's administrative record under.
    ///
    /// **Prefixed**, unlike a Job's, which registers under its bare id. The two
    /// share one namespace in `.git/worktrees/`, and a prefix is what makes a
    /// record found there attributable to the thing that made it without
    /// knowing the shape of a Job id.
    pub fn registration_name(&self) -> String {
        let mut name = String::from("base-");
        name.push_str(&self.commit);
        name
    }
}

/// A base checkout that exists.
///
/// **The receipt, and a different type from
/// [`Worktree`](crate::Worktree) rather than a flag on it.** Every method that
/// takes a Job's worktree would do something wrong with this one — commit into
/// a detached head, push a branch that is not there, read a work product that
/// is by definition empty — and a type those methods cannot accept says so
/// without anybody having to remember.
///
/// It carries the commit where a `Worktree` carries a branch, because that is
/// what there is: a photograph is identified by what it is of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCheckout {
    path: String,
    commit: String,
    prepared: bool,
}

impl BaseCheckout {
    /// Record a checkout that exists. Called by the implementation that made or
    /// found it, and by a fake standing in for one.
    ///
    /// `prepared` is whether [`BaseSpec::ready_marker`] was there — the fact
    /// the caller needs and cannot get from the path alone, since a checkout
    /// that was just created has never been prepared and one that was found may
    /// have been.
    pub fn at(path: impl Into<String>, commit: impl Into<String>, prepared: bool) -> BaseCheckout {
        BaseCheckout {
            path: path.into(),
            commit: commit.into(),
            prepared,
        }
    }

    /// The absolute path to the checkout — what a harness is served from.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The commit checked out in it, which is the *before* any frame taken here
    /// is a photograph of.
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// Whether `setup.requires` has already finished in it.
    ///
    /// **False on a checkout that was just created, and on one whose
    /// preparation was interrupted.** Those are the same case to the caller:
    /// run the repository's requirements, then mark it.
    pub fn prepared(&self) -> bool {
        self.prepared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "a787ffc2000000000000000000000000000000ab";

    fn spec() -> BaseSpec {
        BaseSpec::at("/repos/armada", COMMIT).expect("a legal spec")
    }

    #[test]
    fn the_path_is_the_commit_and_never_the_branch_name() {
        assert_eq!(
            spec().path(),
            alloc::format!("/repos/armada/.armada/bases/{COMMIT}")
        );
    }

    #[test]
    fn the_parent_is_the_directory_holding_every_base_checkout() {
        assert_eq!(spec().parent(), "/repos/armada/.armada/bases");
    }

    #[test]
    fn two_commits_derive_two_checkouts_and_a_moved_base_reaches_neither() {
        let one = BaseSpec::at("/repos/armada", COMMIT).expect("a legal spec");
        let two =
            BaseSpec::at("/repos/armada", &COMMIT.replace("a787", "b899")).expect("a legal spec");
        assert_ne!(one.path(), two.path());
        assert_eq!(one.parent(), two.parent());
    }

    #[test]
    fn one_commit_spelled_two_ways_derives_one_checkout() {
        let lower = BaseSpec::at("/repos/armada", COMMIT).expect("a legal spec");
        let upper =
            BaseSpec::at("/repos/armada", &COMMIT.to_ascii_uppercase()).expect("a legal spec");
        assert_eq!(lower.path(), upper.path());
    }

    #[test]
    fn anything_that_is_not_a_commit_id_is_refused() {
        for given in ["main", "../elsewhere", "a787ffc2", "", "origin/main"] {
            assert!(
                matches!(
                    BaseSpec::at("/repos/armada", given),
                    Err(BaseSpecRefused::NotACommit { .. })
                ),
                "`{given}` was not refused"
            );
        }
    }

    #[test]
    fn a_relative_repo_root_is_refused() {
        assert_eq!(
            BaseSpec::at("repos/armada", COMMIT),
            Err(BaseSpecRefused::RepoRootNotAbsolute {
                given: String::from("repos/armada")
            })
        );
    }

    #[test]
    fn the_marker_is_inside_the_checkout_it_is_about() {
        let spec = spec();
        assert!(spec.ready_marker().starts_with(&spec.path()));
    }

    #[test]
    fn the_registration_name_says_what_made_it() {
        assert!(spec().registration_name().starts_with("base-"));
        assert!(spec().registration_name().ends_with(COMMIT));
    }

    #[test]
    fn a_trailing_separator_does_not_change_the_derived_path() {
        let with = BaseSpec::at("/repos/armada/", COMMIT).expect("a legal spec");
        assert_eq!(with.path(), spec().path());
    }
}
