//! A worktree as it stood before a person's run, and putting it back.
//!
//! **Never a discard.** A Job's work is uncommitted in its worktree until
//! delivery (`fleet::landing`), so a checkout would take the Drone's work with
//! the run's. [`undo`] writes back only the paths the run changed, from the tree
//! taken before it, and refuses where any of them has moved since.
//!
//! **Beside the worktree's own state, never in it.** Both trees are read
//! through an index of their own, so the worktree's index, HEAD and branch are
//! untouched; a ref under [`PREFIX`] keeps them reachable until retention.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use adapter_traits::ChangedFile;
use git2::{Commit, DiffOptions, IndexAddOption, Oid, Repository, Signature, Sort, Time, Tree};

use crate::commit::WHO;
use crate::work_product::change;

/// Where a run's snapshot is kept. A namespace no branch or tag lives in.
pub const PREFIX: &str = "refs/armada/rehearsals/";

/// How far back [`last_touched`] walks before it answers that it did not find
/// one. A bound on a read made every time the run sheet opens.
const WALKED: usize = 10_000;

static SCRATCH: AtomicU64 = AtomicU64::new(0);

/// The tree before a run, held until the run ends.
///
/// **No method discards anything.** What can be done with one is settle it,
/// which records the tree after; restoring is [`undo`], by reference.
pub struct Snapshot {
    worktree: PathBuf,
    reference: String,
    before: Oid,
}

/// What a run changed, and where the two trees are kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settled {
    pub reference: String,
    pub changed: Vec<ChangedFile>,
}

/// A snapshot that could not be taken, read or kept.
#[derive(Debug)]
pub enum SnapshotError {
    NotARepository {
        worktree: String,
        cause: git2::Error,
    },
    Git {
        worktree: String,
        doing: &'static str,
        cause: git2::Error,
    },
    /// The name is not one this module wrote.
    NoSuchSnapshot { reference: String },
}

/// Why [`undo`] put nothing back.
#[derive(Debug)]
pub enum NotUndone {
    Unreadable(SnapshotError),
    /// These paths no longer hold what the run left, so writing the older
    /// version over them would take somebody's later work.
    Moved { paths: Vec<String> },
    /// The first path that would not write. The ones before it were restored.
    NotRestored {
        path: String,
        cause: std::io::Error,
    },
}

/// Take the tree as it stands, before a run, under `run`'s name.
pub fn snapshot(worktree: &Path, run: &str, at_seconds: i64) -> Result<Snapshot, SnapshotError> {
    let reference = named(run)?;
    let repo = open(worktree)?;
    let tree = as_it_stands(&repo, worktree)?;
    let head = repo.head().and_then(|head| head.peel_to_commit()).ok();
    let before = commit_over(&repo, worktree, tree, head.as_ref(), "before a run", at_seconds)?;
    repo.reference(&reference, before, true, "a run's snapshot")
        .map_err(|cause| git(worktree, "keeping the snapshot", cause))?;
    Ok(Snapshot {
        worktree: worktree.to_path_buf(),
        reference,
        before,
    })
}

impl Snapshot {
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Take the tree again after the run, and say what moved between the two.
    pub fn settle(self, at_seconds: i64) -> Result<Settled, SnapshotError> {
        let worktree = self.worktree.as_path();
        let repo = open(worktree)?;
        let tree = as_it_stands(&repo, worktree)?;
        let before = repo
            .find_commit(self.before)
            .map_err(|cause| git(worktree, "reading the snapshot back", cause))?;
        let after = commit_over(&repo, worktree, tree, Some(&before), "after a run", at_seconds)?;
        repo.reference(&self.reference, after, true, "a run's snapshot, settled")
            .map_err(|cause| git(worktree, "keeping the snapshot", cause))?;
        let changed = between(&repo, worktree, &before, after)?;
        Ok(Settled {
            reference: self.reference,
            changed,
        })
    }
}

/// Put back what one run changed, from the tree taken before it.
///
/// Answers with the paths restored. Refused, with nothing written, where any
/// of them no longer holds what the run left.
pub fn undo(worktree: &Path, reference: &str) -> Result<Vec<ChangedFile>, NotUndone> {
    let unreadable = NotUndone::Unreadable;
    let repo = open(worktree).map_err(unreadable)?;
    let after = settled_commit(&repo, reference).map_err(unreadable)?;
    let before = after
        .parent(0)
        .map_err(|cause| unreadable(git(worktree, "reading the snapshot back", cause)))?;
    let changed = between(&repo, worktree, &before, after.id()).map_err(unreadable)?;
    let now = as_it_stands(&repo, worktree)
        .and_then(|tree| found_tree(&repo, worktree, tree))
        .map_err(unreadable)?;
    let (before, after) = (
        before
            .tree()
            .map_err(|cause| unreadable(git(worktree, "reading the snapshot back", cause)))?,
        after
            .tree()
            .map_err(|cause| unreadable(git(worktree, "reading the snapshot back", cause)))?,
    );
    let moved: Vec<String> = changed
        .iter()
        .filter(|file| entry(&now, file.path()) != entry(&after, file.path()))
        .map(|file| file.path().to_string())
        .collect();
    if !moved.is_empty() {
        return Err(NotUndone::Moved { paths: moved });
    }
    // Removals first, so a directory the run made where a file was is empty by
    // the time the file is written back.
    let (restore, remove): (Vec<&ChangedFile>, Vec<&ChangedFile>) = changed
        .iter()
        .partition(|file| before.get_path(Path::new(file.path())).is_ok());
    for file in remove {
        removed(&worktree.join(file.path())).map_err(|cause| NotUndone::NotRestored {
            path: file.path().to_string(),
            cause,
        })?;
    }
    for file in restore {
        written(&repo, worktree, &before, file.path())?;
    }
    Ok(changed)
}

/// Let a snapshot go. A name that is already gone is not a failure.
pub fn forget(worktree: &Path, reference: &str) -> Result<(), SnapshotError> {
    if !reference.starts_with(PREFIX) {
        return Err(SnapshotError::NoSuchSnapshot {
            reference: reference.to_string(),
        });
    }
    let repo = open(worktree)?;
    let forgotten = match repo.find_reference(reference) {
        Ok(mut found) => found
            .delete()
            .map_err(|cause| git(worktree, "letting a snapshot go", cause)),
        Err(_) => Ok(()),
    };
    forgotten
}

/// When `file` was last changed by a commit made at or before `at_seconds`,
/// in seconds since the epoch. `None` where no commit in reach touched it.
pub fn last_touched(
    repo: &Path,
    file: &str,
    at_seconds: i64,
) -> Result<Option<i64>, SnapshotError> {
    let opened = open(repo)?;
    let walking = |cause| git(repo, "walking the history", cause);
    let mut walk = opened.revwalk().map_err(walking)?;
    if walk.push_head().is_err() {
        return Ok(None);
    }
    walk.set_sorting(Sort::TIME).map_err(walking)?;
    for oid in walk.take(WALKED) {
        let commit = opened.find_commit(oid.map_err(walking)?).map_err(walking)?;
        if commit.time().seconds() > at_seconds {
            continue;
        }
        let here = commit.tree().ok().and_then(|tree| blob_at(&tree, file));
        let parent = commit
            .parent(0)
            .ok()
            .and_then(|parent| parent.tree().ok())
            .and_then(|tree| blob_at(&tree, file));
        if here != parent {
            return Ok(Some(commit.time().seconds()));
        }
    }
    Ok(None)
}

fn blob_at(tree: &Tree<'_>, file: &str) -> Option<Oid> {
    tree.get_path(Path::new(file)).ok().map(|found| found.id())
}

/// The ref for one run. **One path component**, because the name comes from a
/// record on disk and must not reach a ref this module did not write.
fn named(run: &str) -> Result<String, SnapshotError> {
    let one = !run.is_empty() && run.chars().all(|c| c.is_ascii_alphanumeric());
    match one {
        true => Ok(format!("{PREFIX}{run}")),
        false => Err(SnapshotError::NoSuchSnapshot {
            reference: run.to_string(),
        }),
    }
}

fn settled_commit<'r>(repo: &'r Repository, reference: &str) -> Result<Commit<'r>, SnapshotError> {
    let gone = || SnapshotError::NoSuchSnapshot {
        reference: reference.to_string(),
    };
    let run = reference.strip_prefix(PREFIX).ok_or_else(gone)?;
    named(run)?;
    repo.find_reference(reference)
        .and_then(|found| found.peel_to_commit())
        .map_err(|_| gone())
}

fn open(worktree: &Path) -> Result<Repository, SnapshotError> {
    Repository::open(worktree).map_err(|cause| SnapshotError::NotARepository {
        worktree: worktree.display().to_string(),
        cause,
    })
}

fn git(worktree: &Path, doing: &'static str, cause: git2::Error) -> SnapshotError {
    SnapshotError::Git {
        worktree: worktree.display().to_string(),
        doing,
        cause,
    }
}

/// Everything git can see in the worktree, untracked files included, as a tree.
///
/// **Through a scratch copy of the worktree's index**, seeded from the real one
/// so an unchanged file is not hashed again, and removed after. The real index
/// is never written.
fn as_it_stands(repo: &Repository, worktree: &Path) -> Result<Oid, SnapshotError> {
    let scratch = repo.path().join(format!(
        "armada-run-{}-{}.index",
        std::process::id(),
        SCRATCH.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::copy(repo.path().join("index"), &scratch);
    let read = (|| {
        let mut index = git2::Index::open(&scratch)?;
        repo.set_index(&mut index)?;
        index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
        index.update_all(["*"], None)?;
        index.write_tree()
    })();
    let _ = std::fs::remove_file(&scratch);
    read.map_err(|cause| git(worktree, "reading the worktree as a tree", cause))
}

fn found_tree<'r>(
    repo: &'r Repository,
    worktree: &Path,
    tree: Oid,
) -> Result<Tree<'r>, SnapshotError> {
    repo.find_tree(tree)
        .map_err(|cause| git(worktree, "reading the worktree as a tree", cause))
}

fn commit_over(
    repo: &Repository,
    worktree: &Path,
    tree: Oid,
    parent: Option<&Commit<'_>>,
    message: &str,
    at_seconds: i64,
) -> Result<Oid, SnapshotError> {
    let kept = |cause| git(worktree, "keeping the snapshot", cause);
    let tree = found_tree(repo, worktree, tree)?;
    let who = Signature::new(WHO.0, WHO.1, &Time::new(at_seconds, 0)).map_err(kept)?;
    let parents: Vec<&Commit<'_>> = parent.into_iter().collect();
    repo.commit(None, &who, &who, message, &tree, &parents)
        .map_err(kept)
}

/// The paths that differ between a snapshot and the commit taken after it.
fn between(
    repo: &Repository,
    worktree: &Path,
    before: &Commit<'_>,
    after: Oid,
) -> Result<Vec<ChangedFile>, SnapshotError> {
    let reading = |cause| git(worktree, "comparing the two trees", cause);
    let old = before.tree().map_err(reading)?;
    let new = repo
        .find_commit(after)
        .and_then(|commit| commit.tree())
        .map_err(reading)?;
    let mut options = DiffOptions::new();
    options.include_typechange(true);
    let diff = repo
        .diff_tree_to_tree(Some(&old), Some(&new), Some(&mut options))
        .map_err(reading)?;
    Ok(diff
        .deltas()
        .filter_map(|delta| {
            let path = delta.new_file().path().or_else(|| delta.old_file().path())?;
            Some(ChangedFile::new(
                path.to_string_lossy().into_owned(),
                change(delta.status()),
            ))
        })
        .collect())
}

/// What a tree holds at a path, as a value two trees can be compared on.
fn entry(tree: &Tree<'_>, path: &str) -> Option<(Oid, i32)> {
    tree.get_path(Path::new(path))
        .ok()
        .map(|found| (found.id(), found.filemode()))
}

fn removed(target: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(target) {
        Ok(held) if held.is_dir() => std::fs::remove_dir(target),
        Ok(_) => std::fs::remove_file(target),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(why),
    }
}

/// Write one path back as the snapshot held it: its bytes, and its mode.
fn written(
    repo: &Repository,
    worktree: &Path,
    before: &Tree<'_>,
    path: &str,
) -> Result<(), NotUndone> {
    use std::os::unix::fs::PermissionsExt;
    let failed = |cause| NotUndone::NotRestored {
        path: path.to_string(),
        cause,
    };
    let unreadable = |cause| NotUndone::Unreadable(git(worktree, "reading the snapshot back", cause));
    let held = before.get_path(Path::new(path)).map_err(unreadable)?;
    let blob = repo.find_blob(held.id()).map_err(unreadable)?;
    let target = worktree.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(failed)?;
    }
    removed(&target).map_err(failed)?;
    if held.filemode() == 0o120000 {
        use std::os::unix::ffi::OsStrExt;
        let points_at = std::ffi::OsStr::from_bytes(blob.content());
        return std::os::unix::fs::symlink(points_at, &target).map_err(failed);
    }
    std::fs::write(&target, blob.content()).map_err(failed)?;
    let mode = match held.filemode() {
        0o100755 => 0o755,
        _ => 0o644,
    };
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode)).map_err(failed)
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::NotARepository { worktree, .. } => {
                write!(out, "{worktree} is not a repository git can open")
            }
            SnapshotError::Git {
                worktree, doing, ..
            } => write!(out, "git refused {doing} in {worktree}"),
            SnapshotError::NoSuchSnapshot { reference } => {
                write!(out, "no snapshot is kept under `{reference}`")
            }
        }
    }
}

impl std::error::Error for SnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SnapshotError::NotARepository { cause, .. } | SnapshotError::Git { cause, .. } => {
                Some(cause)
            }
            SnapshotError::NoSuchSnapshot { .. } => None,
        }
    }
}

impl std::fmt::Display for NotUndone {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotUndone::Unreadable(why) => write!(out, "{why}"),
            NotUndone::Moved { paths } => write!(
                out,
                "{} changed again since the run, so putting the run's snapshot back would take \
                 that later work",
                paths.join(", ")
            ),
            NotUndone::NotRestored { path, cause } => {
                write!(out, "{path} could not be written back: {cause}")
            }
        }
    }
}

impl std::error::Error for NotUndone {}
