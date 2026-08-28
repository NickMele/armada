//! What a Job has produced, read out of its own worktree.
//!
//! # Against where the branch started, not against HEAD
//!
//! A Drone commits inside its worktree, so a diff taken against that worktree's
//! own HEAD is empty the moment the Drone commits — which would fail
//! `diff_nonempty` for the best-behaved Drone in the fleet and pass it for one
//! that left everything uncommitted. The base is therefore the **merge base**
//! between the Job's branch and the repository's own HEAD: the commit the
//! branch was cut from, found without storing it anywhere.
//!
//! Storing it was the alternative and it is the failure this project already
//! knows by name — a field on a record that can disagree with the record
//! holding it. Deriving it costs one revision walk.
//!
//! # Untracked files count
//!
//! A Drone that writes a new file and does not stage it has produced work, and
//! a `diff_nonempty` check that ignored it would tell the Drone to do the thing
//! it just did. Ignored files do not count: they are what `.gitignore` says are
//! not part of the repository.
//!
//! # A reading that fails is never an empty answer
//!
//! Every path out of this file is either a list of files or an error. There is
//! no branch that returns "nothing changed" because something could not be
//! opened — that would turn a broken machine into a Drone that did no work.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use adapter_traits::{Change, Changed, ChangedFile, Footprint, Patch, WorkProduct, Worktree};
use git2::{Delta, Diff, DiffOptions, Oid, Repository};

use crate::error::ReadWorkProductError;
use crate::worktree::GitVcs;

impl WorkProduct for GitVcs {
    type Error = ReadWorkProductError;

    fn changed_files(&self, worktree: &Worktree) -> Result<Changed, Self::Error> {
        let (repo, base) = opened(worktree)?;
        let diff = diff_of(&repo, base, worktree)?;
        Ok(Changed::of(files(&diff)))
    }

    fn footprint(&self, worktree: &Worktree) -> Result<Footprint, Self::Error> {
        let path = worktree.path();
        let root = Path::new(path);
        let changed = self.changed_files(worktree)?;
        let mut entries = Vec::with_capacity(changed.len());
        for file in changed.files() {
            entries.push((file.path().to_string(), held(root, file.path(), path)?));
        }
        Ok(Footprint::of(entries))
    }

    fn patch(&self, worktree: &Worktree) -> Result<Patch, Self::Error> {
        let (repo, base) = opened(worktree)?;
        let diff = diff_of(&repo, base, worktree)?;

        // Whole-diff rendering, patches and headers together, in git's own
        // text. A structured walk would be Armada deciding what a hunk means,
        // and what the Judge is handed is the diff a person would read.
        let mut text = String::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            if matches!(line.origin(), '+' | '-' | ' ') {
                text.push(line.origin());
            }
            text.push_str(&String::from_utf8_lossy(line.content()));
            true
        })
        .map_err(|cause| ReadWorkProductError::DiffFailed {
            worktree: worktree.path().to_string(),
            cause,
        })?;
        Ok(Patch::of(text))
    }
}

/// The branch's whole diff: every change since the commit the branch was cut
/// from, untracked files included.
fn diff_of<'r>(
    repo: &'r Repository,
    base: Oid,
    worktree: &Worktree,
) -> Result<Diff<'r>, ReadWorkProductError> {
    let path = worktree.path();
    let tree = repo
        .find_commit(base)
        .and_then(|commit| commit.tree())
        .map_err(|cause| ReadWorkProductError::BaseUnreadable {
            worktree: path.to_string(),
            base: base.to_string(),
            cause,
        })?;

    let mut options = DiffOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        // Without it a new file the Drone never staged prints as a header and
        // no lines, so the Judge is handed an empty patch for work that is
        // plainly there — the same silence `include_untracked` is set to stop
        // one call earlier. The file list already counted it; this is the
        // rendering catching up with what this module says it does.
        .show_untracked_content(true)
        .include_typechange(true);
    repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut options))
        .map_err(|cause| ReadWorkProductError::DiffFailed {
            worktree: path.to_string(),
            cause,
        })
}

/// What one changed path holds, as a value two readings can be compared on.
///
/// **A hash of the bytes, not the bytes.** A footprint is taken at every step
/// start and every gate, and holding the contents of the work would put the
/// whole diff in memory twice for a question whose answer is one bit.
///
/// **Not `git`'s object id.** A blob id exists for what git has stored, and the
/// interesting case here is the opposite one — a Drone's edit sitting in the
/// working tree, unstaged, which is precisely what a step produces before it
/// commits anything.
///
/// The hasher is `std`'s, so the value is stable within a process and promises
/// nothing across builds. That is the whole of what
/// [`Footprint::differs_from`] needs, and the type's own comment says a
/// footprint is comparable only against another reading in the same process.
///
/// A path the diff named and the filesystem no longer has is a **deletion**,
/// which is a change and reads as one rather than as a failure. Anything else
/// the filesystem refuses is an error: see
/// [`ReadWorkProductError::ContentUnreadable`].
fn held(root: &Path, named: &str, worktree: &str) -> Result<String, ReadWorkProductError> {
    match std::fs::read(root.join(named)) {
        Ok(bytes) => {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            Ok(format!("{:016x}", hasher.finish()))
        }
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => Ok(String::from("gone")),
        Err(cause) => Err(ReadWorkProductError::ContentUnreadable {
            worktree: worktree.to_string(),
            path: named.to_string(),
            cause,
        }),
    }
}

/// The worktree's repository and the commit its branch was cut from — what
/// every reading above starts from.
fn opened(worktree: &Worktree) -> Result<(Repository, Oid), ReadWorkProductError> {
    let path = worktree.path();
    let repo =
        Repository::open(path).map_err(|cause| ReadWorkProductError::WorktreeUnreadable {
            worktree: path.to_string(),
            cause,
        })?;
    let base = base_of(&repo, worktree)?;
    Ok((repo, base))
}

/// The commit the Job's branch was cut from.
///
/// The repository's own HEAD is read through the common directory, which is
/// where a linked worktree's shared administrative state lives — a worktree has
/// a HEAD of its own, and reading that one would compare the branch against
/// itself.
fn base_of(worktree_repo: &Repository, worktree: &Worktree) -> Result<Oid, ReadWorkProductError> {
    let path = worktree.path();
    let tip = worktree_repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .map(|commit| commit.id())
        .map_err(|cause| ReadWorkProductError::BranchUnreadable {
            worktree: path.to_string(),
            branch: worktree.branch().to_string(),
            cause,
        })?;

    let common = Repository::open(shared_git_dir(worktree_repo)).map_err(|cause| {
        ReadWorkProductError::RepositoryUnreadable {
            worktree: path.to_string(),
            cause,
        }
    })?;
    let Ok(main) = common.head().and_then(|head| head.peel_to_commit()) else {
        // The repository has no HEAD to compare against — nothing has been
        // committed on the main line since this branch was made, or the main
        // checkout is on an unborn branch. The branch tip is then the base a
        // worktree diff should be taken against, which is the honest answer
        // rather than a failure.
        return Ok(tip);
    };

    common
        .merge_base(tip, main.id())
        .or(Ok(tip))
        .map_err(|cause: git2::Error| ReadWorkProductError::BaseUnreadable {
            worktree: path.to_string(),
            base: tip.to_string(),
            cause,
        })
}

/// The git directory the repository and all its worktrees share.
///
/// libgit2 0.19 exposes no `commondir`, so it is derived from git's own layout:
/// a linked worktree's git directory is `<repo>/.git/worktrees/<name>`, and the
/// shared one is two levels above it. A path that is not that shape belongs to
/// an ordinary checkout, which is its own shared directory.
///
/// Derived rather than stored, for the reason the worktree path itself is: a
/// second copy of a location can disagree with the first.
fn shared_git_dir(repo: &Repository) -> PathBuf {
    let git_dir = repo.path();
    let under_worktrees = git_dir
        .parent()
        .is_some_and(|parent| parent.file_name() == Some("worktrees".as_ref()));
    if under_worktrees {
        if let Some(shared) = git_dir.parent().and_then(Path::parent) {
            return shared.to_path_buf();
        }
    }
    git_dir.to_path_buf()
}

/// Every path the diff names with what happened to it, deletions included.
///
/// A delta's new path is `None` for a deletion, and a deletion is a change —
/// so the old path stands in. Nothing is deduplicated: git reports one delta
/// per path already, and a rename arrives as two.
fn files(diff: &Diff<'_>) -> Vec<ChangedFile> {
    diff.deltas()
        .filter_map(|delta| {
            delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|path| {
                    ChangedFile::new(path.to_string_lossy().into_owned(), change(delta.status()))
                })
        })
        .collect()
}

/// git's word for what happened, in the vocabulary the wire carries.
///
/// **Untracked is added.** The options above ask for untracked files precisely
/// because a Drone that wrote a file and did not stage it has produced work,
/// and reporting the staging rather than the writing would answer a question
/// nobody asked.
///
/// `Unmodified` and `Ignored` are not requested — no `include_unmodified`, no
/// `include_ignored` — so neither reaches this function. They read as
/// `Modified` rather than as a variant of their own, because a variant that
/// cannot arrive is a case every reader has to handle and none can produce.
fn change(status: Delta) -> Change {
    match status {
        Delta::Added | Delta::Untracked => Change::Added,
        Delta::Deleted => Change::Deleted,
        Delta::Renamed => Change::Renamed,
        Delta::Copied => Change::Copied,
        Delta::Typechange => Change::TypeChanged,
        Delta::Conflicted => Change::Conflicted,
        Delta::Unreadable => Change::Unreadable,
        Delta::Modified | Delta::Unmodified | Delta::Ignored => Change::Modified,
    }
}
