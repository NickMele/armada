//! `~/.armada/failures.jsonl` — **Armada's record of its own failures**
//! (PLAN.md §15.3.5).
//!
//! The format and the fold are [`armada_core::failure`]; this is the file. It is
//! the same shape as the inbox and for the same stated reason: **append-only, so
//! it survives every kind of crash.** That property is not incidental here — the
//! thing being recorded *is* the crash, and a store that had to be rewritten
//! consistently would be at its least trustworthy at the one moment it is
//! written.
//!
//! ## Why this lives in Manifest
//!
//! `~/.armada/` is Manifest's ([`crate::machine::armada_home`]), and this is the
//! lowest layer that can open a file at all — the core may not
//! (`ARCHITECTURE.md` §1.5). It matters that it is the lowest: recording happens
//! at the entrypoint, on the path taken when **workspace resolution itself
//! failed**, so a recorder that lived in Fleet would make the one write that
//! must never fail depend on the module with the most machinery under it.
//!
//! ## One file for the machine, not one per repository
//!
//! Every entry carries the directory it happened in, so "the failures from this
//! repo" is a filter rather than a second store. Keying the file by project
//! instead would have meant resolving the project identity — a `git` call — on
//! the error path, before the write, in the one code path whose entire premise
//! is that something has already gone wrong. The machine-wide file needs
//! nothing but `$HOME`, which the entrypoint has already read.
//!
//! ## Every failure here is silent
//!
//! Nothing in this module returns an error to a caller who was reporting one.
//! **Recording must never change an exit code or swallow a failure**: the
//! failure still renders and still exits as it did before, and a logger that
//! turned a bug into a different bug would be worse than no logger. So the write
//! returns `bool` — it happened, or it did not — and only the reading verbs,
//! which a person invoked deliberately, are allowed to complain.

use armada_core::error::{ArmadaError, ErrClass};
use armada_core::failure::{Entry, Line};
use std::io::Write;
use std::path::{Path, PathBuf};

/// `~/.armada/failures.jsonl`.
///
/// **Beside `inbox.jsonl` rather than under a directory**, because it is one
/// file for the machine and a directory of one file is a directory somebody has
/// to explain.
pub fn path(armada_home: &Path) -> PathBuf {
    armada_home.join("failures.jsonl")
}

/// Append one line. **True if it landed**, and nothing at all if it did not.
///
/// The caller is on its way to reporting a failure and exiting by its class;
/// there is no useful thing it could do with a second failure, and the one
/// harmful thing — reporting this one instead — is what returning a `Result`
/// here would eventually tempt somebody into.
pub fn append(path: &Path, line: &Line) -> bool {
    let Ok(mut text) = serde_json::to_string(line) else {
        return false;
    };
    text.push('\n');
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return false;
    };
    file.write_all(text.as_bytes()).is_ok()
}

/// Every entry, folded, most recently seen first.
///
/// **An absent file is an empty log, not an error.** It is what every machine
/// looks like before Armada has failed on it, and `armada failures` exits 0
/// there.
pub fn read(path: &Path) -> Result<Vec<Entry>, ArmadaError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(unreadable(path, &e)),
    };
    Ok(armada_core::failure::fold(&text))
}

fn unreadable(path: &Path, error: &std::io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("the failure log is not readable: {error}"),
        next_action: Some("check ~/.armada/ is readable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::failure::State;

    fn scratch() -> (tempfile::TempDir, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let path = path(&home.path().join(".armada"));
        (home, path)
    }

    fn failure() -> ArmadaError {
        ArmadaError {
            class: ErrClass::Environment,
            r#where: "~/.cargo/bin/armada".to_string(),
            message: "`armada manifest clean` could not be found to run".to_string(),
            next_action: Some("reinstall armada, then retry unchanged".to_string()),
        }
    }

    fn record(path: &Path, at_ms: u64) -> String {
        let (id, line) = armada_core::failure::failed(
            &failure(),
            Path::new("/scratch/home"),
            Path::new("/scratch/home/code/api"),
            &["bridge".to_string()],
            "2026-08-14T09:00:00Z",
            at_ms,
        );
        assert!(append(path, &line));
        id
    }

    /// **A machine Armada has never failed on has an empty log**, and asking is
    /// not an error there.
    #[test]
    fn a_log_that_does_not_exist_yet_is_empty_rather_than_broken() {
        let (_home, path) = scratch();
        assert!(read(&path).unwrap().is_empty());
    }

    /// The directory under `~/.armada/` is made on the way, because the first
    /// failure on a machine can easily precede `armada init`.
    #[test]
    fn the_first_failure_creates_the_file_it_needs() {
        let (_home, path) = scratch();
        assert!(!path.exists());
        record(&path, 1_000);
        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].state, State::Open);
        assert_eq!(entries[0].count, 1);
    }

    /// **The same failure twice is one row with a count**, which is the property
    /// that makes this readable after a day of use.
    #[test]
    fn twice_recorded_is_one_entry_with_a_count_of_two() {
        let (_home, path) = scratch();
        let first = record(&path, 1_000);
        let second = record(&path, 2_000);
        assert_eq!(first, second);

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2, "both occurrences are on disk");

        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1, "two lines, one entry");
        assert_eq!(entries[0].count, 2);
    }

    /// Clearing appends rather than rewrites — the same rule the inbox's answer
    /// follows, and for the same reason: a rewrite is three chances to lose
    /// every other entry in the one file whose job is to lose nothing.
    #[test]
    fn clearing_appends_and_leaves_what_it_cleared_on_disk() {
        let (_home, path) = scratch();
        let id = record(&path, 1_000);
        assert!(append(&path, &Line::Cleared { id, at_ms: 2_000 }));

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("could not be found"),
            "nothing was overwritten"
        );
        assert_eq!(read(&path).unwrap()[0].state, State::Cleared);
    }

    /// **A write that cannot happen is not an error anybody is told about.** The
    /// caller is on its way to reporting a different failure and exiting by its
    /// class; there is nothing useful it could do with this one.
    #[test]
    fn a_log_that_cannot_be_written_reports_false_and_nothing_else() {
        let home = tempfile::tempdir().unwrap();
        // A file where the directory has to go: `create_dir_all` fails, and so
        // does everything after it.
        let blocked = home.path().join("blocked");
        std::fs::write(&blocked, "not a directory").unwrap();
        let path = path(&blocked);
        assert!(!append(
            &path,
            &Line::Cleared {
                id: "a1b2c3d4".to_string(),
                at_ms: 1
            }
        ));
    }

    /// An unreadable log is worth complaining about, because the only caller
    /// left is a person who asked to see it.
    #[test]
    fn a_log_that_is_a_directory_is_reported_to_the_reader() {
        let home = tempfile::tempdir().unwrap();
        let path = path(home.path());
        std::fs::create_dir_all(&path).unwrap();
        let error = read(&path).unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert!(error.next_action.is_some());
    }
}
