//! Armada's own failures, kept so you can come back to one.
//!
//! **Raised in use rather than planned, and `PLAN.md` has no section for it.**
//! `armada bridge` failed, blamed the install, and was wrong — and there was
//! nowhere for that to be written down. The design is here, in the module that
//! implements it, until the reserved designs have a home to move it to; what it
//! borrows from the plan is cited where it is borrowed, and nothing here claims
//! a section that does not exist.
//!
//! **Recording a failure is appending an envelope that already exists.** Every
//! failure Armada reports is an [`ArmadaError`](crate::error::ArmadaError) — a
//! class, a `where`, a message and a next action — so nothing here invents a
//! shape. It writes down the one the terminal has just printed, with the time,
//! the argv and the directory attached.
//!
//! This module is the **format** and the **fold**: what a line looks like, what
//! makes two failures the same one, and what a reader sees after a week of
//! them. Reading and writing the file is `armada_manifest::failures` — parsing a
//! string is pure and lives here, opening the file is not and does not
//! (`ARCHITECTURE.md` §1.5).
//!
//! ## What gets recorded: everything that reaches the report
//!
//! **The decision is to record every failure, and not to filter on class.** Two
//! arguments settled it, and the second is the one that matters.
//!
//! A class filter would have discarded the report that prompted this. `armada
//! bridge` failed with `class: environment` and `where:` the binary's path,
//! blaming the install — and the real fault was Armada mishandling a missing
//! worktree. **A wrong class is itself a symptom**, so a filter that trusts the
//! class throws away exactly the failures worth keeping, and it throws them away
//! at the one moment they cannot be recovered.
//!
//! And the site does the filtering that a class never could. The recorder sits
//! where the binary reports an `ArmadaError` and exits by its class — the path
//! taken when **Armada could not answer**. A check whose tests fail is an
//! *answer*: it comes back as an envelope with `FAILED` in it, exits 1, and is
//! never seen here. So "a failing test suite fills the log" cannot happen, and
//! it cannot happen structurally rather than by a rule somebody has to maintain.
//!
//! What that does admit is the mistyped verb — `bad_invocation` is a person
//! being human, not Armada being broken. That is the price, and [dedup](fold)
//! is what makes it affordable: four typos are one row saying `×4`, and one
//! `armada failures clear` is the end of it. Deciding at write time is
//! irreversible; deciding at read time is one keystroke.
//!
//! ## Privacy: no absolute `$HOME` is ever written
//!
//! Recorded failures carry paths, and this repository is public permanently.
//! Every string that reaches the file goes through [`tilde`] first, so
//! `/home/you/.cargo/bin/armada` is stored as `~/.cargo/bin/armada` and the log
//! has no absolute home path in it to leak — not into a fixture, not into a
//! `--json` payload, and not into the task text of a Job promoted out of it.

use crate::dispatch::Scrub;
use crate::error::{ArmadaError, ErrClass};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Hex characters kept from the digest. Eight, the length both derived
/// identities use (PLAN.md §2.2), for the same reason: it is short enough to
/// retype into `armada failures show` and long enough that two failures on one
/// machine do not collide. A collision merges two rows, which is the mild
/// failure of the two available.
const ID_LEN: usize = 8;

/// Where an entry stands with you.
///
/// **Three words rather than a tick**, and the reason is PLAN.md §15.3.1's:
/// *done*, *not doing it* and *being worked on* are different answers, and a
/// list that collapses them stops being trustworthy after the first row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Recorded, and nothing has been done about it.
    Open,
    /// A Job is on it — `armada failures fix` spawned one.
    Fixing,
    /// Discarded. Hidden unless asked for, and a recurrence reopens it.
    Cleared,
}

impl State {
    /// The word the STATUS column prints, in both audiences.
    pub const fn word(self) -> &'static str {
        match self {
            State::Open => "OPEN",
            State::Fixing => "FIXING",
            State::Cleared => "CLEARED",
        }
    }
}

/// One line of the log.
///
/// **Append-only, three shapes**, exactly as the inbox is: a failure happened, a
/// Job was spawned for one, an entry was discarded. Nothing rewrites a line —
/// which is what makes this survive the crash that produced the entry
/// (PLAN.md §15.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Line {
    /// A failure was reported to somebody.
    Failed {
        /// The fingerprint — the same failure always produces the same id.
        id: String,
        /// When, wall clock, RFC 3339.
        at: String,
        /// Wall clock milliseconds, so "9m ago" is a subtraction.
        at_ms: u64,
        /// The class Armada assigned. **Recorded, not trusted** — a wrong class
        /// is a symptom, and the whole triage question is whether this one is
        /// right.
        class: ErrClass,
        /// The `where` from the envelope.
        r#where: String,
        /// The one-line message.
        message: String,
        /// What Armada said to do next, if it knew.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        next: Option<String>,
        /// The command that was typed, with `$HOME` abbreviated.
        argv: String,
        /// Where it was typed, with `$HOME` abbreviated.
        cwd: String,
    },
    /// A Job was spawned to fix this entry.
    Promoted {
        /// Which entry.
        id: String,
        /// Wall clock milliseconds.
        at_ms: u64,
        /// The Job's handle.
        job: String,
    },
    /// An entry was discarded.
    Cleared {
        /// Which entry.
        id: String,
        /// Wall clock milliseconds.
        at_ms: u64,
    },
}

/// One failure, folded from every line that mentions it.
///
/// **The count and the last-seen are what make this observability rather than a
/// log.** The same failure eight times is a different fact from eight failures
/// once each, and a list that does not collapse repeats is unreadable after a
/// day of real use.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// The fingerprint, and the handle every verb takes.
    pub id: String,
    /// Where it stands with you.
    #[serde(serialize_with = "state_word")]
    pub state: State,
    /// The class Armada assigned, most recently.
    pub class: ErrClass,
    /// The `where`.
    pub r#where: String,
    /// The message.
    pub message: String,
    /// What Armada said to do next.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// The command that was typed, most recently.
    pub argv: String,
    /// Where it was typed, most recently.
    pub cwd: String,
    /// How many times this failure has happened since it was last cleared.
    pub count: usize,
    /// The first of those, RFC 3339.
    pub first_at: String,
    /// The most recent, RFC 3339.
    pub last_at: String,
    /// The most recent, wall clock milliseconds — the sort key.
    pub last_ms: u64,
    /// How long ago that was, in seconds.
    ///
    /// **Filled by the reader, not by the fold.** "Nine minutes ago" is a
    /// subtraction against a clock, and this module is pure — so [`fold`] leaves
    /// it zero and [`age`] is what a verb holding a clock calls. It is in the
    /// payload rather than left to the renderer because `--json` wants the same
    /// answer the table gives (PLAN.md §3.1.1).
    pub age_s: u64,
    /// The Job spawned for it, if one was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
}

fn state_word<S: serde::Serializer>(state: &State, out: S) -> Result<S::Ok, S::Error> {
    out.serialize_str(state.word())
}

/// The line to append for a failure that has just been reported.
///
/// Returns the id it was given, so the caller can say it. **`home` is required
/// rather than optional** because it is what [`tilde`] needs, and a log written
/// without it would be the one thing this module promises never to contain.
pub fn failed(
    error: &ArmadaError,
    home: &Path,
    cwd: &Path,
    argv: &[String],
    at: &str,
    at_ms: u64,
) -> (String, Line) {
    let r#where = tilde(&error.r#where, home);
    let message = tilde(&error.message, home);
    let id = fingerprint(error.class, &r#where, &message);
    (
        id.clone(),
        Line::Failed {
            id,
            at: at.to_string(),
            at_ms,
            class: error.class,
            r#where,
            message,
            next: error.next_action.as_deref().map(|next| tilde(next, home)),
            argv: tilde(&format!("armada {}", argv.join(" ")), home),
            cwd: tilde(&cwd.display().to_string(), home),
        },
    )
}

/// **What counts as "the same" failure**: the class, the `where` and a
/// normalised message.
///
/// The message is normalised because two runs of one bug differ in the things
/// that are about the *run* rather than the bug — a pid, a duration, somebody
/// else's absolute path. [`Scrub`] is the same normaliser `armada manifest
/// explain`'s failure signature uses (PLAN.md §3.4), and reusing it is
/// deliberate: "is this the same failure" is one question and it should not have
/// two answers in one binary.
///
/// **The class is in the fingerprint even though a wrong class is a symptom.**
/// Two failures Armada classified differently are two things to look at, and
/// collapsing them would hide the reclassification — which is itself the fix
/// landing.
///
/// **The directory is not in it.** The same bug in two repositories is one bug
/// with more evidence, not two; the entry carries the most recent `cwd` so the
/// Job that fixes it still has somewhere to start.
pub fn fingerprint(class: ErrClass, r#where: &str, message: &str) -> String {
    let scrub = Scrub::anywhere();
    let subject = format!(
        "{class}|{}|{}",
        scrub.normalise(r#where),
        scrub.normalise(message)
    );
    let mut hex = blake3::hash(subject.as_bytes()).to_hex().to_string();
    hex.truncate(ID_LEN);
    hex
}

/// A path as a person writes it, wherever it appears in a string.
///
/// **A substring replacement rather than a path operation**, because the strings
/// this runs over are prose: `could not read /home/you/.armada/guild/voice.md`
/// has a path in the middle of a sentence, and only the sentence is available.
///
/// **The empty home is left alone.** `$HOME` unset is one of the failures being
/// recorded, and replacing every empty string with `~` would rewrite the whole
/// message into tildes.
pub fn tilde(text: &str, home: &Path) -> String {
    let home = home.display().to_string();
    if home.is_empty() || home == "/" {
        return text.to_string();
    }
    text.replace(&home, "~")
}

/// Every entry, folded from the file, **most recently seen first**.
///
/// A line that does not parse is skipped: a file that is only appended to can
/// still end mid-write if the machine lost power, and one torn last line must
/// not hide the entries before it — the same rule the inbox follows.
///
/// **Clearing resets rather than deletes.** A cleared entry keeps its id and its
/// place in the file; a later occurrence of the same failure reopens it with a
/// count of one, because a bug you dismissed and that came back is news.
pub fn fold(text: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<Line>(line) else {
            continue;
        };
        match parsed {
            Line::Failed {
                id,
                at,
                at_ms,
                class,
                r#where,
                message,
                next,
                argv,
                cwd,
            } => match entries.iter_mut().find(|entry| entry.id == id) {
                Some(entry) => {
                    // A recurrence after a clear is a new sighting of an old
                    // bug: the count starts again, and the row comes back.
                    if entry.state == State::Cleared {
                        entry.state = match entry.job {
                            Some(_) => State::Fixing,
                            None => State::Open,
                        };
                        entry.count = 0;
                        entry.first_at = at.clone();
                    }
                    entry.count += 1;
                    entry.class = class;
                    entry.r#where = r#where;
                    entry.message = message;
                    entry.next = next;
                    entry.argv = argv;
                    entry.cwd = cwd;
                    entry.last_at = at;
                    entry.last_ms = at_ms;
                }
                None => entries.push(Entry {
                    id,
                    state: State::Open,
                    class,
                    r#where,
                    message,
                    next,
                    argv,
                    cwd,
                    count: 1,
                    first_at: at.clone(),
                    last_at: at,
                    last_ms: at_ms,
                    age_s: 0,
                    job: None,
                }),
            },
            // **A line about an id nobody recorded changes nothing.** The file
            // is a log, and a log may mention things that were never there.
            Line::Promoted { id, job, .. } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
                    entry.job = Some(job);
                    if entry.state == State::Open {
                        entry.state = State::Fixing;
                    }
                }
            }
            Line::Cleared { id, .. } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
                    entry.state = State::Cleared;
                }
            }
        }
    }
    // **Most recent first**, because the failure you are looking for is almost
    // always the one that just happened. Stable, so two entries last seen in
    // the same millisecond keep the order the file put them in.
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.last_ms));
    entries
}

/// Fill in how long ago each entry was last seen.
///
/// **Saturating, because a clock can go backwards.** A machine whose time was
/// corrected between the failure and the reading would otherwise underflow into
/// an age of half a billion years, and the row would be unreadable rather than
/// slightly wrong.
pub fn age(entries: &mut [Entry], now_ms: u64) {
    for entry in entries {
        entry.age_s = now_ms.saturating_sub(entry.last_ms) / 1_000;
    }
}

/// The prompt a Job gets when an entry is promoted.
///
/// **The recorded failure is the task, verbatim.** The point of the log is that
/// he does not have to come back and describe the bug; a promotion that made him
/// write a sentence would have spent the record and asked for it anyway.
///
/// Every string in it has already been through [`tilde`], so a Job's transcript
/// — which leaves this machine — cannot carry an absolute home path out of the
/// log.
pub fn task(entry: &Entry) -> String {
    let mut out = format!(
        "Armada failed with this, and the failure was recorded as `{}`:\n\n\
         \x20   class: {}\n\
         \x20   where: {}\n\
         \x20   error: {}\n",
        entry.id, entry.class, entry.r#where, entry.message
    );
    if let Some(next) = &entry.next {
        out.push_str(&format!("    next:  {next}\n"));
    }
    out.push_str(&format!(
        "\nIt was reported by `{}`, run in {}, and has happened {}.\n\n\
         Reproduce it, then fix it. The class above is what Armada assigned and \
         may itself be wrong: a failure attributed to the environment that turns \
         out to be Armada's own is the bug, and the misattribution is part of it.",
        entry.argv,
        entry.cwd,
        match entry.count {
            1 => "once".to_string(),
            n => format!("{n} times, most recently at {}", entry.last_at),
        }
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(class: ErrClass, r#where: &str, message: &str) -> ArmadaError {
        ArmadaError {
            class,
            r#where: r#where.to_string(),
            message: message.to_string(),
            next_action: None,
        }
    }

    fn record(text: &mut String, error: &ArmadaError, at_ms: u64) -> String {
        let (id, line) = failed(
            error,
            Path::new("/scratch/home"),
            Path::new("/scratch/home/code/api"),
            &["bridge".to_string()],
            "2026-08-14T09:00:00Z",
            at_ms,
        );
        text.push_str(&serde_json::to_string(&line).unwrap());
        text.push('\n');
        id
    }

    /// **The failure that prompted this feature**, and the two things it has to
    /// prove: the absolute home path never reaches the file, and the class it
    /// was given is recorded rather than believed.
    #[test]
    fn the_home_path_is_abbreviated_before_anything_is_written() {
        let mut text = String::new();
        record(
            &mut text,
            &ArmadaError {
                class: ErrClass::Environment,
                r#where: "/scratch/home/.cargo/bin/armada".to_string(),
                message: "`armada manifest clean` could not be found to run".to_string(),
                next_action: Some("reinstall armada, then retry unchanged".to_string()),
            },
            1_000,
        );
        assert!(
            !text.contains("/scratch/home"),
            "an absolute $HOME reached the log:\n{text}"
        );
        assert!(text.contains("~/.cargo/bin/armada"), "{text}");

        let folded = fold(&text);
        assert_eq!(folded[0].class, ErrClass::Environment);
        assert_eq!(folded[0].r#where, "~/.cargo/bin/armada");
        assert_eq!(folded[0].cwd, "~/code/api");
    }

    /// **The same failure eight times is one row.** A list that does not
    /// collapse repeats is unreadable after a day of use.
    #[test]
    fn the_same_failure_twice_is_one_entry_with_a_count() {
        let mut text = String::new();
        let broken = error(ErrClass::Environment, "~/bin/armada", "could not be found");
        let first = record(&mut text, &broken, 1_000);
        let second = record(&mut text, &broken, 5_000);
        assert_eq!(first, second, "one failure, one fingerprint");

        let folded = fold(&text);
        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].count, 2);
        assert_eq!(folded[0].last_ms, 5_000);
    }

    /// Two runs of one bug differ in the things that are about the run. The
    /// signature normaliser already knows which those are.
    #[test]
    fn a_pid_and_a_duration_do_not_make_two_failures_out_of_one() {
        let a = fingerprint(
            ErrClass::ToolFailed,
            "api:test",
            "the child died pid=4021 after 1.2s",
        );
        let b = fingerprint(
            ErrClass::ToolFailed,
            "api:test",
            "the child died pid=9773 after 4.8s",
        );
        assert_eq!(a, b);
    }

    /// And two *different* failures must not collapse. A fingerprint that
    /// collides is worse than one that is too specific — a reader told "same as
    /// last time" stops reading.
    #[test]
    fn two_different_failures_keep_two_ids() {
        let a = fingerprint(ErrClass::BadConfig, "armada.yml:12:7", "expected a mapping");
        let b = fingerprint(ErrClass::BadConfig, "armada.yml:31:2", "expected a mapping");
        let c = fingerprint(ErrClass::ArmadaBug, "armada.yml:12:7", "expected a mapping");
        assert_ne!(a, b, "two locations are two failures");
        assert_ne!(a, c, "a reclassification is not the same entry");
    }

    /// **A cleared entry that happens again is news.** The count starts over so
    /// the row says what has happened since you dismissed it, not since the
    /// beginning of time.
    #[test]
    fn clearing_resets_and_a_recurrence_reopens() {
        let mut text = String::new();
        let broken = error(ErrClass::ArmadaBug, "spawn", "the worktree was not there");
        let id = record(&mut text, &broken, 1_000);
        record(&mut text, &broken, 2_000);
        text.push_str(
            &serde_json::to_string(&Line::Cleared {
                id: id.clone(),
                at_ms: 3_000,
            })
            .unwrap(),
        );
        text.push('\n');

        let cleared = fold(&text);
        assert_eq!(cleared[0].state, State::Cleared);
        assert_eq!(cleared[0].count, 2);

        record(&mut text, &broken, 4_000);
        let again = fold(&text);
        assert_eq!(again.len(), 1, "one id, however many times it came back");
        assert_eq!(again[0].state, State::Open);
        assert_eq!(again[0].count, 1, "the count is since the clear");
    }

    /// Promotion is the third line shape, and it is what puts a Job's name on
    /// the row (PLAN.md §15.3.2's "links the two").
    #[test]
    fn a_promoted_entry_names_the_job_and_says_fixing() {
        let mut text = String::new();
        let id = record(
            &mut text,
            &error(ErrClass::ArmadaBug, "spawn", "no worktree"),
            1_000,
        );
        text.push_str(
            &serde_json::to_string(&Line::Promoted {
                id,
                at_ms: 2_000,
                job: "fix-a1b2c3d4".to_string(),
            })
            .unwrap(),
        );
        text.push('\n');

        let folded = fold(&text);
        assert_eq!(folded[0].state, State::Fixing);
        assert_eq!(folded[0].job.as_deref(), Some("fix-a1b2c3d4"));
        assert_eq!(State::Fixing.word(), "FIXING");
    }

    /// A machine that lost power leaves a half-written last line. The entries
    /// before it are still real.
    #[test]
    fn a_torn_last_line_does_not_hide_the_entries_before_it() {
        let mut text = String::new();
        record(
            &mut text,
            &error(ErrClass::Timeout, "api:test", "deadline"),
            1_000,
        );
        text.push_str("{\"type\":\"fai");
        assert_eq!(fold(&text).len(), 1);
    }

    /// **Most recent first.** The failure being looked for is almost always the
    /// one that just happened.
    #[test]
    fn the_listing_leads_with_what_just_happened() {
        let mut text = String::new();
        record(&mut text, &error(ErrClass::Timeout, "a", "old"), 1_000);
        record(&mut text, &error(ErrClass::Timeout, "b", "new"), 9_000);
        let folded = fold(&text);
        assert_eq!(folded[0].message, "new");
        assert_eq!(folded[1].message, "old");
    }

    /// An entry a person never recorded is not invented by a line that mentions
    /// it.
    #[test]
    fn clearing_an_unknown_id_leaves_the_log_as_it_was() {
        let mut text = String::new();
        record(&mut text, &error(ErrClass::Aborted, "a", "stopped"), 1_000);
        text.push_str(
            &serde_json::to_string(&Line::Cleared {
                id: "nonesuch".to_string(),
                at_ms: 2_000,
            })
            .unwrap(),
        );
        text.push('\n');
        let folded = fold(&text);
        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].state, State::Open);
    }

    /// The task a Job is given carries the whole failure and no absolute home
    /// path, because the entry it is built from carries neither.
    #[test]
    fn the_promoted_task_is_the_recorded_failure_and_nothing_of_the_machine() {
        let mut text = String::new();
        record(
            &mut text,
            &ArmadaError {
                class: ErrClass::Environment,
                r#where: "/scratch/home/.cargo/bin/armada".to_string(),
                message: "`armada manifest clean` could not be found to run".to_string(),
                next_action: Some("reinstall armada, then retry unchanged".to_string()),
            },
            1_000,
        );
        let task = task(&fold(&text)[0]);
        assert!(task.contains("could not be found to run"), "{task}");
        assert!(task.contains("~/.cargo/bin/armada"), "{task}");
        assert!(task.contains("reinstall armada"), "{task}");
        assert!(!task.contains("/scratch/home"), "{task}");
        assert!(
            task.contains("may itself be wrong"),
            "the class is handed over as a claim, not a diagnosis:\n{task}"
        );
    }

    /// The word the STATUS column prints obeys the one-spelling rule every
    /// other status word does (PLAN.md §3).
    #[test]
    fn every_state_has_one_screaming_spelling() {
        for state in [State::Open, State::Fixing, State::Cleared] {
            assert_eq!(state.word(), state.word().to_uppercase());
        }
    }
}
