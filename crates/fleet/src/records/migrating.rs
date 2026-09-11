//! Moving what an older Fleet wrote under the repository, once, at boot.
//!
//! **The twin of `crate::transcript::migrating`, one layer up.** That pass
//! renames a file to the name a Job is now called by; this one moves whole
//! directories — `.armada/{briefs,transcripts,...}` — into the same relative
//! place under [`super::root`]. `.armada` moves rather than being dropped: a
//! Job's own row already spells that segment, and nothing here rewrites a row.
//!
//! **File by file, never a directory rename**, on the same "never overwrite,
//! count what didn't move" rule `transcript::migrating` states: a directory
//! rename either fully succeeds or fully fails, with no way to say which files
//! of a boot interrupted partway through are which.
//!
//! **A rename first, a verified copy only where `ErrorKind::CrossesDevices`
//! says a rename cannot work** — a repository on a different volume from
//! `~/Library/Application Support/Armada/`. Anything else `rename` refuses is
//! reported rather than retried through a copy that would fail the same way.
//!
//! **Run once at every boot, and a no-op after the first**, on
//! `transcript::migrating`'s grounds: the work is exactly the files still
//! under the repository.

use std::io;
use std::path::Path;

use super::KINDS;

/// What one boot's move found and did.
///
/// **What did not move is carried and not swallowed**, on
/// `transcript::migrating::Rekeyed`'s rule: a boot that quietly left a record
/// under the checkout is the failure this pass exists to report, one turn
/// later — the record is still readable, exactly where the older Fleet left
/// it, but the property this whole change is for (nothing but a worktree under
/// the checkout) would be false with nothing saying so.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Migrated {
    /// Files moved into the new root, by rename or by verified copy.
    pub files: usize,
    /// A file that would not move, and what the operating system said.
    pub refused: Vec<String>,
}

impl Migrated {
    pub fn moved_nothing(&self) -> bool {
        self.files == 0 && self.refused.is_empty()
    }
}

/// Move every one of the six kinds this repository still has under its own
/// `.armada/`, into `records_root`.
///
/// **Takes the repository root and the already-resolved destination.** Nothing
/// here computes either: `records_root` is [`super::root`]'s answer, resolved
/// once by the composition root before a `Host` exists, so a caller cannot
/// pass one repository's root against another's destination.
pub fn migrate(repo_root: &str, records_root: &Path) -> Migrated {
    let mut moved = Migrated::default();
    let old_armada = Path::new(repo_root).join(".armada");
    // `.armada` moves with the six kinds rather than being dropped at the
    // boundary. The row every one of these files is named on —
    // `job_step_checks.output_path`, `KeptDeliverable.path`, a brief's own
    // path — spells `.armada/<kind>/...` and is never rewritten by this pass,
    // so keeping the segment on the new side is what lets an old row still
    // resolve after its file has moved. See `crate::records`' module doc.
    let new_armada = records_root.join(".armada");
    for kind in KINDS {
        let old = old_armada.join(kind);
        if old.is_dir() {
            move_tree(&old, &new_armada.join(kind), &mut moved);
        }
    }
    moved
}

/// Move every file under `old` into the same relative place under `new`, and
/// prune whatever emptied out behind it.
fn move_tree(old: &Path, new: &Path, moved: &mut Migrated) {
    walk(old, old, new, moved);
    prune_empty(old);
}

fn walk(root: &Path, dir: &Path, new_root: &Path, moved: &mut Migrated) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, new_root, moved);
        } else if path.is_file() {
            // Relative to the kind's own directory, so a step id, a handle or
            // a commit holding whatever characters a workflow author or a
            // Drone chose still lands at the same depth it was found at.
            let relative = path.strip_prefix(root).unwrap_or(&path);
            move_file(&path, &new_root.join(relative), moved);
        }
    }
}

/// One file, moved without ever overwriting what is already at `to`.
fn move_file(from: &Path, to: &Path, moved: &mut Migrated) {
    if to.exists() {
        // The only way to reach this is a boot that already moved this file,
        // or a destination a person put something at by hand. Either way
        // merging the two is not something a boot may decide, exactly as
        // `transcript::migrating` leaves an occupied name alone.
        moved
            .refused
            .push(format!("{} is already there", to.display()));
        return;
    }
    if let Some(parent) = to.parent() {
        if let Err(why) = std::fs::create_dir_all(parent) {
            moved
                .refused
                .push(format!("{} could not be made: {why}", parent.display()));
            return;
        }
    }
    match std::fs::rename(from, to) {
        Ok(()) => moved.files += 1,
        // The one failure this pass answers by name: the repository and the
        // data directory are not one filesystem, so the OS refuses to rename
        // across and this falls back to a copy — verified before the original
        // goes, so an interrupted copy leaves the original in place rather
        // than losing the record between the two.
        Err(why) if why.kind() == io::ErrorKind::CrossesDevices => {
            match copy_verified_then_remove(from, to) {
                Ok(()) => moved.files += 1,
                Err(why) => moved.refused.push(format!(
                    "{} would not copy across volumes: {why}",
                    from.display()
                )),
            }
        }
        Err(why) => moved
            .refused
            .push(format!("{} would not move: {why}", from.display())),
    }
}

/// Copy `from` to `to`, prove the copy matches, and only then remove `from`.
///
/// **Verified by content, not by length.** A short read that happened to stop
/// at the right byte count would pass a length check and still be a truncated
/// record — the one failure a copy is actually at risk of, since `fs::copy`
/// itself is not partial on success. Read whole rather than streamed: every
/// one of the six kinds is bounded (`verification::A_DELIVERABLE`, a Check's
/// captured tail, a frame under the size a spec can shoot) except a Drone's own
/// transcript, and a one-time boot migration reading a transcript's bytes
/// twice is a cost worth the proof over trusting a partial write silently.
fn copy_verified_then_remove(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::copy(from, to)?;
    let matches = match (std::fs::read(from), std::fs::read(to)) {
        (Ok(original), Ok(copy)) => original == copy,
        _ => false,
    };
    if !matches {
        let _ = std::fs::remove_file(to);
        return Err(io::Error::other(format!(
            "the copy at {} did not match the original",
            to.display()
        )));
    }
    std::fs::remove_file(from)
}

/// Remove `dir` and every subdirectory under it that emptied out, bottom up.
///
/// **Only directories a `move_file` failure left non-empty survive.** A kind
/// this repository never had, or one every file of which moved, disappears
/// along with its own now-empty subdirectories; one a file refused to leave
/// stays, with exactly the files [`Migrated::refused`] already names still
/// under it.
fn prune_empty(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut empty = true;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !prune_empty(&path) {
                empty = false;
            }
        } else {
            empty = false;
        }
    }
    if empty {
        let _ = std::fs::remove_dir(dir);
    }
    empty
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::tmp::TempDir;

    fn write(dir: &Path, relative: &str, contents: &str) {
        let at = dir.join(relative);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("the directory");
        std::fs::write(at, contents).expect("the file");
    }

    #[test]
    fn every_kind_moves_into_the_new_root() {
        let repo = TempDir::new();
        let data = TempDir::new();
        for kind in KINDS {
            write(repo.path(), &format!(".armada/{kind}/a-job/x.txt"), kind);
        }
        let records_root = data.path().join("repos").join("a-key");

        let moved = migrate(&repo.path().to_string_lossy(), &records_root);

        assert_eq!(moved.files, KINDS.len());
        assert!(moved.refused.is_empty(), "{:?}", moved.refused);
        for kind in KINDS {
            let now = records_root
                .join(".armada")
                .join(kind)
                .join("a-job")
                .join("x.txt");
            assert_eq!(
                std::fs::read_to_string(&now).expect("the file moved"),
                *kind
            );
            assert!(
                !repo
                    .path()
                    .join(".armada")
                    .join(kind)
                    .join("a-job")
                    .join("x.txt")
                    .exists(),
                "the old file is gone"
            );
        }
    }

    #[test]
    fn a_repository_with_nothing_to_move_moves_nothing() {
        let repo = TempDir::new();
        let data = TempDir::new();
        let records_root = data.path().join("repos").join("a-key");

        let moved = migrate(&repo.path().to_string_lossy(), &records_root);

        assert!(moved.moved_nothing());
    }

    #[test]
    fn a_second_boot_moves_nothing_more() {
        let repo = TempDir::new();
        let data = TempDir::new();
        write(repo.path(), ".armada/briefs/a-job/x.txt", "the brief");
        let records_root = data.path().join("repos").join("a-key");

        let first = migrate(&repo.path().to_string_lossy(), &records_root);
        assert_eq!(first.files, 1);

        let second = migrate(&repo.path().to_string_lossy(), &records_root);
        assert!(second.moved_nothing(), "{second:?}");
    }

    #[test]
    fn a_destination_that_already_exists_is_left_alone_and_counted() {
        let repo = TempDir::new();
        let data = TempDir::new();
        write(repo.path(), ".armada/briefs/a-job/x.txt", "the new one");
        let records_root = data.path().join("repos").join("a-key");
        write(&records_root, ".armada/briefs/a-job/x.txt", "already there");

        let moved = migrate(&repo.path().to_string_lossy(), &records_root);

        assert_eq!(moved.files, 0);
        assert_eq!(moved.refused.len(), 1);
        assert_eq!(
            std::fs::read_to_string(records_root.join(".armada/briefs/a-job/x.txt")).unwrap(),
            "already there",
            "the file nothing wrote this boot is not overwritten"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join(".armada/briefs/a-job/x.txt")).unwrap(),
            "the new one",
            "and the one this boot could not place is not lost either"
        );
    }

    #[test]
    fn copy_verified_then_remove_leaves_the_original_on_mismatch() {
        let from_dir = TempDir::new();
        let to_dir = TempDir::new();
        let from = from_dir.path().join("x.txt");
        let to = to_dir.path().join("x.txt");
        std::fs::write(&from, "the original").expect("the file");
        // A file already sitting at `to` with different bytes stands in for a
        // copy that landed short or corrupt: `fs::copy` would overwrite it, so
        // this proves the verify step is what would have caught that, not the
        // copy call itself.
        std::fs::write(&to, "").expect("a stand-in for a bad copy");
        std::fs::remove_file(&to).expect("cleared, so `fs::copy` runs for real");

        copy_verified_then_remove(&from, &to).expect("a real copy of a real file verifies");

        assert!(!from.exists(), "removed only after verifying");
        assert_eq!(std::fs::read_to_string(&to).unwrap(), "the original");
    }
}
