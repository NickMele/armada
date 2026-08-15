//! `~/.armada/recent.jsonl` — **the last few runs, so a report can attach one.**
//!
//! The format and the bound are [`armada_core::recent`]; this is the file. It
//! sits beside `failures.jsonl` and `inbox.jsonl`, in Manifest and for the same
//! reason [`crate::failures`] gives: `~/.armada/` is Manifest's, and this is the
//! lowest layer that may open a file at all (`ARCHITECTURE.md` §1.5).
//!
//! ## Rewritten, where the failure log is appended — and why the difference
//!
//! The failure log is append-only because *the thing it records is the crash*,
//! so it must survive one. This file records runs that already ended, and its
//! whole design is that it is **bounded** — see [`armada_core::recent::KEEP`],
//! where the bound is half the argument for the file existing at all. An
//! append-only file that has to be compacted is two mechanisms; a file rewritten
//! at ten lines is one.
//!
//! **Written through a temporary and renamed**, so a machine that dies mid-write
//! keeps the previous ten runs rather than a torn file. `parse` skips a torn line
//! anyway; the rename means it will not have to.
//!
//! ## Every failure here is silent
//!
//! Nothing in this module returns an error. Recording a run is a side effect of
//! a run that has already finished and already printed its answer — the rule
//! [`crate::failures`] states, arriving at a second file: **a recorder that
//! turns a working command into a failing one is worse than no recorder.** So
//! the write returns `bool`, and only `armada report`, which a person invoked
//! deliberately, is allowed to notice that the buffer was empty.

use armada_core::recent::Ran;
use std::path::{Path, PathBuf};

/// `~/.armada/recent.jsonl`.
pub fn path(armada_home: &Path) -> PathBuf {
    armada_home.join("recent.jsonl")
}

/// The runs the file holds, most recent first.
///
/// **An absent file is an empty buffer, not an error.** It is what every machine
/// looks like before Armada has been run on it, and a report filed there is
/// still a report — with one fewer attachment and a reader who can see that.
pub fn read(path: &Path) -> Vec<Ran> {
    match std::fs::read_to_string(path) {
        Ok(text) => armada_core::recent::parse(&text),
        Err(_) => Vec::new(),
    }
}

/// Replace the buffer. **True if it landed**, and nothing at all if it did not.
pub fn write(path: &Path, runs: &[Ran]) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    // **A temporary named for this process**, so two Armadas finishing at the
    // same moment do not write over each other's half-written file. The loser
    // of the rename race loses its own entry, which is the mild failure of the
    // two available — the alternative is a buffer that sometimes cannot be
    // parsed at all.
    let temporary = path.with_extension(format!("jsonl.{}", std::process::id()));
    if std::fs::write(&temporary, armada_core::recent::render(runs)).is_err() {
        let _ = std::fs::remove_file(&temporary);
        return false;
    }
    if std::fs::rename(&temporary, path).is_err() {
        let _ = std::fs::remove_file(&temporary);
        return false;
    }
    true
}

/// Read, roll this run in, write back. **The whole of what the entrypoint
/// does**, so that the ordering cannot be got wrong at a call site.
pub fn record(path: &Path, latest: Ran) -> bool {
    write(path, &armada_core::recent::roll(read(path), latest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> String {
        text.to_string()
    }

    fn ran(verb: &str, at_ms: u64) -> Ran {
        armada_core::recent::note(
            &[verb.to_string()],
            Path::new("/scratch/home"),
            Path::new("/scratch/home/code/api"),
            0,
            Some("{\"status\":\"ok\"}"),
            "2026-08-15T09:00:00Z",
            at_ms,
            &plain,
        )
    }

    fn scratch() -> (tempfile::TempDir, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let path = path(&home.path().join(".armada"));
        (home, path)
    }

    /// A machine Armada has never run on has an empty buffer, and asking is not
    /// an error there.
    #[test]
    fn a_buffer_that_does_not_exist_yet_is_empty_rather_than_broken() {
        let (_home, path) = scratch();
        assert!(read(&path).is_empty());
    }

    /// The directory under `~/.armada/` is made on the way, because the first
    /// run on a machine precedes `armada init`.
    #[test]
    fn the_first_run_creates_the_file_it_needs() {
        let (_home, path) = scratch();
        assert!(record(&path, ran("doctor", 1_000)));
        assert_eq!(read(&path).len(), 1);
    }

    /// **The file is bounded on disk, not only in memory.** Twelve runs leave
    /// ten lines, which is the property the privacy argument rests on.
    #[test]
    fn the_file_never_holds_more_than_the_bound() {
        let (_home, path) = scratch();
        for step in 0..12 {
            assert!(record(&path, ran("doctor", step)));
        }
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), armada_core::recent::KEEP);
        assert_eq!(read(&path)[0].at_ms, 11, "most recent first");
    }

    /// **A write that cannot happen is not an error anybody is told about.** The
    /// run it describes has already answered.
    #[test]
    fn a_buffer_that_cannot_be_written_reports_false_and_nothing_else() {
        let home = tempfile::tempdir().unwrap();
        let blocked = home.path().join("blocked");
        std::fs::write(&blocked, "not a directory").unwrap();
        assert!(!record(&path(&blocked), ran("doctor", 1)));
    }

    /// No temporary is left behind on the ordinary path.
    #[test]
    fn the_rename_leaves_nothing_beside_the_file() {
        let (home, path) = scratch();
        assert!(record(&path, ran("bridge", 1)));
        let beside: Vec<_> = std::fs::read_dir(home.path().join(".armada"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(beside, vec![std::ffi::OsString::from("recent.jsonl")]);
    }
}
