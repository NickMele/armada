//! `~/.armada/daemon.jsonl` — what the daemon did that was **about no Job**
//! (`034` §6.5).
//!
//! Every daemon act that concerns one Job is recorded on that Job's own
//! record ([`armada_core::fleet::job::DaemonAct`]), because the Bridge draws
//! its detail pane off `armada fleet show` and never a second source. This
//! file is the other half: the daemon starting, stopping, and the machine
//! facts that are true of the daemon itself rather than of anything it acted
//! on — `armada doctor` reads it to answer *is the daemon running and what
//! did it last do*.
//!
//! **Append-only, folded on read, and a torn last line is skipped rather
//! than fatal** — the identical discipline [`crate::inbox`] and
//! `armada_core::failure` already use, for the same reason: a file that is
//! only ever appended to can still end mid-write if the machine lost power,
//! and one torn last line must not hide the entries before it.
//!
//! **No lock.** The daemon is the file's only writer — one process, per
//! `034` §1 — so the two-writers problem [`crate::inbox`]'s doc comment
//! argues against does not arise here; `OpenOptions::append` is sufficient on
//! its own.

use armada_core::error::{ArmadaError, ErrClass};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// One fact the daemon recorded about itself, or about the machine it is
/// running on — never about a Job, which is [`armada_core::fleet::job::DaemonAct`]'s
/// alone (`034` §6.5).
///
/// **Three shapes, and each line already is the entry.** Unlike the inbox,
/// nothing here is raised and later answered — a line is a complete fact the
/// moment it is written, so reading the file back is a filter over what
/// parses rather than a fold across several lines naming one id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    /// The daemon started.
    Started {
        /// When, wall clock, RFC 3339.
        at: String,
        /// Wall clock milliseconds, so "started 9m ago" is a subtraction.
        at_ms: u64,
        /// The pid it started as.
        pid: u32,
    },
    /// The daemon stopped.
    Stopped {
        /// When, wall clock, RFC 3339.
        at: String,
        /// Wall clock milliseconds.
        at_ms: u64,
        /// Why — `armada daemon disable`, a signal, or the reason it is about
        /// to exit having noticed something fatal.
        reason: String,
    },
    /// `gh` could not be reached — a machine fact, not a Job fact, so it
    /// lands here rather than as a `daemon_acts` entry on whichever Job's
    /// push or PR was in flight when it happened.
    GhUnreachable {
        /// When, wall clock, RFC 3339.
        at: String,
        /// Wall clock milliseconds.
        at_ms: u64,
        /// What `gh` said, or that it was not found on `PATH` at all.
        detail: String,
    },
}

/// Append one line.
pub fn append(armada_home: &Path, entry: &Entry) -> Result<(), ArmadaError> {
    let path = crate::home::daemon_log(armada_home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| unreadable(&path, &e))?;
    }
    let mut text = serde_json::to_string(entry).map_err(|e| ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: "daemon_log".to_string(),
        message: format!("a daemon log line would not serialize: {e}"),
        next_action: None,
    })?;
    text.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| unreadable(&path, &e))?;
    file.write_all(text.as_bytes())
        .map_err(|e| unreadable(&path, &e))
}

/// Every line that parses, oldest first.
///
/// **A line that does not parse is skipped**, not fatal — the file is only
/// ever appended to, and a crash can still leave its last line torn. The
/// entries before it are still real.
pub fn fold(text: &str) -> Vec<Entry> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Entry>(line).ok())
        .collect()
}

/// The most recent entry, if there is one — what `armada doctor` reports as
/// *what the daemon last did*.
pub fn last(entries: &[Entry]) -> Option<&Entry> {
    entries.last()
}

/// Every entry on disk, oldest first.
///
/// **An absent file is an empty log, not an error** — the same rule the
/// inbox follows, for a machine that has never enabled the daemon.
pub fn read(armada_home: &Path) -> Result<Vec<Entry>, ArmadaError> {
    let path = crate::home::daemon_log(armada_home);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(unreadable(&path, &e)),
    };
    Ok(fold(&text))
}

fn unreadable(path: &Path, error: &std::io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("the daemon log is not usable: {error}"),
        next_action: Some("check ~/.armada/ is writable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn a_daemon_log_that_does_not_exist_yet_is_empty_rather_than_broken() {
        let home = scratch();
        assert!(read(home.path()).unwrap().is_empty());
    }

    #[test]
    fn a_started_line_comes_back_as_the_entry_it_was_written() {
        let home = scratch();
        append(
            home.path(),
            &Entry::Started {
                at: "2026-08-17T09:00:00Z".to_string(),
                at_ms: 1_000,
                pid: 4212,
            },
        )
        .unwrap();
        let entries = read(home.path()).unwrap();
        assert_eq!(
            entries,
            vec![Entry::Started {
                at: "2026-08-17T09:00:00Z".to_string(),
                at_ms: 1_000,
                pid: 4212,
            }]
        );
    }

    /// **The three shapes round-trip distinctly**, so a reader folding the
    /// file back never confuses a stop for a start.
    #[test]
    fn all_three_shapes_round_trip() {
        let home = scratch();
        append(
            home.path(),
            &Entry::Started {
                at: "t1".to_string(),
                at_ms: 1,
                pid: 100,
            },
        )
        .unwrap();
        append(
            home.path(),
            &Entry::GhUnreachable {
                at: "t2".to_string(),
                at_ms: 2,
                detail: "gh: command not found".to_string(),
            },
        )
        .unwrap();
        append(
            home.path(),
            &Entry::Stopped {
                at: "t3".to_string(),
                at_ms: 3,
                reason: "armada daemon disable".to_string(),
            },
        )
        .unwrap();

        let entries = read(home.path()).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(matches!(entries[0], Entry::Started { .. }));
        assert!(matches!(entries[1], Entry::GhUnreachable { .. }));
        assert!(matches!(entries[2], Entry::Stopped { .. }));
        assert_eq!(last(&entries), Some(&entries[2]));
    }

    /// **A torn last line does not hide the entries before it** — mirroring
    /// `crate::inbox`'s own test of the same fact, for the same reason: the
    /// file is only ever appended to, and a crash can leave its last write
    /// half-finished.
    #[test]
    fn a_torn_last_line_does_not_hide_the_entries_before_it() {
        let home = scratch();
        append(
            home.path(),
            &Entry::Started {
                at: "t1".to_string(),
                at_ms: 1,
                pid: 100,
            },
        )
        .unwrap();
        let path = crate::home::daemon_log(home.path());
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"{\"type\":\"stop").unwrap();

        let entries = read(home.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0], Entry::Started { .. }));
    }

    /// `last` on an empty log is `None`, not a panic.
    #[test]
    fn last_on_an_empty_log_is_none() {
        assert_eq!(last(&[]), None);
    }
}
