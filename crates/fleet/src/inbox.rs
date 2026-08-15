//! `~/.armada/inbox.jsonl` — **what the fleet needs from you** (PLAN.md §15.3).
//!
//! **Append-only, so it survives every kind of crash.** That is the same
//! reasoning that put Manifest's ownership store on disk rather than in a
//! process: a file that is only ever appended to has no half-written state a
//! reader can be confused by, and no lock two writers have to agree on.
//!
//! **Answering appends rather than rewrites.** A rewrite would mean reading the
//! whole file, editing one row and writing it back — three chances to lose every
//! other Job's entries, in the one file whose job is to never lose anything. So
//! an answer is its own line, and a reader folds the two.
//!
//! **Every entry has an id**, which is the half PLAN.md §15.3.1 says is missing
//! from anything Helm raises in prose: an item you cannot name is an item you
//! cannot acknowledge one row at a time.

use armada_core::error::{ArmadaError, ErrClass};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// Why a Job wants you.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A judgement call is yours, or a ceiling was reached.
    NeedsHuman,
    /// It cannot proceed without an external change.
    Blocked,
    /// A turn ended. **Raised by the `Stop` hook rather than by the Drone** —
    /// an agent can forget to report progress, but it cannot forget to stop,
    /// which is what makes "needs my attention" reliable rather than
    /// best-effort.
    Idle,
}

impl Kind {
    /// The word, in both audiences.
    pub const fn word(self) -> &'static str {
        match self {
            Kind::NeedsHuman => "needs_human",
            Kind::Blocked => "blocked",
            Kind::Idle => "idle",
        }
    }
}

/// One thing the fleet needs from you, folded from the lines that mention it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// The entry's own id — **the identity PLAN.md §15.3.1 is about.** It is
    /// what `armada fleet answer` acknowledges, and what a keystroke in the
    /// Bridge will bind to.
    pub uuid: String,
    /// The Job that raised it, by name.
    pub job: String,
    /// Why.
    pub kind: Kind,
    /// When, wall clock, RFC 3339.
    pub raised_at: String,
    /// Wall clock milliseconds, so "9m ago" is a subtraction.
    pub raised_ms: u64,
    /// What it wants to tell you.
    pub body: String,
    /// Your answer, once you have given one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered: Option<String>,
}

impl Entry {
    /// Whether this entry is still waiting on you.
    pub fn is_open(&self) -> bool {
        self.answered.is_none()
    }
}

/// One line of the file: an entry raised, or an entry answered.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Line {
    Raised {
        uuid: String,
        job: String,
        kind: Kind,
        raised_at: String,
        raised_ms: u64,
        body: String,
    },
    Answered {
        uuid: String,
        answer: String,
    },
}

/// Raise an entry. Returns the id it was given.
pub fn raise(
    path: &Path,
    id: &str,
    job: &str,
    kind: Kind,
    at: &str,
    at_ms: u64,
    body: &str,
) -> Result<String, ArmadaError> {
    append(
        path,
        &Line::Raised {
            uuid: id.to_string(),
            job: job.to_string(),
            kind,
            raised_at: at.to_string(),
            raised_ms: at_ms,
            body: body.to_string(),
        },
    )?;
    Ok(id.to_string())
}

/// Record an answer against an entry.
pub fn answer(path: &Path, id: &str, answer: &str) -> Result<(), ArmadaError> {
    append(
        path,
        &Line::Answered {
            uuid: id.to_string(),
            answer: answer.to_string(),
        },
    )
}

/// Every entry, oldest first, with its answer folded in.
///
/// **An absent file is an empty inbox, not an error** (`commands/fleet/inbox.md`).
pub fn read(path: &Path) -> Result<Vec<Entry>, ArmadaError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(unreadable(path, &e)),
    };

    let mut entries: Vec<Entry> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // **A line that does not parse is skipped.** A file that is only
        // appended to can still end mid-write if the machine lost power, and
        // one torn last line must not hide the entries before it.
        let Ok(parsed) = serde_json::from_str::<Line>(line) else {
            continue;
        };
        match parsed {
            Line::Raised {
                uuid,
                job,
                kind,
                raised_at,
                raised_ms,
                body,
            } => entries.push(Entry {
                uuid,
                job,
                kind,
                raised_at,
                raised_ms,
                body,
                answered: None,
            }),
            Line::Answered { uuid, answer } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.uuid == uuid) {
                    entry.answered = Some(answer);
                }
            }
        }
    }
    Ok(entries)
}

/// The oldest open entry for a Job, which is what `armada fleet answer`
/// answers.
///
/// **Oldest rather than newest.** A Job that asked twice is waiting on the first
/// question; answering the second and leaving the first would leave the Job
/// stuck on something it already told you about.
pub fn open_for<'a>(entries: &'a [Entry], job: &str) -> Option<&'a Entry> {
    entries
        .iter()
        .find(|entry| entry.job == job && entry.is_open())
}

fn append(path: &Path, line: &Line) -> Result<(), ArmadaError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| unreadable(parent, &e))?;
    }
    let mut text = serde_json::to_string(line).map_err(|e| ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: "inbox".to_string(),
        message: format!("an inbox line would not serialize: {e}"),
        next_action: None,
    })?;
    text.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| unreadable(path, &e))?;
    file.write_all(text.as_bytes())
        .map_err(|e| unreadable(path, &e))
}

fn unreadable(path: &Path, error: &std::io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("the inbox is not usable: {error}"),
        next_action: Some("check ~/.armada/ is writable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> (tempfile::TempDir, std::path::PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(".armada/inbox.jsonl");
        (home, path)
    }

    /// **An absent inbox is an empty one.** It is what every machine looks like
    /// before its first Job asks anything, and `armada fleet inbox` exits 0.
    #[test]
    fn an_inbox_that_does_not_exist_yet_is_empty_rather_than_broken() {
        let (_home, path) = scratch();
        assert!(read(&path).unwrap().is_empty());
    }

    #[test]
    fn a_raised_entry_comes_back_with_the_id_it_was_given() {
        let (_home, path) = scratch();
        raise(
            &path,
            "e1",
            "nightly-flake",
            Kind::NeedsHuman,
            "2026-08-09T14:02:11Z",
            1_000,
            "Wants to raise the CI timeout 30s to 90s.",
        )
        .unwrap();
        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, "e1");
        assert_eq!(entries[0].job, "nightly-flake");
        assert_eq!(entries[0].kind, Kind::NeedsHuman);
        assert!(entries[0].is_open());
    }

    /// **Answering appends rather than rewrites**, and the fold is what a reader
    /// sees. A rewrite is three chances to lose every other Job's entries in the
    /// one file whose job is to never lose anything.
    #[test]
    fn an_answer_is_a_second_line_that_the_reader_folds_in() {
        let (_home, path) = scratch();
        raise(&path, "e1", "flake", Kind::Blocked, "t", 1, "raise it?").unwrap();
        answer(&path, "e1", "yes, 90s").unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2, "the raise is still there:\n{text}");
        assert!(text.contains("raise it?"), "nothing was overwritten");

        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1, "two lines, one entry");
        assert_eq!(entries[0].answered.as_deref(), Some("yes, 90s"));
        assert!(!entries[0].is_open());
    }

    /// A torn last line is what a machine that lost power leaves behind. The
    /// entries before it are still real.
    #[test]
    fn a_torn_last_line_does_not_hide_the_entries_before_it() {
        let (_home, path) = scratch();
        raise(&path, "e1", "flake", Kind::Idle, "t", 1, "went idle").unwrap();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"{\"type\":\"rai").unwrap();
        assert_eq!(read(&path).unwrap().len(), 1);
    }

    /// **Oldest first.** A Job that asked twice is waiting on the first
    /// question; answering the second would leave it stuck on something it
    /// already told you about.
    #[test]
    fn the_entry_to_answer_is_the_oldest_open_one_for_that_job() {
        let (_home, path) = scratch();
        raise(&path, "e1", "flake", Kind::NeedsHuman, "t", 1, "first").unwrap();
        raise(&path, "e2", "flake", Kind::NeedsHuman, "t", 2, "second").unwrap();
        raise(&path, "e3", "other", Kind::NeedsHuman, "t", 3, "elsewhere").unwrap();

        let entries = read(&path).unwrap();
        assert_eq!(open_for(&entries, "flake").unwrap().uuid, "e1");

        answer(&path, "e1", "done").unwrap();
        let entries = read(&path).unwrap();
        assert_eq!(open_for(&entries, "flake").unwrap().uuid, "e2");
    }

    #[test]
    fn a_job_with_nothing_open_has_nothing_to_answer() {
        let (_home, path) = scratch();
        raise(&path, "e1", "flake", Kind::Idle, "t", 1, "idle").unwrap();
        answer(&path, "e1", "seen").unwrap();
        let entries = read(&path).unwrap();
        assert!(open_for(&entries, "flake").is_none());
        assert!(open_for(&entries, "nonesuch").is_none());
    }

    /// An answer against an id nobody raised changes nothing rather than
    /// inventing an entry. The file is a log, and a log may mention things that
    /// were never there.
    #[test]
    fn an_answer_to_an_unknown_id_leaves_the_inbox_as_it_was() {
        let (_home, path) = scratch();
        raise(&path, "e1", "flake", Kind::Idle, "t", 1, "idle").unwrap();
        answer(&path, "nonesuch", "hello").unwrap();
        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_open());
    }

    #[test]
    fn every_kind_is_spelled_the_same_in_both_audiences() {
        for (kind, word) in [
            (Kind::NeedsHuman, "needs_human"),
            (Kind::Blocked, "blocked"),
            (Kind::Idle, "idle"),
        ] {
            assert_eq!(kind.word(), word);
            assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{word}\""));
        }
    }
}
