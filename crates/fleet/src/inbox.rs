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
//!
//! # An entry names its Job by uuid, and the name is only a label
//!
//! **This file used to key an entry by the Job's *name*, and that was a bug**
//! (`docs/reserved/005-inbox-label-not-identity.md`). A name is a handle a
//! person types and [`crate::jobs::Store::free_name`] hands out again once the
//! Job holding it is over — so two Jobs can be called `this-test`, and an entry
//! that says `this-test` cannot be resolved to either of them. It was found in
//! ordinary use: `armada fleet ls` reported *no Jobs* while `armada fleet inbox`
//! reported five open entries, and both were telling the truth.
//!
//! So [`Entry::job_uuid`] is the identity and [`Entry::job`] is a label that is
//! shown and never resolved against. This is
//! `docs/reserved/001-raised-items-need-identity.md` one level down — an item
//! raised against prose has no identity, and a name is prose with a narrower
//! grammar — and it is deliberately the *same* scheme 001 will generalise
//! rather than a second one.
//!
//! # An entry does not outlive its Job
//!
//! A Job that reaches `DONE` or `ABORTED` has nothing left to answer, so
//! [`close`] marks its open entries `ENDED` and they stop being open. **Marked
//! and retained rather than deleted**, because the file is a log: the reason a
//! Job stopped is often the last thing written about it, and `armada fleet
//! inbox --all` and `armada fleet show` both read entries back afterwards. A
//! delete would make the one record of why something ended disappear at the
//! moment it became history.

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
            Kind::NeedsHuman => "NEEDS_HUMAN",
            Kind::Blocked => "BLOCKED",
            Kind::Idle => "IDLE",
        }
    }
}

/// Why an entry stopped waiting on you without being answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Closed {
    /// Its Job reached `DONE` or `ABORTED`. There is nothing left to answer,
    /// and offering the answer anyway is the second half of the defect
    /// `docs/reserved/005-inbox-label-not-identity.md` records.
    Ended,
    /// **A legacy entry whose name could not be resolved to one Job**, found by
    /// [`migrate`]. It named zero Jobs, or it named several — which is the
    /// collision the whole change is about — and there is no honest way to
    /// guess which. It is kept and marked rather than deleted or left open: an
    /// entry that cannot name its Job cannot be acted on, and a list of things
    /// you cannot act on is what stops a list being trusted.
    Unresolvable,
}

impl Closed {
    /// The word, for a reader.
    pub const fn word(self) -> &'static str {
        match self {
            Closed::Ended => "ENDED",
            Closed::Unresolvable => "UNRESOLVABLE",
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
    /// **The Job that raised it — the identity, and the only field resolved
    /// against.**
    ///
    /// `None` for an entry written before this field existed, and only until
    /// [`migrate`] has run over it. Nothing else may produce a `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_uuid: Option<String>,
    /// The Job's name when the entry was raised — **a label, shown and never
    /// resolved against.** Names are handed out again once the Job holding one
    /// is over, so two entries carrying the same name may belong to two
    /// different Jobs.
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
    /// Why it stopped waiting on you without an answer, if it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<Closed>,
}

impl Entry {
    /// Whether this entry is still waiting on you.
    ///
    /// **An entry with no `job_uuid` is not open**, whatever else is true of
    /// it. It is a legacy line [`migrate`] has not reached yet, and until it
    /// has there is no Job the answer could reach.
    pub fn is_open(&self) -> bool {
        self.answered.is_none() && self.closed.is_none() && self.job_uuid.is_some()
    }

    /// Whether [`migrate`] still has work to do on this entry.
    pub fn is_legacy(&self) -> bool {
        self.job_uuid.is_none() && self.closed.is_none()
    }
}

/// One line of the file: an entry raised, answered, closed, or given the
/// identity it was written without.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Line {
    Raised {
        uuid: String,
        /// **Absent in every line written before `005` was fixed**, which is
        /// why it is `default` rather than required: the alternative is a file
        /// on somebody's machine that stops parsing, and a skipped line is an
        /// entry silently lost.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        job_uuid: Option<String>,
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
    Closed {
        uuid: String,
        why: Closed,
    },
    /// The identity a legacy `raised` line was written without, appended by
    /// [`migrate`] rather than edited in — the file is append-only, and a
    /// rewrite is three chances to lose every other Job's entries.
    Bound {
        uuid: String,
        job_uuid: String,
    },
}

/// Raise an entry. Returns the id it was given.
///
/// **`job_uuid` is the Job and `job` is its name**, in that order of
/// importance: the first is what the entry is resolved by, the second is only
/// what a reader is shown. A caller that has one has the other, because both
/// come off the same Job record.
pub fn raise(
    path: &Path,
    id: &str,
    job_uuid: &str,
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
            job_uuid: Some(job_uuid.to_string()),
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

/// Close every open entry a Job raised, because the Job has ended.
///
/// **Called where the terminal state is written, not where it is observed.**
/// `DONE` and `ABORTED` are the two states a verb writes deliberately
/// ([`armada_core::fleet::job::observe`] never invents either), so closing
/// beside that write is complete — and it keeps `armada fleet ls` a read.
///
/// Returns how many were closed, which is what a caller reports.
pub fn close(path: &Path, job_uuid: &str, why: Closed) -> Result<usize, ArmadaError> {
    let mut closed = 0;
    for entry in read(path)? {
        if entry.is_open() && entry.job_uuid.as_deref() == Some(job_uuid) {
            append(
                path,
                &Line::Closed {
                    uuid: entry.uuid,
                    why,
                },
            )?;
            closed += 1;
        }
    }
    Ok(closed)
}

/// What one [`migrate`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Migrated {
    /// Legacy entries whose name resolved to exactly one Job.
    pub bound: usize,
    /// Legacy entries whose name named no Job, or several.
    pub unresolvable: usize,
    /// Entries that resolved to a Job which has since ended.
    pub ended: usize,
}

impl Migrated {
    /// Whether anything was written.
    pub fn is_empty(self) -> bool {
        self.bound == 0 && self.unresolvable == 0 && self.ended == 0
    }
}

/// Give every legacy entry the identity it was written without.
///
/// `jobs` is `(name, uuid, is_over)`, one per Job on the machine — the whole
/// index, because that is the only thing that can say whether a name means one
/// Job or two.
///
/// **A name that means more than one Job is not guessed at.** That is the
/// entire defect: the user who found it had two Jobs called `this-test`, and
/// picking the newer, the older or the live one would be the same coin flip
/// `armada fleet show` already refuses to take ([`crate::jobs::Store::find`]).
/// Such an entry is closed [`Closed::Unresolvable`] — kept, marked, and never
/// offered an action that cannot work. A name matching no Job at all is the
/// same answer for the same reason.
///
/// **An entry that binds to a Job which has already ended is closed too**, in
/// the same pass. Otherwise the migration would hand a machine a set of
/// correctly-identified entries that are all still open against Jobs that are
/// over, which is the symptom rather than the fix.
///
/// **Append-only and idempotent.** Nothing is rewritten, and a second run finds
/// no legacy entries and writes nothing — so two processes racing on it append
/// the same lines and fold to the same result.
pub fn migrate(path: &Path, jobs: &[(String, String, bool)]) -> Result<Migrated, ArmadaError> {
    let mut done = Migrated::default();
    for entry in read(path)? {
        let bound = match &entry.job_uuid {
            Some(uuid) => uuid.clone(),
            None if entry.closed.is_some() => continue,
            None => {
                let named: Vec<&(String, String, bool)> =
                    jobs.iter().filter(|(name, _, _)| *name == entry.job).collect();
                let [(_, uuid, _)] = named.as_slice() else {
                    append(
                        path,
                        &Line::Closed {
                            uuid: entry.uuid,
                            why: Closed::Unresolvable,
                        },
                    )?;
                    done.unresolvable += 1;
                    continue;
                };
                append(
                    path,
                    &Line::Bound {
                        uuid: entry.uuid.clone(),
                        job_uuid: uuid.clone(),
                    },
                )?;
                done.bound += 1;
                uuid.clone()
            }
        };
        if entry.answered.is_some() || entry.closed.is_some() {
            continue;
        }
        if jobs
            .iter()
            .any(|(_, uuid, over)| *uuid == bound && *over)
        {
            append(
                path,
                &Line::Closed {
                    uuid: entry.uuid,
                    why: Closed::Ended,
                },
            )?;
            done.ended += 1;
        }
    }
    Ok(done)
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
                job_uuid,
                job,
                kind,
                raised_at,
                raised_ms,
                body,
            } => entries.push(Entry {
                uuid,
                job_uuid,
                job,
                kind,
                raised_at,
                raised_ms,
                body,
                answered: None,
                closed: None,
            }),
            Line::Answered { uuid, answer } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.uuid == uuid) {
                    entry.answered = Some(answer);
                }
            }
            Line::Closed { uuid, why } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.uuid == uuid) {
                    entry.closed = Some(why);
                }
            }
            Line::Bound { uuid, job_uuid } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.uuid == uuid) {
                    // **First binding wins.** Two migrations racing append the
                    // same uuid, so this is a formality — but a later line that
                    // disagreed would be re-pointing an entry at a different
                    // Job, which nothing is allowed to do.
                    entry.job_uuid.get_or_insert(job_uuid);
                }
            }
        }
    }
    Ok(entries)
}

/// The oldest open entry for a Job, which is what `armada fleet answer`
/// answers.
///
/// **By uuid, never by name** — that is the whole of
/// `docs/reserved/005-inbox-label-not-identity.md`. Answering by name meant a
/// second Job called `this-test` inherited the first one's questions, and
/// neither could be told which entries were its own.
///
/// **Oldest rather than newest.** A Job that asked twice is waiting on the first
/// question; answering the second and leaving the first would leave the Job
/// stuck on something it already told you about.
pub fn open_for<'a>(entries: &'a [Entry], job_uuid: &str) -> Option<&'a Entry> {
    entries
        .iter()
        .find(|entry| entry.job_uuid.as_deref() == Some(job_uuid) && entry.is_open())
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

    /// A raise with the two facts the tests below vary, in one place.
    fn raised(path: &Path, id: &str, job_uuid: &str, name: &str, at_ms: u64, body: &str) {
        raise(
            path,
            id,
            job_uuid,
            name,
            Kind::NeedsHuman,
            "2026-08-09T14:02:11Z",
            at_ms,
            body,
        )
        .unwrap();
    }

    /// A legacy line: what every entry on a machine looked like before `005`
    /// was fixed. Written as text rather than through [`raise`], because the
    /// writer that produced it no longer exists and the migration's real input
    /// is the file, not the API.
    fn legacy(path: &Path, id: &str, name: &str, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(
            file,
            r#"{{"type":"raised","uuid":"{id}","job":"{name}","kind":"needs_human","raised_at":"2026-08-09T14:02:11Z","raised_ms":1,"body":"{body}"}}"#
        )
        .unwrap();
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
        raised(
            &path,
            "e1",
            "c19d0a34-3069",
            "nightly-flake",
            1_000,
            "Wants to raise the CI timeout 30s to 90s.",
        );
        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, "e1");
        assert_eq!(entries[0].job_uuid.as_deref(), Some("c19d0a34-3069"));
        assert_eq!(entries[0].job, "nightly-flake");
        assert_eq!(entries[0].kind, Kind::NeedsHuman);
        assert!(entries[0].is_open());
    }

    /// **The bug, at the layer it lives in.** Two Jobs called `this-test` is
    /// what the user actually had — `armada fleet ls` said *no Jobs* while the
    /// inbox said five were open, all naming `this-test`, and no entry could be
    /// matched to either Job.
    ///
    /// Resolution is by uuid, so each Job sees its own and neither sees the
    /// other's.
    #[test]
    fn two_jobs_sharing_a_name_each_keep_their_own_entries() {
        let (_home, path) = scratch();
        raised(&path, "e1", "c19d0a34", "this-test", 1, "the first one asks");
        raised(&path, "e2", "94b1fd2e", "this-test", 2, "the second one asks");

        let entries = read(&path).unwrap();
        assert_eq!(open_for(&entries, "c19d0a34").unwrap().uuid, "e1");
        assert_eq!(open_for(&entries, "94b1fd2e").unwrap().uuid, "e2");

        // Answering one leaves the other exactly where it was.
        answer(&path, "e1", "yes").unwrap();
        let entries = read(&path).unwrap();
        assert!(open_for(&entries, "c19d0a34").is_none());
        assert_eq!(open_for(&entries, "94b1fd2e").unwrap().uuid, "e2");
    }

    /// **Answering appends rather than rewrites**, and the fold is what a reader
    /// sees. A rewrite is three chances to lose every other Job's entries in the
    /// one file whose job is to never lose anything.
    #[test]
    fn an_answer_is_a_second_line_that_the_reader_folds_in() {
        let (_home, path) = scratch();
        raised(&path, "e1", "u1", "flake", 1, "raise it?");
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
        raised(&path, "e1", "u1", "flake", 1, "went idle");
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
        raised(&path, "e1", "u1", "flake", 1, "first");
        raised(&path, "e2", "u1", "flake", 2, "second");
        raised(&path, "e3", "u2", "other", 3, "elsewhere");

        let entries = read(&path).unwrap();
        assert_eq!(open_for(&entries, "u1").unwrap().uuid, "e1");

        answer(&path, "e1", "done").unwrap();
        let entries = read(&path).unwrap();
        assert_eq!(open_for(&entries, "u1").unwrap().uuid, "e2");
    }

    #[test]
    fn a_job_with_nothing_open_has_nothing_to_answer() {
        let (_home, path) = scratch();
        raised(&path, "e1", "u1", "flake", 1, "idle");
        answer(&path, "e1", "seen").unwrap();
        let entries = read(&path).unwrap();
        assert!(open_for(&entries, "u1").is_none());
        assert!(open_for(&entries, "nonesuch").is_none());
    }

    /// An answer against an id nobody raised changes nothing rather than
    /// inventing an entry. The file is a log, and a log may mention things that
    /// were never there.
    #[test]
    fn an_answer_to_an_unknown_id_leaves_the_inbox_as_it_was() {
        let (_home, path) = scratch();
        raised(&path, "e1", "u1", "flake", 1, "idle");
        answer(&path, "nonesuch", "hello").unwrap();
        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_open());
    }

    /// **An entry does not outlive its Job.** Both of the user's Jobs reached
    /// `ABORTED` and five entries stayed open against Jobs that no longer
    /// existed — the first of the two consequences `005` records.
    ///
    /// **Marked, not deleted.** The body is still readable afterwards, because
    /// the reason a Job stopped is often the last thing written about it.
    #[test]
    fn a_jobs_entries_close_when_the_job_ends_and_are_still_readable() {
        let (_home, path) = scratch();
        raised(&path, "e1", "u1", "flake", 1, "reached its wall clock ceiling");
        raised(&path, "e2", "u1", "flake", 2, "and again");
        raised(&path, "e3", "u2", "other", 3, "someone else's");

        assert_eq!(close(&path, "u1", Closed::Ended).unwrap(), 2);

        let entries = read(&path).unwrap();
        assert!(open_for(&entries, "u1").is_none());
        assert_eq!(
            open_for(&entries, "u2").unwrap().uuid,
            "e3",
            "another Job's entries were closed with it"
        );
        assert_eq!(entries[0].closed, Some(Closed::Ended));
        assert_eq!(
            entries[0].body, "reached its wall clock ceiling",
            "a closed entry is still readable"
        );
    }

    /// Closing twice writes nothing the second time, so a `kill` retried after
    /// a failure does not double the file.
    #[test]
    fn closing_a_job_that_has_nothing_open_writes_nothing() {
        let (_home, path) = scratch();
        raised(&path, "e1", "u1", "flake", 1, "idle");
        assert_eq!(close(&path, "u1", Closed::Ended).unwrap(), 1);
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(close(&path, "u1", Closed::Ended).unwrap(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), after);
    }

    /// **A legacy entry is not open until it has an identity.** Between the
    /// upgrade and the migration it is a row nothing can answer, and showing it
    /// as open is the defect rather than a smaller version of it.
    #[test]
    fn a_legacy_entry_is_not_open_because_nothing_can_answer_it() {
        let (_home, path) = scratch();
        legacy(&path, "e1", "this-test", "reached its wall clock ceiling");
        let entries = read(&path).unwrap();
        assert_eq!(entries.len(), 1, "a legacy line still parses");
        assert_eq!(entries[0].job, "this-test");
        assert_eq!(entries[0].job_uuid, None);
        assert!(!entries[0].is_open());
        assert!(entries[0].is_legacy());
    }

    /// **A name that means exactly one Job is bound to it.** The ordinary case,
    /// and the one that must not lose anything.
    #[test]
    fn migration_binds_a_legacy_entry_whose_name_means_one_job() {
        let (_home, path) = scratch();
        legacy(&path, "e1", "nightly-flake", "wants the CI timeout raised");
        let jobs = vec![(
            "nightly-flake".to_string(),
            "c19d0a34-3069".to_string(),
            false,
        )];

        let done = migrate(&path, &jobs).unwrap();
        assert_eq!(done.bound, 1);
        assert_eq!(done.unresolvable, 0);

        let entries = read(&path).unwrap();
        assert_eq!(entries[0].job_uuid.as_deref(), Some("c19d0a34-3069"));
        assert!(entries[0].is_open(), "it is answerable now");
        assert_eq!(open_for(&entries, "c19d0a34-3069").unwrap().uuid, "e1");
    }

    /// **The migration does not guess.** This is the user's own machine: two
    /// Jobs called `this-test`, and five entries naming it. There is no honest
    /// way to say which Job asked, so each entry is closed
    /// [`Closed::Unresolvable`] rather than attached to a coin flip.
    #[test]
    fn migration_refuses_to_guess_when_a_name_means_two_jobs() {
        let (_home, path) = scratch();
        legacy(&path, "e1", "this-test", "reached its wall clock ceiling");
        let jobs = vec![
            ("this-test".to_string(), "c19d0a34".to_string(), true),
            ("this-test".to_string(), "94b1fd2e".to_string(), true),
        ];

        let done = migrate(&path, &jobs).unwrap();
        assert_eq!(done.bound, 0);
        assert_eq!(done.unresolvable, 1);

        let entries = read(&path).unwrap();
        assert_eq!(entries[0].closed, Some(Closed::Unresolvable));
        assert!(!entries[0].is_open());
        assert!(open_for(&entries, "c19d0a34").is_none());
        assert!(open_for(&entries, "94b1fd2e").is_none());
        assert_eq!(
            entries[0].body, "reached its wall clock ceiling",
            "the reason is still readable"
        );
    }

    /// A name naming no Job at all gets the same answer as an ambiguous one:
    /// there is nothing to bind it to, and nothing to answer.
    #[test]
    fn migration_closes_a_legacy_entry_whose_job_is_gone() {
        let (_home, path) = scratch();
        legacy(&path, "e1", "long-forgotten", "asked something");
        let done = migrate(&path, &[]).unwrap();
        assert_eq!(done.unresolvable, 1);
        assert_eq!(read(&path).unwrap()[0].closed, Some(Closed::Unresolvable));
    }

    /// **Binding a legacy entry to a Job that has already ended closes it in
    /// the same pass.** Otherwise the migration hands back correctly-identified
    /// entries that are all open against Jobs that are over — the symptom, not
    /// the fix.
    #[test]
    fn migration_closes_what_it_binds_to_a_job_that_has_ended() {
        let (_home, path) = scratch();
        legacy(&path, "e1", "nightly-flake", "wants the CI timeout raised");
        let jobs = vec![("nightly-flake".to_string(), "c19d0a34".to_string(), true)];

        let done = migrate(&path, &jobs).unwrap();
        assert_eq!(done.bound, 1);
        assert_eq!(done.ended, 1);

        let entries = read(&path).unwrap();
        assert_eq!(entries[0].job_uuid.as_deref(), Some("c19d0a34"));
        assert_eq!(entries[0].closed, Some(Closed::Ended));
        assert!(!entries[0].is_open());
    }

    /// **An entry a person already answered is left alone.** An answer is a
    /// decision, and it outranks anything a sweep would write over it.
    #[test]
    fn migration_leaves_an_answered_legacy_entry_answered() {
        let (_home, path) = scratch();
        legacy(&path, "e1", "nightly-flake", "wants the CI timeout raised");
        answer(&path, "e1", "yes, 90s").unwrap();
        let jobs = vec![("nightly-flake".to_string(), "c19d0a34".to_string(), true)];

        assert_eq!(migrate(&path, &jobs).unwrap().ended, 0);
        let entries = read(&path).unwrap();
        assert_eq!(entries[0].answered.as_deref(), Some("yes, 90s"));
        assert_eq!(entries[0].closed, None);
    }

    /// **Idempotent, because two readers may run it at once.** The second pass
    /// finds nothing legacy and writes nothing, so an inbox does not grow a
    /// line every time it is read.
    #[test]
    fn migrating_twice_writes_nothing_the_second_time() {
        let (_home, path) = scratch();
        legacy(&path, "e1", "nightly-flake", "one");
        legacy(&path, "e2", "this-test", "two");
        let jobs = vec![
            ("nightly-flake".to_string(), "c19d0a34".to_string(), false),
            ("this-test".to_string(), "94b1fd2e".to_string(), true),
            ("this-test".to_string(), "3d9cc7ba".to_string(), true),
        ];

        assert!(!migrate(&path, &jobs).unwrap().is_empty());
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(migrate(&path, &jobs).unwrap().is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), after);
        assert!(read(&path).unwrap().iter().all(|entry| !entry.is_legacy()));
    }

    /// **The reported word screams; the file on disk does not, and they are two
    /// different contracts.**
    ///
    /// [`Kind::word`] is what the envelope's `results[].kind` carries and what
    /// the STATUS column prints, so it obeys the one-spelling rule with every
    /// other status word. The `Serialize`/`Deserialize` pair is the inbox
    /// *file*, which is Armada's own state under `~/.armada/` and is read back
    /// by later runs — changing it would make every inbox already on a machine
    /// unreadable, to no reader's benefit, since nobody is shown it.
    ///
    /// Both halves are asserted here so the distinction is deliberate rather
    /// than something that drifted.
    #[test]
    fn the_reported_kind_screams_and_the_file_on_disk_is_unchanged() {
        for (kind, shown, stored) in [
            (Kind::NeedsHuman, "NEEDS_HUMAN", "needs_human"),
            (Kind::Blocked, "BLOCKED", "blocked"),
            (Kind::Idle, "IDLE", "idle"),
        ] {
            assert_eq!(kind.word(), shown);
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{stored}\"")
            );
            assert_eq!(
                serde_json::from_str::<Kind>(&format!("\"{stored}\"")).unwrap(),
                kind,
                "an inbox written by an earlier run stopped being readable"
            );
        }
    }

    /// The same contract for the reason an entry closed, and for the same
    /// reason: the word is read by a person, the file is read by the next run.
    #[test]
    fn the_reason_an_entry_closed_survives_a_round_trip() {
        for (why, shown, stored) in [
            (Closed::Ended, "ENDED", "ended"),
            (Closed::Unresolvable, "UNRESOLVABLE", "unresolvable"),
        ] {
            assert_eq!(why.word(), shown);
            assert_eq!(serde_json::to_string(&why).unwrap(), format!("\"{stored}\""));
            assert_eq!(
                serde_json::from_str::<Closed>(&format!("\"{stored}\"")).unwrap(),
                why
            );
        }
    }
}
