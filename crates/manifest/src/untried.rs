//! `~/.armada/untried.jsonl` — **which of Armada's verbs this machine has run.**
//!
//! The format and the fold are [`armada_core::untried`]; this is the file. It
//! sits beside `recent.jsonl` and `failures.jsonl`, in Manifest and for the
//! reason [`crate::failures`] gives: `~/.armada/` is Manifest's, and this is the
//! lowest layer that may open a file at all (`ARCHITECTURE.md` §1.5).
//!
//! **Rewritten rather than appended**, the choice [`crate::recent`] makes and for
//! its reason: this file is bounded — one line per verb on the roster, never more
//! — so an append-only file that has to be compacted would be two mechanisms
//! where one will do. Through a temporary and renamed, so a machine that dies
//! mid-write keeps the counts it had.
//!
//! **Every failure here is silent.** Counting a run is a side effect of a run
//! that has already answered, and the rule the other two recorders state holds a
//! third time: *a recorder that turns a working command into a failing one is
//! worse than no recorder.* Nothing in this module returns an error.

use armada_core::untried::Seen;
use std::path::{Path, PathBuf};

/// `~/.armada/untried.jsonl`.
pub fn path(armada_home: &Path) -> PathBuf {
    armada_home.join("untried.jsonl")
}

/// What the file holds. **An absent file is an empty count, not an error** — it
/// is what every machine looks like before Armada has been run on it, and that
/// is a perfectly good answer to *what have I tried*.
pub fn read(path: &Path) -> Vec<Seen> {
    match std::fs::read_to_string(path) {
        Ok(text) => armada_core::untried::parse(&text),
        Err(_) => Vec::new(),
    }
}

/// Replace the counts. **True if it landed**, and nothing at all if it did not.
pub fn write(path: &Path, seen: &[Seen]) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    // A temporary named for this process, so two Armadas finishing at the same
    // moment do not write over each other's half-written file. The loser of the
    // rename race loses its own increment, which is the milder of the two
    // failures available — the alternative is counts that cannot be parsed.
    let temporary = path.with_extension(format!("jsonl.{}", std::process::id()));
    if std::fs::write(&temporary, armada_core::untried::render(seen)).is_err() {
        let _ = std::fs::remove_file(&temporary);
        return false;
    }
    if std::fs::rename(&temporary, path).is_err() {
        let _ = std::fs::remove_file(&temporary);
        return false;
    }
    true
}

/// Read, fold this run in, write back. **The whole of what the entrypoint
/// does**, so the ordering cannot be got wrong at a call site.
pub fn record(path: &Path, verb: &str, at_ms: u64, ok: bool) -> bool {
    write(
        path,
        &armada_core::untried::note(read(path), verb, at_ms, ok),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> (tempfile::TempDir, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let path = path(&home.path().join(".armada"));
        (home, path)
    }

    /// A machine Armada has never run on has counted nothing, and asking is not
    /// an error there.
    #[test]
    fn a_file_that_does_not_exist_yet_is_empty_rather_than_broken() {
        let (_home, path) = scratch();
        assert!(read(&path).is_empty());
    }

    /// **One line per verb, however many times it runs** — the bound the whole
    /// rewrite-rather-than-append choice rests on.
    #[test]
    fn the_file_holds_one_line_per_verb_and_never_grows_with_use() {
        let (_home, path) = scratch();
        for step in 0..12 {
            assert!(record(&path, "doctor", step, true));
        }
        assert!(record(&path, "failures", 99, true));
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2, "{text}");
        let seen = read(&path);
        let doctor = seen.iter().find(|entry| entry.verb == "doctor").unwrap();
        assert_eq!(doctor.count, 12);
        assert_eq!(doctor.last_ms, 11);
    }

    /// **A write that cannot happen is not an error anybody is told about.** The
    /// run it counts has already answered.
    #[test]
    fn counts_that_cannot_be_written_report_false_and_nothing_else() {
        let home = tempfile::tempdir().unwrap();
        let blocked = home.path().join("blocked");
        std::fs::write(&blocked, "not a directory").unwrap();
        assert!(!record(&path(&blocked), "doctor", 1, true));
    }

    /// No temporary is left behind on the ordinary path.
    #[test]
    fn the_rename_leaves_nothing_beside_the_file() {
        let (home, path) = scratch();
        assert!(record(&path, "bridge", 1, true));
        let beside: Vec<_> = std::fs::read_dir(home.path().join(".armada"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(beside, vec![std::ffi::OsString::from("untried.jsonl")]);
    }
}
